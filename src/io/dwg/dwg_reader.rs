//! DWG file reader
//!
//! Reads DWG binary files and extracts structural information including
//! file headers, section metadata, and integrity checksums (CRC values).
//!
//! ## AC1021 (R2007) CRC-64 Extraction
//!
//! The AC1021 format stores a 64-bit CRC in the compressed metadata header.
//! This reader extracts and reports all CRC values, including:
//! - **Header CRC-64**: The master integrity checksum at offset 0x108
//! - **Pages Map CRC**: Checksums for the page directory
//! - **Sections Map CRC**: Checksums for the section directory
//! - **Per-page CRC**: Individual page checksums (in section map entries)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use acadrust::io::dwg::dwg_reader::DwgReader;
//!
//! let reader = DwgReader::from_file("drawing.dwg")?;
//! let info = reader.read_file_header()?;
//!
//! // Access CRC-64 from AC1021 files
//! if let Some(metadata) = &info.ac21_metadata {
//!     println!("Header CRC-64: {:#018X}", metadata.header_crc64);
//! }
//! ```

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};

use crate::error::DxfError;
use crate::io::dwg::checksum::{apply_magic_sequence, apply_mask};
use crate::io::dwg::decompressor_ac18::decompress_ac18;
use crate::io::dwg::decompressor_ac21::decompress_ac21;
use crate::io::dwg::dwg21_metadata::Dwg21CompressedMetadata;
use crate::io::dwg::dwg_version::DwgVersion;
use crate::io::dwg::parallel::{map_ordered, worker_count};
use crate::io::dwg::reed_solomon::reed_solomon_decode;
use crate::io::read::{push_read_diagnostic, ReadDiagnostic, ReadStage, SourceFormat};
use crate::notification::{NotificationCollection, NotificationType};

fn report_read_error(
    notifications: &mut NotificationCollection,
    diagnostics: &mut Vec<ReadDiagnostic>,
    code: &str,
    stage: ReadStage,
    section: Option<&str>,
    message: String,
) {
    notifications.notify(NotificationType::Error, message.clone());
    let mut diagnostic = ReadDiagnostic::new(code, stage, message);
    diagnostic.section = section.map(str::to_owned);
    push_read_diagnostic(diagnostics, diagnostic);
}

fn parse_template(data: &[u8], version: crate::types::DxfVersion) -> Result<i16, DxfError> {
    let length_bytes = data
        .get(..2)
        .ok_or_else(|| DxfError::InvalidFormat("Truncated Template length".into()))?;
    let description_len = i16::from_le_bytes(length_bytes.try_into().unwrap());
    if description_len < 0 {
        return Err(DxfError::InvalidFormat(
            "Negative Template description length".into(),
        ));
    }

    let character_width = if version >= crate::types::DxfVersion::AC1021 {
        2
    } else {
        1
    };
    let description_bytes = (description_len as usize)
        .checked_mul(character_width)
        .ok_or_else(|| DxfError::InvalidFormat("Invalid Template length".into()))?;
    let measurement_offset = 2usize
        .checked_add(description_bytes)
        .ok_or_else(|| DxfError::InvalidFormat("Invalid Template length".into()))?;
    let measurement_bytes = data
        .get(measurement_offset..measurement_offset + 2)
        .ok_or_else(|| DxfError::InvalidFormat("Truncated Template payload".into()))?;
    let measurement = i16::from_le_bytes(measurement_bytes.try_into().unwrap());
    if !matches!(measurement, 0 | 1) {
        return Err(DxfError::InvalidFormat(format!(
            "Invalid MEASUREMENT value: {measurement}"
        )));
    }
    Ok(measurement)
}

#[cfg(test)]
mod template_tests {
    use super::parse_template;
    use crate::types::DxfVersion;

    #[test]
    fn parses_nonempty_description_and_rejects_malformed_payloads() {
        assert_eq!(
            parse_template(&[3, 0, b'a', b'b', b'c', 1, 0], DxfVersion::AC1018).unwrap(),
            1
        );
        assert_eq!(
            parse_template(&[1, 0, 0, 0, 1, 0], DxfVersion::AC1032).unwrap(),
            1
        );
        assert!(parse_template(&[], DxfVersion::AC1032).is_err());
        assert!(parse_template(&[0xFF, 0xFF, 0, 0], DxfVersion::AC1032).is_err());
        assert!(parse_template(&[3, 0, b'a', b'b', b'c'], DxfVersion::AC1018).is_err());
        assert!(parse_template(&[0, 0, 2, 0], DxfVersion::AC1032).is_err());
    }
}

/// AC1021 file header offset (data pages start after this)
const AC21_FILE_HEADER_SIZE: u64 = 0x480;

fn ac21_page_layout(
    compressed_size: u64,
    correction_factor: u64,
    block_size: usize,
) -> Result<(usize, usize, usize), DxfError> {
    let aligned = compressed_size.wrapping_add(7) & 0xFFFF_FFF8;
    let total_size = aligned.wrapping_mul(correction_factor) as usize;
    if total_size == 0 || total_size > 100_000_000 {
        return Err(DxfError::InvalidFormat(format!(
            "Invalid page buffer size: {} (compressed={}, factor={}, aligned={})",
            total_size, compressed_size, correction_factor, aligned
        )));
    }
    let factor = (total_size + block_size - 1) / block_size;
    Ok((total_size, factor, factor * 255))
}

fn decode_ac21_page(
    encoded: &[u8],
    total_size: usize,
    factor: usize,
    block_size: usize,
    compressed_size: u64,
    uncompressed_size: u64,
) -> Vec<u8> {
    let mut compressed_data = vec![0u8; total_size];
    reed_solomon_decode(encoded, &mut compressed_data, factor, block_size);
    if compressed_size == uncompressed_size {
        compressed_data.truncate(uncompressed_size as usize);
        return compressed_data;
    }

    let mut padded_source = vec![0u8; compressed_data.len() + 64];
    padded_source[..compressed_data.len()].copy_from_slice(&compressed_data);
    let mut decompressed_data = vec![0u8; uncompressed_size as usize + 64];
    decompress_ac21(
        &padded_source,
        0,
        compressed_size as u32,
        &mut decompressed_data,
    );
    decompressed_data.truncate(uncompressed_size as usize);
    decompressed_data
}

/// Results from reading a DWG file header.
///
/// Contains version info, section layout, and all extracted CRC values.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DwgFileHeaderInfo {
    /// DWG version string (e.g., "AC1021")
    pub version_string: String,
    /// Parsed DWG version enum
    pub version: DwgVersion,
    /// AutoCAD maintenance version
    pub acad_maintenance_version: u8,
    /// Preview image address
    pub preview_address: i32,
    /// DWG internal version byte
    pub dwg_version: u8,
    /// Application release version
    pub app_release_version: u8,
    /// Drawing code page
    pub code_page: u16,
    /// Security type
    pub security_type: i32,
    /// Summary info address
    pub summary_info_addr: i32,
    /// VBA project address
    pub vba_project_addr: i32,

    // ── gold's FILEHEADER fields retained for the structure axis (§19 H2).
    // These bytes were all parsed at their correct offsets before — but
    // skipped or mislabeled: `zero_one_or_three` was "the unknown byte",
    // the R2000 dwg_version/maint_version pair was "magic 0x1B/0x19",
    // and the R2004+ tail (unknown_0/app_dwg/app_maint, rl_1c_address,
    // r2004_header_address) was skipped wholesale. Byte layout facts
    // pinned by hand-decoding sample_2000/2018 against gold's observed
    // FILEHEADER values (the spec's field ORDER is misleading; the
    // @0x0d comment and gold's JSON carry the truth):
    // [0..6] version, [6..11] 5 zero bytes, [11] maint_rel_version,
    // [12] zero_one_or_three, [13..17] thumbnail_address, [17] dwg_version,
    // [18] maint_version, [19..21] codepage, then R2000: [21..25] sections
    // (the locator-record count); R2004+: [21] unknown_0, [22] app_dwg,
    // [23] app_maint, [24..28] security, [28..32] rl_1c_address,
    // [32..36] summaryinfo, [36..40] vbaproj, [40..44] r2004_header addr.
    /// Gold's `zero_one_or_three` (byte 12)
    pub zero_one_or_three: u8,
    /// Gold's `maint_version` (byte 18 — the same byte historically read
    /// into `app_release_version`; both stay set from the one read)
    pub maint_version: u8,
    /// Gold's `sections` (R2000: the section-locator record count)
    pub sections: i32,
    /// Gold's R2004+ `unknown_0` (byte 21)
    pub unknown_0: u8,
    /// Gold's R2004+ `app_dwg_version` (byte 22)
    pub app_dwg_version: u8,
    /// Gold's R2004+ `app_maint_version` (byte 23)
    pub app_maint_version: u8,
    /// Gold's R2004+ `rl_1c_address` (mostly 0)
    pub rl_1c_address: i32,
    /// Gold's R2004+ `r2004_header_address` (mostly 128/0x80)
    pub r2004_header_address: i32,
    /// The unmasked 120-byte system section, gold's `R2004_Header` shape
    /// (None on AC21-format files — gold's separate R2007_Header row —
    /// and pre-R2004 formats). §19 H2.
    pub r2004_system: Option<crate::document::DwgR2004SystemHeader>,
    /// The sentinel-located R13–R2000 second header, gold's
    /// `SecondHeader` shape (§19 H2). None when the sentinel is not
    /// found or on R2004+ formats.
    pub second_header: Option<crate::document::DwgSecondHeaderSummary>,
    /// The R13c3+ AuxHeader at the section locator, gold's `AuxHeader`
    /// shape (§19 H2). None when the FILEHEADER carries fewer than 6
    /// section records.
    pub aux_header: Option<crate::document::DwgAuxHeaderSummary>,

    // ── AC1021-specific data ──
    /// AC1021 compressed metadata (contains CRC-64 and section layout)
    pub ac21_metadata: Option<Dwg21CompressedMetadata>,
    /// Raw Reed-Solomon decoded values from file header
    pub ac21_header_crc: Option<i64>,
    /// Unknown key from AC1021 header
    pub ac21_unknown_key: Option<i64>,
    /// CRC of compressed data in AC1021 header
    pub ac21_compressed_data_crc: Option<i64>,
    /// Page records: page_id → (offset, size)
    pub page_records: HashMap<i32, (i64, i64)>,
    /// Section descriptors from the section map
    pub section_descriptors: Vec<DwgSectionInfo>,

    // ── AC15-specific data ──
    /// Section locator records for AC15 format: name → (file_offset, size)
    pub section_locators: HashMap<String, (i64, i64)>,
    /// Base file offset of AcDb:AcDbObjects section (AC15 only).
    /// Handle offsets in AC15 are absolute; subtract this to get buffer-relative.
    pub objects_base_offset: i64,
    /// Whether this file uses AC18 format (R2004/R2010/R2013/R2018).
    /// Determines which decompression path to use in `get_section_buffer`.
    pub is_ac18_format: bool,

    // ── The §19 H7g container-shape retention (R2004 family) ──
    /// The section page map entries in the author's physical order
    /// (page id + on-disk size), from `read_page_map_ac18`.
    pub ac18_map_order: Vec<crate::document::DwgAc18PageEntry>,
    /// The per-descriptor page shapes in the author's table order,
    /// from `read_section_map_ac18` (every descriptor, including the
    /// unnamed AcDs ones).
    pub ac18_section_shapes: Vec<crate::document::DwgAc18SectionShape>,

    // ── The §19 H8 container-shape retention (AC1021 family) ──
    /// The pages-map entries in the author's physical order, from
    /// `read_page_map_ac21`.
    pub ac21_map_order: Vec<crate::document::DwgAc21PageEntry>,
    /// The composed AC21 container shape (built from the page-map
    /// order and the section descriptors once both AC21 maps parse).
    pub ac21_shape: Option<crate::document::DwgAc21ContainerShape>,
}

/// Information about a DWG section (from the section map).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DwgSectionInfo {
    /// Section name (e.g., "AcDb:Header")
    pub name: String,
    /// Compressed size
    pub compressed_size: u64,
    /// Decompressed size
    pub decompressed_size: u64,
    /// Encryption flag
    pub encrypted: u64,
    /// Hash code
    pub hash_code: u64,
    /// Encoding type (4 = Reed-Solomon + LZ77)
    pub encoding: u64,
    /// Number of pages
    pub page_count: u64,
    /// Per-page CRC values and metadata
    pub pages: Vec<DwgPageCrcInfo>,
}

/// CRC information for a single page within a section.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DwgPageCrcInfo {
    /// Page number
    pub page_number: i64,
    /// Page offset within section
    pub offset: u64,
    /// Page size
    pub size: i64,
    /// Decompressed size
    pub decompressed_size: u64,
    /// Compressed size
    pub compressed_size: u64,
    /// Checksum value
    pub checksum: u64,
    /// **CRC value for this page**
    pub crc: u64,
}

/// Options controlling how DWG files are read.
///
/// When `failsafe` is enabled the reader will attempt to recover as
/// much data as possible from damaged or partially-corrupt files
/// instead of returning an error.  Specific behaviours:
///
/// * File-header parsing errors are caught and an empty/partial
///   result is returned rather than propagating the error.
/// * Missing pages in a section are skipped instead of aborting.
/// * Skipped records and sections are reported through the
///   [`CadDocument::notifications`](crate::document::CadDocument::notifications) collection.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DwgReadOptions {
    /// When `true`, recover as much data as possible from corrupt files.
    pub failsafe: bool,
}

impl Default for DwgReadOptions {
    fn default() -> Self {
        Self { failsafe: false }
    }
}

impl DwgReadOptions {
    /// Create options with failsafe mode enabled.
    pub fn failsafe() -> Self {
        Self { failsafe: true }
    }
}

/// Find the first occurrence of `needle` in `haystack` at or after `from`.
fn find_subsequence(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    (from..=haystack.len() - needle.len()).find(|&i| &haystack[i..i + needle.len()] == needle)
}

/// First index `>= from` where a SAB blob header magic begins — either
/// `"ACIS BinaryFile"` or `"ASM BinaryFile"`, whichever comes first. A single
/// forward scan over `buf` (both magics start with `b'A'`, used as a cheap
/// first-byte gate), so callers that advance `from` past each match walk the
/// buffer once in total rather than re-scanning for each magic separately.
fn find_acds_magic(buf: &[u8], from: usize) -> Option<usize> {
    const ACIS_MAGIC: &[u8] = b"ACIS BinaryFile";
    const ASM_MAGIC: &[u8] = b"ASM BinaryFile";
    let mut i = from;
    while i < buf.len() {
        if buf[i] == b'A' {
            let rest = &buf[i..];
            if rest.starts_with(ACIS_MAGIC) || rest.starts_with(ASM_MAGIC) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// End-of-body terminator for an Autodesk ShapeManager (ASM) SAB blob, written
/// as separate tagged tokens: `0E 03 "End" 0E 02 "of" 0E 03 "ASM" 0D 04 "data"`.
/// This is what AutoCAD 2013+ / BricsCAD emit.
const ASM_END_MARKER: &[u8] = b"\x0E\x03End\x0E\x02of\x0E\x03ASM\x0D\x04data";
/// End-of-body terminator for a classic ACIS SAB blob, written as one tagged
/// identifier string. This is what acadrust's own `SabWriter` emits, so the
/// reader must recognise it to round-trip natively-built solids (primitives and
/// the exact planar/NURBS export), not just ASM bodies read from other apps.
const ACIS_END_MARKER: &[u8] = b"End-of-ACIS-data";
/// Native ACIS may split the terminator into tagged identifier components.
const ACIS_TAGGED_END_MARKER: &[u8] = b"\x0e\x03End\x0e\x02of\x0e\x04ACIS\x0d\x04data";

/// First end-marker (ASM or classic ACIS) in `buf[from..to]`, with its length so
/// the caller can advance past the whole terminator.
///
/// `to` bounds the search: a SAB body carries only ONE of the two end-marker
/// families, so the search for the absent one would otherwise run to the end of
/// the whole (often hundreds-of-MB) AcDs buffer on every call. With thousands of
/// records that is O(records × buf) — the dominant cost of opening a 3D-heavy
/// DWG. Callers pass the tightest known upper bound (segment end, next blob, or
/// the pre-table region) so a missing marker costs one bounded scan, not a full
/// one. (#203)
fn find_acds_end(buf: &[u8], from: usize, to: usize) -> Option<(usize, usize)> {
    let hay = &buf[..to.min(buf.len())];
    [ASM_END_MARKER, ACIS_END_MARKER, ACIS_TAGGED_END_MARKER]
        .into_iter()
        .filter_map(|marker| find_subsequence(hay, marker, from).map(|e| (e, marker.len())))
        .min_by_key(|(offset, _)| *offset)
}

/// Each AcDs SAB blob paired with its owning entity handle, read from the
/// `_data_` record table(s) — authoritative regardless of blob / record / handle
/// ordering (that ordering diverges in some BIM exports, so guessing it gave one
/// solid another solid's geometry and body transform).
///
/// A record-table `_data_` segment's content (after the 48-byte header) opens
/// with a `col0 = 0x14` entry; its id is 2 in small files but a high index in
/// large multi-segment datastores, so match on that content marker, not the id,
/// and process every such segment. Each 20-byte entry is
/// `col0(0x14) idx handle data_offset`, where `data_offset` is the cumulative
/// start of the record's blob in the blob data that follows the table. Record r's
/// blob therefore lives in `[base + off[r] .. base + off[r+1]]`; locate the SAB
/// header there and read through its end marker (which can run a few bytes past
/// `off[r+1]`). Records with an empty range carry no geometry and are skipped.
/// Empty when there is no such table (the caller then falls back to order-based
/// attachment).
fn extract_acds_record_blobs(buf: &[u8], modeler_handles: &HashSet<u64>) -> Vec<(u64, Vec<u8>)> {
    let rd = |p: usize| -> Option<u32> {
        buf.get(p..p + 4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
    };
    let marker = [0xACu8, 0xD5, 0x5F, 0x64, 0x61, 0x74, 0x61, 0x5F]; // "\xAC\xD5_data_"
    let mut out: Vec<(u64, Vec<u8>)> = Vec::new();
    // Records whose blob is not in the table's own data — handled below from
    // unclaimed blob segments elsewhere in the datastore. Kept in record order.
    let mut orphan_handles: Vec<u64> = Vec::new();
    let mut claimed_starts: HashSet<usize> = HashSet::new();
    // A large datastore splits its records across several `_data_` segments, so
    // process every one whose content is a record table (opens with `col0=0x14`).
    let mut scan = 0;
    while let Some(i) = buf[scan..].windows(8).position(|w| w == marker) {
        let seg = scan + i;
        scan = seg + 8;
        if rd(seg + 48) != Some(0x14) {
            continue; // e.g. the empty thumbnail `_data_` segment
        }
        // Segment size (RQ at header offset 16) bounds this segment's blob data,
        // so the last record does not run into the following segment.
        let seg_size = buf
            .get(seg + 16..seg + 24)
            .map_or(0, |b| u64::from_le_bytes(b.try_into().unwrap()) as usize);
        let seg_end = seg.saturating_add(seg_size).min(buf.len());
        // Record table: (handle, cumulative data offset) per record.
        let mut recs: Vec<(u64, usize)> = Vec::new();
        let mut p = seg + 48;
        while rd(p) == Some(0x14) {
            let (Some(handle), Some(off)) = (rd(p + 8), rd(p + 16)) else {
                break;
            };
            recs.push((handle as u64, off as usize));
            p += 20;
        }
        // A single entry can also be the interleaved layout emitted by older
        // acadrust versions. Use the order-based fallback for those files.
        if recs.len() < 2 {
            continue;
        }
        let table_end = seg + 48 + recs.len() * 20;
        let aligned = rd(seg + 36)
            .and_then(|units| seg.checked_add(units as usize * 16))
            .filter(|base| *base >= table_end && *base < seg_end);
        let base = aligned.unwrap_or(table_end);
        for k in 0..recs.len() {
            let (handle, off) = recs[k];
            let region_start = base + off;
            let region_end = recs.get(k + 1).map_or(seg_end, |&(_, o)| base + o);
            if region_start >= seg_end || region_end <= region_start {
                if modeler_handles.contains(&handle) {
                    orphan_handles.push(handle); // blob lives in an external segment
                }
                continue;
            }
            let region = &buf[region_start..region_end.min(seg_end)];
            let Some(mp) = region
                .windows(14)
                .position(|w| w == b"ASM BinaryFile")
                .or_else(|| region.windows(15).position(|w| w == b"ACIS BinaryFile"))
            else {
                if modeler_handles.contains(&handle) {
                    orphan_handles.push(handle); // blob lives in an external segment
                }
                continue;
            };
            let start = region_start + mp;
            // This record's blob ends at its region boundary; the end marker can
            // trail a few bytes past `off[r+1]`, so bound to `region_end` plus a
            // small slack (clamped to the segment). Bounding to `region_end`, not
            // `seg_end`, is what makes this linear: a big datastore is often ONE
            // segment (seg_end ≈ buf.len()), so a per-record scan to seg_end for
            // the absent marker family is still O(records × buf). (#203)
            let end_bound = (region_end + ACIS_TAGGED_END_MARKER.len()).min(seg_end);
            if let Some((end, marker_len)) = find_acds_end(buf, start, end_bound) {
                claimed_starts.insert(start);
                out.push((handle, buf[start..end + marker_len].to_vec()));
            }
        }
    }
    // A record can point to an external blob segment before or after its table.
    // Pair only blobs not already claimed by table offsets, and require an exact
    // count so stray magic cannot shift the association.
    if !orphan_handles.is_empty() {
        let pool: Vec<Vec<u8>> = extract_acds_sab_blob_ranges(buf)
            .into_iter()
            .filter(|(start, _)| !claimed_starts.contains(start))
            .map(|(start, stop)| buf[start..stop].to_vec())
            .collect();
        if pool.len() == orphan_handles.len() {
            out.extend(orphan_handles.into_iter().zip(pool));
        }
    }
    out
}

/// Extract every SAB (ACIS/ASM binary) blob from a decompressed AcDs section, in
/// the order they appear — the `_data_` record order, matching the handle list
/// order they appear.
///
/// Each blob runs from its header magic — `"ACIS BinaryFile"` (classic ACIS) or
/// `"ASM BinaryFile"` (Autodesk ShapeManager, AutoCAD 2013+) — through its
/// end-of-body terminator (`End-of-ASM-data` or `End-of-ACIS-data`).
fn extract_acds_sab_blob_ranges(buf: &[u8]) -> Vec<(usize, usize)> {
    let mut blobs = Vec::new();
    let mut pos = 0usize;
    // A single forward walk: each `find_acds_magic` resumes from `pos`, and the
    // end-marker search and `pos = stop` advance past the blob just taken, so
    // the buffer is scanned once overall — not once per blob per magic, which
    // was quadratic on 3D-heavy files with many SAB bodies (issue #203).
    while let Some(start) = find_acds_magic(buf, pos) {
        // A body's end marker precedes the next blob's header magic, so bound
        // the (possibly-absent) marker search there instead of running to the
        // end of the whole buffer on every blob — the same O(n²) trap as the
        // record-table path. (#203)
        let bound = find_acds_magic(buf, start + 1).unwrap_or(buf.len());
        match find_acds_end(buf, start, bound) {
            Some((end, marker_len)) => {
                let stop = end + marker_len;
                blobs.push((start, stop));
                pos = stop;
            }
            None => break,
        }
    }
    blobs
}

fn extract_acds_sab_blobs(buf: &[u8]) -> Vec<Vec<u8>> {
    extract_acds_sab_blob_ranges(buf)
        .into_iter()
        .map(|(start, stop)| buf[start..stop].to_vec())
        .collect()
}

/// Remove AcDs `blob01` continuation frames embedded at 1 MiB boundaries.
///
/// Large ASM bodies are split across storage segments. The decompressed AcDs
/// byte stream places an 80-byte segment header/continuation descriptor between
/// consecutive SAB chunks:
/// `AC D5 "blob01" ... 55×8 ...`. Those bytes belong to the datastore, not the
/// SAB token stream; leaving them in makes the ACIS decoder fail at 0x0f_ffb0.
fn strip_acds_blob01_frames(blob: Vec<u8>) -> Vec<u8> {
    const PREFIX: &[u8] = b"\xAC\xD5blob01";
    const FRAME_LEN: usize = 80;
    const PADDING: [u8; 8] = [0x55; 8];
    let mut clean = Vec::with_capacity(blob.len());
    let mut cursor = 0usize;
    let mut scan = 0usize;
    while let Some(relative) = blob[scan..].windows(PREFIX.len()).position(|w| w == PREFIX) {
        let start = scan + relative;
        let valid = start + FRAME_LEN <= blob.len()
            && blob.get(start + 12..start + 16) == Some(&1u32.to_le_bytes())
            && blob.get(start + 16..start + 24) == Some(&48u64.to_le_bytes())
            && blob.get(start + 40..start + 48) == Some(&PADDING);
        if valid {
            clean.extend_from_slice(&blob[cursor..start]);
            cursor = start + FRAME_LEN;
            scan = cursor;
        } else {
            scan = start + 1;
        }
    }
    if cursor == 0 {
        return blob;
    }
    clean.extend_from_slice(&blob[cursor..]);
    clean
}

/// Fill an `AcisData` from a SAB blob and mark it binary v2.
fn acds_fill(acis: &mut crate::entities::solid3d::AcisData, blob: Vec<u8>) {
    acis.sab_data = strip_acds_blob01_frames(blob);
    acis.sat_data = String::new();
    acis.is_binary = true;
    acis.version = crate::entities::solid3d::AcisVersion::Version2;
}

/// Apply a SAB blob to a modeler entity, deriving its placement reference point
/// now that the geometry is available. Returns false for non-modeler entities.
fn acds_apply(entity: &mut crate::entities::EntityType, blob: Vec<u8>) -> bool {
    use crate::entities::EntityType;
    match entity {
        EntityType::Solid3D(s) => {
            acds_fill(&mut s.acis_data, blob);
            // The inline wireframe anchor (bbox centre) is the preferred
            // reference point; fall back to the SAB placement translation
            // only when the file carried none.
            if s.point_of_reference == crate::types::Vector3::ZERO {
                if let Some(p) = s
                    .acis_data
                    .geometry_centre()
                    .or_else(|| s.acis_data.placement_origin())
                {
                    s.point_of_reference = p;
                }
            }
            true
        }
        EntityType::Region(r) => {
            acds_fill(&mut r.acis_data, blob);
            // The inline wireframe anchor (bbox centre) is the preferred
            // reference point; fall back to the SAB placement translation
            // only when the file carried none.
            if r.point_of_reference == crate::types::Vector3::ZERO {
                if let Some(p) = r
                    .acis_data
                    .geometry_centre()
                    .or_else(|| r.acis_data.placement_origin())
                {
                    r.point_of_reference = p;
                }
            }
            true
        }
        EntityType::Body(b) => {
            acds_fill(&mut b.acis_data, blob);
            // The inline wireframe anchor (bbox centre) is the preferred
            // reference point; fall back to the SAB placement translation
            // only when the file carried none.
            if b.point_of_reference == crate::types::Vector3::ZERO {
                if let Some(p) = b
                    .acis_data
                    .geometry_centre()
                    .or_else(|| b.acis_data.placement_origin())
                {
                    b.point_of_reference = p;
                }
            }
            true
        }
        EntityType::Surface(s) => {
            acds_fill(&mut s.acis_data, blob);
            true
        }
        _ => false,
    }
}

/// Attach handle-paired AcDs SAB blobs (from [`extract_acds_record_blobs`]) to
/// their owning modeler entities — authoritative, each blob to the entity the
/// record table named. Returns the number attached.
fn attach_acds_record_blobs(
    document: &mut crate::document::CadDocument,
    record_blobs: Vec<(u64, Vec<u8>)>,
) -> usize {
    document.acis_sab_handles.clear();
    let mut attached = 0usize;
    for (h, blob) in record_blobs {
        if let Some(entity) = document.get_entity_mut(crate::Handle::new(h)) {
            if acds_apply(entity, blob) {
                attached += 1;
            }
        }
    }
    attached
}

/// Attach order-extracted AcDs SAB blobs when no record table is parseable: pair
/// by the object-stream-ordered handle list (`document.acis_sab_handles`), else
/// positionally in document order. Returns the number attached.
fn attach_acds_sab_blobs(
    document: &mut crate::document::CadDocument,
    blobs: Vec<Vec<u8>>,
) -> usize {
    use crate::entities::EntityType;

    // Fallback A: attach by the object-stream-ordered handle list.
    let ordered = std::mem::take(&mut document.acis_sab_handles);
    if !ordered.is_empty() {
        let mut attached = 0usize;
        for (handle, blob) in ordered.into_iter().zip(blobs.into_iter()) {
            if let Some(entity) = document.get_entity_mut(handle) {
                if acds_apply(entity, blob) {
                    attached += 1;
                }
            }
        }
        return attached;
    }

    // Fallback B: positional attach in document order.
    let mut it = blobs.into_iter();
    let mut attached = 0usize;
    for entity in document.entities_mut() {
        // Only consume a blob for ACIS-backed entities, in document order.
        if matches!(
            entity,
            EntityType::Solid3D(_)
                | EntityType::Region(_)
                | EntityType::Body(_)
                | EntityType::Surface(_)
        ) {
            let Some(blob) = it.next() else { break };
            if acds_apply(entity, blob) {
                attached += 1;
            }
            continue;
        }
    }
    attached
}

/// Decode a section name from a fixed 64-byte, null-terminated field.
///
/// The name ends at the first null byte. Some writers leave non-zero garbage
/// in the bytes *after* the terminator instead of zero-padding the field, so
/// trimming trailing nulls is not enough — it would keep the embedded null and
/// the junk that follows (e.g. `"AcDb:Handles\0t…"`), and the name would then
/// fail to match when looking the section up.
fn section_name_from_field(name_buf: &[u8; 64]) -> String {
    let end = name_buf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_buf.len());
    String::from_utf8_lossy(&name_buf[..end]).into_owned()
}

/// DWG file reader with CRC-64 extraction support.
///
/// Reads DWG binary files and provides access to all internal
/// integrity checksums including the AC1021 Header CRC-64.
/// Read one `T16` string (2-byte character-count prefix). R2007+ stores the
/// characters as UTF-16LE; earlier versions as one byte per character.
fn read_t16(cur: &mut &[u8], utf16: bool) -> String {
    if cur.len() < 2 {
        *cur = &[];
        return String::new();
    }
    let count = u16::from_le_bytes([cur[0], cur[1]]) as usize;
    *cur = &cur[2..];
    if utf16 {
        let bytes = count.saturating_mul(2).min(cur.len());
        let units: Vec<u16> = cur[..bytes]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        *cur = &cur[bytes..];
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string()
    } else {
        let bytes = count.min(cur.len());
        let s = String::from_utf8_lossy(&cur[..bytes]).into_owned();
        *cur = &cur[bytes..];
        s.trim_end_matches('\0').to_string()
    }
}

/// Parse the decompressed `AcDb:SummaryInfo` section (R2004+): eight fixed
/// strings, three 8-byte timers, then the custom-property pairs.
fn parse_summary_info(buf: &[u8], utf16: bool) -> crate::document::SummaryInfo {
    let mut cur: &[u8] = buf;
    let mut si = crate::document::SummaryInfo {
        title: read_t16(&mut cur, utf16),
        subject: read_t16(&mut cur, utf16),
        author: read_t16(&mut cur, utf16),
        keywords: read_t16(&mut cur, utf16),
        comments: read_t16(&mut cur, utf16),
        last_saved_by: read_t16(&mut cur, utf16),
        revision_number: read_t16(&mut cur, utf16),
        hyperlink_base: read_t16(&mut cur, utf16),
        custom_properties: Vec::new(),
        ..Default::default()
    };
    // TDINDWG, TDCREATE, TDUPDATE — each a TIMERLL (2×u32 = 8 bytes;
    // gold prints each as a `[days, ms]` pair).
    if cur.len() >= 24 {
        let timer = |cur: &mut &[u8]| -> [u32; 2] {
            let days = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
            let ms = u32::from_le_bytes([cur[4], cur[5], cur[6], cur[7]]);
            *cur = &cur[8..];
            [days, ms]
        };
        si.tdindwg = timer(&mut cur);
        si.tdcreate = timer(&mut cur);
        si.tdupdate = timer(&mut cur);
    } else {
        cur = &[];
    }
    if cur.len() >= 2 {
        let n = u16::from_le_bytes([cur[0], cur[1]]) as usize;
        cur = &cur[2..];
        for _ in 0..n.min(256) {
            let tag = read_t16(&mut cur, utf16);
            let val = read_t16(&mut cur, utf16);
            if tag.is_empty() && val.is_empty() {
                break;
            }
            si.custom_properties.push((tag, val));
        }
    }
    // The two trailing raw longs (gold's `unknown1`/`unknown2`).
    if cur.len() >= 8 {
        si.unknown1 = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
        si.unknown2 = u32::from_le_bytes([cur[4], cur[5], cur[6], cur[7]]);
    }
    si
}

/// Read one `TU32` string — gold's `bit_read_TU32` exactly (§19 H4):
/// 4-byte BYTE-count prefix; pre-2007 `size` single-byte chars; R2007+
/// the char width is sniffed — the overflow check comes FIRST (`size +
/// byte >= total` → NULL with only the prefix consumed; in remaining-
/// slice terms `size >= remaining`), then the next RL is peeked
/// (consumed): if its `0x00ff0000` bits are set the payload is UCS-2
/// (rewind, `size/2` RS chars = `size` bytes), else 4-byte chars
/// (`size/4` RLs with the peek as the first char, low 16 bits kept).
/// `FIELD_T32` expands to this same reader on R2007+ (dec_macros.h:616)
/// — an "empty" string still consumes the peek (8 bytes total), which
/// is load-bearing for the FileDepList record alignment (pinned by
/// 2018/Arc.dwg: three empty strings consume 8+8+4 — the third's size RL
/// is 0xFFFFFFFF and overflows).
fn read_tu32(cur: &mut &[u8], utf16: bool) -> String {
    if cur.len() < 4 {
        *cur = &[];
        return String::new();
    }
    let size = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]) as u64;
    *cur = &cur[4..];
    if !utf16 {
        let bytes = (size as usize).min(cur.len());
        let s = String::from_utf8_lossy(&cur[..bytes]).into_owned();
        *cur = &cur[bytes..];
        return s.trim_end_matches('\0').to_string();
    }
    // R2007+: the overflow check precedes the peek — NULL with only the
    // prefix consumed (gold: `size + dat->byte >= dat->size || size >
    // dat->size`; in remaining-slice terms size >= remaining).
    if size >= cur.len() as u64 {
        return String::new();
    }
    if cur.len() < 4 {
        *cur = &[];
        return String::new();
    }
    let pre_peek: &[u8] = cur;
    let peek = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
    *cur = &cur[4..];
    if peek & 0x00ff_0000 != 0 {
        // UCS-2 payload: rewind to before the peek, size/2 RS chars
        let bytes = (size as usize).min(pre_peek.len());
        let units: Vec<u16> = pre_peek[..bytes]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        *cur = &pre_peek[bytes..];
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string()
    } else {
        // 4-byte chars: the peeked RL is the first char (low half kept),
        // then size/4 − 1 more RLs
        let count = (size / 4) as usize;
        let mut units = Vec::with_capacity(count);
        units.push(peek as u16);
        for _ in 1..count {
            if cur.len() < 4 {
                *cur = &[];
                break;
            }
            let rl = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
            *cur = &cur[4..];
            units.push(rl as u16);
        }
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string()
    }
}

/// Parse the `AcDb:Template` section — gold's `Template` shape (§19 H4):
/// description (T16) + MEASUREMENT (RS).
fn parse_template_section(buf: &[u8], utf16: bool) -> crate::document::DwgTemplateSummary {
    let mut cur: &[u8] = buf;
    let description = read_t16(&mut cur, utf16);
    let measurement = if cur.len() >= 2 {
        i16::from_le_bytes([cur[0], cur[1]])
    } else {
        0
    };
    crate::document::DwgTemplateSummary {
        description,
        measurement,
    }
}

/// Parse the `AcDb:FileDepList` section — gold's `FileDepList` shape
/// (§19 H4): num_features RL + features (TU32 vector) + num_files RL +
/// the file records (T32 filename/filepath/fingerprint/version + the
/// numeric fields). The count fields do not print.
fn parse_file_dep_list_section(
    buf: &[u8],
    utf16: bool,
) -> crate::document::DwgFileDepListSummary {
    let mut cur: &[u8] = buf;
    let num_features = if cur.len() >= 4 {
        let v = i32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
        cur = &cur[4..];
        v
    } else {
        return Default::default();
    };
    let mut features = Vec::new();
    for _ in 0..num_features.clamp(0, 4096) {
        features.push(read_tu32(&mut cur, utf16));
    }
    let num_files = if cur.len() >= 4 {
        let v = i32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
        cur = &cur[4..];
        v
    } else {
        return crate::document::DwgFileDepListSummary { features, files: Vec::new() };
    };
    let mut files = Vec::new();
    for _ in 0..num_files.clamp(0, 4096) {
        // FIELD_T32 = TU32 semantics on R2007+ (dec_macros.h:616) — the
        // sniffing reader, whose peek/overflow behavior is load-bearing
        // for the record alignment.
        let filename = read_tu32(&mut cur, utf16);
        let filepath = read_tu32(&mut cur, utf16);
        let fingerprint = read_tu32(&mut cur, utf16);
        let version = read_tu32(&mut cur, utf16);
        let rl = |cur: &mut &[u8]| -> i32 {
            if cur.len() < 4 {
                *cur = &[];
                return 0;
            }
            let v = i32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
            *cur = &cur[4..];
            v
        };
        let feature_index = rl(&mut cur);
        let timestamp = rl(&mut cur);
        let filesize = rl(&mut cur);
        let affects_graphics = if cur.len() >= 2 {
            let v = i16::from_le_bytes([cur[0], cur[1]]);
            cur = &cur[2..];
            v
        } else {
            0
        };
        let refcount = rl(&mut cur);
        files.push(crate::document::DwgFileDepFileInfo {
            filename,
            filepath,
            fingerprint,
            version,
            feature_index,
            timestamp,
            filesize,
            affects_graphics,
            refcount,
        });
    }
    crate::document::DwgFileDepListSummary { features, files }
}

/// Parse the `AcDb:RevHistory` section — gold's `RevHistory` shape
/// (§19 H4): class_version RL, class_minor RL, num_histories RL +
/// the RL values (the count does not print).
fn parse_rev_history_section(buf: &[u8]) -> crate::document::DwgRevHistorySummary {
    let mut cur: &[u8] = buf;
    let mut rl = || -> i32 {
        if cur.len() < 4 {
            cur = &[];
            return 0;
        }
        let v = i32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
        cur = &cur[4..];
        v
    };
    let class_version = rl();
    let class_minor = rl();
    let num_histories = rl().clamp(0, 65536);
    let mut histories = Vec::with_capacity(num_histories as usize);
    for _ in 0..num_histories {
        histories.push(rl());
    }
    crate::document::DwgRevHistorySummary {
        class_version,
        class_minor,
        histories,
    }
}

/// Parse the `AcDb:Security` section — gold's `Security` shape (§19 H4):
/// three RLx unknowns, crypto_id, T32 crypto_name, algo_id, key_len,
/// encr_size + the encr_size bytes as hex.
fn parse_security_section(buf: &[u8], utf16: bool) -> crate::document::DwgSecuritySummary {
    let mut cur: &[u8] = buf;
    fn rl(cur: &mut &[u8]) -> u32 {
        if cur.len() < 4 {
            *cur = &[];
            return 0;
        }
        let v = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
        *cur = &cur[4..];
        v
    }
    let unknown_1 = rl(&mut cur);
    let unknown_2 = rl(&mut cur);
    let unknown_3 = rl(&mut cur);
    let crypto_id = rl(&mut cur);
    // FIELD_T32 → TU32 semantics on R2007+ (see read_tu32)
    let crypto_name = read_tu32(&mut cur, utf16);
    let algo_id = rl(&mut cur);
    let key_len = rl(&mut cur);
    let encr_size = rl(&mut cur);
    let n = (encr_size as usize).min(cur.len());
    let mut encr_buffer = String::with_capacity(n * 2);
    for b in &cur[..n] {
        use std::fmt::Write;
        let _ = write!(encr_buffer, "{:02X}", b);
    }
    crate::document::DwgSecuritySummary {
        unknown_1,
        unknown_2,
        unknown_3,
        crypto_id,
        crypto_name,
        algo_id,
        key_len,
        encr_size,
        encr_buffer,
    }
}

/// Parse the `AcDb:ObjFreeSpace` section — gold's `ObjFreeSpace` shape
/// (§19 H4), version-gated per objfreespace.spec: ≤R2007 (incl. R2000)
/// reads 4-byte `zero`/`numhandles` (FIELD_CAST into 64-bit stores),
/// TDUPDATE TIMERLL, `objects_address` RLx, `numnums` RC, then the
/// four plain RLL maxes; R2010+ reads 64-bit `zero`/`numhandles`, no
/// objects_address, `numnums` RC, and each max as the 128-bit lo/hi
/// pair.
fn parse_obj_free_space_section(
    buf: &[u8],
    r2010_plus: bool,
) -> crate::document::DwgObjFreeSpaceSummary {
    let mut cur: &[u8] = buf;
    fn rll(cur: &mut &[u8]) -> u64 {
        if cur.len() < 8 {
            *cur = &[];
            return 0;
        }
        let v = u64::from_le_bytes([
            cur[0], cur[1], cur[2], cur[3], cur[4], cur[5], cur[6], cur[7],
        ]);
        *cur = &cur[8..];
        v
    }
    fn rl32(cur: &mut &[u8]) -> u32 {
        if cur.len() < 4 {
            *cur = &[];
            return 0;
        }
        let v = u32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
        *cur = &cur[4..];
        v
    }
    fn rc1(cur: &mut &[u8]) -> u8 {
        if cur.is_empty() {
            return 0;
        }
        let v = cur[0];
        *cur = &cur[1..];
        v
    }
    if r2010_plus {
        let zero = rll(&mut cur);
        let numhandles = rll(&mut cur);
        let tdupdate = [rl32(&mut cur), rl32(&mut cur)];
        let numnums = rc1(&mut cur);
        let max32 = rll(&mut cur);
        let max32_hi = rll(&mut cur);
        let max64 = rll(&mut cur);
        let max64_hi = rll(&mut cur);
        let maxtbl = rll(&mut cur);
        let maxtbl_hi = rll(&mut cur);
        let maxrl = rll(&mut cur);
        let maxrl_hi = rll(&mut cur);
        crate::document::DwgObjFreeSpaceSummary {
            zero,
            numhandles,
            tdupdate,
            numnums,
            objects_address: None,
            max32,
            max32_hi: Some(max32_hi),
            max64,
            max64_hi: Some(max64_hi),
            maxtbl,
            maxtbl_hi: Some(maxtbl_hi),
            maxrl,
            maxrl_hi: Some(maxrl_hi),
        }
    } else {
        let zero = rl32(&mut cur) as u64;
        let numhandles = rl32(&mut cur) as u64;
        let tdupdate = [rl32(&mut cur), rl32(&mut cur)];
        let objects_address = rl32(&mut cur);
        let numnums = rc1(&mut cur);
        let max32 = rll(&mut cur);
        let max64 = rll(&mut cur);
        let maxtbl = rll(&mut cur);
        let maxrl = rll(&mut cur);
        crate::document::DwgObjFreeSpaceSummary {
            zero,
            numhandles,
            tdupdate,
            numnums,
            objects_address: Some(objects_address),
            max32,
            max32_hi: None,
            max64,
            max64_hi: None,
            maxtbl,
            maxtbl_hi: None,
            maxrl,
            maxrl_hi: None,
        }
    }
}

/// Parse the `AcDb:AppInfo` section — gold's `AppInfo` shape (§19 H4):
/// the parsed fields plus, on the R2004-format containers (AC1018 and
/// AC1024+), the whole section as `size` + `unknown_bits` hex. The
/// AC1021 (R2007) container's reader never sets size/unknown_bits —
/// gold prints 0 and '' there (the container split pinned empirically:
/// example_2007 size=0 vs sample_2018/example_2004 size=698).
fn parse_app_info_section(
    buf: &[u8],
    utf16: bool,
    ac1021: bool,
) -> crate::document::DwgAppInfoSummary {
    let mut cur: &[u8] = buf;
    let mut summary = crate::document::DwgAppInfoSummary::default();
    if !ac1021 {
        let mut hex = String::with_capacity(buf.len() * 2);
        for b in buf {
            use std::fmt::Write;
            let _ = write!(hex, "{:02X}", b);
        }
        summary.size = buf.len() as i32;
        summary.unknown_bits = hex;
    }
    if !utf16 {
        // R2004 (AC1018): the pre-2007 branch — gold's bit_read_T16
        // semantics are C-string truncation at the first NUL plus
        // overflow-to-empty (CHK_OVERFLOW returns NULL after consuming
        // only the RS). The R2004 corpus files carry R2007-format
        // content (class_version=3 first), so this branch misparses:
        // appinfo_name reads a 3-byte NUL-led prefix (→ ""), the bogus
        // num_strings length then overflows every later T16 (→ "") —
        // gold's emission is size + hex + four empty strings.
        fn t16_pre2007(cur: &mut &[u8]) -> String {
            if cur.len() < 2 {
                *cur = &[];
                return String::new();
            }
            let count = u16::from_le_bytes([cur[0], cur[1]]) as usize;
            *cur = &cur[2..];
            if count > cur.len() {
                // CHK_OVERFLOW: NULL, cursor stays after the RS
                return String::new();
            }
            let bytes = &cur[..count];
            *cur = &cur[count..];
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(count);
            String::from_utf8_lossy(&bytes[..end]).into_owned()
        }
        summary.appinfo_name = t16_pre2007(&mut cur);
        if cur.len() >= 4 {
            cur = &cur[4..];
        }
        summary.comment = t16_pre2007(&mut cur);
        summary.product_info = t16_pre2007(&mut cur);
        summary.version = t16_pre2007(&mut cur);
    } else {
        // R2007+: class_version RL, appinfo_name, num_strings RL,
        // version_checksum (16 raw bytes), version, then per num_strings
        // the comment/product checksum+string pairs.
        fn rl(cur: &mut &[u8]) -> i32 {
            if cur.len() < 4 {
                *cur = &[];
                return 0;
            }
            let v = i32::from_le_bytes([cur[0], cur[1], cur[2], cur[3]]);
            *cur = &cur[4..];
            v
        }
        fn checksum16(cur: &mut &[u8]) -> String {
            if cur.len() < 16 {
                *cur = &[];
                return String::new();
            }
            let mut s = String::with_capacity(32);
            for b in &cur[..16] {
                use std::fmt::Write;
                let _ = write!(s, "{:02X}", b);
            }
            *cur = &cur[16..];
            s
        }
        summary.class_version = Some(rl(&mut cur));
        summary.appinfo_name = read_t16(&mut cur, utf16);
        let num_strings = rl(&mut cur);
        summary.version_checksum = Some(checksum16(&mut cur));
        summary.version = read_t16(&mut cur, utf16);
        if num_strings >= 2 {
            summary.comment_checksum = Some(checksum16(&mut cur));
            summary.comment = read_t16(&mut cur, utf16);
        }
        if num_strings >= 3 {
            summary.product_checksum = Some(checksum16(&mut cur));
            summary.product_info = read_t16(&mut cur, utf16);
        }
    }
    summary
}

/// Parse the `AcDb:AppInfoHistory` section — gold's `AppInfoHistory`
/// shape (§19 H4): the whole section as `size` + `unknown_bits` hex
/// (gold's spec include for it is commented out — never parsed).
fn parse_app_info_history_section(buf: &[u8]) -> crate::document::DwgAppInfoHistorySummary {
    let mut hex = String::with_capacity(buf.len() * 2);
    for b in buf {
        use std::fmt::Write;
        let _ = write!(hex, "{:02X}", b);
    }
    crate::document::DwgAppInfoHistorySummary {
        size: buf.len() as i32,
        unknown_bits: hex,
    }
}

pub struct DwgReader<R: Read + Seek> {
    stream: R,
    /// Options controlling read behaviour.
    pub options: DwgReadOptions,
    /// Notifications collected during reading
    pub notifications: NotificationCollection,
    /// Source path, when opened from a file — copied onto the document so the
    /// `Filename` / `FilePath` fields can resolve.
    source_path: Option<String>,
    /// Optional monotonic read progress callback. The value is in 0..=1000 so
    /// callers can map DWG parsing into a larger file-open pipeline without
    /// coupling this crate to a UI or an async runtime.
    progress: Option<std::sync::Arc<dyn Fn(u16) + Send + Sync>>,
}

impl DwgReader<File> {
    /// Open a DWG file from a filesystem path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, DxfError> {
        Self::from_file_with_options(path, DwgReadOptions::default())
    }

    /// Open a DWG file from a filesystem path with custom options.
    pub fn from_file_with_options<P: AsRef<Path>>(
        path: P,
        options: DwgReadOptions,
    ) -> Result<Self, DxfError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        Ok(Self {
            stream: file,
            options,
            notifications: NotificationCollection::new(),
            source_path: Some(path.to_string_lossy().into_owned()),
            progress: None,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DwgReader<Cursor<memmap2::Mmap>> {
    /// Open through a read-only memory map. Section seeks then become pointer
    /// moves and the OS can page/readahead the file without repeated syscalls.
    pub fn from_mmap<P: AsRef<Path>>(path: P) -> Result<Self, DxfError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        // SAFETY: read-only map owns its file-backed virtual-memory mapping;
        // this reader never mutates the file while the mapping is alive.
        let map = unsafe { memmap2::MmapOptions::new().map(&file)? };
        Ok(Self {
            stream: Cursor::new(map),
            options: DwgReadOptions::default(),
            notifications: NotificationCollection::new(),
            source_path: Some(path.to_string_lossy().into_owned()),
            progress: None,
        })
    }
}

impl<R: Read + Seek> DwgReader<R> {
    /// Create a reader from any seekable stream.
    pub fn from_stream(stream: R) -> Self {
        Self {
            stream,
            options: DwgReadOptions::default(),
            notifications: NotificationCollection::new(),
            source_path: None,
            progress: None,
        }
    }

    /// Create a reader from any seekable stream with custom options.
    pub fn from_stream_with_options(stream: R, options: DwgReadOptions) -> Self {
        Self {
            stream,
            options,
            notifications: NotificationCollection::new(),
            source_path: None,
            progress: None,
        }
    }

    /// Receive monotonic parsing progress in the inclusive range 0..=1000.
    ///
    /// The callback may run from Rayon worker threads while object records are
    /// decoded, so it must be thread-safe and should only perform cheap atomic
    /// updates.
    pub fn set_progress_callback(&mut self, progress: std::sync::Arc<dyn Fn(u16) + Send + Sync>) {
        self.progress = Some(progress);
    }

    #[inline]
    fn report_progress(&self, value: u16) {
        if let Some(progress) = &self.progress {
            progress(value.min(1000));
        }
    }

    /// Read the DWG file and reconstruct a `CadDocument`.
    ///
    /// This is the main entry point for reading a DWG file into a usable
    /// document. It orchestrates:
    /// 1. File header parsing
    /// 2. Section buffer extraction (Classes, Header, Handles, Objects)
    /// 3. Classes, header variables, and handle map parsing
    /// 4. Object dispatch and entity/object mapping via `DwgDocumentBuilder`
    ///
    /// In failsafe mode, file-header errors are caught and a partial
    /// document is returned instead of propagating the error.
    pub fn read(&mut self) -> std::result::Result<crate::document::CadDocument, DxfError> {
        self.read_with_stats().map(|outcome| outcome.document)
    }

    /// Like [`read`], but each pass-2 entity is offered to `visit` before it
    /// is stored on the document. Return `None` to keep it out of
    /// `CadDocument::entities` (the visitor may consume the value).
    pub fn read_visiting<F>(
        &mut self,
        mut visit: F,
    ) -> std::result::Result<crate::document::CadDocument, DxfError>
    where
        F: FnMut(
            &crate::document::CadDocument,
            crate::entities::EntityType,
        ) -> Option<crate::entities::EntityType>,
    {
        self.read_with_optional_visitor(Some(&mut visit))
            .map(|outcome| outcome.document)
    }

    /// Read the file and return the document with source/decode statistics.
    pub fn read_with_stats(
        &mut self,
    ) -> std::result::Result<crate::io::read::ReadOutcome, DxfError> {
        self.read_with_optional_visitor(
            None::<
                &mut dyn FnMut(
                    &crate::document::CadDocument,
                    crate::entities::EntityType,
                ) -> Option<crate::entities::EntityType>,
            >,
        )
    }

    fn read_with_optional_visitor(
        &mut self,
        visit: Option<
            &mut dyn FnMut(
                &crate::document::CadDocument,
                crate::entities::EntityType,
            ) -> Option<crate::entities::EntityType>,
        >,
    ) -> std::result::Result<crate::io::read::ReadOutcome, DxfError> {
        let failsafe = self.options.failsafe;
        let perf = std::env::var_os("PERF").is_some();
        let total_started = web_time::Instant::now();
        let mut diagnostics = Vec::new();
        self.report_progress(0);

        // 1. Read the DWG file header and section map
        let stage_started = web_time::Instant::now();
        let mut info = match self.read_file_header() {
            Ok(info) => info,
            Err(e) if failsafe => {
                report_read_error(
                    &mut self.notifications,
                    &mut diagnostics,
                    "file-header-read-failed",
                    ReadStage::FileHeader,
                    None,
                    format!(
                        "Failsafe: file header read failed, returning partial document: {}",
                        e
                    ),
                );
                let mut doc = crate::document::CadDocument::default();
                doc.notifications
                    .extend(std::mem::take(&mut self.notifications));
                let stats = crate::io::read::ReadStats::from_document(
                    &doc,
                    SourceFormat::Dwg,
                    0,
                    0,
                    0,
                    0,
                    false,
                    true,
                    false,
                    diagnostics,
                );
                return Ok(crate::io::read::ReadOutcome::new(doc, stats));
            }
            Err(e) => return Err(e),
        };
        if perf {
            eprintln!(
                "[perf] dwg-read header={:.1}ms sections={} pages={}",
                stage_started.elapsed().as_secs_f64() * 1000.0,
                info.section_descriptors.len(),
                info.page_records.len(),
            );
        }
        let dxf_version = crate::types::DxfVersion::parse(&info.version_string)
            .unwrap_or(crate::types::DxfVersion::Unknown);
        self.report_progress(20);
        let mut document = crate::document::CadDocument::with_version(dxf_version);
        document.maintenance_version = info.acad_maintenance_version;
        document.dwg_source_version = Some(dxf_version);
        // The file-header summary (§19 H2): gold's FILEHEADER shape, kept
        // in the document so the structure axis can compare it (the dump
        // emits it automatically through CadDocument's serde).
        document.dwg_file_header = Some(crate::document::DwgFileHeaderSummary {
            version: info.version_string.clone(),
            maint_rel_version: info.acad_maintenance_version,
            zero_one_or_three: info.zero_one_or_three,
            thumbnail_address: info.preview_address,
            dwg_version: info.dwg_version,
            maint_version: info.maint_version,
            codepage: info.code_page,
            sections: info.sections,
            unknown_0: info.unknown_0,
            app_dwg_version: info.app_dwg_version,
            app_maint_version: info.app_maint_version,
            security_type: info.security_type,
            rl_1c_address: info.rl_1c_address,
            summaryinfo_address: info.summary_info_addr,
            vbaproj_address: info.vba_project_addr,
            r2004_header_address: info.r2004_header_address,
        });
        // The R2004-format system-section summary (§19 H2's second
        // sub-row) — set on AC18-format files only.
        document.dwg_r2004_header = info.r2004_system.clone();
        // The §19 H7g container shape: the author's page space
        // (per-descriptor page boundaries/ids, the physical map order,
        // the box-page identity), retained for the same-version
        // roundtrip's container-shape mirror. AC18-family files only —
        // the write gate below falls back to the conventional layout
        // whenever the re-encoded content does not fit the author's
        // page space.
        if info.is_ac18_format {
            if let Some(sys) = info.r2004_system.as_ref() {
                document.dwg_ac18_shape = Some(crate::document::DwgAc18ContainerShape {
                    sections: std::mem::take(&mut info.ac18_section_shapes),
                    map_order: std::mem::take(&mut info.ac18_map_order),
                    section_map_id: sys.section_map_id,
                    section_info_id: sys.section_info_id as u32,
                    section_array_size: sys.section_array_size as u32,
                });
            }
        }
        // The R2007-format system-section summary (§19 H2's third
        // sub-row) — the gold-named projection of the AC1021 container
        // metadata silver already parses (Dwg21CompressedMetadata). The
        // container names differ from gold's; sections_amount has no
        // gold-emitted counterpart and is dropped.
        if let Some(m) = &info.ac21_metadata {
            document.dwg_r2007_header = Some(crate::document::DwgR2007SystemHeader {
                header_size: m.header_size,
                file_size: m.file_size,
                pages_map_crc_compressed: m.pages_map_crc_compressed,
                pages_map_correction: m.pages_map_correction_factor,
                pages_map_crc_seed: m.pages_map_crc_seed,
                pages_map2_offset: m.map2_offset,
                pages_map2_id: m.map2_id,
                pages_map_offset: m.pages_map_offset,
                pages_map_id: m.pages_map_id,
                header2_offset: m.header2_offset,
                pages_map_size_comp: m.pages_map_size_compressed,
                pages_map_size_uncomp: m.pages_map_size_uncompressed,
                pages_amount: m.pages_amount,
                pages_maxid: m.pages_max_id,
                unknown1: m.unknown_0x20,
                unknown2: m.unknown_0x40,
                pages_map_crc_uncomp: m.pages_map_crc_uncompressed,
                unknown3: m.unknown_0xf800,
                unknown4: m.unknown_4,
                unknown5: m.unknown_1,
                sections_map_crc_uncomp: m.sections_map_crc_uncompressed,
                sections_map_size_comp: m.sections_map_size_compressed,
                sections_map2_id: m.sections_map2_id,
                sections_map_id: m.sections_map_id,
                sections_map_size_uncomp: m.sections_map_size_uncompressed,
                sections_map_crc_comp: m.sections_map_crc_compressed,
                sections_map_correction: m.sections_map_correction_factor,
                sections_map_crc_seed: m.sections_map_crc_seed,
                stream_version: m.stream_version,
                crc_seed: m.crc_seed,
                crc_seed_encoded: m.crc_seed_encoded,
                random_seed: m.random_seed,
                header_crc: m.header_crc64,
            });
        }
        // The §19 H8 AC21 container shape — set on AC1021-format
        // files only (the pages map and the sections table both parse
        // on that format; both stay empty on every other one).
        if info.ac21_metadata.is_some() {
            document.dwg_ac21_shape = info.ac21_shape.take();
        }
        // The R13-R2000 structural pair (§19 H2's fourth and fifth
        // sub-rows): the sentinel-located SecondHeader and the
        // locator-addressed AuxHeader — both parsed during the AC15
        // file-header read, both gold-JSON-shaped.
        document.dwg_second_header = info.second_header.clone();
        document.dwg_aux_header = info.aux_header.clone();

        // 2. Read Classes (AcDb:Classes)
        match self.get_section_buffer("AcDb:Classes", &info) {
            Ok(classes_buf) => {
                // §19 H7 CLASSES row: retain the raw section bytes for the
                // verbatim same-version re-emission — the desynced tables
                // (the AutoCAD-2027.1 fixture set) round-trip gold's walk
                // only byte-exactly, and the bytes also carry the author's
                // counts for the raw-passthrough classes.
                document.raw_classes_data = Some(std::sync::Arc::new(classes_buf.clone()));
                match crate::io::dwg::dwg_stream_readers::classes_reader::read_classes_with_encoding(
                    &classes_buf,
                    dxf_version,
                    info.acad_maintenance_version,
                    crate::io::dxf::code_page::encoding_from_dwg_code_page(info.code_page),
                ) {
                    Ok(classes) => document.classes = classes,
                    Err(e) => report_read_error(
                        &mut self.notifications,
                        &mut diagnostics,
                        "classes-decode-failed",
                        ReadStage::Classes,
                        Some("AcDb:Classes"),
                        format!("Failed to read classes: {}", e),
                    ),
                }
            }
            Err(e) => report_read_error(
                &mut self.notifications,
                &mut diagnostics,
                "classes-extract-failed",
                ReadStage::Classes,
                Some("AcDb:Classes"),
                format!("Failed to extract classes section: {}", e),
            ),
        }
        self.report_progress(30);

        // 3. Read Header Variables (AcDb:Header)
        match self.get_section_buffer("AcDb:Header", &info) {
            Ok(header_buf) => {
                match crate::io::dwg::dwg_stream_readers::header_reader::read_header_with_encoding(
                    &header_buf,
                    dxf_version,
                    info.acad_maintenance_version,
                    crate::io::dxf::code_page::encoding_from_dwg_code_page(info.code_page),
                ) {
                    Ok((header_vars, header_raw)) => {
                        document.header = header_vars;
                        document.dwg_header_raw = Some(header_raw);
                    }
                    Err(e) => report_read_error(
                        &mut self.notifications,
                        &mut diagnostics,
                        "header-decode-failed",
                        ReadStage::Header,
                        Some("AcDb:Header"),
                        format!("Failed to read header: {}", e),
                    ),
                }
            }
            Err(e) => report_read_error(
                &mut self.notifications,
                &mut diagnostics,
                "header-extract-failed",
                ReadStage::Header,
                Some("AcDb:Header"),
                format!("Failed to extract header section: {}", e),
            ),
        }
        document.header.code_page =
            crate::io::dxf::code_page::dwg_code_page_name(info.code_page).to_string();

        // MEASUREMENT is stored in the optional Template section, independently
        // of insertion units. Older files may omit the section entirely.
        if let Ok(template_buf) = self.get_section_buffer("AcDb:Template", &info) {
            match parse_template(&template_buf, dxf_version) {
                Ok(measurement) => document.header.measurement = measurement,
                Err(error) if failsafe => report_read_error(
                    &mut self.notifications,
                    &mut diagnostics,
                    "template-decode-failed",
                    ReadStage::Header,
                    Some("AcDb:Template"),
                    error.to_string(),
                ),
                Err(error) => return Err(error),
            }
        }
        self.report_progress(40);

        // 4. Read Handle Map (AcDb:Handles)
        let mut handles_section_read = false;
        let handle_map = match self.get_section_buffer("AcDb:Handles", &info) {
            Ok(handle_buf) => {
                handles_section_read = true;
                match crate::io::dwg::dwg_stream_readers::handle_reader::read_handles(&handle_buf) {
                    Ok(mut hm) => {
                        // AC15: Handle offsets are absolute file positions.
                        // Convert to buffer-relative by subtracting the objects
                        // section base offset.
                        if info.objects_base_offset != 0 {
                            let base = info.objects_base_offset;
                            for offset in hm.values_mut() {
                                *offset -= base;
                            }
                        }
                        hm
                    }
                    Err(e) => {
                        report_read_error(
                            &mut self.notifications,
                            &mut diagnostics,
                            "handles-decode-failed",
                            ReadStage::Handles,
                            Some("AcDb:Handles"),
                            format!("Failed to read handles: {}", e),
                        );
                        std::collections::HashMap::new()
                    }
                }
            }
            Err(e) => {
                report_read_error(
                    &mut self.notifications,
                    &mut diagnostics,
                    "handles-extract-failed",
                    ReadStage::Handles,
                    Some("AcDb:Handles"),
                    format!("Failed to extract handles section: {}", e),
                );
                std::collections::HashMap::new()
            }
        };
        self.report_progress(50);

        // 5. Read Objects (AcDb:AcDbObjects) and build document
        let handle_count = handle_map.len();
        let mut objects_section_read = false;
        let mut record_stream_read = false;
        let mut decoded_source_records = 0usize;
        let mut skipped_source_records = 0usize;
        let objects_started = web_time::Instant::now();
        if !handle_map.is_empty() {
            match self.get_section_buffer("AcDb:AcDbObjects", &info) {
                 Ok(objects_buf) => {
                     objects_section_read = true;
                     // §19 H8d: retain her reconstructed objects-section
                     // stream plus her handle map on AC1021-format files —
                     // the mirror arm's raw-echo source. Her physical
                     // layout is her editor's incremental-save allocation
                     // history; the echo engages only when the document's
                     // object identity is unchanged at the write gate.
                     if info.ac21_metadata.is_some() {
                         document.raw_acdb_objects_data =
                             Some(std::sync::Arc::new(objects_buf.clone()));
                         let mut her_handles: Vec<(u64, i64)> =
                             handle_map.iter().map(|(&h, &o)| (h, o)).collect();
                         her_handles.sort_by_key(|&(h, _)| h);
                         document.raw_acdb_objects_handles =
                             Some(std::sync::Arc::new(her_handles));
                     }
                     // §19 H8e-2/H8f: her whole on-disk file (the header
                     // blocks with their unknown-region bytes, her page
                     // walk or flat section layout exactly as it sits)
                     // for the compressed-page echo — the AC21 page
                     // system and the R2000 flat container alike (the
                     // write gates decide per format). A failed read just
                     // leaves the echo unarmed.
                     if info.ac21_metadata.is_some()
                        || matches!(
                            dxf_version,
                            crate::types::DxfVersion::AC1012
                                | crate::types::DxfVersion::AC1014
                                | crate::types::DxfVersion::AC1015
                        ) {
                         let saved = self.stream.seek(std::io::SeekFrom::Current(0)).ok();
                         if let Some(saved) = saved {
                             let probed = self
                                 .stream
                                 .seek(std::io::SeekFrom::End(0))
                                 .ok()
                                 .and_then(|len| {
                                     if len > 0x80 {
                                         self.stream
                                             .seek(std::io::SeekFrom::Start(0))
                                             .ok()
                                             .map(|_| len)
                                     } else {
                                         None
                                     }
                                 });
                             if let Some(len) = probed {
                                 let mut tail = vec![0u8; len as usize];
                                 if self.stream.read_exact(&mut tail).is_ok() {
                                     document.raw_ac21_tail = Some(std::sync::Arc::new(tail));
                                 }
                             }
                             let _ = self.stream.seek(std::io::SeekFrom::Start(saved));
                         }
                     }
                    match crate::io::dwg::dwg_stream_readers::object_reader::DwgObjectReader::with_encoding(
                    objects_buf,
                    dxf_version,
                    handle_map,
                    crate::io::dxf::code_page::encoding_from_dwg_code_page(info.code_page),
                ) {
                    Ok(obj_reader) => {
                        record_stream_read = true;
                        let mut builder = crate::io::dwg::dwg_document_builder::DwgDocumentBuilder::new(obj_reader);
                        builder.set_failsafe(failsafe);
                        if let Some(progress) = &self.progress {
                            builder.set_progress_callback(progress.clone());
                        }
                        let build_outcome = match visit {
                            Some(visit) => builder.build_with_visitor_stats(&mut document, visit),
                            None => builder.build_with_stats(&mut document),
                        };
                        // §19 H8e/H8f: the identity capture runs POST-BUILD,
                        // over the document's object universe (entities +
                        // objects + the tables' control/record handles) —
                        // not the source's handles map. Reader-skipped
                        // orphans (the author's edit-history residue —
                        // e.g. the ACIS-solids files' single orphan,
                        // ATMOS's 84) are outside the universe on both
                        // sides, so an unedited document engages. Captured
                        // for both echo-capable container families (the
                        // AC21 page system and the R2000 flat container).
                        if info.ac21_metadata.is_some()
                        || matches!(
                            dxf_version,
                            crate::types::DxfVersion::AC1012
                                | crate::types::DxfVersion::AC1014
                                | crate::types::DxfVersion::AC1015
                        ) {
                            document.raw_acdb_objects_fingerprint =
                                super::objects_handle_set_fingerprint(
                                    super::document_object_handles(&document).into_iter(),
                                );
                        }
                        decoded_source_records = build_outcome.decoded_records;
                        skipped_source_records = build_outcome.skipped_records;
                        diagnostics.extend(build_outcome.diagnostics);
                        self.notifications.extend(build_outcome.notifications);
                    },
                    Err(e) => report_read_error(
                        &mut self.notifications,
                        &mut diagnostics,
                        "objects-init-failed",
                        ReadStage::Objects,
                        Some("AcDb:AcDbObjects"),
                        format!("Failed to init object reader: {}", e),
                    ),
                    }
                }
                Err(e) => report_read_error(
                    &mut self.notifications,
                    &mut diagnostics,
                    "objects-extract-failed",
                    ReadStage::Objects,
                    Some("AcDb:AcDbObjects"),
                    format!("Failed to extract objects section: {}", e),
                ),
            }
        }
        self.report_progress(930);
        if perf {
            eprintln!(
                "[perf] dwg-read objects={:.1}ms handles={} entities={} objects={}",
                objects_started.elapsed().as_secs_f64() * 1000.0,
                handle_count,
                document.entities().count(),
                document.objects.len(),
            );
        }

        // 6. R2013+ (AC1027+): 3DSOLID / REGION / BODY ACIS geometry is not
        //    stored inline — it lives as SAB blobs in the AcDs (Autodesk Data
        //    Store) section. Extract those blobs and attach them, in document
        //    order, to the modeler-geometry entities that arrived with only a
        //    stub. Files without an AcDs section keep their inline data.
        if let Ok(acds_buf) = self.get_section_buffer("AcDb:AcDsPrototype_1b", &info) {
            // Authoritative: the `_data_` record table(s) bind each blob to its
            // owning handle. When present, attach by handle — the only mapping
            // that survives BIM exports whose blob/record/handle orders diverge.
            let modeler_handles: HashSet<u64> = document
                .entities()
                .filter(|entity| {
                    matches!(
                        entity,
                        crate::entities::EntityType::Solid3D(_)
                            | crate::entities::EntityType::Region(_)
                            | crate::entities::EntityType::Body(_)
                            | crate::entities::EntityType::Surface(_)
                    ) && entity.common().has_ds_data
                })
                .map(|entity| entity.common().handle.value())
                .collect();
            let record_blobs = extract_acds_record_blobs(&acds_buf, &modeler_handles);
            let attached = if !record_blobs.is_empty() {
                attach_acds_record_blobs(&mut document, record_blobs)
            } else {
                // No parseable record table: magic-scan the blobs and pair by
                // the object-stream handle list, else positionally.
                let blobs = extract_acds_sab_blobs(&acds_buf);
                if blobs.is_empty() {
                    0
                } else {
                    attach_acds_sab_blobs(&mut document, blobs)
                }
            };
            let fingerprint = super::sab_fingerprint(document.entities().filter_map(|entity| {
                let acis = match entity {
                    crate::entities::EntityType::Solid3D(entity) => &entity.acis_data,
                    crate::entities::EntityType::Region(entity) => &entity.acis_data,
                    crate::entities::EntityType::Body(entity) => &entity.acis_data,
                    crate::entities::EntityType::Surface(entity) => &entity.acis_data,
                    _ => return None,
                };
                (!acis.sab_data.is_empty())
                    .then_some((entity.common().handle, acis.sab_data.as_slice()))
            }));
            // §19 H5a: gold's `AcDs` structure view — the section outline
            // (the 13 header fields + the segment index + the per-type
            // sub-blocks). A fetched-but-unreadable section still projects
            // a zeroed header, as gold's JSON does.
            let acds_summary =
                crate::io::dwg::acds::parse_acds_section(&acds_buf).unwrap_or_default();
            document.raw_acds_data = Some(std::sync::Arc::new(acds_buf));
            document.raw_acds_fingerprint = fingerprint;
            document.dwg_acds = Some(acds_summary);
            if attached > 0 {
                self.notifications.notify(
                    NotificationType::Warning,
                    format!(
                        "AcDs: attached {} SAB blob(s) to modeler entities",
                        attached
                    ),
                );
            }
        } else if crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
            .map(|v| v.r2004_plus())
            .unwrap_or(false)
        {
            // §19 H5a: gold's AcDs emission is UNCONDITIONAL on the
            // R2004+ arm (out_json.c:2675) — a file without the section
            // still prints the zeroed 13-field header (2004/Line pinned:
            // all-zero values, no segidx/segments keys).
            document.dwg_acds = Some(Default::default());
        }
        self.report_progress(970);

        // 7. Preview / thumbnail image. Best-effort: a malformed or absent
        //    preview leaves `document.preview` as `None`. The container
        //    source is version-split (§19 H5c, gold-pinned):
        //    - R2004+ (incl. AC1021): gold reads the thumbnail ONLY from
        //      the decompressed AcDb:Preview section
        //      (read_2004_section_preview / decode_R2007 — gold size =
        //      sec−16 there, sec−32 on AC1021). The raw bytes at the
        //      preview seeker are not the container on compressed stores
        //      (the 17 AC1032 corpus files: compressed bytes whose
        //      overall-size field still passes the allocation guard), and
        //      the overall-window length formula truncates sections whose
        //      author undershot the window (2013/RAY: container_len
        //      (overall) = 1076 vs the true 1114-byte section). Fetch the
        //      section first there; keep the raw read as the fallback.
        //    - Pre-R2004: the container is sentinel-bracketed at the raw
        //      recorded address and the raw read is exact there.
        if info.preview_address > 0 {
            let base = info.preview_address as u64;
            let r2004_plus =
                crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
                    .map(|v| v.r2004_plus())
                    .unwrap_or(false);
            let mut container: Option<Vec<u8>> = None;
            if r2004_plus {
                if let Ok(buf) = self.get_section_buffer(
                    crate::io::dwg::file_headers::section_definition::names::PREVIEW,
                    &info,
                ) {
                    container = Some(buf);
                }
            }
            if container.is_none() {
                if self.stream.seek(SeekFrom::Start(base)).is_ok() {
                    let mut head = [0u8; 20];
                    if self.stream.read_exact(&mut head).is_ok() {
                        if let Some(overall) = crate::io::dwg::preview::overall_size(&head) {
                            // Guard against a garbage size before allocating.
                            if overall > 0 && overall < 64 * 1024 * 1024 {
                                let total = crate::io::dwg::preview::container_len(overall);
                                let mut buf = vec![0u8; total];
                                if self.stream.seek(SeekFrom::Start(base)).is_ok()
                                    && self.stream.read_exact(&mut buf).is_ok()
                                {
                                    container = Some(buf);
                                }
                            }
                        }
                    }
                }
            }
            // Parse the container. A fetched container with NO decodable
            // image descriptor (the header-only previews — the 80-byte
            // reserved block with no BMP/WMF/PNG behind it) still carries
            // gold's THUMBNAILIMAGE size/chain, so retain the raw bytes
            // with empty `data` either way; the writer treats empty data
            // exactly like a missing preview.
            let parsed = container
                .as_ref()
                .and_then(|buf| crate::io::dwg::preview::parse_preview(buf, base));
            match (container, parsed) {
                (Some(buf), Some(p)) => {
                    // Retain the raw container for the §19 H5c structure
                    // projection (gold's THUMBNAILIMAGE size/chain).
                    document.preview = Some(crate::document::Preview { raw: buf, ..p });
                }
                (Some(buf), None) => {
                    document.preview = Some(crate::document::Preview {
                        format: crate::document::PreviewFormat::Unknown,
                        data: Vec::new(),
                        raw: buf,
                    });
                }
                _ => {}
            }
        }

        // Record the source path (Filename / FilePath fields) when opened from a
        // file rather than a bare stream.
        document.source_path = self.source_path.clone();

        // Document summary information (Author/Title/Subject/… → the
        // Document-category dynamic-text fields).
        if let Ok(buf) = self.get_section_buffer("AcDb:SummaryInfo", &info) {
            let utf16 = crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
                .map(|v| v.r2007_plus())
                .unwrap_or(true);
            document.summary_info = parse_summary_info(&buf, utf16);
        }

        // The §19 H4 metadata sections (gold-JSON-shaped summaries for
        // the structure axis; every section optional — a missing section
        // is a skip, never an error).
        self.read_metadata_sections(&info, dxf_version, &mut document);

        // R2000/R14 down-saved gradient hatches store their gradient in the
        // ACAD round-trip mechanism, not the object stream (the DWG gradient
        // block is R2004+): the two colours in EED (GradientColor1ACI /
        // GradientColor2ACI) and the gradient type in an ACAD_XREC_ROUNDTRIP
        // XRecord under the hatch's extension dictionary. Reconstruct the
        // gradient so it renders as AutoCAD/ODA show it instead of a flat
        // solid fill.
        //
        // Gate strictly to pre-R2004: a native R2004+ file carries the real
        // gradient in its hatch object (read_hatch reads `is_gradient` and the
        // colours directly), so that flag is authoritative. Such hatches often
        // ALSO keep stale GradientColor1/2ACI round-trip EED from an earlier
        // edit; recovering from it would resurrect a gradient the drawing has
        // since turned off (native `is_gradient` = 0), painting a solid-fill
        // hatch as a spurious gradient. Only R2000/R14, whose object stream has
        // no gradient block, need the round-trip fallback.
        let pre_r2004 = crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
            .map(|v| !v.r2004_plus())
            .unwrap_or(false);
        if pre_r2004 {
            recover_roundtrip_gradients(&mut document);
            // Pre-R2004 stores an MTEXT background fill as round-trip EED
            // instead of the native codes (dimension text fills, etc.).
            recover_mtext_bg_roundtrip(&mut document);
        }

        // 7. Preview / thumbnail image. Best-effort: a malformed or absent
        //    preview leaves `document.preview` as `None`. The container
        //    source is version-split (§19 H5c, gold-pinned):
        //    - R2004+ (incl. AC1021): gold reads the thumbnail ONLY from
        //      the decompressed AcDb:Preview section
        //      (read_2004_section_preview / decode_R2007 — gold size =
        //      sec−16 there, sec−32 on AC1021). The raw bytes at the
        //      preview seeker are not the container on compressed stores
        //      (the 17 AC1032 corpus files: compressed bytes whose
        //      overall-size field still passes the allocation guard), and
        //      the overall-window length formula truncates sections whose
        //      author undershot the window (2013/RAY: container_len
        //      (overall) = 1076 vs the true 1114-byte section). Fetch the
        //      section first there; keep the raw read as the fallback.
        //    - Pre-R2004: the container is sentinel-bracketed at the raw
        //      recorded address and the raw read is exact there.
        if info.preview_address > 0 {
            let base = info.preview_address as u64;
            let r2004_plus =
                crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
                    .map(|v| v.r2004_plus())
                    .unwrap_or(false);
            let mut container: Option<Vec<u8>> = None;
            if r2004_plus {
                if let Ok(buf) = self.get_section_buffer(
                    crate::io::dwg::file_headers::section_definition::names::PREVIEW,
                    &info,
                ) {
                    container = Some(buf);
                }
            }
            if container.is_none() {
                if self.stream.seek(SeekFrom::Start(base)).is_ok() {
                    let mut head = [0u8; 20];
                    if self.stream.read_exact(&mut head).is_ok() {
                        if let Some(overall) = crate::io::dwg::preview::overall_size(&head) {
                            // Guard against a garbage size before allocating.
                            if overall > 0 && overall < 64 * 1024 * 1024 {
                                let total = crate::io::dwg::preview::container_len(overall);
                                let mut buf = vec![0u8; total];
                                if self.stream.seek(SeekFrom::Start(base)).is_ok()
                                    && self.stream.read_exact(&mut buf).is_ok()
                                {
                                    container = Some(buf);
                                }
                            }
                        }
                    }
                }
            }
            // Parse the container. A fetched container with NO decodable
            // image descriptor (the header-only previews — the 80-byte
            // reserved block with no BMP/WMF/PNG behind it) still carries
            // gold's THUMBNAILIMAGE size/chain, so retain the raw bytes
            // with empty `data` either way; the writer treats empty data
            // exactly like a missing preview.
            let parsed = container
                .as_ref()
                .and_then(|buf| crate::io::dwg::preview::parse_preview(buf, base));
            match (container, parsed) {
                (Some(buf), Some(p)) => {
                    // Retain the raw container for the §19 H5c structure
                    // projection (gold's THUMBNAILIMAGE size/chain).
                    document.preview = Some(crate::document::Preview { raw: buf, ..p });
                }
                (Some(buf), None) => {
                    document.preview = Some(crate::document::Preview {
                        format: crate::document::PreviewFormat::Unknown,
                        data: Vec::new(),
                        raw: buf,
                    });
                }
                _ => {}
            }
        }

        // Record the source path (Filename / FilePath fields) when opened from a
        // file rather than a bare stream.
        document.source_path = self.source_path.clone();

        // Document summary information (Author/Title/Subject/… → the
        // Document-category dynamic-text fields).
        if let Ok(buf) = self.get_section_buffer("AcDb:SummaryInfo", &info) {
            let utf16 = crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
                .map(|v| v.r2007_plus())
                .unwrap_or(true);
            document.summary_info = parse_summary_info(&buf, utf16);
        }

        // The §19 H4 metadata sections (both read flows).
        self.read_metadata_sections(&info, dxf_version, &mut document);

        // §19 H7 CLASSES row: capture the read-time state hash (the
        // ordered class identity + the document's per-class object
        // census) guarding the verbatim re-emission — the write-time
        // gate recomputes it and falls back to the sane encoding when
        // the class table or any class's census changed.
        document.raw_classes_fingerprint = super::classes_state_fingerprint(&document);

        // Transfer reader notifications to the document so callers can
        // inspect them via `document.notifications`.
        document
            .notifications
            .extend(std::mem::take(&mut self.notifications));

        if perf {
            eprintln!(
                "[perf] dwg-read total={:.1}ms entities={} objects={} classes={}",
                total_started.elapsed().as_secs_f64() * 1000.0,
                document.entities().count(),
                document.objects.len(),
                document.classes.iter().count(),
            );
        }
        self.report_progress(1000);
        let source_sections =
            usize::from(handles_section_read).saturating_add(usize::from(objects_section_read));
        let stats = crate::io::read::ReadStats::from_document(
            &document,
            SourceFormat::Dwg,
            source_sections,
            handle_count,
            decoded_source_records,
            skipped_source_records,
            record_stream_read,
            failsafe,
            true,
            diagnostics,
        );
        Ok(crate::io::read::ReadOutcome::new(document, stats))
    }

    /// Read the file header and extract all CRC values.
    ///
    /// For AC1021 files, this extracts:
    /// - The Header CRC-64 from the compressed metadata
    /// - Page map CRC values
    /// - Section map CRC values
    /// - Per-page CRC values
    ///
    /// # Returns
    /// A `DwgFileHeaderInfo` containing all extracted data.
    pub fn read_file_header(&mut self) -> Result<DwgFileHeaderInfo, DxfError> {
        self.stream.seek(SeekFrom::Start(0))?;

        // Read version string (6 bytes)
        let mut version_buf = [0u8; 6];
        self.stream.read_exact(&mut version_buf)?;
        let version_string = String::from_utf8_lossy(&version_buf).to_string();

        let version = DwgVersion::from_version_string(&version_string)
            .ok_or_else(|| DxfError::UnsupportedVersion(version_string.clone()))?;

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "Reading DWG file version: {} ({:?})",
                version_string, version
            ),
        );

        let mut info = DwgFileHeaderInfo {
            version_string,
            version,
            acad_maintenance_version: 0,
            preview_address: 0,
            dwg_version: 0,
            app_release_version: 0,
            code_page: 0,
            security_type: 0,
            summary_info_addr: 0,
            vba_project_addr: 0,
            zero_one_or_three: 0,
            maint_version: 0,
            sections: 0,
            unknown_0: 0,
            app_dwg_version: 0,
            app_maint_version: 0,
            rl_1c_address: 0,
            r2004_header_address: 0,
            r2004_system: None,
            second_header: None,
            aux_header: None,
            ac21_metadata: None,
            ac21_header_crc: None,
            ac21_unknown_key: None,
            ac21_compressed_data_crc: None,
            page_records: HashMap::new(),
            section_descriptors: Vec::new(),
            ac21_map_order: Vec::new(),
            ac21_shape: None,
            section_locators: HashMap::new(),
            objects_base_offset: 0,
            is_ac18_format: false,
            ac18_map_order: Vec::new(),
            ac18_section_shapes: Vec::new(),
        };

        match version {
            DwgVersion::AC21 => {
                self.read_file_metadata(&mut info)?;
                self.read_file_header_ac21(&mut info)?;
            }
            DwgVersion::AC18 | DwgVersion::AC24 => {
                self.read_file_metadata(&mut info)?;
                self.read_file_header_ac18(&mut info)?;
            }
            _ => {
                // AC15 format (R13/R14/R2000) — linear file with section locator records
                self.read_file_header_ac15(&mut info)?;
            }
        }

        Ok(info)
    }

    /// Read common file metadata shared between AC18 and AC21 formats.
    ///
    /// This reads bytes 6–0xFF of the file (after the version string).
    fn read_file_metadata(&mut self, info: &mut DwgFileHeaderInfo) -> Result<(), DxfError> {
        // Skip 5 bytes after version string
        let mut skip = [0u8; 5];
        self.stream.read_exact(&mut skip)?;

        // Maintenance version (1 byte)
        info.acad_maintenance_version = self.stream.read_u8()?;

        // Gold's `zero_one_or_three` (1 byte — previously "skip 1")
        info.zero_one_or_three = self.stream.read_u8()?;

        // Preview address (4 bytes)
        info.preview_address = self.stream.read_i32::<LittleEndian>()?;

        // DWG version (1 byte)
        info.dwg_version = self.stream.read_u8()?;

        // App release version (1 byte) — this byte IS gold's `maint_version`
        // (the same position ledger as the R2000 path: byte 18)
        info.app_release_version = self.stream.read_u8()?;
        info.maint_version = info.app_release_version;

        // Drawing code page (2 bytes)
        info.code_page = self.stream.read_u16::<LittleEndian>()?;

        // Gold's R2004+ tail (previously "skip 3"): unknown_0 (1 byte),
        // app_dwg_version (1 byte), app_maint_version (1 byte)
        info.unknown_0 = self.stream.read_u8()?;
        info.app_dwg_version = self.stream.read_u8()?;
        info.app_maint_version = self.stream.read_u8()?;

        // Security type (4 bytes)
        info.security_type = self.stream.read_i32::<LittleEndian>()?;

        // Gold's `rl_1c_address` (4 bytes — previously "skip unknown")
        info.rl_1c_address = self.stream.read_i32::<LittleEndian>()?;

        // Summary info address (4 bytes)
        info.summary_info_addr = self.stream.read_i32::<LittleEndian>()?;

        // VBA project address (4 bytes)
        info.vba_project_addr = self.stream.read_i32::<LittleEndian>()?;

        // Gold's `r2004_header_address` (4 bytes — previously the first
        // half of the "skip 2 unknown ints" tail)
        info.r2004_header_address = self.stream.read_i32::<LittleEndian>()?;
        // and the remaining 4 stub bytes of the old 8-byte skip
        self.stream.read_i32::<LittleEndian>()?;

        // Skip 80 bytes of trailing padding — the byte ledger is
        // unchanged: the r2004_header_address+stub pair above covers
        // exactly the old 8-byte skip, and this lands the cursor at the
        // 0x100 file-header end (byte 128) exactly as before.
        let mut pad = [0u8; 80];
        self.stream.read_exact(&mut pad)?;

        Ok(())
    }

    /// Read AC15 (R13/R14/R2000) file header with section locator records.
    ///
    /// The AC15 file header is 0x61 (97) bytes:
    /// ```text
    /// [0x00] Version string (6 bytes)
    /// [0x06] Padding + maintenance version (7 bytes)
    /// [0x0D] Preview seeker (4 bytes)
    /// [0x11] Magic bytes (2 bytes: 0x1B, 0x19)
    /// [0x13] Code page (2 bytes LE)
    /// [0x15] Record count (4 bytes LE) — always 6
    /// [0x19] 6 × Section locator records (9 bytes each)
    /// [0x4F] CRC-16 (2 bytes)
    /// [0x51] End sentinel (16 bytes)
    /// [0x61] End of header → section data starts
    /// ```
    ///
    /// Section numbers: 0=Header, 1=Classes, 2=Handles,
    /// 3=ObjFreeSpace, 4=Template, 5=AuxHeader.
    /// AcDbObjects is not in the locator table — its position is
    /// inferred from the gap between the Classes section end and the
    /// Handles section start (see the calculation below for why the
    /// AuxHeader position is only used as a legacy fallback).
    fn read_file_header_ac15(&mut self, info: &mut DwgFileHeaderInfo) -> Result<(), DxfError> {
        use crate::io::dwg::file_headers::section_definition::names;

        // Seek past the version string (6 bytes already read)
        self.stream.seek(SeekFrom::Start(6))?;

        // 0x06: 5 zero bytes + maintenance version + 1 unknown byte (7 bytes total)
        let mut pad = [0u8; 5];
        self.stream.read_exact(&mut pad)?;
        info.acad_maintenance_version = self.stream.read_u8()?;
        // Byte 12 — gold's `zero_one_or_three` (pinned by hand-decoding
        // sample_2000: gold's observed 1 sits at exactly this byte; the
        // historical "unknown" label was wrong)
        info.zero_one_or_three = self.stream.read_u8()?;

        // 0x0D: Preview seeker (4 bytes LE)
        info.preview_address = self.stream.read_i32::<LittleEndian>()?;

        // 0x11/0x12: gold's `dwg_version` (byte 17 — the historical "magic
        // 0x1B/0x19" was gold's dwg_version=25 on sample_2000) and
        // `maint_version` (byte 18)
        info.dwg_version = self.stream.read_u8()?;
        info.maint_version = self.stream.read_u8()?;
        info.app_release_version = info.maint_version;

        // 0x13: Code page (2 bytes LE)
        info.code_page = self.stream.read_u16::<LittleEndian>()?;

        // 0x15: Number of locator records (4 bytes LE) — should be 6.
        // This IS gold's FILEHEADER `sections` field (sample_2000:
        // gold's "sections": 6 = this count).
        let record_count = self.stream.read_i32::<LittleEndian>()?;
        info.sections = record_count;

        // 0x19: Read locator records
        // Each record: number(1) + seeker(4) + size(4) = 9 bytes
        let section_name_for = |n: u8| -> &str {
            match n {
                0 => names::HEADER,
                1 => names::CLASSES,
                2 => names::HANDLES,
                3 => names::OBJ_FREE_SPACE,
                4 => names::TEMPLATE,
                5 => names::AUX_HEADER,
                _ => "Unknown",
            }
        };

        let mut handles_seeker: i64 = 0;
        let mut aux_header_end: i64 = 0;
        let mut classes_end: i64 = 0;

        for _ in 0..record_count.min(6) {
            let number = self.stream.read_u8()?;
            let seeker = self.stream.read_i32::<LittleEndian>()? as i64;
            let size = self.stream.read_i32::<LittleEndian>()? as i64;

            let name = section_name_for(number);
            info.section_locators
                .insert(name.to_string(), (seeker, size));

            // Track offsets for AcDbObjects position calculation
            if number == 1 {
                // Classes section
                classes_end = seeker + size;
            }
            if number == 2 {
                // Handles section
                handles_seeker = seeker;
            }
            if number == 5 {
                // AuxHeader — objects start right after this
                aux_header_end = seeker + size;
            }
        }

        // Calculate AcDbObjects position.
        //
        // AcDbObjects is not in the locator table. It occupies the gap between
        // the end of the Classes section and the start of the Handles section,
        // regardless of where Template/AuxHeader are physically placed: many
        // real-world R2000 files store Template/AuxHeader *after* Handles (and
        // acadrust's own R13/R14 writer places ObjFreeSpace/Template after
        // Handles too), so inferring the region from the AuxHeader end yields
        // a negative size and an empty document (issue #55).
        //
        // Handle-map offsets are absolute file positions, so starting the
        // region at classes_end is always safe: the buffer may then include a
        // few leading non-object bytes (e.g. ObjFreeSpace/Template), but those
        // are never referenced by any handle offset.
        let objects_start: i64;
        let objects_size: i64;
        if classes_end > 0 && handles_seeker > classes_end {
            objects_start = classes_end;
            objects_size = handles_seeker - classes_end;
        } else {
            // Malformed Classes locator — fall back to the legacy inference:
            // objects start right after the AuxHeader, or, when AuxHeader is
            // missing (R13/R14 layout), at 0x61 + the sum of all known
            // sections that precede it.
            if aux_header_end == 0 {
                let file_header_size: i64 = 0x61;
                let mut offset = file_header_size;
                for &sect in &[
                    names::HEADER,
                    names::CLASSES,
                    names::OBJ_FREE_SPACE,
                    names::TEMPLATE,
                    names::AUX_HEADER,
                ] {
                    if let Some(&(_, size)) = info.section_locators.get(sect) {
                        offset += size;
                    }
                }
                aux_header_end = offset;
            }
            objects_start = aux_header_end;
            objects_size = handles_seeker - aux_header_end;
        }

        if objects_size > 0 {
            info.section_locators.insert(
                names::ACDB_OBJECTS.to_string(),
                (objects_start, objects_size),
            );
            info.objects_base_offset = objects_start;
        }

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC15 file header: {} locator records, objects at offset {}, size {}",
                record_count, objects_start, objects_size
            ),
        );

        // The R13-R2000 structural pair (§19 H2): the AuxHeader at the
        // section locator (gold: decode.c:373-405, gated on sections==6)
        // and the sentinel-located SecondHeader (gold: decode.c:907).
        // Both parse into gold's JSON shapes for the structure axis;
        // failures are non-fatal (the sections are informational).
        if info.sections == 6 {
            if let Err(e) = self.read_aux_header_r13(info) {
                self.notifications.notify(
                    NotificationType::Warning,
                    format!("AuxHeader read failed (non-fatal): {}", e),
                );
            }
        }
        if let Err(e) = self.read_second_header_r13(info) {
            self.notifications.notify(
                NotificationType::Warning,
                format!("SecondHeader read failed (non-fatal): {}", e),
            );
        }

        Ok(())
    }

    /// Read the R13c3+ AuxHeader at its section-locator address — gold's
    /// `AuxHeader` shape (§19 H2). Byte-aligned fields per `auxheader.spec`
    /// (no sentinels since R13c3; gold decode.c:373-405). The field order
    /// was hand-decoded byte-for-byte against gold's JSON on sample_2000
    /// before implementation (every field matched).
    fn read_aux_header_r13(
        &mut self,
        info: &mut DwgFileHeaderInfo,
    ) -> Result<(), DxfError> {
        use std::io::Cursor as IoCursor;
        let (address, size) = info
            .section_locators
            .get(crate::io::dwg::file_headers::section_definition::names::AUX_HEADER)
            .copied()
            .ok_or_else(|| DxfError::Parse("AuxHeader locator missing".into()))?;
        if address < 0 || size <= 0 {
            return Err(DxfError::Parse("AuxHeader locator invalid".into()));
        }
        self.stream.seek(SeekFrom::Start(address as u64))?;
        let mut buf = vec![0u8; size as usize];
        self.stream.read_exact(&mut buf)?;
        let mut c = IoCursor::new(buf);

        let rc = |c: &mut IoCursor<Vec<u8>>| -> Result<u8, DxfError> {
            use std::io::Read;
            let mut b = [0u8; 1];
            c.read_exact(&mut b)?;
            Ok(b[0])
        };
        let rs = |c: &mut IoCursor<Vec<u8>>| -> Result<i16, DxfError> {
            use std::io::Read;
            let mut b = [0u8; 2];
            c.read_exact(&mut b)?;
            Ok(i16::from_le_bytes(b))
        };
        let rl = |c: &mut IoCursor<Vec<u8>>| -> Result<i32, DxfError> {
            use std::io::Read;
            let mut b = [0u8; 4];
            c.read_exact(&mut b)?;
            Ok(i32::from_le_bytes(b))
        };

        let aux_intro = vec![rc(&mut c)?, rc(&mut c)?, rc(&mut c)?];
        let dwg_version = rs(&mut c)?;
        let maint_version = rs(&mut c)?;
        let numsaves = rl(&mut c)?;
        let minus_1 = rl(&mut c)?;
        let numsaves_1 = rs(&mut c)?;
        let numsaves_2 = rs(&mut c)?;
        let zero = rl(&mut c)?;
        let dwg_version_1 = rs(&mut c)?;
        let maint_version_1 = rs(&mut c)?;
        let dwg_version_2 = rs(&mut c)?;
        let maint_version_2 = rs(&mut c)?;
        let unknown_6rs = (0..6).map(|_| rs(&mut c)).collect::<Result<Vec<_>, _>>()?;
        let unknown_5rl = (0..5).map(|_| rl(&mut c)).collect::<Result<Vec<_>, _>>()?;
        // TIMERLL: days + milliseconds (2 x raw long, unsigned print)
        let tdcreate = {
            let days = rl(&mut c)? as u32;
            let ms = rl(&mut c)? as u32;
            vec![days, ms]
        };
        let tdupdate = {
            let days = rl(&mut c)? as u32;
            let ms = rl(&mut c)? as u32;
            vec![days, ms]
        };
        let mut handseed_bytes = [0u8; 8];
        {
            use std::io::Read;
            c.read_exact(&mut handseed_bytes)?;
        }
        let handseed = u64::from_le_bytes(handseed_bytes);
        let zero_1 = rs(&mut c)?;
        let numsaves_3 = rs(&mut c)?;
        let zero_2 = rl(&mut c)?;
        let zero_3 = rl(&mut c)?;
        let zero_4 = rl(&mut c)?;
        let numsaves_4 = rl(&mut c)?;
        let zero_5 = rl(&mut c)?;
        let zero_6 = rl(&mut c)?;

        info.aux_header = Some(crate::document::DwgAuxHeaderSummary {
            aux_intro,
            dwg_version,
            maint_version,
            numsaves,
            minus_1,
            numsaves_1,
            numsaves_2,
            zero,
            dwg_version_1,
            maint_version_1,
            dwg_version_2,
            maint_version_2,
            unknown_6rs,
            unknown_5rl,
            tdcreate,
            tdupdate,
            handseed,
            zero_1,
            numsaves_3,
            zero_2,
            zero_3,
            zero_4,
            numsaves_4,
            zero_5,
            zero_6,
        });
        Ok(())
    }

    /// Read the R13–R2000 SecondHeader — gold's `SecondHeader` shape
    /// (§19 H2). Located by the 2NDHEADER_BEGIN sentinel (gold searches
    /// forward from the ObjFreeSpace read position, decode.c:907; the
    /// sentinel is unique, so the search starts at the ObjFreeSpace
    /// locator address with a 0 fallback). Parsed per `2ndheader.spec`
    /// via `secondheader_private`: RL size, BL address, 11-byte version,
    /// RC maint_rel_version, RC zero_one_or_three, BS dwg_versions,
    /// RS codepage, BS num_sections (≤6) + the section records
    /// (RC nr, BL address, BL size), BS num_handles (≤14) + the handle
    /// records (RC num_hdl ≤8, RC nr, num_hdl raw bytes), the trailing
    /// RS CRC (not printed), and `junk_r14` (RLL) on R14/R2000 only.
    fn read_second_header_r13(
        &mut self,
        info: &mut DwgFileHeaderInfo,
    ) -> Result<(), DxfError> {
        const SENTINEL_2NDHEADER_BEGIN: [u8; 16] = [
            0xD4, 0x7B, 0x21, 0xCE, 0x28, 0x93, 0x9F, 0xBF, 0x53, 0x24, 0x40, 0x09,
            0x12, 0x3C, 0xAA, 0x01,
        ];
        let search_start = info
            .section_locators
            .get(crate::io::dwg::file_headers::section_definition::names::OBJ_FREE_SPACE)
            .map(|&(address, _)| address.max(0) as u64)
            .unwrap_or(0);
        let file_len = self.stream.seek(SeekFrom::End(0))?;
        let search_start = search_start.min(file_len);
        self.stream.seek(SeekFrom::Start(search_start))?;
        let mut tail = vec![0u8; (file_len - search_start) as usize];
        self.stream.read_exact(&mut tail)?;

        let sentinel_pos = tail
            .windows(16)
            .position(|w| w == SENTINEL_2NDHEADER_BEGIN)
            .ok_or_else(|| DxfError::Parse("2NDHEADER sentinel not found".into()))?;

        let dxf_version = crate::types::DxfVersion::parse(&info.version_string)
            .unwrap_or(crate::types::DxfVersion::Unknown);
        let dwg_version = info.version;
        let encoding =
            crate::io::dxf::code_page::encoding_from_dwg_code_page(info.code_page);
        let mut reader =
            crate::io::dwg::dwg_stream_readers::bit_reader::DwgBitReader::with_encoding(
                tail[sentinel_pos + 16..].to_vec(),
                dwg_version,
                dxf_version,
                encoding,
            );

        let size = reader.read_raw_long() as i32;
        let address = reader.read_bit_long() as u32;
        let mut version_bytes = [0u8; 11];
        for b in version_bytes.iter_mut() {
            *b = reader.read_byte();
        }
        let version_end = version_bytes.iter().position(|&b| b == 0).unwrap_or(11);
        let version = String::from_utf8_lossy(&version_bytes[..version_end]).to_string();
        let maint_rel_version = reader.read_byte();
        let zero_one_or_three = reader.read_byte();
        let dwg_versions = reader.read_bit_short();
        let codepage = reader.read_raw_short();
        let num_sections = reader.read_bit_short().clamp(0, 6);
        let mut sections = Vec::with_capacity(num_sections as usize);
        for _ in 0..num_sections {
            let nr = reader.read_byte();
            let address = reader.read_bit_long() as u32;
            let size = reader.read_bit_long() as u32;
            sections.push(crate::document::DwgSecondHeaderSection { nr, address, size });
        }
        let num_handles = reader.read_bit_short().clamp(0, 14);
        let mut handles = Vec::with_capacity(num_handles as usize);
        for _ in 0..num_handles {
            let num_hdl = reader.read_byte().min(8);
            let nr = reader.read_byte();
            let mut hdl = Vec::with_capacity(num_hdl as usize);
            for _ in 0..num_hdl {
                hdl.push(reader.read_byte());
            }
            handles.push(crate::document::DwgSecondHeaderHandle { nr, hdl });
        }
        let _crc = reader.read_raw_short();
        // junk_r14: RLL, R14/R2000 only (VERSIONS (R_14, R_2000) in
        // secondheader_private — R13 files stop at the CRC).
        let junk_r14 = if matches!(dxf_version, crate::types::DxfVersion::AC1014 | crate::types::DxfVersion::AC1015)
        {
            let mut b = [0u8; 8];
            for slot in b.iter_mut() {
                *slot = reader.read_byte();
            }
            u64::from_le_bytes(b)
        } else {
            0
        };

        info.second_header = Some(crate::document::DwgSecondHeaderSummary {
            size,
            address,
            version,
            maint_rel_version,
            zero_one_or_three,
            dwg_versions,
            codepage,
            sections,
            handles,
            junk_r14,
        });
        Ok(())
    }

    /// Read the §19 H4 metadata sections into their gold-JSON-shaped
    /// summaries (Template, ObjFreeSpace, FileDepList, RevHistory,
    /// Security, AppInfo, AppInfoHistory). Every section is optional:
    /// a missing section (no locator / no map entry) is a skip, never
    /// an error — gold's own emission gates (the FILEHEADER address
    /// fields and the R2000 locator counts) make presence file-driven,
    /// and the structure axis compares only what both sides carry.
    fn read_metadata_sections(
        &mut self,
        info: &DwgFileHeaderInfo,
        dxf_version: crate::types::DxfVersion,
        document: &mut crate::document::CadDocument,
    ) {
        use crate::io::dwg::file_headers::section_definition::names;
        let utf16 = crate::io::dwg::dwg_version::DwgVersion::from_dxf_version(dxf_version)
            .map(|v| v.r2007_plus())
            .unwrap_or(true);
        // ObjFreeSpace's 128-bit max split is R2010+ (objfreespace.spec's
        // UNTIL (R_2007) branch — inclusive — covers R2000/R2004/R2007).
        let r2010_plus = matches!(
            dxf_version,
            crate::types::DxfVersion::AC1024
                | crate::types::DxfVersion::AC1027
                | crate::types::DxfVersion::AC1032
        );
        // The R2004+ emission arm (out_json.c's `dat->version >= R_2004`
        // gate) — the metadata sections exist there (or print zeroed).
        let r2004_plus = matches!(
            dxf_version,
            crate::types::DxfVersion::AC1018
                | crate::types::DxfVersion::AC1021
                | crate::types::DxfVersion::AC1024
                | crate::types::DxfVersion::AC1027
                | crate::types::DxfVersion::AC1032
        );

        // The R2004+-only sections. Gold emits these UNCONDITIONALLY on
        // R2004+ files (out_json.c's R_2004 arm has no address gates for
        // them — only SummaryInfo/VBAProject are gated): when the section
        // is absent from the map, gold prints the ZEROED struct (e.g.
        // Security's 9 zero constants on files without the section —
        // pinned by sample_2018 whose map carries only the 13 core
        // names). Parsing the empty buffer reproduces the zeroed
        // emission exactly. On R2000 the sections do not exist at all
        // (locator-gated emission there), so the fetch-failure skip is
        // the correct gate.
        if r2004_plus {
            let buf = self
                .get_section_buffer(names::FILE_DEP_LIST, info)
                .unwrap_or_default();
            document.dwg_file_dep_list = Some(parse_file_dep_list_section(&buf, utf16));
            let buf = self.get_section_buffer(names::REV_HISTORY, info).unwrap_or_default();
            document.dwg_rev_history = Some(parse_rev_history_section(&buf));
            let buf = self.get_section_buffer(names::SECURITY, info).unwrap_or_default();
            document.dwg_security = Some(parse_security_section(&buf, utf16));
            let buf = self.get_section_buffer(names::APP_INFO, info).unwrap_or_default();
            let ac1021 = dxf_version == crate::types::DxfVersion::AC1021;
            // §19 H7 AppInfo row: retain the raw section bytes for the
            // verbatim same-version re-emission — gold prints the section
            // unconditionally (zeroed when absent), so a source without
            // one must not get a boilerplate section materialized.
            if !buf.is_empty() {
                document.raw_app_info_data = Some(std::sync::Arc::new(buf.clone()));
            }
            document.dwg_app_info = Some(parse_app_info_section(&buf, utf16, ac1021));
            let buf = self
                .get_section_buffer(names::APP_INFO_HISTORY, info)
                .unwrap_or_default();
            // §19 H7 AppInfoHistory row: same verbatim rule — the section
            // was never written before this row.
            if !buf.is_empty() {
                document.raw_app_info_history_data = Some(std::sync::Arc::new(buf.clone()));
            }
            document.dwg_app_info_history = Some(parse_app_info_history_section(&buf));
            // ObjFreeSpace/Template: emitted on R2000 too (locator-gated
            // there), so their zeroed fallback belongs to the R2004+ arm
            // only; the R2000 arm below re-reads them when present.
            let buf = self.get_section_buffer(names::OBJ_FREE_SPACE, info).unwrap_or_default();
            // §19 H7e ObjFreeSpace row: retain the raw section bytes for
            // the verbatim same-version re-emission — the content is the
            // author's (numhandles/TDUPDATE/max pattern words), never
            // derivable from the model.
            if !buf.is_empty() {
                document.raw_obj_free_space_data = Some(std::sync::Arc::new(buf.clone()));
            }
            document.dwg_obj_free_space = Some(parse_obj_free_space_section(&buf, r2010_plus));
            // §19 H7g: the XrefManifest bytes (the R2013+ external-
            // reference table — authored state, unmodeled, unprinted
            // by gold) retained verbatim for the same-version
            // container mirror.
            let buf = self
                .get_section_buffer(
                    crate::io::dwg::file_headers::section_definition::names::XREF_MANIFEST,
                    info,
                )
                .unwrap_or_default();
            if !buf.is_empty() {
                document.raw_xref_manifest_data = Some(std::sync::Arc::new(buf.clone()));
            }
            let buf = self.get_section_buffer(names::TEMPLATE, info).unwrap_or_default();
            document.dwg_template = Some(parse_template_section(&buf, utf16));
        } else {
            // R2000: emission gated on the locator counts (Template
            // sections>=4, ObjFreeSpace sections>=3) — the locator
            // lookup IS the gate.
            if let Ok(buf) = self.get_section_buffer(names::OBJ_FREE_SPACE, info) {
                if !buf.is_empty() {
                    document.raw_obj_free_space_data =
                        Some(std::sync::Arc::new(buf.clone()));
                }
                document.dwg_obj_free_space = Some(parse_obj_free_space_section(&buf, r2010_plus));
            }
            if let Ok(buf) = self.get_section_buffer(names::TEMPLATE, info) {
                document.dwg_template = Some(parse_template_section(&buf, utf16));
            }
        }
    }

    /// Read AC18 (R2004/R2010/R2013/R2018) inner file header, page map, and section map.
    ///
    /// The AC18 format stores a 0x78-byte (120) inner file header at file offset 0x80,
    /// XOR'd with a magic sequence. This header contains pointers to the page map
    /// and section map, which together describe the layout of all section pages.
    fn read_file_header_ac18(&mut self, info: &mut DwgFileHeaderInfo) -> Result<(), DxfError> {
        // Read the 0x78-byte (120) encrypted block at offset 0x80: gold's
        // r2004_file_header.spec reads 108 bytes of fields PLUS the 12-byte
        // padding tail ("the padding is also encrypted, but ODA didn't
        // grok that") as one unmasked region. The 256-byte magic sequence
        // masks cyclically (i % 256), so unmasking the full 120 bytes is
        // byte-ledger-compatible with the historical 0x6C read.
        self.stream.seek(SeekFrom::Start(0x80))?;
        let mut inner = [0u8; 0x78];
        self.stream.read_exact(&mut inner)?;

        // XOR unmask with magic sequence
        apply_magic_sequence(&mut inner);

        // Verify identifier "AcFssFcAJMB\0"
        if &inner[..12] != b"AcFssFcAJMB\0" {
            return Err(DxfError::InvalidFormat(
                "Invalid AC18 inner file header identifier".into(),
            ));
        }

        // Parse the inner file header — gold's field ledger
        // (r2004_file_header.spec): file_ID_string @0x00 (12 bytes,
        // NUL-terminated), header_address/size, x04, the three tree-node
        // gaps, unknown_long (=1), last_section_id @0x28, the two u64
        // addresses, numgaps/numsections, x20/x80/x40,
        // section_map_id @0x50, section_map_address @0x54 (stored =
        // actual − 0x100; gold prints the RAW stored value — the +0x100
        // is decode-side navigation only), section_info_id @0x5C,
        // section_array_size, gap_array_size, crc32 @0x68, and the
        // 12-byte padding @0x6C.
        let mut cursor = Cursor::new(&inner[..]);
        cursor.set_position(0x0C);
        let header_address = cursor.read_i32::<LittleEndian>()?;
        let header_size = cursor.read_i32::<LittleEndian>()?;
        let x04 = cursor.read_i32::<LittleEndian>()?;
        let root_tree_node_gap = cursor.read_i32::<LittleEndian>()?;
        let lowermost_left_tree_node_gap = cursor.read_i32::<LittleEndian>()?;
        let lowermost_right_tree_node_gap = cursor.read_i32::<LittleEndian>()?;
        let unknown_long = cursor.read_i32::<LittleEndian>()?;

        let last_section_id = cursor.read_i32::<LittleEndian>()?;
        let last_section_address = cursor.read_u64::<LittleEndian>()?;
        let secondheader_address = cursor.read_u64::<LittleEndian>()?;
        let numgaps = cursor.read_u32::<LittleEndian>()?;
        let numsections = cursor.read_u32::<LittleEndian>()?;
        let x20 = cursor.read_i32::<LittleEndian>()?;
        let x80 = cursor.read_i32::<LittleEndian>()?;
        let x40 = cursor.read_i32::<LittleEndian>()?;

        let section_map_id_hdr = cursor.read_u32::<LittleEndian>()?;
        let page_map_address_stored = cursor.read_u64::<LittleEndian>()?;
        // Gold's `section_info_id` @0x5C — historically mislabeled
        // section_map_id (the real section_map_id sits @0x50).
        let section_info_id = cursor.read_i32::<LittleEndian>()?;
        let section_array_size = cursor.read_i32::<LittleEndian>()?;
        let gap_array_size = cursor.read_i32::<LittleEndian>()?;
        let crc32 = cursor.read_u32::<LittleEndian>()?;
        let padding = {
            use std::fmt::Write;
            let mut s = String::new();
            for b in &inner[0x6C..0x78] {
                let _ = write!(s, "{:02X}", b);
            }
            s
        };

        info.r2004_system = Some(crate::document::DwgR2004SystemHeader {
            file_ID_string: String::from_utf8_lossy(&inner[..12])
                .trim_end_matches('\0')
                .to_string(),
            header_address,
            header_size,
            x04,
            root_tree_node_gap,
            lowermost_left_tree_node_gap,
            lowermost_right_tree_node_gap,
            unknown_long,
            last_section_id,
            last_section_address,
            secondheader_address,
            numgaps,
            numsections,
            x20,
            x80,
            x40,
            section_map_id: section_map_id_hdr,
            // Gold prints the RAW stored value (pinned by sample_2018:
            // gold 19328 = the raw; stored+0x100 = 19584 is the
            // decode-side navigation address only).
            section_map_address: page_map_address_stored,
            section_info_id,
            section_array_size,
            gap_array_size,
            crc32,
            padding,
        });

        // The stored address is (actual - 0x100)
        let page_map_address = page_map_address_stored + 0x100;
        let section_map_id = section_info_id as u32;

        info.is_ac18_format = true;

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC18 inner header: page_map_address={:#X}, section_map_id={}",
                page_map_address, section_map_id
            ),
        );

        // Read page map (page_number → file_offset mapping)
        self.read_page_map_ac18(info, page_map_address)?;

        // Read section map (section descriptors with per-page info)
        self.read_section_map_ac18(info, section_map_id as i32)?;

        Ok(())
    }

    /// Read the AC18 page map from a known file offset.
    ///
    /// The page map is a system page (20-byte unmasked header + LZ77 data)
    /// containing (page_number, page_size) pairs. File offsets are computed
    /// by accumulating page sizes from offset 0x100.
    fn read_page_map_ac18(
        &mut self,
        info: &mut DwgFileHeaderInfo,
        page_map_address: u64,
    ) -> Result<(), DxfError> {
        self.stream.seek(SeekFrom::Start(page_map_address))?;

        // Read 20-byte system page header (NOT XOR-masked)
        let _section_type = self.stream.read_i32::<LittleEndian>()?;
        let decomp_size = self.stream.read_i32::<LittleEndian>()?;
        let comp_size = self.stream.read_i32::<LittleEndian>()?;
        let compression = self.stream.read_i32::<LittleEndian>()?;
        let _checksum = self.stream.read_u32::<LittleEndian>()?;

        // Read compressed data
        if comp_size <= 0 || comp_size > 10_000_000 {
            return Err(DxfError::InvalidFormat(format!(
                "Invalid AC18 page map compressed size: {}",
                comp_size
            )));
        }
        let mut compressed = vec![0u8; comp_size as usize];
        self.stream.read_exact(&mut compressed)?;

        // Decompress
        let decompressed = if compression == 2 {
            decompress_ac18(&compressed, decomp_size as usize)
        } else {
            compressed
        };

        // Parse (page_number, page_size) pairs.
        // Pages are written sequentially starting at file offset 0x100.
        let mut cursor = Cursor::new(&decompressed);
        let mut file_offset: i64 = 0x100;

        while (cursor.position() as usize) + 8 <= decompressed.len() {
            let page_number = cursor.read_i32::<LittleEndian>()?;
            let page_size = cursor.read_i32::<LittleEndian>()?;

            // Entries with page_size <= 0 are alignment/padding markers
            // emitted by AutoCAD; skip them so they don't corrupt the
            // running file offset.
            if page_size <= 0 {
                continue;
            }

            if page_number > 0 {
                info.page_records
                    .insert(page_number, (file_offset, page_size as i64));
                // §19 H7g: the map order is the author's physical page
                // order — the container-shape mirror's emission order.
                info.ac18_map_order
                    .push(crate::document::DwgAc18PageEntry {
                        id: page_number,
                        on_disk_size: page_size as i64,
                    });
            }
            // Only advance for positive sizes; negative/zero sizes in gap entries are
            // invalid and must not corrupt subsequent page offsets.
            if page_size > 0 {
                file_offset += page_size as i64;
            }
        }

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC18: Read {} page records from page map",
                info.page_records.len()
            ),
        );

        Ok(())
    }

    /// Read the AC18 section map from a system page.
    ///
    /// The section map page describes all logical sections (Header, Classes,
    /// Handles, Objects, etc.) with their compression settings and per-page info.
    fn read_section_map_ac18(
        &mut self,
        info: &mut DwgFileHeaderInfo,
        section_map_id: i32,
    ) -> Result<(), DxfError> {
        let &(page_offset, _) = info.page_records.get(&section_map_id).ok_or_else(|| {
            DxfError::InvalidFormat(format!(
                "AC18 section map page {} not found in page records",
                section_map_id
            ))
        })?;

        self.stream.seek(SeekFrom::Start(page_offset as u64))?;

        // Read 20-byte system page header (NOT XOR-masked)
        let _section_type = self.stream.read_i32::<LittleEndian>()?;
        let decomp_size = self.stream.read_i32::<LittleEndian>()?;
        let comp_size = self.stream.read_i32::<LittleEndian>()?;
        let compression = self.stream.read_i32::<LittleEndian>()?;
        let _checksum = self.stream.read_u32::<LittleEndian>()?;

        // Read compressed data
        if comp_size <= 0 || comp_size > 10_000_000 {
            return Err(DxfError::InvalidFormat(format!(
                "Invalid AC18 section map compressed size: {}",
                comp_size
            )));
        }
        let mut compressed = vec![0u8; comp_size as usize];
        self.stream.read_exact(&mut compressed)?;

        // Decompress
        let decompressed = if compression == 2 {
            decompress_ac18(&compressed, decomp_size as usize)
        } else {
            compressed
        };

        // Parse section descriptors
        let mut cursor = Cursor::new(&decompressed);

        // Header: numDescriptions(4), 0x02(4), 0x7400(4), 0x00(4), numDescriptions(4)
        let num_descriptions = cursor.read_i32::<LittleEndian>()?;
        let _marker = cursor.read_i32::<LittleEndian>()?; // 0x02
        let _max_decomp = cursor.read_i32::<LittleEndian>()?; // 0x7400
        let _unknown = cursor.read_i32::<LittleEndian>()?; // 0x00
        let _num_desc2 = cursor.read_i32::<LittleEndian>()?; // repeat

        for _ in 0..num_descriptions {
            // Per-descriptor: size(8), pageCount(4), maxDecompSize(4),
            //   unknown(4), compressedCode(4), sectionId(4), encrypted(4), name(64)
            let data_size = cursor.read_u64::<LittleEndian>()?;
            let page_count = cursor.read_i32::<LittleEndian>()?;
            let max_decomp_page_size = cursor.read_i32::<LittleEndian>()?;
            let _unknown = cursor.read_i32::<LittleEndian>()?;
            let compressed_code = cursor.read_i32::<LittleEndian>()?;
            // The section TYPE id (gold's DWG_SECTION_TYPE): the only key
            // when the writer left the 64-byte name field empty (the R2004
            // corpus files' AcDs sections).
            let section_id = cursor.read_u32::<LittleEndian>()?;
            let encrypted = cursor.read_i32::<LittleEndian>()?;

            // Section name (64-byte field, null-terminated). Some writers leave
            // non-zero garbage in the bytes *after* the terminator instead of
            // zero-padding, so cut at the first null rather than trimming
            // trailing nulls — otherwise the embedded null plus trailing junk
            // survives and the name fails to match (e.g. "AcDb:Handles\0t…").
            let mut name_buf = [0u8; 64];
            cursor.read_exact(&mut name_buf)?;
            let raw_name = section_name_from_field(&name_buf);
            let mut name = raw_name.clone();
            if name.is_empty() {
                // Nameless descriptor: resolve by the section type id
                // (gold's type-based lookups find these; the R2004 corpus
                // files' AcDs sections carry no name).
                if let Some(type_name) =
                    crate::io::dwg::file_headers::section_definition::names::name_from_section_type(
                        section_id,
                    )
                {
                    name = type_name.to_string();
                }
            }

            // Per-page entries: pageNumber(4), compressedSize(4), offset(8)
            let mut pages = Vec::new();
            for _ in 0..page_count {
                let page_number = cursor.read_i32::<LittleEndian>()?;
                let page_compressed_size = cursor.read_i32::<LittleEndian>()?;
                let page_offset_in_section = cursor.read_u64::<LittleEndian>()?;

                pages.push(DwgPageCrcInfo {
                    page_number: page_number as i64,
                    offset: page_offset_in_section,
                    size: 0,
                    decompressed_size: max_decomp_page_size as u64,
                    compressed_size: page_compressed_size as u64,
                    checksum: 0,
                    crc: 0,
                });
            }

            if !name.is_empty() {
                info.section_descriptors.push(DwgSectionInfo {
                    name: name.clone(),
                    compressed_size: data_size,
                    decompressed_size: max_decomp_page_size as u64,
                    encrypted: encrypted as u64,
                    hash_code: 0,
                    encoding: compressed_code as u64,
                    page_count: page_count as u64,
                    pages: pages.clone(),
                });
            }

            // §19 H7g: retain the descriptor's container shape —
            // every descriptor (the unnamed AcDs ones included; their
            // page lists may be empty), with the raw 64-byte name so
            // the re-emitted table matches the author's bytes.
            info.ac18_section_shapes
                .push(crate::document::DwgAc18SectionShape {
                    name,
                    raw_name,
                    size: data_size,
                    max_decomp: max_decomp_page_size as u32,
                    compressed_code,
                    pages: pages
                        .iter()
                        .map(|p| (p.page_number as i32, p.offset))
                        .collect(),
                });
        }

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC18: Read {} section descriptors from section map",
                info.section_descriptors.len()
            ),
        );

        Ok(())
    }

    /// Read AC1021 (R2007) file header with CRC-64 extraction.
    ///
    /// This performs the full AC1021 header decoding pipeline:
    /// 1. Reed-Solomon decode the 0x400-byte encoded header
    /// 2. Extract CRC, key, and compression parameters
    /// 3. LZ77 AC21 decompress into 0x110-byte metadata buffer
    /// 4. Parse the `Dwg21CompressedMetadata` (including Header CRC-64)
    /// 5. Decode the page map and section map
    fn read_file_header_ac21(&mut self, info: &mut DwgFileHeaderInfo) -> Result<(), DxfError> {
        // After read_file_metadata, stream is at position 0x80 (128).
        // The Reed-Solomon encoded data follows immediately.
        // Do NOT seek — continue reading from current position.

        // Step 1: Read 0x400 bytes of Reed-Solomon encoded data
        let mut compressed_data = [0u8; 0x400];
        self.stream.read_exact(&mut compressed_data)?;

        // Step 2: Reed-Solomon decode (factor=3, block_size=239)
        let mut decoded_data = vec![0u8; 3 * 239]; // 717 bytes
        reed_solomon_decode(&compressed_data, &mut decoded_data, 3, 239);

        // Step 3: Extract header values from decoded data
        let mut cursor = Cursor::new(&decoded_data);

        let crc = cursor.read_i64::<LittleEndian>()?;
        let unknown_key = cursor.read_i64::<LittleEndian>()?;
        let compressed_data_crc = cursor.read_i64::<LittleEndian>()?;
        let compr_len = cursor.read_i32::<LittleEndian>()?;
        let _length2 = cursor.read_i32::<LittleEndian>()?;

        info.ac21_header_crc = Some(crc);
        info.ac21_unknown_key = Some(unknown_key);
        info.ac21_compressed_data_crc = Some(compressed_data_crc);

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021 header: CRC={:#018X}, UnknownKey={:#018X}, CompressedDataCRC={:#018X}, ComprLen={}",
                crc as u64, unknown_key as u64, compressed_data_crc as u64, compr_len
            ),
        );

        // Step 4: Extract 0x110-byte metadata
        let mut metadata_buffer = vec![0u8; 0x110];

        if compr_len < 0 {
            // Negative ComprLen means data is stored uncompressed (raw).
            // |ComprLen| = raw data length. Copy directly from offset 0x20.
            let raw_len = (-compr_len) as usize;
            let src_start = 32; // offset 0x20 in decoded data
            let copy_len = raw_len
                .min(0x110)
                .min(decoded_data.len().saturating_sub(src_start));
            metadata_buffer[..copy_len]
                .copy_from_slice(&decoded_data[src_start..src_start + copy_len]);
        } else {
            // Positive ComprLen means data is LZ77 compressed.
            // Decompress from byte offset 32 in decoded_data.
            decompress_ac21(&decoded_data, 32, compr_len as u32, &mut metadata_buffer);
        }

        // Step 5: Parse compressed metadata (extracts CRC-64)
        let metadata = Dwg21CompressedMetadata::from_bytes(&metadata_buffer)?;

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021 Header CRC-64 extracted: {:#018X}",
                metadata.header_crc64
            ),
        );

        // Note: The exact CRC-64 algorithm used by Autodesk for this field is
        // undocumented. No known open reference validates
        // this value. It is stored for informational/round-trip purposes.

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021 CRC Seeds: CrcSeed={:#018X}, CrcSeedEncoded={:#018X}, RandomSeed={:#018X}",
                metadata.crc_seed, metadata.crc_seed_encoded, metadata.random_seed
            ),
        );

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021 Pages Map CRC: compressed={:#018X}, uncompressed={:#018X}, seed={:#018X}",
                metadata.pages_map_crc_compressed,
                metadata.pages_map_crc_uncompressed,
                metadata.pages_map_crc_seed
            ),
        );

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021 Sections Map CRC: compressed={:#018X}, uncompressed={:#018X}, seed={:#018X}",
                metadata.sections_map_crc_compressed,
                metadata.sections_map_crc_uncompressed,
                metadata.sections_map_crc_seed
            ),
        );

        // Step 6: Read page map
        self.read_page_map_ac21(info, &metadata)?;

        // Step 7: Read section map
        self.read_section_map_ac21(info, &metadata)?;

        // The §19 H8a container shape: compose the author's AC1021
        // page space from the parsed maps — the sections-table order
        // with each record's pages-map id resolved to its physical
        // extent. Retained for the H8 same-version roundtrip mirror
        // (the write gate falls back to the conventional layout when
        // the re-encoded content does not fit the author's page
        // space).
        if !info.section_descriptors.is_empty() && !info.ac21_map_order.is_empty() {
            let mut sections = Vec::with_capacity(info.section_descriptors.len());
            for descriptor in &info.section_descriptors {
                let mut pages = Vec::with_capacity(descriptor.pages.len());
                for page in &descriptor.pages {
                    let on_disk_size = info
                        .page_records
                        .get(&(page.page_number.unsigned_abs() as i32))
                        .map(|&(_, size)| size)
                        .unwrap_or(0);
                    pages.push(crate::document::DwgAc21SectionPageShape {
                        offset: page.offset,
                        size: page.size,
                        id: page.page_number,
                        uncomp_size: page.decompressed_size,
                        on_disk_size,
                    });
                }
                sections.push(crate::document::DwgAc21SectionShape {
                    name: descriptor.name.clone(),
                    encoding: descriptor.encoding,
                    data_size: descriptor.decompressed_size,
                    pages,
                });
            }
            info.ac21_shape = Some(crate::document::DwgAc21ContainerShape {
                map_order: std::mem::take(&mut info.ac21_map_order),
                sections,
            });
        }

        // Store metadata even if reads above fail (for diagnostics)
        info.ac21_metadata = Some(metadata.clone());

        Ok(())
    }

    /// Read the page map from an AC1021 file.
    ///
    /// The page map lists all data pages and their sizes, allowing
    /// the reader to build a page ID → file offset lookup table.
    fn read_page_map_ac21(
        &mut self,
        info: &mut DwgFileHeaderInfo,
        metadata: &Dwg21CompressedMetadata,
    ) -> Result<(), DxfError> {
        let page_buffer = self.get_page_buffer(
            metadata.pages_map_offset,
            metadata.pages_map_size_compressed,
            metadata.pages_map_size_uncompressed,
            metadata.pages_map_correction_factor,
            0xEF,
        )?;

        let mut cursor = Cursor::new(&page_buffer);
        let mut offset: i64 = 0;

        while (cursor.position() as usize) < page_buffer.len() {
            let size = cursor.read_i64::<LittleEndian>()?;
            let id = cursor.read_i64::<LittleEndian>()?;

            if size == 0 && id == 0 {
                // Terminator — all remaining bytes are padding
                break;
            }

            let ind = id.unsigned_abs();

            info.page_records.insert(ind as i32, (offset, size));
            info.ac21_map_order.push(crate::document::DwgAc21PageEntry {
                id,
                on_disk_size: size,
            });
            offset += size;
        }

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021: Read {} page records from page map",
                info.page_records.len()
            ),
        );

        Ok(())
    }

    /// Read the section map from an AC1021 file.
    ///
    /// The section map describes all logical sections (Header, Classes,
    /// Handles, Objects, etc.) and their per-page CRC values.
    fn read_section_map_ac21(
        &mut self,
        info: &mut DwgFileHeaderInfo,
        metadata: &Dwg21CompressedMetadata,
    ) -> Result<(), DxfError> {
        // Look up the section map page
        let sections_map_id = metadata.sections_map_id as i32;
        let seeker = info
            .page_records
            .get(&sections_map_id)
            .map(|&(offset, _)| offset)
            .ok_or_else(|| {
                DxfError::InvalidFormat(format!(
                    "Section map page ID {} not found in page records",
                    sections_map_id
                ))
            })?;

        let section_buffer = self.get_page_buffer_at(
            seeker as u64,
            metadata.sections_map_size_compressed,
            metadata.sections_map_size_uncompressed,
            metadata.sections_map_correction_factor,
            239,
        )?;

        let mut cursor = Cursor::new(&section_buffer);

        while (cursor.position() as usize) < section_buffer.len() {
            // Check if there's enough data for at least the fixed header fields
            if section_buffer.len() - (cursor.position() as usize) < 64 {
                break;
            }

            let compressed_size = cursor.read_u64::<LittleEndian>()?;
            let decompressed_size = cursor.read_u64::<LittleEndian>()?;
            let encrypted = cursor.read_u64::<LittleEndian>()?;
            let hash_code = cursor.read_u64::<LittleEndian>()?;
            let section_name_length = cursor.read_i64::<LittleEndian>()?;
            let _unknown = cursor.read_u64::<LittleEndian>()?;
            let encoding = cursor.read_u64::<LittleEndian>()?;
            let page_count = cursor.read_u64::<LittleEndian>()?;

            // Read section name (UTF-16LE)
            let name = if section_name_length > 0 {
                let byte_len = section_name_length as usize;
                if cursor.position() as usize + byte_len > section_buffer.len() {
                    break;
                }
                let mut name_bytes = vec![0u8; byte_len];
                cursor.read_exact(&mut name_bytes)?;
                // Decode UTF-16LE
                let words: Vec<u16> = name_bytes
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&words)
                    .trim_end_matches('\0')
                    .to_string()
            } else {
                String::new()
            };

            // Read per-page CRC information
            let mut pages = Vec::new();
            for _ in 0..page_count {
                if section_buffer.len() - (cursor.position() as usize) < 56 {
                    break;
                }
                let page_offset = cursor.read_u64::<LittleEndian>()?;
                let page_size = cursor.read_i64::<LittleEndian>()?;
                let page_number = cursor.read_i64::<LittleEndian>()?;
                let page_decompressed_size = cursor.read_u64::<LittleEndian>()?;
                let page_compressed_size = cursor.read_u64::<LittleEndian>()?;
                let page_checksum = cursor.read_u64::<LittleEndian>()?;
                let page_crc = cursor.read_u64::<LittleEndian>()?;

                self.notifications.notify(
                    NotificationType::Warning,
                    format!(
                        "  Section '{}' page {}: CRC={:#018X}, Checksum={:#018X}, CompSize={}, DecompSize={}",
                        name, page_number, page_crc, page_checksum,
                        page_compressed_size, page_decompressed_size
                    ),
                );

                pages.push(DwgPageCrcInfo {
                    page_number,
                    offset: page_offset,
                    size: page_size,
                    decompressed_size: page_decompressed_size,
                    compressed_size: page_compressed_size,
                    checksum: page_checksum,
                    crc: page_crc,
                });
            }

            if section_name_length > 0 {
                info.section_descriptors.push(DwgSectionInfo {
                    name: name.clone(),
                    compressed_size,
                    decompressed_size,
                    encrypted,
                    hash_code,
                    encoding,
                    page_count,
                    pages,
                });
            }
        }

        self.notifications.notify(
            NotificationType::Warning,
            format!(
                "AC1021: Read {} section descriptors from section map",
                info.section_descriptors.len()
            ),
        );

        Ok(())
    }

    /// Get a decompressed page buffer from the file.
    ///
    /// Handles the Reed-Solomon + LZ77 AC21 decompression pipeline.
    fn get_page_buffer(
        &mut self,
        page_offset: u64,
        compressed_size: u64,
        uncompressed_size: u64,
        correction_factor: u64,
        block_size: usize,
    ) -> Result<Vec<u8>, DxfError> {
        self.get_page_buffer_at(
            page_offset,
            compressed_size,
            uncompressed_size,
            correction_factor,
            block_size,
        )
    }

    /// Get a decompressed page buffer from a specific file offset.
    fn get_page_buffer_at(
        &mut self,
        page_offset: u64,
        compressed_size: u64,
        uncompressed_size: u64,
        correction_factor: u64,
        block_size: usize,
    ) -> Result<Vec<u8>, DxfError> {
        let (total_size, factor, read_length) =
            ac21_page_layout(compressed_size, correction_factor, block_size)?;

        // Read encoded data from file
        self.stream
            .seek(SeekFrom::Start(AC21_FILE_HEADER_SIZE + page_offset))?;
        let mut encoded_buffer = vec![0u8; read_length];
        let bytes_read = self.stream.read(&mut encoded_buffer)?;
        if bytes_read < read_length {
            // Pad remaining with zeros
            encoded_buffer[bytes_read..].fill(0);
        }

        Ok(decode_ac21_page(
            &encoded_buffer,
            total_size,
            factor,
            block_size,
            compressed_size,
            uncompressed_size,
        ))
    }

    /// Get the merged decompressed buffer for a named section (AC21).
    ///
    /// Reads and concatenates all pages belonging to the given section,
    /// producing the complete section data ready for parsing.
    ///
    /// In failsafe mode, missing or unreadable pages are skipped and a
    /// warning notification is emitted rather than aborting the entire
    /// section read.
    ///
    /// # Arguments
    /// * `section_name` - Section name (e.g., "AcDb:Header", "AcDb:Classes")
    /// * `info` - Previously read file header info containing section descriptors
    ///
    /// # Returns
    /// The complete decompressed section buffer, or an error if the section
    /// is not found or a page cannot be read (unless in failsafe mode).
    pub fn get_section_buffer(
        &mut self,
        section_name: &str,
        info: &DwgFileHeaderInfo,
    ) -> Result<Vec<u8>, DxfError> {
        let failsafe = self.options.failsafe;
        let perf = std::env::var_os("PERF").is_some();
        let started = web_time::Instant::now();

        // ── AC15 path: direct read from section locators ──
        // If we have section_locators (AC15 format), read raw bytes
        // directly from the file at the recorded offset.
        if !info.section_locators.is_empty() {
            if let Some(&(offset, size)) = info.section_locators.get(section_name) {
                if size <= 0 {
                    return Err(DxfError::Parse(format!(
                        "Section '{}' has zero size",
                        section_name
                    )));
                }
                self.stream.seek(SeekFrom::Start(offset as u64))?;
                let mut buf = vec![0u8; size as usize];
                self.stream.read_exact(&mut buf)?;
                if perf {
                    eprintln!(
                        "[perf] dwg-section name={} format=ac15 time={:.1}ms bytes={}",
                        section_name,
                        started.elapsed().as_secs_f64() * 1000.0,
                        buf.len(),
                    );
                }
                return Ok(buf);
            } else {
                return Err(DxfError::Parse(format!(
                    "Section '{}' not found in AC15 locator records",
                    section_name
                )));
            }
        }

        // ── AC18 path: page-based with LZ77 AC18 compression ──
        if info.is_ac18_format {
            let result = self.get_section_buffer_ac18(section_name, info);
            if perf {
                eprintln!(
                    "[perf] dwg-section name={} format=ac18 time={:.1}ms bytes={}",
                    section_name,
                    started.elapsed().as_secs_f64() * 1000.0,
                    result.as_ref().map_or(0, Vec::len),
                );
            }
            return result;
        }

        // ── AC21 path: page-based section descriptors ──
        // Find the section descriptor
        let section = info
            .section_descriptors
            .iter()
            .find(|s| s.name == section_name)
            .ok_or_else(|| {
                DxfError::Parse(format!("Section '{}' not found in file", section_name))
            })?;

        // Field 0x00 ("Data size") holds the total section data size.
        // Field 0x08 ("Max size") is the page partition size per spec §5.4.
        // We truncate to the total data size, not the page size.
        let total_size = section.compressed_size as usize;
        let mut result = Vec::with_capacity(total_size);

        // encoding=1 (stored): contiguous data followed by non-interleaved RS
        // parity. Reading only the payload deliberately skips that parity.
        // encoding=4 (compressed): data is LZ77-compressed then RS-encoded with RS(255,251).
        // System pages (page map, section map) use RS(255,239) per §5.3,
        // but those are decoded separately in read_page_map_ac21 / read_section_map_ac21.
        let encoding = section.encoding;
        let block_size: usize = 251;

        enum PagePayload {
            Ready(Vec<u8>),
            Encoded {
                bytes: Vec<u8>,
                total_size: usize,
                factor: usize,
                compressed_size: u64,
                uncompressed_size: u64,
            },
            Failed(String),
        }
        struct PreparedPage {
            number: i64,
            fill_size: usize,
            payload: PagePayload,
        }

        // File seeks stay ordered; CPU-heavy Reed-Solomon and LZ77 work runs
        // concurrently in bounded batches. This keeps peak memory near
        // `result + 2 * worker pages` instead of retaining encoded and decoded
        // copies of the whole section at once.
        let batch_pages = worker_count().saturating_mul(2);
        let mut skipped_pages = 0u32;
        for page_batch in section.pages.chunks(batch_pages) {
            let mut prepared = Vec::with_capacity(page_batch.len());
            for page in page_batch {
                let fill_size = page.decompressed_size as usize;
                let payload = match info.page_records.get(&(page.page_number as i32)) {
                    Some(&(page_offset, _)) if encoding == 1 => {
                        let read = (|| -> Result<Vec<u8>, DxfError> {
                            self.stream.seek(SeekFrom::Start(
                                AC21_FILE_HEADER_SIZE + page_offset as u64,
                            ))?;
                            let mut bytes = vec![0u8; fill_size];
                            self.stream.read_exact(&mut bytes)?;
                            Ok(bytes)
                        })();
                        match read {
                            Ok(bytes) => PagePayload::Ready(bytes),
                            Err(error) if failsafe => PagePayload::Failed(error.to_string()),
                            Err(error) => return Err(error),
                        }
                    }
                    Some(&(page_offset, _)) => {
                        match ac21_page_layout(page.compressed_size, 1, block_size) {
                            Ok((total_size, factor, read_length)) => {
                                let read = (|| -> Result<Vec<u8>, DxfError> {
                                    self.stream.seek(SeekFrom::Start(
                                        AC21_FILE_HEADER_SIZE + page_offset as u64,
                                    ))?;
                                    let mut bytes = vec![0u8; read_length];
                                    let read = self.stream.read(&mut bytes)?;
                                    bytes[read..].fill(0);
                                    Ok(bytes)
                                })();
                                match read {
                                    Ok(bytes) => PagePayload::Encoded {
                                        bytes,
                                        total_size,
                                        factor,
                                        compressed_size: page.compressed_size,
                                        uncompressed_size: page.decompressed_size,
                                    },
                                    Err(error) if failsafe => {
                                        PagePayload::Failed(error.to_string())
                                    }
                                    Err(error) => return Err(error),
                                }
                            }
                            Err(error) if failsafe => PagePayload::Failed(error.to_string()),
                            Err(error) => return Err(error),
                        }
                    }
                    None if failsafe => PagePayload::Failed(format!(
                        "page {} not found in page map",
                        page.page_number
                    )),
                    None => {
                        return Err(DxfError::Parse(format!(
                            "Page {} not found in page map",
                            page.page_number
                        )))
                    }
                };
                prepared.push(PreparedPage {
                    number: page.page_number,
                    fill_size,
                    payload,
                });
            }

            let decoded = map_ordered(prepared, |page| {
                let data = match page.payload {
                    PagePayload::Ready(bytes) => Ok(bytes),
                    PagePayload::Encoded {
                        bytes,
                        total_size,
                        factor,
                        compressed_size,
                        uncompressed_size,
                    } => Ok(decode_ac21_page(
                        &bytes,
                        total_size,
                        factor,
                        block_size,
                        compressed_size,
                        uncompressed_size,
                    )),
                    PagePayload::Failed(error) => Err(error),
                };
                (page.number, page.fill_size, data)
            });

            for (page_number, fill_size, page_result) in decoded {
                match page_result {
                    Ok(page_data) => result.extend_from_slice(&page_data),
                    Err(error) => {
                        skipped_pages += 1;
                        self.notifications.notify(
                            NotificationType::Error,
                            format!(
                                "Failsafe: skipped corrupt page {} in section '{}': {}",
                                page_number, section_name, error
                            ),
                        );
                        result.resize(result.len() + fill_size, 0);
                    }
                }
            }
        }

        if skipped_pages > 0 {
            self.notifications.notify(
                NotificationType::Warning,
                format!(
                    "Failsafe: {} of {} pages skipped in section '{}'",
                    skipped_pages,
                    section.pages.len(),
                    section_name
                ),
            );
        }

        // Truncate to the declared section size (last page may be padded)
        result.truncate(total_size);

        if perf {
            eprintln!(
                "[perf] dwg-section name={} format=ac21 time={:.1}ms bytes={} pages={}",
                section_name,
                started.elapsed().as_secs_f64() * 1000.0,
                result.len(),
                section.pages.len(),
            );
        }
        Ok(result)
    }

    /// Get the merged decompressed buffer for a named section (AC18).
    ///
    /// Reads data pages for the given section, XOR-unmasks their 32-byte
    /// headers, and decompresses the LZ77 AC18 data.
    fn get_section_buffer_ac18(
        &mut self,
        section_name: &str,
        info: &DwgFileHeaderInfo,
    ) -> Result<Vec<u8>, DxfError> {
        let section = info
            .section_descriptors
            .iter()
            .find(|s| s.name == section_name)
            .ok_or_else(|| {
                DxfError::Parse(format!("Section '{}' not found in AC18 file", section_name))
            })?;

        // compressed_size field actually holds the total uncompressed section data size
        let total_size = section.compressed_size as usize;
        let is_compressed = section.encoding == 2;
        let max_page_size = section.decompressed_size as usize;

        let mut result = vec![0u8; total_size];
        let batch_pages = worker_count().saturating_mul(2);

        for page_batch in section.pages.chunks(batch_pages) {
            let mut prepared = Vec::with_capacity(page_batch.len());
            for page in page_batch {
                let page_number = page.page_number as i32;

                let &(page_file_offset, _page_total_size) =
                    info.page_records.get(&page_number).ok_or_else(|| {
                        DxfError::Parse(format!(
                            "AC18 page {} not found in page records for section '{}'",
                            page_number, section_name
                        ))
                    })?;

                self.stream.seek(SeekFrom::Start(page_file_offset as u64))?;

                // Read 32-byte data section header (XOR-masked)
                let mut header = [0u8; 32];
                self.stream.read_exact(&mut header)?;

                // XOR unmask using the page's file position
                apply_mask(&mut header, page_file_offset as u64);

                // Parse header fields
                let mut hcursor = Cursor::new(&header[..]);
                let _section_type = hcursor.read_i32::<LittleEndian>()?;
                let _section_id = hcursor.read_i32::<LittleEndian>()?;
                let data_compressed_size = hcursor.read_i32::<LittleEndian>()?;
                let _page_size = hcursor.read_i32::<LittleEndian>()?;
                let data_offset = hcursor.read_i64::<LittleEndian>()?;

                // Read compressed data
                if data_compressed_size <= 0 || data_compressed_size > 10_000_000 {
                    self.notifications.notify(
                        NotificationType::Warning,
                        format!(
                            "AC18: Invalid compressed size {} for page {} in section '{}'",
                            data_compressed_size, page_number, section_name
                        ),
                    );
                    continue;
                }
                let mut compressed = vec![0u8; data_compressed_size as usize];
                self.stream.read_exact(&mut compressed)?;

                prepared.push((data_offset as usize, compressed));
            }

            let decoded = map_ordered(prepared, |(dst_start, compressed)| {
                let decompressed = if is_compressed {
                    decompress_ac18(&compressed, max_page_size)
                } else {
                    compressed
                };
                (dst_start, decompressed)
            });

            for (dst_start, decompressed) in decoded {
                if dst_start < total_size {
                    let copy_len = decompressed.len().min(total_size - dst_start);
                    result[dst_start..dst_start + copy_len]
                        .copy_from_slice(&decompressed[..copy_len]);
                }
            }
        }

        Ok(result)
    }

    /// Find a section descriptor by name.
    pub fn find_section<'a>(info: &'a DwgFileHeaderInfo, name: &str) -> Option<&'a DwgSectionInfo> {
        info.section_descriptors.iter().find(|s| s.name == name)
    }

    /// Extract all CRC values from the file and return them as a summary.
    ///
    /// This is a convenience method that reads the file header and
    /// formats all CRC values for display.
    pub fn extract_all_crcs(&mut self) -> Result<CrcExtractionReport, DxfError> {
        let info = self.read_file_header()?;

        let mut report = CrcExtractionReport {
            version: info.version_string.clone(),
            header_crc64: None,
            header_crc: info.ac21_header_crc,
            compressed_data_crc: info.ac21_compressed_data_crc,
            pages_map_crc_compressed: None,
            pages_map_crc_uncompressed: None,
            pages_map_crc_seed: None,
            sections_map_crc_compressed: None,
            sections_map_crc_uncompressed: None,
            sections_map_crc_seed: None,
            crc_seed: None,
            crc_seed_encoded: None,
            random_seed: None,
            page_crcs: Vec::new(),
            notifications: std::mem::take(&mut self.notifications),
        };

        if let Some(ref metadata) = info.ac21_metadata {
            report.header_crc64 = Some(metadata.header_crc64);
            report.pages_map_crc_compressed = Some(metadata.pages_map_crc_compressed);
            report.pages_map_crc_uncompressed = Some(metadata.pages_map_crc_uncompressed);
            report.pages_map_crc_seed = Some(metadata.pages_map_crc_seed);
            report.sections_map_crc_compressed = Some(metadata.sections_map_crc_compressed);
            report.sections_map_crc_uncompressed = Some(metadata.sections_map_crc_uncompressed);
            report.sections_map_crc_seed = Some(metadata.sections_map_crc_seed);
            report.crc_seed = Some(metadata.crc_seed);
            report.crc_seed_encoded = Some(metadata.crc_seed_encoded);
            report.random_seed = Some(metadata.random_seed);
        }

        for section in &info.section_descriptors {
            for page in &section.pages {
                report.page_crcs.push(PageCrcEntry {
                    section_name: section.name.clone(),
                    page_number: page.page_number,
                    crc: page.crc,
                    checksum: page.checksum,
                    compressed_size: page.compressed_size,
                    decompressed_size: page.decompressed_size,
                });
            }
        }

        Ok(report)
    }
}

/// Complete CRC extraction report from a DWG file.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CrcExtractionReport {
    /// DWG version string
    pub version: String,
    /// Header CRC-64 (AC1021 only)
    pub header_crc64: Option<u64>,
    /// Header CRC from Reed-Solomon decoded data
    pub header_crc: Option<i64>,
    /// Compressed data CRC
    pub compressed_data_crc: Option<i64>,
    /// Pages map CRC (compressed)
    pub pages_map_crc_compressed: Option<u64>,
    /// Pages map CRC (uncompressed)
    pub pages_map_crc_uncompressed: Option<u64>,
    /// Pages map CRC seed
    pub pages_map_crc_seed: Option<u64>,
    /// Sections map CRC (compressed)
    pub sections_map_crc_compressed: Option<u64>,
    /// Sections map CRC (uncompressed)
    pub sections_map_crc_uncompressed: Option<u64>,
    /// Sections map CRC seed
    pub sections_map_crc_seed: Option<u64>,
    /// Global CRC seed
    pub crc_seed: Option<u64>,
    /// Encoded CRC seed
    pub crc_seed_encoded: Option<u64>,
    /// Random seed
    pub random_seed: Option<u64>,
    /// Per-page CRC values
    pub page_crcs: Vec<PageCrcEntry>,
    /// Notifications collected during reading
    pub notifications: NotificationCollection,
}

/// CRC entry for a single page.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PageCrcEntry {
    /// Section name
    pub section_name: String,
    /// Page number
    pub page_number: i64,
    /// CRC value
    pub crc: u64,
    /// Checksum value
    pub checksum: u64,
    /// Compressed size
    pub compressed_size: u64,
    /// Decompressed size
    pub decompressed_size: u64,
}

impl std::fmt::Display for CrcExtractionReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== DWG CRC Extraction Report ===")?;
        writeln!(f, "Version: {}", self.version)?;
        writeln!(f)?;

        if let Some(crc64) = self.header_crc64 {
            writeln!(f, "--- Header CRC-64 ---")?;
            writeln!(f, "  Header CRC-64:              {:#018X}", crc64)?;
        }

        if let Some(crc) = self.header_crc {
            writeln!(f, "  Header CRC (RS decoded):    {:#018X}", crc as u64)?;
        }
        if let Some(crc) = self.compressed_data_crc {
            writeln!(f, "  Compressed Data CRC:        {:#018X}", crc as u64)?;
        }

        if self.crc_seed.is_some() {
            writeln!(f)?;
            writeln!(f, "--- CRC Seeds ---")?;
            if let Some(v) = self.crc_seed {
                writeln!(f, "  CRC Seed:                   {:#018X}", v)?;
            }
            if let Some(v) = self.crc_seed_encoded {
                writeln!(f, "  CRC Seed Encoded:           {:#018X}", v)?;
            }
            if let Some(v) = self.random_seed {
                writeln!(f, "  Random Seed:                {:#018X}", v)?;
            }
        }

        if self.pages_map_crc_compressed.is_some() {
            writeln!(f)?;
            writeln!(f, "--- Pages Map CRC ---")?;
            if let Some(v) = self.pages_map_crc_compressed {
                writeln!(f, "  Compressed:                 {:#018X}", v)?;
            }
            if let Some(v) = self.pages_map_crc_uncompressed {
                writeln!(f, "  Uncompressed:               {:#018X}", v)?;
            }
            if let Some(v) = self.pages_map_crc_seed {
                writeln!(f, "  Seed:                       {:#018X}", v)?;
            }
        }

        if self.sections_map_crc_compressed.is_some() {
            writeln!(f)?;
            writeln!(f, "--- Sections Map CRC ---")?;
            if let Some(v) = self.sections_map_crc_compressed {
                writeln!(f, "  Compressed:                 {:#018X}", v)?;
            }
            if let Some(v) = self.sections_map_crc_uncompressed {
                writeln!(f, "  Uncompressed:               {:#018X}", v)?;
            }
            if let Some(v) = self.sections_map_crc_seed {
                writeln!(f, "  Seed:                       {:#018X}", v)?;
            }
        }

        if !self.page_crcs.is_empty() {
            writeln!(f)?;
            writeln!(f, "--- Per-Page CRC Values ---")?;
            for entry in &self.page_crcs {
                writeln!(
                    f,
                    "  {} [page {}]: CRC={:#018X}, Checksum={:#018X} (comp={}, decomp={})",
                    entry.section_name,
                    entry.page_number,
                    entry.crc,
                    entry.checksum,
                    entry.compressed_size,
                    entry.decompressed_size
                )?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod dwg_reader_tests {
    use super::{extract_acds_record_blobs, section_name_from_field, ACIS_END_MARKER};
    use std::collections::HashSet;

    fn field(prefix: &[u8]) -> [u8; 64] {
        let mut b = [0u8; 64];
        b[..prefix.len()].copy_from_slice(prefix);
        b
    }

    #[test]
    fn clean_zero_padded_name() {
        assert_eq!(
            section_name_from_field(&field(b"AcDb:Handles")),
            "AcDb:Handles"
        );
    }

    #[test]
    fn stops_at_first_null_ignoring_trailing_garbage() {
        // Terminator at index 12, then non-zero junk — the real-world case that
        // broke `trim_end_matches('\0')`.
        let mut b = field(b"AcDb:Handles");
        b[13] = b't';
        b[14] = 0x01;
        b[20] = b'X';
        assert_eq!(section_name_from_field(&b), "AcDb:Handles");
    }

    #[test]
    fn empty_when_first_byte_null() {
        assert_eq!(section_name_from_field(&[0u8; 64]), "");
    }

    #[test]
    fn acds_record_table_recovers_a_trailing_external_blob_segment() {
        const HEADER_SIZE: usize = 48;
        const RECORD_SIZE: usize = 20;
        const RECORDS: usize = 3;

        fn blob(label: u8) -> Vec<u8> {
            let mut value = b"ASM BinaryFile".to_vec();
            value.push(label);
            value.extend_from_slice(ACIS_END_MARKER);
            value
        }

        let external = blob(3);
        let physical = [(0x91u32, blob(1)), (0x92, blob(2))];
        let mut offsets = vec![(0x89u32, 0usize)];
        let mut data = vec![0u8; 44];
        for (handle, value) in &physical {
            offsets.push((*handle, data.len()));
            data.extend_from_slice(value);
        }

        let base = HEADER_SIZE + RECORD_SIZE * RECORDS;
        let segment_size = base + data.len();
        let external_segment = segment_size;
        let external_header_size = 64;
        let mut buffer = vec![0u8; segment_size + external_header_size + external.len()];
        buffer[..8].copy_from_slice(b"\xAC\xD5_data_");
        buffer[16..24].copy_from_slice(&(segment_size as u64).to_le_bytes());
        buffer[external_segment..external_segment + 8].copy_from_slice(b"\xAC\xD5_data_");
        buffer[external_segment + 16..external_segment + 24]
            .copy_from_slice(&((external_header_size + external.len()) as u64).to_le_bytes());

        for (index, handle) in [0x89u32, 0x91, 0x92].into_iter().enumerate() {
            let entry = HEADER_SIZE + index * RECORD_SIZE;
            let offset = offsets
                .iter()
                .find(|(candidate, _)| *candidate == handle)
                .unwrap()
                .1;
            buffer[entry..entry + 4].copy_from_slice(&0x14u32.to_le_bytes());
            buffer[entry + 8..entry + 12].copy_from_slice(&handle.to_le_bytes());
            buffer[entry + 16..entry + 20].copy_from_slice(&(offset as u32).to_le_bytes());
        }
        buffer[base..segment_size].copy_from_slice(&data);
        buffer[external_segment + external_header_size..].copy_from_slice(&external);

        let handles = HashSet::from([0x89u64, 0x91, 0x92]);
        let extracted = extract_acds_record_blobs(&buffer, &handles);
        assert_eq!(extracted.len(), RECORDS);
        for (handle, expected) in physical.into_iter().chain([(0x89, external)]) {
            assert_eq!(
                extracted
                    .iter()
                    .find(|(candidate, _)| *candidate == handle as u64)
                    .unwrap()
                    .1,
                expected,
            );
        }
    }
}

/// Reconstruct a gradient hatch that was down-saved (R2000/R2004) as a solid
/// fill plus round-trip metadata: the two colours live in EED
/// (`GradientColor1ACI` / `GradientColor2ACI`) and the gradient type name in an
/// `ACAD_XREC_ROUNDTRIP` XRecord under the hatch's extension dictionary. When a
/// solid hatch carries this metadata but no live gradient block, rebuild
/// `gradient_color` so it renders as the intended gradient.
pub(crate) fn recover_roundtrip_gradients(document: &mut crate::document::CadDocument) {
    use crate::entities::EntityType;
    use crate::objects::ObjectType;
    use crate::types::{Color, Handle};

    const GRADIENT_NAMES: &[&str] = &[
        "SPHERICAL",
        "HEMISPHERICAL",
        "CURVED",
        "CYLINDER",
        "INVSPHERICAL",
        "INVHEMISPHERICAL",
        "INVCURVED",
        "INVCYLINDER",
        "LINEAR",
    ];

    // Phase 1 — collect (immutable): which hatches need a gradient, and what.
    let mut recovered: Vec<(Handle, u8, u8, String)> = Vec::new();
    for e in document.entities() {
        let EntityType::Hatch(h) = e else { continue };
        if h.gradient_color.enabled || !h.is_solid {
            continue;
        }
        let eed = &h.common.extended_data;
        // EED app names vary in case across versions (R14 upper-cases them:
        // GRADIENTCOLOR1ACI vs R2000's GradientColor1ACI), so match loosely.
        let aci = |app: &str| -> Option<u8> {
            eed.records()
                .iter()
                .find(|r| r.application_name.eq_ignore_ascii_case(app))
                .and_then(|r| {
                    r.values.iter().find_map(|v| match v {
                        crate::xdata::XDataValue::Integer16(n) => Some(*n as u8),
                        _ => None,
                    })
                })
        };
        let (Some(c1), Some(c2)) = (aci("GradientColor1ACI"), aci("GradientColor2ACI")) else {
            continue;
        };

        // A live down-saved gradient always carries an ACAD_XREC_ROUNDTRIP
        // XRecord (under the hatch's extension dictionary) alongside the
        // GradientColor*ACI EED. When only the EED survives — no extension
        // dictionary, or one without that XRecord — the colours are stale
        // metadata left behind by an edit that turned a gradient into a plain
        // solid fill (or recoloured it). Resurrecting a gradient then paints a
        // genuine single-colour solid hatch as a two-colour gradient, so treat
        // the missing round-trip XRecord as proof the hatch is really solid.
        //
        // Gradient type: the trailing name string in that same XRecord
        // (hatch xdict → "ACAD_XREC_ROUNDTRIP" → XRecord raw_data).
        let mut name = String::new();
        let mut has_roundtrip = false;
        if let Some(xd) = h.common.xdictionary_handle {
            if let Some(ObjectType::Dictionary(d)) = document.objects.get(&xd) {
                // R14 mis-sizes dictionary key strings, leaving trailing
                // garbage bytes ("ACAD_XREC_ROUNDTRIP\x03q0…"), so match by
                // prefix rather than equality.
                if let Some((_, xrec_h)) = d.entries.iter().find(|(k, _)| {
                    k.len() >= 19 && k[..19].eq_ignore_ascii_case("ACAD_XREC_ROUNDTRIP")
                }) {
                    has_roundtrip = true;
                    if let Some(ObjectType::XRecord(xr)) = document.objects.get(xrec_h) {
                        let ascii: String = xr
                            .raw_data
                            .iter()
                            .map(|&b| {
                                if (32..127).contains(&b) {
                                    b as char
                                } else {
                                    ' '
                                }
                            })
                            .collect();
                        let up = ascii.to_ascii_uppercase();
                        for cand in GRADIENT_NAMES {
                            if up.contains(cand) {
                                name = (*cand).to_string();
                                break;
                            }
                        }
                        // A DXF-sourced round-trip XRecord encodes the gradient
                        // type as a number in the 1004 binary (first LE int32 =
                        // 123 + gradient index in AutoCAD's dropdown order),
                        // not as an ASCII name; decode it when no name matched.
                        if name.is_empty() && xr.raw_data.len() >= 4 {
                            const BY_INDEX: &[&str] = &[
                                "LINEAR",
                                "CYLINDER",
                                "INVCYLINDER",
                                "SPHERICAL",
                                "HEMISPHERICAL",
                                "CURVED",
                                "INVSPHERICAL",
                                "INVHEMISPHERICAL",
                                "INVCURVED",
                            ];
                            let v = u32::from_le_bytes([
                                xr.raw_data[0],
                                xr.raw_data[1],
                                xr.raw_data[2],
                                xr.raw_data[3],
                            ]);
                            if (123..123 + BY_INDEX.len() as u32).contains(&v) {
                                name = BY_INDEX[(v - 123) as usize].to_string();
                            }
                        }
                    }
                }
            }
        }
        // No round-trip XRecord ⇒ the gradient EED is stale; keep it solid.
        if !has_roundtrip {
            continue;
        }
        if name.is_empty() {
            name = "LINEAR".to_string();
        }
        recovered.push((e.common().handle, c1, c2, name));
    }

    // Phase 2 — apply (mutable).
    for (handle, c1, c2, name) in recovered {
        if let Some(EntityType::Hatch(h)) = document.get_entity_mut(handle) {
            h.gradient_color.enabled = true;
            h.gradient_color.is_single_color = c1 == c2;
            h.gradient_color.colors = vec![
                crate::entities::hatch::GradientColorEntry {
                    value: 0.0,
                    color: Color::from_index(c1 as i16),
                },
                crate::entities::hatch::GradientColorEntry {
                    value: 1.0,
                    color: Color::from_index(c2 as i16),
                },
            ];
            h.gradient_color.name = name;
        }
    }
}

/// Recover an MTEXT background fill that a pre-R2004 (R2000/R14) save stored as
/// round-trip EED (`ACAD_MTEXT_BBRT` … `ACAD_MTEXT_BERT`) instead of the native
/// codes (90 flags / 63|421 colour / 45 scale / 441 transparency), which those
/// versions predate. AutoCAD/ODA render the fill from this metadata; without it
/// a down-saved dimension text (or any MTEXT) shows no background. Applied only
/// when the entity has no native fill flag. The colour is stored with the
/// AutoCAD "method byte" (0xC2 = RGB, 0xC3 = ACI, 0xC0/0xC1 = ByLayer/ByBlock).
pub(crate) fn recover_mtext_bg_roundtrip(document: &mut crate::document::CadDocument) {
    use crate::entities::EntityType;
    use crate::types::{Color, Handle};
    use crate::xdata::XDataValue;

    fn color_from_method(v: i32) -> Color {
        let u = v as u32;
        match (u >> 24) & 0xFF {
            0xC0 => Color::ByLayer,
            0xC1 => Color::ByBlock,
            0xC3 => Color::from_index((u & 0xFF) as i16),
            _ => Color::Rgb {
                r: ((u >> 16) & 0xFF) as u8,
                g: ((u >> 8) & 0xFF) as u8,
                b: (u & 0xFF) as u8,
            },
        }
    }

    let mut recovered: Vec<(Handle, i32, f64, Color, i32)> = Vec::new();
    for e in document.entities() {
        let EntityType::MText(m) = e else { continue };
        if m.background_fill_flags != 0 {
            continue; // native fill present — nothing to recover
        }
        let Some(rec) = m
            .common
            .extended_data
            .records()
            .iter()
            .find(|r| r.application_name.eq_ignore_ascii_case("ACAD"))
        else {
            continue;
        };
        let vals = &rec.values;
        let Some(begin) = vals
            .iter()
            .position(|v| matches!(v, XDataValue::String(s) if s == "ACAD_MTEXT_BBRT"))
        else {
            continue;
        };
        // Walk the BBRT…BERT block as (Integer16 code, value) pairs.
        let (mut flags, mut scale, mut color, mut transp) = (0i32, 1.0f64, Color::ByLayer, 0i32);
        let mut i = begin + 1;
        while i + 1 < vals.len() {
            if matches!(&vals[i], XDataValue::String(s) if s == "ACAD_MTEXT_BERT") {
                break;
            }
            if let XDataValue::Integer16(code) = vals[i] {
                match (code, &vals[i + 1]) {
                    (91, XDataValue::Integer32(v)) => flags = *v,
                    (46, XDataValue::Real(v)) => scale = *v,
                    (64, XDataValue::Integer32(v)) => color = color_from_method(*v),
                    (442, XDataValue::Integer32(v)) => transp = *v,
                    _ => {}
                }
                i += 2;
            } else {
                i += 1;
            }
        }
        if flags != 0 {
            recovered.push((m.common.handle, flags, scale, color, transp));
        }
    }
    for (h, flags, scale, color, transp) in recovered {
        if let Some(EntityType::MText(m)) = document.get_entity_mut(h) {
            m.background_fill_flags = flags;
            m.background_scale = scale;
            m.background_color = color;
            m.background_transparency = transp;
        }
    }
}
