//! AC1021 (R2007) file header writer
//!
//! Implements the page-based DWG file format for R2007, which differs from
//! AC18 in three critical ways:
//!
//! 1. **Reed-Solomon encoding** — all pages are RS-encoded for error correction
//! 2. **LZ77 AC21 compression** — a different LZ77 variant than AC18
//! 3. **CRC-64 checksums** — 64-bit CRC (normal + mirrored) instead of CRC-32
//! 4. **UTF-16LE section names** — section map uses 2-byte Unicode names
//!
//! ## File Layout (spec §5.1)
//!
//! ```text
//! [0x80 bytes:  Metadata (version string, addresses)]
//! [0x400 bytes: File header (RS-encoded compressed metadata)]
//! [0x400 bytes: Page map (system page)]
//! [0x400 bytes: Page map copy]
//! [Data section pages ...]
//! [System page:  Section map]
//! [System page:  Section map copy]
//! [0x400 bytes: File header copy]
//! ```
//!
//! Based on ODA spec sections 5.1–5.13.

use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{Cursor, Seek, SeekFrom, Write};

use super::section_definition::{ac21_section_info, names};
use crate::document::{DwgAc21ContainerShape, DwgR2007SystemHeader};
use crate::error::DxfError;
use crate::io::dwg::compressor_ac21::compress_ac21;
use crate::io::dwg::crc::{
    dwg_ac21_check_data_mirrored_crc64, dwg_ac21_check_data_normal_crc64, dwg_ac21_header_crc64,
    dwg_ac21_mirrored_crc64, dwg_ac21_normal_crc64, dwg_ac21_normal_crc64_seed1,
    dwg_ac21_page_checksum,
};
use crate::io::dwg::dwg21_metadata::{Dwg21CompressedMetadata, METADATA_SIZE};
use crate::io::dwg::reed_solomon::{
    reed_solomon_encode, RS_DATA_K, RS_DATA_PRIM_POLY, RS_N, RS_SYSTEM_K, RS_SYSTEM_PRIM_POLY,
};
use crate::types::DxfVersion;

// ════════════════════════════════════════════════════════════════════════════
//  Constants
// ════════════════════════════════════════════════════════════════════════════

/// Size of the metadata block at the start of the file.
const METADATA_BLOCK_SIZE: usize = 0x80;

/// Size of the RS-encoded file header page.
const FILE_HEADER_PAGE_SIZE: usize = 0x400;

/// Total reserved size at the start: metadata + file header.
const RESERVED_HEADER_SIZE: usize = METADATA_BLOCK_SIZE + FILE_HEADER_PAGE_SIZE;

/// CRC block size (alignment unit) per spec §5.3.1.
const CRC_BLOCK_SIZE: usize = 8;

/// Page alignment size per spec §5.3.1.
const PAGE_ALIGN_SIZE: usize = 0x20;

/// Minimum system page size per spec §5.3.1.
const MIN_SYSTEM_PAGE_SIZE: usize = 0x400;

/// Check data size at end of file header page (5 × u64 = 40 = 0x28 bytes).
const CHECK_DATA_SIZE: usize = 0x28;

/// RS data available in the file header (0x400 - check_data = 0x3D8).
const RS_DATA_IN_HEADER: usize = FILE_HEADER_PAGE_SIZE - CHECK_DATA_SIZE;

/// Factor for file header RS encoding (3 sub-streams).
const FILE_HEADER_RS_FACTOR: usize = 3;

/// Shared by the header and implicit RS padding; fixed for deterministic output.
const FILE_RANDOM_SEED: u64 = 0;

// ════════════════════════════════════════════════════════════════════════════
//  CRC Random Encoder (spec §5.11)
// ════════════════════════════════════════════════════════════════════════════

/// Pseudo-random number generator for CRC encoding (spec §5.11).
///
/// Generates pseudo-random values from a seed for:
/// - CRC seed encoding
/// - Check data generation
/// - Random padding
///
/// Implements the ODA spec §5.11 CRC random encoding algorithm:
/// - LCG initialization for table entries 0-1
/// - MT-style initialization for entries 2-623
/// - InitPadding phase consuming first 128 entries
/// - No MT tempering on reads (direct table access)
/// - 10-bit spread encoding into specific bit positions
struct CrcRandomEncoder {
    table: Vec<u32>,
    index: usize,
    padding: Vec<u32>,
}

impl CrcRandomEncoder {
    /// Create a new encoder from a 64-bit seed (spec §5.11 Init).
    ///
    /// The seed is the RandomSeed value from the file header metadata.
    fn new(seed: u64) -> Self {
        let mut table = vec![0u32; 0x270];

        // LCG init for entries 0 and 1 (MSLCG: val * 0x343FD + 0x269EC3)
        table[0] = (seed as u32).wrapping_mul(0x343fd).wrapping_add(0x269ec3);
        table[1] = ((seed >> 32) as u32)
            .wrapping_mul(0x343fd)
            .wrapping_add(0x269ec3);

        // MT-style init for entries 2..624
        let mut value = table[1];
        for i in 2..0x270usize {
            value = ((value >> 30) ^ value)
                .wrapping_mul(0x6c078965)
                .wrapping_add(i as u32);
            table[i] = value;
        }

        let mut encoder = Self {
            table,
            index: 0,
            padding: vec![0u32; 0x80],
        };
        encoder.init_padding();
        encoder
    }

    /// Pre-compute padding table by consuming first 128 table entries (spec §5.11 InitPadding).
    fn init_padding(&mut self) {
        for i in 0..0x80 {
            self.update_index();
            self.padding[i] = self.table[self.index];
            self.index += 1;
        }
    }

    /// Check and handle table wrap-around (spec §5.11 UpdateIndex).
    /// When index reaches 0x270, regenerate using MT twist and reset.
    fn update_index(&mut self) {
        if self.index >= 0x270 {
            // Mersenne Twister twist step
            for i in 0..0x270 {
                let y = (self.table[i] & 0x80000000) | (self.table[(i + 1) % 0x270] & 0x7FFFFFFF);
                self.table[i] = self.table[(i + 0x18D) % 0x270] ^ (y >> 1);
                if y & 1 != 0 {
                    self.table[i] ^= 0x9908B0DF;
                }
            }
            self.index = 0;
        }
    }

    /// Get the next u64 value from two consecutive table entries (spec §5.11 GetNextUInt64).
    /// No MT tempering — reads raw table values.
    fn next_u64(&mut self) -> u64 {
        self.index += 2;
        self.update_index();
        let lo = self.table[self.index] as u64;
        let hi = self.table[self.index + 1] as u64;
        lo | (hi << 32)
    }

    /// Encode a value using 10-bit spread encoding (spec §5.11 Encode).
    ///
    /// Places 10 bits of the value at specific positions in a random u64,
    /// preserving the remaining random bits for obfuscation.
    fn encode(&mut self, value: u32) -> u64 {
        let random = self.next_u64();
        let mut lo = (random as u32) & 0xdf7df7dfu32;
        let mut hi = ((random >> 32) as u32) & 0xf7df7df7u32;

        // Place value bits at specific positions
        if value & 0x200 != 0 {
            lo |= 0x20;
        } // bit 9 → lo bit 5
        if value & 0x100 != 0 {
            lo |= 0x800;
        } // bit 8 → lo bit 11
        if value & 0x80 != 0 {
            lo |= 0x20000;
        } // bit 7 → lo bit 17
        if value & 0x40 != 0 {
            lo |= 0x800000;
        } // bit 6 → lo bit 23
        if value & 0x20 != 0 {
            lo |= 0x20000000;
        } // bit 5 → lo bit 29
        if value & 0x10 != 0 {
            hi |= 0x08;
        } // bit 4 → hi bit 3
        if value & 0x8 != 0 {
            hi |= 0x200;
        } // bit 3 → hi bit 9
        if value & 0x4 != 0 {
            hi |= 0x8000;
        } // bit 2 → hi bit 15
        if value & 0x2 != 0 {
            hi |= 0x200000;
        } // bit 1 → hi bit 21
        if value & 0x1 != 0 {
            hi |= 0x8000000;
        } // bit 0 → hi bit 27

        (lo as u64) | ((hi as u64) << 32)
    }

    /// Encode CRC seed (spec §5.2.1.1.3/4/6, §5.11).
    /// CrcSeed is u64 but only the low 10 bits are encoded per spec.
    fn encode_crc_seed(&mut self, seed: u64) -> u64 {
        self.encode(seed as u32)
    }

    /// Fill a buffer with random padding bytes from the padding table (spec §5.11).
    fn fill_random(&mut self, buffer: &mut [u8]) {
        let mut buf_idx = 0;
        let mut pad_idx = 0;
        while buf_idx + 4 <= buffer.len() {
            let val = if pad_idx < self.padding.len() {
                let v = self.padding[pad_idx];
                pad_idx += 1;
                v
            } else {
                // Fallback: get from main table
                self.index += 1;
                self.update_index();
                self.table[self.index]
            };
            buffer[buf_idx..buf_idx + 4].copy_from_slice(&val.to_le_bytes());
            buf_idx += 4;
        }
        if buf_idx < buffer.len() {
            let val = if pad_idx < self.padding.len() {
                self.padding[pad_idx]
            } else {
                self.index += 1;
                self.update_index();
                self.table[self.index]
            };
            let bytes = val.to_le_bytes();
            for j in 0..(buffer.len() - buf_idx) {
                buffer[buf_idx + j] = bytes[j];
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Section and Page tracking
// ════════════════════════════════════════════════════════════════════════════

/// Record for a single data page written to the stream.
#[derive(Debug, Clone)]
struct AC21PageRecord {
    /// Unique page ID (positive for data pages, negative for gaps).
    id: i64,
    /// Size of the RS-encoded page data in the file stream.
    size: i64,
    /// Absolute file offset where this page starts.
    offset: u64,
}

/// Information about a section and its pages.
#[derive(Debug, Clone)]
struct AC21SectionInfo {
    /// Section name (e.g., "AcDb:Header").
    name: String,
    /// Hash code from spec §5.2.
    hash_code: u32,
    /// Encoding mode (4=compressed, 1=uncompressed).
    encoding: u64,
    /// Default encryption mode.
    encryption: u64,
    /// Max decompressed page size (from spec §5.2 section table).
    max_page_size: u64,
    /// Total uncompressed data size.
    data_size: u64,
    /// Per-page records within this section.
    pages: Vec<AC21SectionPageRecord>,
}

/// Per-page record within a section (for section map serialization).
#[derive(Debug, Clone)]
struct AC21SectionPageRecord {
    /// Offset of this page's data within the decompressed section.
    data_offset: u64,
    /// Size of the RS-encoded page in the file stream.
    page_size: u64,
    /// Page ID (references the page map).
    page_id: i64,
    /// Decompressed size of this page.
    uncompressed_size: u64,
    /// Compressed size (before RS encoding).
    compressed_size: u64,
    /// Adler-32 variant checksum of decompressed data.
    checksum: u64,
    /// CRC-64 of the page.
    crc: u64,
}

struct EncodedDataPage {
    bytes: Vec<u8>,
    uncompressed_size: u64,
    compressed_size: u64,
    checksum: u64,
    crc: u64,
}

fn encode_data_page(data: &[u8], encoding: u64, skip_lz77: bool) -> EncodedDataPage {
    let checksum = dwg_ac21_page_checksum(0, data) as u64;
    if encoding == 1 {
        // §19 H7: the author's encoding-1 (stored) pages are RAW content
        // padded to the 0x32 alignment — no RS framing at all. Gold picks
        // its decode path by the page's physical size (decode_r2007.c:854:
        // the RS path iff page->size == page_size_if_rs_coded(comp)) and
        // the raw path memcpy's the payload, so a raw page must NEVER
        // collide with the RS form's size — the guard pads +0x20 past the
        // collision (the [225..248]-byte contents land exactly on the
        // 1-block RS form 0x100).
        let mut aligned = align32(data.len());
        let pesize = (data.len() + 7) & !7;
        let block_count = (pesize + 251 - 1) / 251;
        let rs_form = (block_count * 255 + 31) & !31;
        if aligned == rs_form {
            aligned += PAGE_ALIGN_SIZE;
        }
        let mut bytes = vec![0u8; aligned];
        bytes[..data.len()].copy_from_slice(data);
        return EncodedDataPage {
            bytes,
            uncompressed_size: data.len() as u64,
            compressed_size: data.len() as u64,
            checksum,
            crc: dwg_ac21_mirrored_crc64(0, data.len() as u32, data),
        };
    }

    let compressed = compress_ac21(data);
    let page_data = if skip_lz77 || compressed.len() >= data.len() {
        data
    } else {
        compressed.as_slice()
    };
    EncodedDataPage {
        bytes: rs_encode_data_page_interleaved(page_data, encoding),
        uncompressed_size: data.len() as u64,
        compressed_size: page_data.len() as u64,
        checksum,
        crc: dwg_ac21_mirrored_crc64(0, page_data.len() as u32, page_data),
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Page alignment and system page size calculation (spec §5.3.1)
// ════════════════════════════════════════════════════════════════════════════

/// Align a size to 32-byte (`PAGE_ALIGN_SIZE`) boundary.
fn align32(size: usize) -> usize {
    (size + PAGE_ALIGN_SIZE - 1) & !(PAGE_ALIGN_SIZE - 1)
}

/// Align a page size to `PAGE_ALIGN_SIZE` (0x20) boundary (u64 variant).
#[allow(dead_code)]
fn get_aligned_page_size(page_size: u64) -> u64 {
    (page_size + PAGE_ALIGN_SIZE as u64 - 1) & !(PAGE_ALIGN_SIZE as u64 - 1)
}

/// Calculate the system page size from uncompressed data size (spec §5.3.1).
///
/// The encoded data must fit at least twice in the page, with RS overhead.
#[allow(dead_code)]
fn get_system_page_size(data_size: u64) -> u64 {
    // Align to CRC block size
    let aligned = (data_size + CRC_BLOCK_SIZE as u64 - 1) & !(CRC_BLOCK_SIZE as u64 - 1);

    // The page should fit the data at least 2 times (with RS overhead)
    let file_page_size =
        ((aligned * 2) + RS_SYSTEM_K as u64 - 1) / RS_SYSTEM_K as u64 * RS_N as u64;

    if file_page_size < MIN_SYSTEM_PAGE_SIZE as u64 {
        MIN_SYSTEM_PAGE_SIZE as u64
    } else {
        get_aligned_page_size(file_page_size)
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Encode function (spec §5.2.1.1)
// ════════════════════════════════════════════════════════════════════════════

/// Rotate-left encode used for check data CRC buffers (spec §5.2.1.1).
fn encode_value(value: u64, control: u64) -> u64 {
    let shift = (control & 0x1F) as u32;
    if shift != 0 {
        (value << shift) | (value >> (64 - shift))
    } else {
        value
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  RS encoding helpers for data pages
// ════════════════════════════════════════════════════════════════════════════

/// Encoding 4 interleaves complete RS(255, 251) codewords, including padding.
fn rs_encode_data_page_interleaved(data: &[u8], _encoding: u64) -> Vec<u8> {
    // All data section pages use RS(255, 251) per spec §5.4 / §5.13.
    // System pages (page map, section map) use RS(255, 239) per §5.3,
    // but those are encoded separately in write_system_page().
    // The encoding field (1=stored, 4=compressed) determines only whether
    // LZ77 compression is applied, NOT the RS block size.
    let block_size = RS_DATA_K; // 251
    let prim_poly = RS_DATA_PRIM_POLY;

    // Match reader's factor computation:
    // total_size = (compressed_size + 7) & ~7   (CRC block alignment)
    // factor = ceil(total_size / block_size)
    let total_size = (data.len() + CRC_BLOCK_SIZE - 1) & !(CRC_BLOCK_SIZE - 1);
    let factor = (total_size + block_size - 1) / block_size;

    // Pad data to factor × block_size bytes
    let padded_len = factor * block_size;
    let mut padded_data = vec![0u8; padded_len];
    let copy_len = data.len().min(padded_len);
    padded_data[..copy_len].copy_from_slice(&data[..copy_len]);

    // RS-encode with interleaving
    let mut encoded = vec![0u8; factor * RS_N];
    reed_solomon_encode(&padded_data, &mut encoded, factor, block_size, prim_poly);

    encoded
}

// ════════════════════════════════════════════════════════════════════════════
//  Main Writer
// ════════════════════════════════════════════════════════════════════════════

/// AC1021 (R2007) file header writer.
///
/// Follows the same pattern as `DwgFileHeaderWriterAC18`:
/// 1. `new()` reserves header space
/// 2. `add_section()` writes data pages incrementally
/// 3. `write_file()` finalizes with section map, page map, file header, metadata
pub struct DwgFileHeaderWriterAC21 {
    /// DXF version being written.
    version: DxfVersion,
    /// Section info records, keyed by section name.
    sections: Vec<AC21SectionInfo>,
    /// Page map: all pages written so far.
    page_records: Vec<AC21PageRecord>,
    /// CRC-64 seed value.
    crc_seed: u64,
    /// Next page ID to assign.
    next_page_id: i64,
    /// Skip LZ77 compression for debugging (store raw + RS only).
    pub skip_lz77: bool,
    /// The source file's FILEHEADER identity bytes on a same-version
    /// roundtrip (§19 H7f + the 2026-09-26 review pass): the tuple is
    /// (`maint_rel_version`, `dwg_version`, `maint_version`, codepage,
    /// the unknown 0x15 byte, `app_dwg_version`, `app_maint_version`).
    /// `None` keeps the historical R2007 constants.
    source_header_bytes: Option<(u8, u8, u8, u16, u8, u8, u8)>,
    /// The author's `random_seed` for a same-version roundtrip (§19
    /// H7g): the seed IS the CRC random encoder's input (spec
    /// §5.2.1.1.1), so adopting it makes the derived fields —
    /// `sections_map_crc_seed`, `pages_map_crc_seed` and
    /// `crc_seed_encoded`, all deterministic draws from (seed,
    /// crc_seed) — land the author's values too (every corpus author
    /// carries `crc_seed` 0 like our writer). `None` keeps the
    /// historical constant (0).
    source_random_seed: Option<u64>,
    /// The §19 H8b mirrored-emission finalize inputs, stored by
    /// `write_mirrored_pages` for `write_file_mirrored`.
    mirror_state: Option<MirrorState>,
}

impl DwgFileHeaderWriterAC21 {
    /// Create a new AC21 file header writer and reserve space.
    ///
    /// Reserves 0x480 bytes at the start:
    /// - 0x80 bytes: metadata placeholder
    /// - 0x400 bytes: file header placeholder
    ///
    /// Data pages start immediately after. Page maps are written at the
    /// end of the file with a dynamically computed size.
    pub fn new<W: Write + Seek>(version: DxfVersion, output: &mut W) -> Result<Self, DxfError> {
        // Reserve space for metadata + file header only
        let zeroes = vec![0u8; RESERVED_HEADER_SIZE];
        output.write_all(&zeroes)?;

        Ok(Self {
            version,
            sections: Vec::new(),
            page_records: Vec::new(),
            crc_seed: 0,
            next_page_id: 1,
            skip_lz77: false,
            source_header_bytes: None,
            source_random_seed: None,
            mirror_state: None,
        })
    }

    /// Mirror the source author's `random_seed` (§19 H7g). Must be set
    /// before the metadata is finalized; the derived CRC-seed fields
    /// follow the author's RNG sequence then.
    pub fn set_source_random_seed(&mut self, random_seed: u64) {
        self.source_random_seed = Some(random_seed);
    }

    /// Mirror the source file's FILEHEADER identity bytes (§19 H7f,
    /// extended by the 2026-09-26 review pass with the 0x15 unknown
    /// byte): the AC18 writer's byte set plus the R2007 codepage (the
    /// AC21 metadata hardcoded it before this row).
    pub fn set_source_header_bytes(
        &mut self,
        maint_rel_version: u8,
        dwg_version: u8,
        maint_version: u8,
        codepage: u16,
        unknown_header_byte: u8,
        app_dwg_version: u8,
        app_maint_version: u8,
    ) {
        self.source_header_bytes = Some((
            maint_rel_version,
            dwg_version,
            maint_version,
            codepage,
            unknown_header_byte,
            app_dwg_version,
            app_maint_version,
        ));
    }

    /// Get the file offset where the AcDbObjects section starts.
    ///
    /// For AC21, handles are relative to decompressed section data (always 0).
    pub fn handle_section_offset(&self) -> usize {
        0
    }

    /// Add a section to the file.
    ///
    /// The section data is partitioned into pages based on the section's
    /// spec-defined page size, each page is optionally compressed, RS-encoded,
    /// and written to the output stream.
    pub fn add_section<W: Write + Seek>(
        &mut self,
        output: &mut W,
        name: &str,
        data: &[u8],
    ) -> Result<(), DxfError> {
        let perf = std::env::var_os("PERF").is_some();
        let started = web_time::Instant::now();
        let hash_code = ac21_section_info::hash_code(name)
            .ok_or_else(|| DxfError::InvalidFormat(format!("Unknown AC21 section: {}", name)))?;
        let encoding = ac21_section_info::encoding(name).unwrap_or(1);
        // We don't implement XOR obfuscation (encryption=2), so always
        // store encryption=0 in the section map.  AutoCAD will read the
        // data without attempting to decrypt, which is correct.
        let encryption: u64 = 0;
        let mut max_page_size = ac21_section_info::page_size(name).unwrap_or(0xF800);
        if name == names::PREVIEW {
            // Preview image offsets address contiguous bytes in the file.
            // Splitting it would insert RS parity in the middle of the image.
            max_page_size = max_page_size.max(align32(data.len()) as u64);
        }
        if name == names::APP_INFO_HISTORY {
            // §19 H7 AppInfoHistory: the author's R2007 files carry the
            // section as ONE page sized to the content rounded up to 0x80
            // (observed: 1296→1408, 1502→1536, 1710→1792) — mirror that
            // form instead of splitting across 0xF800 pages.
            max_page_size = (data.len() as u64 + 0x7F) & !0x7F;
        }

        let mut section = AC21SectionInfo {
            name: name.to_string(),
            hash_code,
            encoding,
            encryption,
            max_page_size,
            data_size: data.len() as u64,
            pages: Vec::new(),
        };

        // Partition and encode pages in parallel. Stream offsets/page ids stay
        // deterministic because completed pages are written serially in input
        // order after CPU-heavy LZ77/RS work finishes.
        let page_size = max_page_size as usize;
        use crate::io::dwg::parallel::{map_chunks_indexed, worker_count};
        let skip_lz77 = self.skip_lz77;
        let batch_pages = worker_count().saturating_mul(2);
        let batch_bytes = page_size.saturating_mul(batch_pages).max(page_size);
        for (batch_index, batch) in data.chunks(batch_bytes).enumerate() {
            let first_page = batch_index.saturating_mul(batch_pages);
            let encoded: Vec<_> = map_chunks_indexed(batch, page_size, |index, bytes| {
                (
                    (first_page + index) as u64 * max_page_size,
                    encode_data_page(bytes, encoding, skip_lz77),
                )
            });

            for (offset, page) in encoded {
                let page_record =
                    self.write_encoded_data_page(output, page, offset, max_page_size)?;
                section.pages.push(page_record);
            }
        }

        let pages = section.pages.len();
        self.sections.push(section);
        if perf {
            eprintln!(
                "[perf] dwg-write-section name={} format=ac21 time={:.1}ms bytes={} pages={} encoding={}",
                name,
                started.elapsed().as_secs_f64() * 1000.0,
                data.len(),
                pages,
                encoding,
            );
        }
        Ok(())
    }

    /// Finalize the file: write section map, page map, file header, metadata.
    ///
    /// ## File layout (spec §5.1):
    /// ```text
    /// [0x80  Metadata]
    /// [0x400 File header]
    /// [0x400 Page map (system page)]
    /// [0x400 Page map copy]
    /// [Data section pages ...]
    /// [Section map (system page)]
    /// [Section map copy]
    /// [0x400 File header copy]
    /// ```
    pub fn write_file<W: Write + Seek>(&mut self, output: &mut W) -> Result<(), DxfError> {
        // Initialize CRC seed (0 matches AutoCAD reference files)
        self.crc_seed = 0;

        // ── Step 1: Write section map + copy (appended after data pages) ──
        let section_map_data = self.build_section_map()?;
        let section_map_result = self.write_system_page(output, &section_map_data)?;
        let section_map2_result = self.write_system_page(output, &section_map_data)?;

        // ── Step 2: Build and write page map at end of file ──
        // Page map pages are appended after section maps (at the current
        // end-of-file position).  Their size is computed dynamically to
        // accommodate any number of data pages.
        let page_map_page_id = self.next_page_id;
        self.next_page_id += 1;
        let page_map2_page_id = self.next_page_id;
        self.next_page_id += 1;

        // Compute the actual required page size for the page map.
        let pm_raw_size_est = (self.page_records.len() + 3) * 16;
        let pm_page_size = get_system_page_size(pm_raw_size_est as u64) as usize;

        // Build the page map: data pages first (lowest file offset after
        // header), then section map pages, then page map pages (at end).
        let page_map_data =
            self.build_page_map_ordered(page_map_page_id, page_map2_page_id, pm_page_size as i64);

        // Write page map pages at current end-of-file position
        let pm_abs_offset = output.seek(SeekFrom::Current(0))?;
        let page_map_result = self.write_system_page_at(
            output,
            &page_map_data,
            pm_abs_offset,
            page_map_page_id,
            pm_page_size,
        )?;
        let pm_abs_offset2 = output.seek(SeekFrom::Current(0))?;
        let _page_map2_result = self.write_system_page_at(
            output,
            &page_map_data,
            pm_abs_offset2,
            page_map2_page_id,
            pm_page_size,
        )?;

        // ── Step 4: Build compressed metadata ──
        let mut metadata = Dwg21CompressedMetadata::default();
        metadata.file_size = 0; // Patched after writing header copy

        // Pages map fields — offsets relative to data start (RESERVED_HEADER_SIZE)
        metadata.pages_map_crc_compressed = page_map_result.crc_compressed;
        metadata.pages_map_correction_factor = page_map_result.correction_factor;
        // pages_map_crc_seed is set later by the CRC random encoder (§5.2.1.1.4)
        metadata.pages_map_offset = pm_abs_offset - RESERVED_HEADER_SIZE as u64;
        metadata.pages_map_id = page_map_page_id as u64;
        metadata.map2_offset = pm_abs_offset2 - RESERVED_HEADER_SIZE as u64;
        metadata.map2_id = page_map2_page_id as u64;
        metadata.pages_map_size_compressed = page_map_result.compressed_size;
        metadata.pages_map_size_uncompressed = page_map_result.uncompressed_size;
        // Total pages = data pages + section map pages + page map pages
        let total_pages = self.page_records.len() as u64 + 2; // +2 for page map pages
        metadata.pages_amount = total_pages;
        metadata.pages_max_id = (self.next_page_id - 1) as u64;
        metadata.pages_map_crc_uncompressed = page_map_result.crc_uncompressed;

        // Sections map fields
        metadata.sections_amount = (self.sections.len() + 1) as u64; // +1 per spec
        metadata.sections_map_crc_uncompressed = section_map_result.crc_uncompressed;
        metadata.sections_map_size_compressed = section_map_result.compressed_size;
        metadata.sections_map2_id = section_map2_result.page_id as u64;
        metadata.sections_map_id = section_map_result.page_id as u64;
        metadata.sections_map_size_uncompressed = section_map_result.uncompressed_size;
        metadata.sections_map_crc_compressed = section_map_result.crc_compressed;
        metadata.sections_map_correction_factor = section_map_result.correction_factor;

        // CRC/random fields (spec §5.2.1.1 — order is critical for RNG state)
        // §5.2.1.1.1: RandomSeed IS the CRC encoder's seed (input, not output).
        // §19 H7g: a same-version roundtrip adopts the AUTHOR's seed —
        // a retained file identity (gold's `R2007_Header.random_seed`),
        // decode-inert (gold only prints it in JSON) — and every
        // downstream draw (`sections_map_crc_seed`,
        // `pages_map_crc_seed`, `crc_seed_encoded`) is a deterministic
        // function of (seed, crc_seed), so those fields land the
        // author's values too.
        let random_seed = self.source_random_seed.unwrap_or(FILE_RANDOM_SEED);
        metadata.random_seed = random_seed;
        metadata.crc_seed = self.crc_seed; // always 0 per §5.2.1.1.2
        self.write_two_pass_header(output, &mut metadata, random_seed)?;

        Ok(())
    }

    /// The shared two-pass file-header + metadata finalization tail
    /// (§19 H8b): the CRC-seed draws and check data (spec §5.2.1.1.3-6),
    /// the 0x400 file header page (written at 0x80 and as the header2
    /// copy at end-of-file), the file_size/header2_offset patch pass,
    /// and the 0x80 metadata block (§5.2.1.7). Extracted verbatim from
    /// the historical `write_file` body — the mirrored finalize shares
    /// this tail, and the conventional path keeps byte-identical
    /// output.
    fn write_two_pass_header<W: Write + Seek>(
        &mut self,
        output: &mut W,
        metadata: &mut Dwg21CompressedMetadata,
        random_seed: u64,
    ) -> Result<(), DxfError> {
        let mut rng = CrcRandomEncoder::new(random_seed);

        // §5.2.1.1.3-4: Encode map CRC seeds
        metadata.sections_map_crc_seed = rng.encode_crc_seed(self.crc_seed);
        metadata.pages_map_crc_seed = rng.encode_crc_seed(self.crc_seed);

        // §5.2.1.1.5: Check data (random1, random2, encoded_seed, normal/mirrored CRC)
        let check_random1 = rng.next_u64();
        let check_random2 = rng.next_u64();
        let check_encoded_seed = rng.encode_crc_seed(self.crc_seed);

        let check_normal_crc = {
            let mut buf = [0u64; 8];
            buf[0] = encode_value(check_random1, check_random2);
            buf[1] = encode_value(buf[0], buf[0]);
            buf[2] = encode_value(check_random2, buf[1]);
            buf[3] = encode_value(buf[2], buf[2]);
            buf[4] = encode_value(check_random1, buf[3]);
            buf[5] = encode_value(buf[4], buf[4]);
            buf[6] = encode_value(buf[5], buf[5]);
            buf[7] = encode_value(buf[6], buf[6]);
            let mut bytes = [0u8; 64];
            for (i, &val) in buf.iter().enumerate() {
                bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
            }
            dwg_ac21_check_data_normal_crc64(check_random2, &bytes)
        };

        let check_mirrored_crc = {
            let mut buf = [0u64; 8];
            buf[0] = encode_value(check_random1, check_random2);
            buf[1] = encode_value(check_normal_crc, buf[0]);
            buf[2] = encode_value(check_random2, buf[1]);
            buf[3] = encode_value(check_normal_crc, buf[2]);
            buf[4] = encode_value(check_random1, buf[3]);
            buf[5] = encode_value(check_normal_crc, buf[4]);
            buf[6] = encode_value(check_random2, buf[5]);
            buf[7] = encode_value(buf[6], buf[6]);
            let mut bytes = [0u8; 64];
            for (i, &val) in buf.iter().enumerate() {
                bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
            }
            dwg_ac21_check_data_mirrored_crc64(check_random1, &bytes)
        };

        let mut check_data = [0u8; CHECK_DATA_SIZE];
        check_data[0..8].copy_from_slice(&check_normal_crc.to_le_bytes());
        check_data[8..16].copy_from_slice(&check_mirrored_crc.to_le_bytes());
        check_data[16..24].copy_from_slice(&check_random1.to_le_bytes());
        check_data[24..32].copy_from_slice(&check_random2.to_le_bytes());
        check_data[32..40].copy_from_slice(&check_encoded_seed.to_le_bytes());

        // §5.2.1.1.6: CrcSeedEncoded (AFTER check data in RNG sequence)
        metadata.crc_seed_encoded = rng.encode_crc_seed(self.crc_seed);

        // Compute header CRC-64 (spec §5.2.1.2)
        let meta_bytes = metadata.to_bytes();
        let header_crc = dwg_ac21_header_crc64(&meta_bytes);
        metadata.header_crc64 = header_crc;

        // §§ Step 5: Build file header page —
        let file_header_page = self.build_file_header_page(metadata, &mut rng, &check_data)?;
        debug_assert_eq!(file_header_page.len(), FILE_HEADER_PAGE_SIZE);

        // Write file header at offset 0x80
        output.seek(SeekFrom::Start(METADATA_BLOCK_SIZE as u64))?;
        output.write_all(&file_header_page)?;

        // Write file header copy at end of file
        output.seek(SeekFrom::End(0))?;
        let header2_offset = output.seek(SeekFrom::Current(0))?;
        output.write_all(&file_header_page)?;

        // ── Step 6: Patch file size and header2_offset ──
        let file_size = output.seek(SeekFrom::Current(0))?;
        metadata.file_size = file_size;
        metadata.header2_offset = header2_offset - RESERVED_HEADER_SIZE as u64;

        // Recompute with updated metadata (same RNG order as pass 1)
        let mut rng2 = CrcRandomEncoder::new(random_seed);
        metadata.sections_map_crc_seed = rng2.encode_crc_seed(self.crc_seed); // §5.2.1.1.3
        metadata.pages_map_crc_seed = rng2.encode_crc_seed(self.crc_seed); // §5.2.1.1.4
        let check_random1_2 = rng2.next_u64();
        let check_random2_2 = rng2.next_u64();
        let check_encoded_seed_2 = rng2.encode_crc_seed(self.crc_seed);
        let check_normal_crc_2 = {
            let mut buf = [0u64; 8];
            buf[0] = encode_value(check_random1_2, check_random2_2);
            buf[1] = encode_value(buf[0], buf[0]);
            buf[2] = encode_value(check_random2_2, buf[1]);
            buf[3] = encode_value(buf[2], buf[2]);
            buf[4] = encode_value(check_random1_2, buf[3]);
            buf[5] = encode_value(buf[4], buf[4]);
            buf[6] = encode_value(buf[5], buf[5]);
            buf[7] = encode_value(buf[6], buf[6]);
            let mut bytes = [0u8; 64];
            for (i, &val) in buf.iter().enumerate() {
                bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
            }
            dwg_ac21_check_data_normal_crc64(check_random2_2, &bytes)
        };
        let check_mirrored_crc_2 = {
            let mut buf = [0u64; 8];
            buf[0] = encode_value(check_random1_2, check_random2_2);
            buf[1] = encode_value(check_normal_crc_2, buf[0]);
            buf[2] = encode_value(check_random2_2, buf[1]);
            buf[3] = encode_value(check_normal_crc_2, buf[2]);
            buf[4] = encode_value(check_random1_2, buf[3]);
            buf[5] = encode_value(check_normal_crc_2, buf[4]);
            buf[6] = encode_value(check_random2_2, buf[5]);
            buf[7] = encode_value(buf[6], buf[6]);
            let mut bytes = [0u8; 64];
            for (i, &val) in buf.iter().enumerate() {
                bytes[i * 8..(i + 1) * 8].copy_from_slice(&val.to_le_bytes());
            }
            dwg_ac21_check_data_mirrored_crc64(check_random1_2, &bytes)
        };
        let mut check_data_2 = [0u8; CHECK_DATA_SIZE];
        check_data_2[0..8].copy_from_slice(&check_normal_crc_2.to_le_bytes());
        check_data_2[8..16].copy_from_slice(&check_mirrored_crc_2.to_le_bytes());
        check_data_2[16..24].copy_from_slice(&check_random1_2.to_le_bytes());
        check_data_2[24..32].copy_from_slice(&check_random2_2.to_le_bytes());
        check_data_2[32..40].copy_from_slice(&check_encoded_seed_2.to_le_bytes());
        metadata.crc_seed_encoded = rng2.encode_crc_seed(self.crc_seed); // §5.2.1.1.6
        let meta_bytes = metadata.to_bytes();
        let header_crc = dwg_ac21_header_crc64(&meta_bytes);
        metadata.header_crc64 = header_crc;

        let file_header_page = self.build_file_header_page(metadata, &mut rng2, &check_data_2)?;

        // Overwrite file header at 0x80
        output.seek(SeekFrom::Start(METADATA_BLOCK_SIZE as u64))?;
        output.write_all(&file_header_page)?;

        // Overwrite file header copy at end
        output.seek(SeekFrom::Start(header2_offset))?;
        output.write_all(&file_header_page)?;

        // ── Step 7: Write metadata at offset 0x00 ──
        self.write_metadata(output, metadata)?;

        // Seek to end
        output.seek(SeekFrom::End(0))?;

        Ok(())
    }

    // ── Data page writing ──

    /// Write a single data page to the output stream.
    ///
    /// Write a pre-encoded page and assign its deterministic stream id/offset.
    fn write_encoded_data_page<W: Write + Seek>(
        &mut self,
        output: &mut W,
        page: EncodedDataPage,
        data_offset: u64,
        max_page_size: u64,
    ) -> Result<AC21SectionPageRecord, DxfError> {
        let page_id = self.next_page_id;
        self.next_page_id += 1;

        let aligned_size = align32(page.bytes.len());
        let offset = output.seek(SeekFrom::Current(0))?;
        output.write_all(&page.bytes)?;
        let padding = aligned_size - page.bytes.len();
        if padding > 0 {
            output.write_all(&[0u8; PAGE_ALIGN_SIZE][..padding])?;
        }

        self.page_records.push(AC21PageRecord {
            id: page_id,
            size: aligned_size as i64,
            offset,
        });
        Ok(AC21SectionPageRecord {
            data_offset,
            page_size: max_page_size,
            page_id,
            uncompressed_size: page.uncompressed_size,
            compressed_size: page.compressed_size,
            checksum: page.checksum,
            crc: page.crc,
        })
    }

    // ── System page writing ──

    /// Compress and RS-encode system page data.
    ///
    /// The `target_page_size` determines the correction factor: we compute the
    /// maximum correction factor that fills the page without exceeding it.
    /// This matches how AutoCAD computes the factor (spec §5.3).
    ///
    /// Returns the encoded bytes and metadata (does NOT write to stream).
    fn encode_system_page(
        &self,
        data: &[u8],
        target_page_size: usize,
    ) -> (Vec<u8>, u64, u64, u64, u64, u64) {
        // CRCs on uncompressed data (Mirrored CRC-64 per spec §5.3, §7.2)
        let uncompressed_size = data.len() as u64;
        let crc_uncompressed = dwg_ac21_mirrored_crc64(self.crc_seed, data.len() as u32, data);

        // Compress
        let compressed = compress_ac21(data);
        let use_compressed = compressed.len() < data.len();
        let page_data = if use_compressed { &compressed } else { data };
        let compressed_size = if use_compressed {
            compressed.len() as u64
        } else {
            data.len() as u64
        };

        // Compressed CRC (Mirrored CRC-64 per spec §5.3, §7.2)
        let crc_compressed =
            dwg_ac21_mirrored_crc64(self.crc_seed, page_data.len() as u32, page_data);

        // Compute correction factor to fill the target page size optimally.
        // max_factor = max RS blocks that fit in the page
        // correction_factor = floor(max_factor * RS_SYSTEM_K / align8(compressed_size))
        // This maximizes error correction redundancy, matching ref file behavior.
        let aligned_comp = ((page_data.len() as u64 + 7) & !7) as usize;
        let correction_factor: u64 = if aligned_comp == 0 {
            1
        } else {
            let max_rs_blocks = target_page_size / RS_N; // floor(target / 255)
            let max_data_capacity = max_rs_blocks * RS_SYSTEM_K;
            let cf = max_data_capacity / aligned_comp;
            cf.max(2) as u64 // minimum correction factor of 2
        };
        let total_size = aligned_comp * correction_factor as usize;
        let factor = (total_size + RS_SYSTEM_K - 1) / RS_SYSTEM_K;

        // RS-encode with RS(255, 239)
        let mut padded_data = vec![0u8; factor * RS_SYSTEM_K];
        // The correction factor advertises repeated copies, not zero padding.
        if aligned_comp != 0 {
            for copy in padded_data[..total_size].chunks_mut(aligned_comp) {
                copy[..page_data.len()].copy_from_slice(page_data);
            }
        }

        let mut encoded = vec![0u8; factor * RS_N];
        reed_solomon_encode(
            &padded_data,
            &mut encoded,
            factor,
            RS_SYSTEM_K,
            RS_SYSTEM_PRIM_POLY,
        );

        (
            encoded,
            compressed_size,
            uncompressed_size,
            crc_compressed,
            crc_uncompressed,
            correction_factor,
        )
    }

    /// Write a system page (section map) per spec §5.3.
    ///
    /// Page size is computed from the uncompressed data size per §5.3.1,
    /// ensuring the RS-encoded data fits the page at least 2 times.
    fn write_system_page<W: Write + Seek>(
        &mut self,
        output: &mut W,
        data: &[u8],
    ) -> Result<SystemPageResult, DxfError> {
        let page_id = self.next_page_id;
        self.next_page_id += 1;

        // Compute page size from uncompressed data size per spec §5.3.1
        let target_page_size = get_system_page_size(data.len() as u64) as usize;
        let (
            encoded,
            compressed_size,
            uncompressed_size,
            crc_compressed,
            crc_uncompressed,
            correction_factor,
        ) = self.encode_system_page(data, target_page_size);

        // Page size must accommodate the encoded data
        let page_size = align32(encoded.len()).max(target_page_size);

        let offset = output.seek(SeekFrom::Current(0))?;
        output.write_all(&encoded)?;
        let padding = page_size - encoded.len();
        if padding > 0 {
            output.write_all(&vec![0u8; padding])?;
        }

        self.page_records.push(AC21PageRecord {
            id: page_id,
            size: page_size as i64,
            offset,
        });

        Ok(SystemPageResult {
            page_id,
            offset,
            compressed_size,
            uncompressed_size,
            crc_compressed,
            crc_uncompressed,
            correction_factor,
        })
    }

    /// Write a system page at a specific absolute position (for page map).
    ///
    /// Unlike `write_system_page`, this seeks to `abs_position` and does NOT
    /// add to `page_records` (caller handles page map entries separately).
    fn write_system_page_at<W: Write + Seek>(
        &self,
        output: &mut W,
        data: &[u8],
        abs_position: u64,
        page_id: i64,
        target_size: usize,
    ) -> Result<SystemPageResult, DxfError> {
        let (
            encoded,
            compressed_size,
            uncompressed_size,
            crc_compressed,
            crc_uncompressed,
            correction_factor,
        ) = self.encode_system_page(data, target_size);

        // Use the larger of encoded size and target size
        let actual_page_size = align32(encoded.len()).max(target_size);

        output.seek(SeekFrom::Start(abs_position))?;
        output.write_all(&encoded)?;
        let padding = actual_page_size - encoded.len();
        if padding > 0 {
            output.write_all(&vec![0u8; padding])?;
        }

        Ok(SystemPageResult {
            page_id,
            offset: abs_position,
            compressed_size,
            uncompressed_size,
            crc_compressed,
            crc_uncompressed,
            correction_factor,
        })
    }

    // ── Section map building ──

    /// Build the section map data (spec §5.2 section map format).
    ///
    /// Entries are emitted in the spec-mandated section map order
    /// (`SECTION_MAP_ORDER`), which differs from the physical stream order.
    /// For each section: 0x40-byte header + UTF-16LE name + per-page records.
    fn build_section_map(&self) -> Result<Vec<u8>, DxfError> {
        serialize_section_map(&self.sections)
    }
}

/// Serialize the section map data (spec §5.2 section map format) from a
/// section-record list — the writer's own table (conventional passes
/// `self.sections`; the §19 H8b mirror passes its prepared records).
/// Sorting and field order are identical for both callers.
fn serialize_section_map(sections: &[AC21SectionInfo]) -> Result<Vec<u8>, DxfError> {
    let mut stream = Vec::new();

    // Build index mapping section name → position in spec section-map order.
    let map_order = ac21_section_info::SECTION_MAP_ORDER;

    // Sort sections by the spec section-map order.
    let mut sorted: Vec<&AC21SectionInfo> = sections.iter().collect();
    sorted.sort_by_key(|s| {
        map_order
            .iter()
            .position(|&n| n == s.name)
            .unwrap_or(usize::MAX)
    });

    for section in &sorted {
            // Section header (8 fields × 8 bytes = 0x40 bytes)

            // 0x00: Data size (8 bytes)
            stream.write_u64::<LittleEndian>(section.data_size)?;
            // 0x08: Max size / page size (8 bytes)
            stream.write_u64::<LittleEndian>(section.max_page_size)?;
            // 0x10: Encryption (8 bytes)
            stream.write_u64::<LittleEndian>(section.encryption)?;
            // 0x18: HashCode (8 bytes)
            stream.write_u64::<LittleEndian>(section.hash_code as u64)?;
            // 0x20: SectionNameLength (8 bytes) — byte count of name data
            //   The reader reads exactly this many bytes, so it must be the
            //   total UTF-16LE byte count including the null terminator.
            let name_chars: Vec<u16> = section.name.encode_utf16().collect();
            let name_byte_len = if name_chars.is_empty() {
                0u64
            } else {
                (name_chars.len() as u64 + 1) * 2 // chars + null, each 2 bytes
            };
            stream.write_u64::<LittleEndian>(name_byte_len)?;
            // 0x28: Unknown (8 bytes)
            stream.write_u64::<LittleEndian>(0)?;
            // 0x30: Encoding (8 bytes)
            stream.write_u64::<LittleEndian>(section.encoding)?;
            // 0x38: NumPages (8 bytes)
            stream.write_u64::<LittleEndian>(section.pages.len() as u64)?;

            // 0x40: UTF-16LE section name + null terminator (if name_len > 0)
            if !name_chars.is_empty() {
                for &ch in &name_chars {
                    stream.write_u16::<LittleEndian>(ch)?;
                }
                // Null terminator (2 bytes)
                stream.write_u16::<LittleEndian>(0)?;
            }

            // Per-page records (7 × u64 = 56 bytes each)
            for page in &section.pages {
                stream.write_u64::<LittleEndian>(page.data_offset)?;
                stream.write_u64::<LittleEndian>(page.page_size)?;
                stream.write_u64::<LittleEndian>(page.page_id as u64)?;
                stream.write_u64::<LittleEndian>(page.uncompressed_size)?;
                stream.write_u64::<LittleEndian>(page.compressed_size)?;
                stream.write_u64::<LittleEndian>(page.checksum)?;
                stream.write_u64::<LittleEndian>(page.crc)?;
            }
        }

        Ok(stream)
}

impl DwgFileHeaderWriterAC21 {
    // ── Page map building ──

    /// Build the page map data (spec §5.2 page map format).
    ///
    /// Entries are ordered by file position:
    /// 1. Data pages + section map pages (from self.page_records, in write order)
    /// 2. Page map pages (written last, at end of file)
    ///
    /// Each entry is (size: i64, id: i64).
    fn build_page_map_ordered(
        &self,
        page_map_id: i64,
        page_map2_id: i64,
        page_map_page_size: i64,
    ) -> Vec<u8> {
        // +2 for page map pages, +1 for null terminator
        let capacity = (self.page_records.len() + 3) * 16;
        let mut stream = Vec::with_capacity(capacity);

        // 1. Data pages + section map pages (in file-offset order, starting at RESERVED_HEADER_SIZE)
        for record in &self.page_records {
            stream.extend_from_slice(&record.size.to_le_bytes());
            stream.extend_from_slice(&record.id.to_le_bytes());
        }

        // 2. Page map pages come last (they're at the end of the file)
        stream.extend_from_slice(&page_map_page_size.to_le_bytes());
        stream.extend_from_slice(&page_map_id.to_le_bytes());
        stream.extend_from_slice(&page_map_page_size.to_le_bytes());
        stream.extend_from_slice(&page_map2_id.to_le_bytes());

        // 3. Null terminator (size=0, id=0) per spec §5.3
        stream.extend_from_slice(&0i64.to_le_bytes());
        stream.extend_from_slice(&0i64.to_le_bytes());

        stream
    }

    // ── File header page building (spec §5.2.1) ──

    /// Build the 0x400-byte file header page (spec §5.2.1 steps 2-7).
    /// Check data is pre-computed by caller (spec §5.2.1.1.5) for correct RNG ordering.
    fn build_file_header_page(
        &self,
        metadata: &Dwg21CompressedMetadata,
        rng: &mut CrcRandomEncoder,
        check_data: &[u8; CHECK_DATA_SIZE],
    ) -> Result<Vec<u8>, DxfError> {
        // Step 2: Serialize metadata and compute header CRC-64 (already done in caller)
        let meta_bytes = metadata.to_bytes();
        debug_assert_eq!(meta_bytes.len(), METADATA_SIZE);

        // Step 3: Compress metadata (spec §5.2.1.3)
        let compressed = compress_ac21(&meta_bytes);
        let use_compressed = compressed.len() < meta_bytes.len();
        let compr_data = if use_compressed {
            &compressed
        } else {
            &meta_bytes
        };
        let compr_len = if use_compressed {
            compr_data.len() as i32
        } else {
            -(meta_bytes.len() as i32)
        };

        // Compressed CRC (spec §5.2.1.3)
        let compr_crc = dwg_ac21_normal_crc64(0, compr_data.len() as u32, compr_data);

        // Step 4: Checking sequence (spec §5.2.1.4)
        let check_seq_val1 = rng.next_u64();
        let check_seq_val2 = encode_value(check_seq_val1, check_seq_val1);

        let mut check_seq_bytes = [0u8; 16];
        check_seq_bytes[0..8].copy_from_slice(&check_seq_val1.to_le_bytes());
        check_seq_bytes[8..16].copy_from_slice(&check_seq_val2.to_le_bytes());

        let check_seq_crc = dwg_ac21_normal_crc64_seed1(0, 16, &check_seq_bytes);

        // Step 5: Build pre-RS buffer (3 × 239 bytes) (spec §5.2.1.5)
        let total_rs_data = FILE_HEADER_RS_FACTOR * RS_SYSTEM_K; // 3 × 239 = 717
        let mut pre_rs = vec![0u8; total_rs_data];

        // Build the inner block
        let mut block = Vec::with_capacity(64);
        // 0x00: Checking sequence CRC (8 bytes)
        block.extend_from_slice(&check_seq_crc.to_le_bytes());
        // 0x08: Checking sequence val1 (8 bytes)
        block.extend_from_slice(&check_seq_val1.to_le_bytes());
        // 0x10: Compressed data CRC (8 bytes)
        block.extend_from_slice(&compr_crc.to_le_bytes());
        // 0x18: Compressed data size (4 bytes) + Length2 (4 bytes)
        block.extend_from_slice(&compr_len.to_le_bytes());
        block.extend_from_slice(&0i32.to_le_bytes()); // Length2 = 0 per spec/reference
                                                      // 0x20: Compressed (or raw) data
        block.extend_from_slice(compr_data);

        // Pad block to multiple of 8 using random padding (spec §5.11)
        let padded_block_size = (block.len() + 7) & !7;
        if block.len() < padded_block_size {
            let pad_needed = padded_block_size - block.len();
            let mut pad_buf = vec![0u8; pad_needed];
            rng.fill_random(&mut pad_buf);
            block.extend_from_slice(&pad_buf);
        }

        // Repeat block as many times as possible within the buffer (spec §5.2.1.5)
        let max_copies = total_rs_data / block.len();
        let mut offset = 0usize;
        for _ in 0..max_copies {
            pre_rs[offset..offset + block.len()].copy_from_slice(&block);
            offset += block.len();
        }
        // Fill remaining bytes with random padding (spec §5.2.1.5)
        if offset < total_rs_data {
            rng.fill_random(&mut pre_rs[offset..total_rs_data]);
        }

        // Step 6: RS-encode with RS(255, 239) factor=3 (spec §5.2.1.6)
        let rs_encoded_size = FILE_HEADER_RS_FACTOR * RS_N; // 3 × 255 = 765
        let mut rs_encoded = vec![0u8; rs_encoded_size];
        reed_solomon_encode(
            &pre_rs,
            &mut rs_encoded,
            FILE_HEADER_RS_FACTOR,
            RS_SYSTEM_K,
            RS_SYSTEM_PRIM_POLY,
        );

        // Build the full 0x400-byte page
        let mut page = vec![0u8; FILE_HEADER_PAGE_SIZE];
        // Copy RS-encoded data
        let copy_len = rs_encoded_size.min(RS_DATA_IN_HEADER);
        page[..copy_len].copy_from_slice(&rs_encoded[..copy_len]);

        // Fill remaining space (between RS data and check data) with random padding
        if copy_len < RS_DATA_IN_HEADER {
            rng.fill_random(&mut page[copy_len..RS_DATA_IN_HEADER]);
        }

        // Step 7: Overwrite last 0x28 bytes with check data (spec §5.2.1.7)
        page[RS_DATA_IN_HEADER..FILE_HEADER_PAGE_SIZE].copy_from_slice(check_data);

        Ok(page)
    }

    // ── Metadata writing ──

    /// Write the 0x80-byte metadata block at offset 0x00 (spec §5.2).
    fn write_metadata<W: Write + Seek>(
        &self,
        output: &mut W,
        _metadata: &Dwg21CompressedMetadata,
    ) -> Result<(), DxfError> {
        output.seek(SeekFrom::Start(0))?;

        let mut buf = [0u8; METADATA_BLOCK_SIZE];
        {
            let mut cursor = Cursor::new(&mut buf[..]);

            // 0x00: "AC1021" version string (6 bytes)
            cursor.write_all(self.version.as_str().as_bytes())?;

            // 0x06: 5 bytes of 0x00
            cursor.write_all(&[0u8; 5])?;

            // 0x0B: Maintenance release version — the source author's byte
            // on a same-version roundtrip (§19 H7f), else the historical
            // 0x19 (AutoCAD 2007).
            cursor.write_all(&[self
                .source_header_bytes
                .map_or(0x19u8, |b| b.0)])?;

            // 0x0C: Byte 0x03 (constant, matches AC18 and AutoCAD reference files)
            cursor.write_all(&[0x03u8])?;

            // 0x0D: Preview address (4 bytes)
            // Find Preview section page offset
            let preview_addr = self.find_section_page_offset(names::PREVIEW);
            cursor.write_u32::<LittleEndian>(preview_addr)?;

            // 0x11: DWG version byte (writer application version) — the
            // source author's byte on a same-version roundtrip.
            cursor.write_all(&[self.source_header_bytes.map_or(0x1Bu8, |b| b.1)])?;

            // 0x12: Maintenance release version (app) — the source
            // author's byte on a same-version roundtrip.
            cursor.write_all(&[self.source_header_bytes.map_or(0x19u8, |b| b.2)])?;

            // 0x13: Codepage (2 bytes) — the source file's on a
            // same-version roundtrip, else ANSI_1252.
            cursor
                .write_u16::<LittleEndian>(self.source_header_bytes.map_or(30u16, |b| b.3))?; // ANSI_1252

            // 0x15: the unknown byte + 0x16/0x17: the app version pair —
            // the source author's bytes on a same-version roundtrip.
            cursor.write_all(&[self
                .source_header_bytes
                .map_or(0u8, |b| b.4)])?;
            cursor.write_all(&[self.source_header_bytes.map_or(0u8, |b| b.5)])?;
            cursor.write_all(&[self.source_header_bytes.map_or(0u8, |b| b.6)])?;

            // 0x18: SecurityType (4 bytes)
            cursor.write_i32::<LittleEndian>(0)?;

            // 0x1C: Unknown long (4 bytes)
            cursor.write_i32::<LittleEndian>(0)?;

            // 0x20: Summary info address (4 bytes)
            let summary_addr = self.find_section_page_offset(names::SUMMARY_INFO);
            cursor.write_u32::<LittleEndian>(summary_addr)?;

            // 0x24: VBA Project address (4 bytes, 0 if not present)
            let vba_addr = self.find_section_page_offset(names::VBA_PROJECT);
            cursor.write_u32::<LittleEndian>(vba_addr)?;

            // 0x28: 0x00000080 (4 bytes)
            cursor.write_u32::<LittleEndian>(0x80)?;

            // 0x2C: App info address (4 bytes)
            let app_info_addr = self.find_section_page_offset(names::APP_INFO);
            cursor.write_u32::<LittleEndian>(app_info_addr)?;

            // 0x30..0x80: remaining zeros (already zeroed)
        }

        output.write_all(&buf)?;

        Ok(())
    }

    /// Find the file offset of the first page of a named section.
    fn find_section_page_offset(&self, name: &str) -> u32 {
        for section in &self.sections {
            if section.name == name {
                if let Some(first_page) = section.pages.first() {
                    // Look up the page record to get its file offset
                    for record in &self.page_records {
                        if record.id == first_page.page_id {
                            return record.offset as u32;
                        }
                    }
                }
            }
        }
        0
    }
}

/// Result from writing a system page, containing metadata for the file header.
#[derive(Debug, Clone)]
struct SystemPageResult {
    /// Page ID assigned to this system page.
    page_id: i64,
    /// Absolute file offset where the page was written.
    #[allow(dead_code)]
    offset: u64,
    /// Compressed size of the data.
    compressed_size: u64,
    /// Uncompressed size of the data.
    uncompressed_size: u64,
    /// CRC-64 of compressed data.
    crc_compressed: u64,
    /// CRC-64 of uncompressed data.
    crc_uncompressed: u64,
    /// RS correction factor (number of interleaved sub-streams).
    correction_factor: u64,
}

// ════════════════════════════════════════════════════════════════════════════
//  AC21 container-shape mirror (§19 H8b)
//
//  The H7g doctrine transferred to the AC1021 (R2007) container: a
//  same-version roundtrip re-emits the author's page space — her
//  pages-map (size, id) pairs in her physical order reproduce her
//  tiling exactly (a running sum from 0x480), closing the
//  pages-map family (pages_amount/pages_maxid, the four map-id
//  fields, both FILEHEADER 0x80-block addresses, the whole
//  R2007_Header pages-map family and the THUMBNAILIMAGE identity)
//  whenever our re-encoded content demonstrably fits her slots:
//  every her data page takes our chunk (LZ77 at her boundaries,
//  RS form zero-padded to her exact on-disk size — gold's decode_rs
//  walks only block_count×251 payload bytes and ignores the per-tab
//  pad, decode_r2007.c:553), the two pages-map system pages carry
//  her map bytes (byte-identical by construction; the encode with
//  her slot as the target lands her correction factor), and the
//  sections-map system pages hold OUR honest table bytes encoded
//  into her tail slots. Any divergence declines to the
//  conventional layout — the rewrite stays valid everywhere and
//  the residue rows stay open on the declined files.
// ════════════════════════════════════════════════════════════════════════════

/// The pre-built AC21 section buffers for the §19 H8b mirror gate.
/// `None` on an `Option` field = our writer skips the section (the
/// H7 verbatim/skip presence rules); the preview is the one lazy
/// build — the retained raw container, or a rebuild around the
/// author's landing address.
pub struct Ac21MirrorBuffers<'a> {
    pub summary: Option<&'a [u8]>,
    pub preview: &'a [u8],
    pub app_info: Option<&'a [u8]>,
    pub app_info_history: Option<&'a [u8]>,
    pub file_dep: Option<&'a [u8]>,
    pub rev_history: &'a [u8],
    pub objects: &'a [u8],
    pub acds: Option<&'a [u8]>,
    pub obj_free_space: Option<&'a [u8]>,
    /// The author's `AcDb:XrefManifest` raw bytes (§19 H7g: authored
    /// state, unmodeled, never printed by gold — the mirror reps
    /// her page verbatim; the conventional path keeps skipping it).
    pub xref_manifest: Option<&'a [u8]>,
    pub template: &'a [u8],
    pub handles: &'a [u8],
    pub classes: &'a [u8],
    pub aux_header: &'a [u8],
    pub header: &'a [u8],
}

impl Ac21MirrorBuffers<'_> {
    /// The buffer for a canonical section name; `None` = our writer
    /// skips the section.
    fn get(&self, name: &str) -> Option<&[u8]> {
        Some(match name {
            names::SUMMARY_INFO => self.summary?,
            names::PREVIEW => self.preview,
            names::APP_INFO => self.app_info?,
            names::APP_INFO_HISTORY => self.app_info_history?,
            names::FILE_DEP_LIST => self.file_dep?,
            names::REV_HISTORY => self.rev_history,
            names::ACDB_OBJECTS => self.objects,
            names::ACDS_PROTOTYPE => self.acds?,
            names::OBJ_FREE_SPACE => self.obj_free_space?,
            names::XREF_MANIFEST => self.xref_manifest?,
            names::TEMPLATE => self.template,
            names::HANDLES => self.handles,
            names::CLASSES => self.classes,
            names::AUX_HEADER => self.aux_header,
            names::HEADER => self.header,
            _ => return None,
        })
    }
}

/// One prepared on-disk slot payload in the author's map order
/// (`bytes.len()` IS the author's on-disk page size).
pub(crate) struct Ac21MirrorPage {
    pub id: i64,
    pub slot: u64,
    pub(crate) bytes: Vec<u8>,
}

/// The §19 H8b mirror emission plan: every page of the author's map
/// walk as an exact-slot payload, plus our honest section-table
/// records and the two system-page encode results the metadata
/// assembly lands.
pub(crate) struct Ac21MirrorPlan {
    pages: Vec<Ac21MirrorPage>,
    sections: Vec<AC21SectionInfo>,
    pages_map_result: SystemPageResult,
    pages_map2_result: SystemPageResult,
    sections_map_result: SystemPageResult,
    sections_map2_result: SystemPageResult,
    pages_map_abs: u64,
    pages_map2_abs: u64,
    max_id: i64,
}

/// gold's `rs_form` — the RS form size of a compressed stream
/// (decode_r2007.c:692 `page_size_if_rs_coded`): block_count =
/// ceil(align8(len)/251) interleaved RS(255, 251) codewords aligned
/// up to 32.
fn page_size_if_rs_coded(len: usize) -> usize {
    let pesize = (len + 7) & !7;
    let block_count = (pesize + 251 - 1) / 251;
    (block_count * 255 + 31) & !31
}

/// The §19 H8b mirrored data-page encode: our chunk in her on-disk
/// slot. Payload, checksum and CRC conventions match the
/// conventional `encode_data_page` exactly — only the padding target
/// differs (the author's slot replaces the align32 forms) plus the
/// RS-form collision rule for stored chunks (gold's dispatch,
/// decode_r2007.c:856: a stored page is raw-memcpy'd only when the
/// page size != rs_form of the declared comp; on collision the RS
/// form of the chunk is emitted and gold de-interleaves it).
fn encode_data_page_mirrored(
    chunk: &[u8],
    encoding: u64,
    skip_lz77: bool,
    slot: u64,
) -> Result<EncodedDataPage, &'static str> {
    let slot = usize::try_from(slot).map_err(|_| "slot overflows usize")?;
    if slot == 0 {
        return Err("zero on-disk slot");
    }
    let checksum = dwg_ac21_page_checksum(0, chunk) as u64;
    let rs_form = page_size_if_rs_coded(chunk.len());

    // Pad helper: zero-pad an encoded payload out to the slot; the
    // per-slot pad is inert (gold's decode_rs consumes only
    // block_count×k codeword bytes; the stored path memcpy's uncomp
    // bytes from the slot start).
    fn pad_to(mut bytes: Vec<u8>, slot: usize, why: &'static str) -> Result<Vec<u8>, &'static str> {
        if bytes.len() > slot {
            return Err(why);
        }
        bytes.resize(slot, 0);
        Ok(bytes)
    }

    if encoding == 1 {
        // Stored section (our registry's encoding-1 pages: raw content).
        let crc = dwg_ac21_mirrored_crc64(0, chunk.len() as u32, chunk);
        if rs_form == slot {
            // The stored form collides with the RS form's size — emit
            // the RS form so gold's de-interleave reads our payload.
            let bytes = pad_to(
                rs_encode_data_page_interleaved(chunk, encoding),
                slot,
                "stored RS form exceeds the slot",
            )?;
            return Ok(EncodedDataPage {
                bytes,
                uncompressed_size: chunk.len() as u64,
                compressed_size: chunk.len() as u64,
                checksum,
                crc,
            });
        }
        let bytes = pad_to(chunk.to_vec(), slot, "stored chunk exceeds the slot")?;
        return Ok(EncodedDataPage {
            bytes,
            uncompressed_size: chunk.len() as u64,
            compressed_size: chunk.len() as u64,
            checksum,
            crc,
        });
    }

    let compressed = compress_ac21(chunk);
    if !skip_lz77 && compressed.len() < chunk.len() {
        // Compressed wins: the RS form of our stream, zero-padded to
        // her slot. gold takes read_data_page unconditionally
        // (comp != uncomp), allocating from OUR declared data_size.
        let crc = dwg_ac21_mirrored_crc64(0, compressed.len() as u32, &compressed);
        let bytes = pad_to(
            rs_encode_data_page_interleaved(&compressed, encoding),
            slot,
            "RS form of our compressed chunk exceeds her page (objects-overflow class)",
        )?;
        return Ok(EncodedDataPage {
            bytes,
            uncompressed_size: chunk.len() as u64,
            compressed_size: compressed.len() as u64,
            checksum,
            crc,
        });
    }

    // Compression lost (or skip_lz77): stored bytes under an
    // encoding-4 section. Same collision rule as encoding-1; the
    // declared comp == uncomp lets gold pick its path by her slot.
    let crc = dwg_ac21_mirrored_crc64(0, chunk.len() as u32, chunk);
    if rs_form == slot {
        let bytes = pad_to(
            rs_encode_data_page_interleaved(chunk, encoding),
            slot,
            "stored RS form exceeds the slot",
        )?;
        return Ok(EncodedDataPage {
            bytes,
            uncompressed_size: chunk.len() as u64,
            compressed_size: chunk.len() as u64,
            checksum,
            crc,
        });
    }
    let bytes = pad_to(chunk.to_vec(), slot, "stored chunk exceeds the slot")?;
    Ok(EncodedDataPage {
        bytes,
        uncompressed_size: chunk.len() as u64,
        compressed_size: chunk.len() as u64,
        checksum,
        crc,
    })
}

/// Build the author's pages-map bytes from her retained map walk —
/// her (size, id) pairs in her physical order, byte-identical by
/// construction (no terminator pair; gold's reader walks to the
/// map's uncompressed end).
fn build_author_pages_map_bytes(shape: &DwgAc21ContainerShape) -> Vec<u8> {
    let mut stream = Vec::with_capacity(shape.map_order.len() * 16);
    for entry in &shape.map_order {
        stream.extend_from_slice(&entry.on_disk_size.to_le_bytes());
        stream.extend_from_slice(&entry.id.to_le_bytes());
    }
    stream
}

/// The author's on-disk address of a page id per her map walk
/// (RESERVED + the running sum of the prior slot sizes) — the
/// caller's preview build uses the author's landing address.
pub fn ac21_author_page_address(shape: &DwgAc21ContainerShape, id: i64) -> Option<u64> {
    let mut offset = RESERVED_HEADER_SIZE as u64;
    for entry in &shape.map_order {
        if entry.id == id {
            return Some(offset);
        }
        offset += u64::try_from(entry.on_disk_size).ok()?;
    }
    None
}

impl DwgFileHeaderWriterAC21 {
    /// The §19 H8b content-parity gate + emission preparation. Every
    /// check is a fall-back-to-conventional trigger: the author's page
    /// space is mirrored only when our re-encoded content demonstrably
    /// fits it and her layout is one our writer can reproduce
    /// byte-exactly. Set `AC21_MIRROR_DEBUG` to trace the decline
    /// reason per file. Returns the prepared plan (every page payload
    /// at her exact slot size) or `None` (decline → conventional).
    pub(crate) fn mirror_plan(
        &self,
        shape: &DwgAc21ContainerShape,
        sys: &DwgR2007SystemHeader,
        buffers: &Ac21MirrorBuffers<'_>,
    ) -> Option<Ac21MirrorPlan> {
        use std::collections::{HashMap, HashSet};

        macro_rules! decline {
            ($why:expr) => {{
                if std::env::var_os("AC21_MIRROR_DEBUG").is_some() {
                    eprintln!("[ac21-mirror] declined: {}", $why);
                }
                return None;
            }};
        }

        // ── The system-page ids must be coherent, positive and pairwise distinct.
        let pm_id = i64::try_from(sys.pages_map_id).ok()?;
        let pm2_id = i64::try_from(sys.pages_map2_id).ok()?;
        let sm_id = i64::try_from(sys.sections_map_id).ok()?;
        let sm2_id = i64::try_from(sys.sections_map2_id).ok()?;
        let mut system_ids = HashSet::new();
        for id in [pm_id, pm2_id, sm_id, sm2_id] {
            if id <= 0 || !system_ids.insert(id) {
                decline!(format!("incoherent system page ids ({pm_id}/{pm2_id}/{sm_id}/{sm2_id})"));
            }
        }

        // ── Her map walk: positive sizes and ids, unique ids, and a
        // halfword count that matches her declared uncomp map size
        // (a terminator pair inside her map bytes is not reproducible
        // from her retained pairs).
        if shape.map_order.is_empty() {
            decline!("empty pages map");
        }
        let mut map_ids = HashMap::<i64, u64>::new();
        let mut slot_by_system_id = HashMap::<i64, u64>::new();
        for entry in &shape.map_order {
            if entry.id <= 0 || entry.on_disk_size <= 0 {
                decline!(format!(
                    "gap/terminator map entry (id {}, size {})",
                    entry.id, entry.on_disk_size
                ));
            }
            let slot = u64::try_from(entry.on_disk_size).ok()?;
            if map_ids.insert(entry.id, slot).is_some() {
                decline!(format!("duplicate map id {}", entry.id));
            }
            if system_ids.contains(&entry.id) {
                slot_by_system_id.insert(entry.id, slot);
            }
        }
        if sys.pages_map_size_uncomp as usize != shape.map_order.len() * 16 {
            decline!(format!(
                "her pages-map bytes {} != 16 × {} entries (a terminator pair is not reproducible)",
                sys.pages_map_size_uncomp,
                shape.map_order.len()
            ));
        }
        for id in [pm_id, pm2_id, sm_id, sm2_id] {
            if !slot_by_system_id.contains_key(&id) {
                decline!(format!("map walk lacks system page id {id}"));
            }
        }

        // ── Her sections table: unique named sections, ascending
        // per-page offsets, and ownership of every non-system map id.
        let mut named = HashMap::<&str, usize>::new();
        for (index, section) in shape.sections.iter().enumerate() {
            if section.name.is_empty() {
                // The unnamed 0-page tail descriptor — gold skips it
                // ("Invalid num_pages 0, skip"); our honest table
                // omits it.
                if !section.pages.is_empty() {
                    decline!("unnamed section carries pages");
                }
                continue;
            }
            if named.insert(section.name.as_str(), index).is_some() {
                decline!(format!("duplicate section name {}", section.name));
            }
            for window in section.pages.windows(2) {
                if window[0].offset >= window[1].offset {
                    decline!(format!("section {} page offsets not ascending", section.name));
                }
            }
        }
        let mut page_owner: HashMap<i64, (usize, usize)> = HashMap::new();
        let mut data_entry_count: usize = 0;
        for entry in &shape.map_order {
            if !system_ids.contains(&entry.id) {
                data_entry_count += 1;
            }
        }
        for (sec_index, section) in shape.sections.iter().enumerate() {
            for (page_index, page) in section.pages.iter().enumerate() {
                if page.id <= 0 {
                    decline!(format!("section {} carries non-positive page id", section.name));
                }
                if page_owner.insert(page.id, (sec_index, page_index)).is_some() {
                    decline!(format!("duplicate page id {}", page.id));
                }
                match map_ids.get(&page.id) {
                    Some(slot) if *slot == u64::try_from(page.on_disk_size).ok()? => {}
                    _ => decline!(format!(
                        "section {} page id {} disagrees with her map extent",
                        section.name, page.id
                    )),
                }
            }
        }
        if page_owner.len() != data_entry_count {
            decline!(format!(
                "map data entries {data_entry_count} vs owned section pages {}",
                page_owner.len()
            ));
        }
        if sys.pages_amount as usize != shape.map_order.len() {
            decline!(format!(
                "pages_amount {} vs map entries {}",
                sys.pages_amount,
                shape.map_order.len()
            ));
        }

        // ── Per-section content parity: our re-encoded buffer must
        // cover her pages; every chunk (our bytes between her page
        // boundaries) must encode into her on-disk slot.
        let mut payloads: HashMap<i64, Vec<u8>> = HashMap::new();
        let mut section_records: Vec<AC21SectionInfo> = Vec::new();
        let mut used_names: HashSet<&str> = HashSet::new();
        for section in &shape.sections {
            if section.name.is_empty() {
                continue;
            }
            let data: &[u8] = match buffers.get(&section.name) {
                Some(data) => data,
                None => {
                    // Our writer skips this section: only a 0-page
                    // descriptor can mirror then (the H7g doctrine).
                    if !section.pages.is_empty() {
                        decline!(format!(
                            "our writer skips {} but the author has {} pages",
                            section.name,
                            section.pages.len()
                        ));
                    }
                    continue;
                }
            };
            // The encoding declaration: OUR registry's value for the
            // sections our writer builds (the conventional convention
            // — the sections-map family stays OURS); for an
            // unregistered section (§19 H7g `AcDb:XrefManifest`
            // under AC21 — authored state the mirror re-emits
            // verbatim) the author's own table encoding is the
            // coherent declaration.
            let encoding = match ac21_section_info::encoding(&section.name) {
                Some(encoding) => encoding,
                None => section.encoding,
            };
            let hash_code = ac21_section_info::hash_code(&section.name).unwrap_or(0);
            used_names.insert(section.name.as_str());
            if section.pages.is_empty() {
                // A 0-page named section: gold skips it on read; our
                // honest table omits it — but only when we do NOT hold
                // real content (our buffer above is just presence).
                continue;
            }

            // Our max-page-size convention (the `add_section`
            // formula — the sections-map family stays OURS).
            let mut max_page_size = ac21_section_info::page_size(&section.name).unwrap_or(0xF800);
            if section.name == names::PREVIEW {
                max_page_size = max_page_size.max(align32(data.len()) as u64);
            }
            if section.name == names::APP_INFO_HISTORY {
                max_page_size = (data.len() as u64 + 0x7F) & !0x7F;
            }

            let mut pages = Vec::with_capacity(section.pages.len());
            for (page_index, page) in section.pages.iter().enumerate() {
                if page.size < 0 || page.id <= 0 {
                    decline!(format!(
                        "section {} page {} carries a negative plan value",
                        section.name, page.id
                    ));
                }
                let start = usize::try_from(page.offset).ok()?;
                if start > data.len() {
                    decline!(format!(
                        "section {} content len {} does not reach page {}'s offset {}",
                        section.name,
                        data.len(),
                        page.id,
                        page.offset
                    ));
                }
                let end = if page_index + 1 < section.pages.len() {
                    let next = usize::try_from(section.pages[page_index + 1].offset).ok()?;
                    if next > data.len() {
                        decline!(format!(
                            "section {} page {}'s next boundary {} exceeds our len {}",
                            section.name, page.id, next, data.len()
                        ));
                    }
                    next
                } else {
                    data.len()
                };
                if start >= end {
                    decline!(format!("section {} page {} has an empty chunk", section.name, page.id));
                }
                let chunk = &data[start..end];
                let slot = u64::try_from(page.on_disk_size).ok()?;
                let encoded = match encode_data_page_mirrored(
                    chunk,
                    encoding,
                    self.skip_lz77,
                    slot,
                ) {
                    Ok(encoded) => encoded,
                    Err(why) => {
                        decline!(format!("section {} page {}: {}", section.name, page.id, why));
                    }
                };
                payloads.insert(page.id, encoded.bytes);
                pages.push(AC21SectionPageRecord {
                    data_offset: page.offset,
                    // gold parses this field but never uses it on the
                    // decode path (read_data_section walks comp/uncomp
                    // only) — the author's frames are retained verbatim.
                    page_size: page.size as u64,
                    page_id: page.id,
                    uncompressed_size: encoded.uncompressed_size,
                    compressed_size: encoded.compressed_size,
                    checksum: encoded.checksum,
                    crc: encoded.crc,
                });
            }
            section_records.push(AC21SectionInfo {
                name: section.name.clone(),
                hash_code,
                encoding,
                encryption: 0,
                max_page_size,
                data_size: data.len() as u64,
                pages,
            });
        }
        // ── The unused-buffer rule: every buffer our writer built
        // must land in her page space, except the FileDepList
        // boilerplate stub (no document state — her table's absence
        // wins, like gold's own zeroed print on both sides).
        let all_names: [(&str, bool); 15] = [
            (names::SUMMARY_INFO, buffers.summary.is_some()),
            (names::PREVIEW, true),
            (names::APP_INFO, buffers.app_info.is_some()),
            (names::APP_INFO_HISTORY, buffers.app_info_history.is_some()),
            (names::FILE_DEP_LIST, buffers.file_dep.is_some()),
            (names::REV_HISTORY, true),
            (names::ACDB_OBJECTS, true),
            (names::ACDS_PROTOTYPE, buffers.acds.is_some()),
            (names::OBJ_FREE_SPACE, buffers.obj_free_space.is_some()),
            (names::XREF_MANIFEST, buffers.xref_manifest.is_some()),
            (names::TEMPLATE, true),
            (names::HANDLES, true),
            (names::CLASSES, true),
            (names::AUX_HEADER, true),
            (names::HEADER, true),
        ];
        for (name, present) in all_names {
            if present && !used_names.contains(name) && name != names::FILE_DEP_LIST {
                decline!(format!(
                    "our writer would drop section {name} (not in her page space)"
                ));
            }
        }

        // ── The two system-page encodes, into HER on-disk slots.
        // The pages-map bytes are hers (the pairs above) — with her
        // slot as the encode target the correction factor lands hers
        // for free. The sections-map bytes are OURS (the honest
        // table serialized in our spec order).
        let map_bytes = build_author_pages_map_bytes(shape);
        let map_slot = slot_by_system_id[&pm_id];
        let (pm_bytes, pm_comp, pm_uncomp, pm_crc_comp, pm_crc_uncomp, pm_correction) =
            self.encode_system_page(&map_bytes, map_slot as usize);
        let pm_bytes = if pm_bytes.len() > map_slot as usize {
            decline!(format!(
                "pages-map RS form {} exceeds her slot {}",
                pm_bytes.len(),
                map_slot
            ));
        } else {
            let mut bytes = pm_bytes;
            bytes.resize(map_slot as usize, 0);
            bytes
        };

        let sections_slot = slot_by_system_id[&sm_id];
        let section_map_bytes = serialize_section_map(&section_records).ok()?;
        let (sm_bytes, sm_comp, sm_uncomp, sm_crc_comp, sm_crc_uncomp, sm_correction) =
            self.encode_system_page(&section_map_bytes, sections_slot as usize);
        let sm_bytes = if sm_bytes.len() > sections_slot as usize {
            decline!(format!(
                "sections-map RS form {} exceeds her slot {}",
                sm_bytes.len(),
                sections_slot
            ));
        } else {
            let mut bytes = sm_bytes;
            bytes.resize(sections_slot as usize, 0);
            bytes
        };

        // ── Assemble the walk: her map order, every entry resolved
        // to its prepared payload, her ids and slot sizes verbatim.
        let mut pages = Vec::with_capacity(shape.map_order.len());
        let mut pages_map_abs = RESERVED_HEADER_SIZE as u64;
        let mut pages_map2_abs = 0u64;
        let mut max_id: i64 = 0;
        for entry in &shape.map_order {
            max_id = max_id.max(entry.id);
            let bytes = if entry.id == pm_id || entry.id == pm2_id {
                pm_bytes.clone()
            } else if entry.id == sm_id || entry.id == sm2_id {
                sm_bytes.clone()
            } else {
                match payloads.remove(&entry.id) {
                    Some(bytes) => bytes,
                    None => decline!(format!("map id {} owns no payload", entry.id)),
                }
            };
            if bytes.len() as u64 != map_ids[&entry.id] {
                decline!(format!(
                    "page {} payload {} != her slot {}",
                    entry.id,
                    bytes.len(),
                    map_ids[&entry.id]
                ));
            }
            if entry.id == pm_id {
                pages_map_abs = RESERVED_HEADER_SIZE as u64
                    + map_order_prefix(shape, entry.id).unwrap_or(0);
            }
            if entry.id == pm2_id {
                pages_map2_abs = RESERVED_HEADER_SIZE as u64
                    + map_order_prefix(shape, entry.id).unwrap_or(0);
            }
            pages.push(Ac21MirrorPage {
                id: entry.id,
                slot: map_ids[&entry.id],
                bytes,
            });
        }
        let pages_map_result = SystemPageResult {
            page_id: pm_id,
            offset: pages_map_abs,
            compressed_size: pm_comp,
            uncompressed_size: pm_uncomp,
            crc_compressed: pm_crc_comp,
            crc_uncompressed: pm_crc_uncomp,
            correction_factor: pm_correction,
        };
        let pages_map2_result = SystemPageResult {
            page_id: pm2_id,
            ..pages_map_result.clone()
        };
        let sm_abs = RESERVED_HEADER_SIZE as u64 + map_order_prefix(shape, sm_id).unwrap_or(0);
        let sections_map_result = SystemPageResult {
            page_id: sm_id,
            offset: sm_abs,
            compressed_size: sm_comp,
            uncompressed_size: sm_uncomp,
            crc_compressed: sm_crc_comp,
            crc_uncompressed: sm_crc_uncomp,
            correction_factor: sm_correction,
        };
        let sections_map2_result = SystemPageResult {
            page_id: sm2_id,
            ..sections_map_result.clone()
        };

        if std::env::var_os("AC21_MIRROR_DEBUG").is_some() {
            eprintln!(
                "[ac21-mirror] engaged: {} pages (ids ≤ {max_id}), pm slot {} cf {}, sm slot {} cf {}, sections {}",
                pages.len(),
                map_slot,
                pm_correction,
                sections_slot,
                sm_correction,
                section_records.len()
            );
        }
        Some(Ac21MirrorPlan {
            pages,
            sections: section_records,
            pages_map_result,
            pages_map2_result,
            sections_map_result,
            sections_map2_result,
            pages_map_abs,
            pages_map2_abs,
            max_id,
        })
    }

    /// Emit the §19 H8b mirrored page space: the prepared payloads
    /// written sequentially in her map order (the running sum
    /// reproduces her tiling), our honest section records and her
    /// page ids/sizes recorded for the finalize pass.
    pub(crate) fn write_mirrored_pages<W: Write + Seek>(
        &mut self,
        output: &mut W,
        plan: &Ac21MirrorPlan,
    ) -> Result<(), DxfError> {
        let mut offset = output.seek(SeekFrom::Current(0))?;
        for page in &plan.pages {
            output.write_all(&page.bytes)?;
            self.page_records.push(AC21PageRecord {
                id: page.id,
                size: page.slot as i64,
                offset,
            });
            offset += page.slot;
        }
        self.sections = plan.sections.clone();
        self.next_page_id = plan.max_id + 1;
        self.mirror_state = Some(MirrorState {
            pages_map_result: plan.pages_map_result.clone(),
            pages_map2_result: plan.pages_map2_result.clone(),
            pages_map_abs: plan.pages_map_abs,
            pages_map2_abs: plan.pages_map2_abs,
            sections_map_result: plan.sections_map_result.clone(),
            sections_map2_result: plan.sections_map2_result.clone(),
        });
        Ok(())
    }

    /// The §19 H8b finalize: the section maps were already emitted
    /// in her page space (the conventional `write_file` writes them
    /// here); the metadata assembly reflects the mirrored tiling —
    /// the pages-map family lands her values naturally (the fields
    /// are our own encode results over her bytes at her offsets and
    /// ids), the sections-map family stays OURS (honest), and the
    /// shared two-pass tail writes the 0x400 file header page, the
    /// header2 copy and the 0x80 block unchanged.
    pub(crate) fn write_file_mirrored<W: Write + Seek>(
        &mut self,
        output: &mut W,
    ) -> Result<(), DxfError> {
        let mirror = self
            .mirror_state
            .take()
            .ok_or_else(|| DxfError::InvalidFormat("AC21 mirror finalize without a mirror emission".into()))?;
        self.crc_seed = 0;

        let mut metadata = Dwg21CompressedMetadata::default();
        metadata.file_size = 0; // Patched after writing header copy

        // Pages map fields — her offsets/ids, our encode results over
        // her map bytes.
        metadata.pages_map_crc_compressed = mirror.pages_map_result.crc_compressed;
        metadata.pages_map_correction_factor = mirror.pages_map_result.correction_factor;
        metadata.pages_map_offset = mirror.pages_map_abs - RESERVED_HEADER_SIZE as u64;
        metadata.pages_map_id = mirror.pages_map_result.page_id as u64;
        metadata.map2_offset = mirror.pages_map2_abs - RESERVED_HEADER_SIZE as u64;
        metadata.map2_id = mirror.pages_map2_result.page_id as u64;
        metadata.pages_map_size_compressed = mirror.pages_map_result.compressed_size;
        metadata.pages_map_size_uncompressed = mirror.pages_map_result.uncompressed_size;
        // Every page of her walk is recorded (the two pages-map boxes
        // included — the conventional flow counts its unrecorded pair).
        metadata.pages_amount = self.page_records.len() as u64;
        metadata.pages_max_id = (self.next_page_id - 1) as u64;
        metadata.pages_map_crc_uncompressed = mirror.pages_map_result.crc_uncompressed;

        // Sections map fields — OURS (honest).
        metadata.sections_amount = (self.sections.len() + 1) as u64;
        metadata.sections_map_crc_uncompressed = mirror.sections_map_result.crc_uncompressed;
        metadata.sections_map_size_compressed = mirror.sections_map_result.compressed_size;
        metadata.sections_map2_id = mirror.sections_map2_result.page_id as u64;
        metadata.sections_map_id = mirror.sections_map_result.page_id as u64;
        metadata.sections_map_size_uncompressed = mirror.sections_map_result.uncompressed_size;
        metadata.sections_map_crc_compressed = mirror.sections_map_result.crc_compressed;
        metadata.sections_map_correction_factor =
            mirror.sections_map_result.correction_factor;

        // CRC/random fields (spec §5.2.1.1 — order is critical for RNG state)
        // §5.2.1.1.1: RandomSeed IS the CRC encoder's seed (input, not output).
        // §19 H7g: a same-version roundtrip adopts the AUTHOR's seed.
        let random_seed = self.source_random_seed.unwrap_or(FILE_RANDOM_SEED);
        metadata.random_seed = random_seed;
        metadata.crc_seed = self.crc_seed; // always 0 per §5.2.1.1.2
        self.write_two_pass_header(output, &mut metadata, random_seed)?;

        Ok(())
    }

    /// §19 H8e-2: the compressed-page echo — the author's on-disk
    /// file re-emitted verbatim: the retained whole-file blob (her
    /// 0x80 metadata block with its unknown-region bytes, the 0x400
    /// file-header page with her check data and MT-derive draws,
    /// every page of her map walk — data, system, gaps alike — at
    /// her exact slot sizes with her RS coding, and her trailing
    /// header2 copy) is written as one blob from offset 0. The
    /// writer's own derivations stay live as the debug oracle (the
    /// fhw state populated below drives the derived-vs-retained
    /// address assertion at the call site). The combined identity
    /// gate at the call site guards the echo; the mirrored and
    /// conventional paths stay for every other case.
    pub(crate) fn write_full_echo<W: Write + Seek>(
        &mut self,
        output: &mut W,
        shape: &DwgAc21ContainerShape,
        tail: &[u8],
    ) -> Result<(), DxfError> {
        // Her walk → the page records (running offsets from 0x480)
        // and her named sections (the 0x80-block address derivation
        // the debug oracle asserts). The section page records carry
        // her declared frame fields; the encode-derived fields
        // (comp/uncomp/checksum/crc) stay 0 — nothing consumes them
        // on this path (the sections map is her bytes, in the tail).
        let mut offset = RESERVED_HEADER_SIZE as u64;
        for entry in &shape.map_order {
            let slot = u64::try_from(entry.on_disk_size).map_err(|_| {
                DxfError::InvalidFormat("negative slot in her map walk".into())
            })?;
            self.page_records.push(AC21PageRecord {
                id: entry.id,
                size: entry.on_disk_size,
                offset,
            });
            offset = offset
                .checked_add(slot)
                .ok_or_else(|| DxfError::InvalidFormat("her map walk overflows".into()))?;
        }
        self.sections = shape
            .sections
            .iter()
            .filter(|section| !section.name.is_empty() && !section.pages.is_empty())
            .map(|section| AC21SectionInfo {
                name: section.name.clone(),
                hash_code: 0,
                encoding: section.encoding,
                encryption: 0,
                max_page_size: 0,
                data_size: section.data_size,
                pages: section
                    .pages
                    .iter()
                    .map(|page| AC21SectionPageRecord {
                        data_offset: page.offset,
                        page_size: page.size as u64,
                        page_id: page.id,
                        uncompressed_size: 0,
                        compressed_size: 0,
                        checksum: 0,
                        crc: 0,
                    })
                    .collect(),
            })
            .collect();
        self.next_page_id = shape.map_order.iter().map(|entry| entry.id).max().unwrap_or(0) + 1;

        // The blob must cover her header blocks plus the whole walk
        // (the header2 copy rides after it).
        let minimum = offset + FILE_HEADER_PAGE_SIZE as u64;
        if (tail.len() as u64) < minimum {
            return Err(DxfError::InvalidFormat(format!(
                "her file blob {} does not cover her walk {}",
                tail.len(),
                minimum
            )));
        }

        // Her file, verbatim, from offset 0.
        output.seek(SeekFrom::Start(0))?;
        output.write_all(tail)?;
        output.seek(SeekFrom::End(0))?;
        Ok(())
    }

    /// The derived 0x80-block address of a named section's first page
    /// (the §19 H8b debug assertion oracle — the mirrored layout
    /// lands the author's retained addresses naturally).
    pub fn section_page_address(&self, name: &str) -> u32 {
        self.find_section_page_offset(name)
    }
}

/// The running byte size of her map walk before a given page id —
/// the page's position relative to the 0x480 reserve.
fn map_order_prefix(shape: &DwgAc21ContainerShape, id: i64) -> Option<u64> {
    let mut offset = 0u64;
    for entry in &shape.map_order {
        if entry.id == id {
            return Some(offset);
        }
        offset += u64::try_from(entry.on_disk_size).ok()?;
    }
    None
}

/// The mirrored finalize inputs, stored by `write_mirrored_pages`.
#[derive(Debug, Clone)]
struct MirrorState {
    pages_map_result: SystemPageResult,
    pages_map2_result: SystemPageResult,
    pages_map_abs: u64,
    pages_map2_abs: u64,
    sections_map_result: SystemPageResult,
    sections_map2_result: SystemPageResult,
}

// ════════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // ─── §19 H8b mirror encode tests ────────────────────────────────

    /// gold's `page_size_if_rs_coded` (decode_r2007.c:692) pinned on the
    /// example_2007 author's own pages: her on-disk slots are the exact
    /// RS forms of her compressed streams.
    #[test]
    fn test_page_size_if_rs_coded_author_pinned() {
        assert_eq!(page_size_if_rs_coded(19401), 19904); // objects id 9
        assert_eq!(page_size_if_rs_coded(14396), 14816); // objects id 8
        assert_eq!(page_size_if_rs_coded(3630), 3840); // objects id 13
        assert_eq!(page_size_if_rs_coded(3859), 4096); // objects id 11
        assert_eq!(page_size_if_rs_coded(28004), 28576); // objects id 10
        assert_eq!(page_size_if_rs_coded(49863), 50752); // objects id 14
        assert_eq!(page_size_if_rs_coded(2836), 3072); // objects id 12
        assert_eq!(page_size_if_rs_coded(6), 256); // stored template: == rs_form -> RS path
        assert_eq!(page_size_if_rs_coded(302255), 307296); // preview: != her slot -> raw
    }

    /// A stored chunk whose RS form is NOT the slot emits raw + zero pad;
    /// the declared comp == uncomp lets gold memcpy from the slot start.
    #[test]
    fn test_mirrored_encode_stored_raw() {
        let chunk = vec![0xA5u8; 200];
        let page = encode_data_page_mirrored(&chunk, 1, false, 1024).unwrap();
        assert_eq!(page.bytes.len(), 1024);
        assert_eq!(&page.bytes[..200], &chunk);
        assert!(page.bytes[200..].iter().all(|&b| b == 0));
        assert_eq!(page.uncompressed_size, 200);
        assert_eq!(page.compressed_size, 200);
    }

    /// The collision rule: a stored chunk whose RS form IS the slot emits
    /// the RS form (gold's dispatch takes the de-interleave path — a raw
    /// emission there would corrupt the read).
    #[test]
    fn test_mirrored_encode_stored_collision() {
        let chunk = vec![0x5Au8; 6];
        let slot = page_size_if_rs_coded(6) as u64;
        assert_eq!(slot, 256);
        let page = encode_data_page_mirrored(&chunk, 1, false, slot).unwrap();
        assert_eq!(page.bytes.len(), 256);
        // The RS-interleaved form is not the raw prefix (byte-interleaved
        // codewords: data 0..6 spread at stride factor 1 -> all at front
        // here, so check the declared pair instead).
        assert_eq!(page.uncompressed_size, 6);
        assert_eq!(page.compressed_size, 6);
    }

    /// A stored chunk larger than the slot declines (the objects-overflow
    /// class analog).
    #[test]
    fn test_mirrored_encode_stored_overflow_declines() {
        let chunk = vec![0u8; 3000];
        assert!(encode_data_page_mirrored(&chunk, 1, false, 2048).is_err());
    }

    /// A compressible chunk under an encoding-4 section emits the RS form
    /// of our compressed stream, zero-padded to her slot, with our honest
    /// comp/uncomp pair.
    #[test]
    fn test_mirrored_encode_compressed() {
        let chunk = vec![0u8; 4096];
        let page = encode_data_page_mirrored(&chunk, 4, false, 1024).unwrap();
        assert_eq!(page.bytes.len(), 1024);
        assert!(page.compressed_size < page.uncompressed_size);
        assert_eq!(page.uncompressed_size, 4096);
    }

    /// An incompressible chunk under an encoding-4 section falls to the
    /// stored rules with comp == uncomp declared.
    #[test]
    fn test_mirrored_encode_incompressible_stores() {
        // High-entropy bytes: literals give the compressor no matches.
        let mut chunk = Vec::with_capacity(2048);
        let mut x: u32 = 0x12345678;
        for _ in 0..2048 {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            chunk.push((x >> 16) as u8);
        }
        let page = encode_data_page_mirrored(&chunk, 4, false, 4096).unwrap();
        assert_eq!(page.bytes.len(), 4096);
        assert_eq!(page.compressed_size, page.uncompressed_size);
    }

    /// The author's pages-map bytes: her (size, id) pairs in her walk
    /// order, no terminator pair (22 entries x 16 bytes on example_2007).
    #[test]
    fn test_author_pages_map_bytes() {
        let shape = DwgAc21ContainerShape {
            map_order: vec![
                crate::document::DwgAc21PageEntry {
                    id: 25,
                    on_disk_size: 1024,
                },
                crate::document::DwgAc21PageEntry {
                    id: 26,
                    on_disk_size: 1024,
                },
            ],
            sections: Vec::new(),
        };
        let bytes = build_author_pages_map_bytes(&shape);
        assert_eq!(bytes.len(), 32);
        assert_eq!(&bytes[..8], &1024i64.to_le_bytes());
        assert_eq!(&bytes[8..16], &25i64.to_le_bytes());
        assert_eq!(&bytes[16..24], &1024i64.to_le_bytes());
        assert_eq!(&bytes[24..32], &26i64.to_le_bytes());
    }

    /// The author's page address per her map walk (RESERVE + the running
    /// sum of the prior slot sizes).
    #[test]
    fn test_ac21_author_page_address() {
        let shape = DwgAc21ContainerShape {
            map_order: vec![
                crate::document::DwgAc21PageEntry {
                    id: 25,
                    on_disk_size: 1024,
                },
                crate::document::DwgAc21PageEntry {
                    id: 3,
                    on_disk_size: 160,
                },
                crate::document::DwgAc21PageEntry {
                    id: 4,
                    on_disk_size: 307936,
                },
            ],
            sections: Vec::new(),
        };
        assert_eq!(
            ac21_author_page_address(&shape, 25),
            Some(RESERVED_HEADER_SIZE as u64)
        );
        assert_eq!(
            ac21_author_page_address(&shape, 3),
            Some(RESERVED_HEADER_SIZE as u64 + 1024)
        );
        assert_eq!(
            ac21_author_page_address(&shape, 4),
            Some(RESERVED_HEADER_SIZE as u64 + 1024 + 160)
        );
        assert_eq!(ac21_author_page_address(&shape, 99), None);
    }

    // ─── CRC Random Encoder tests ───────────────────────────────────

    #[test]
    fn test_crc_random_encoder_deterministic() {
        let mut rng1 = CrcRandomEncoder::new(42);
        let mut rng2 = CrcRandomEncoder::new(42);

        for _ in 0..50 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_crc_random_encoder_different_seeds() {
        let mut rng1 = CrcRandomEncoder::new(0x12345678_9ABCDEF0);
        let mut rng2 = CrcRandomEncoder::new(0xFEDCBA98_76543210);

        // Different seeds should produce different sequences
        let seq1: Vec<u64> = (0..10).map(|_| rng1.next_u64()).collect();
        let seq2: Vec<u64> = (0..10).map(|_| rng2.next_u64()).collect();
        assert_ne!(seq1, seq2);
    }

    #[test]
    fn test_crc_random_encoder_fill_random() {
        let mut rng = CrcRandomEncoder::new(123);
        let mut buf = [0u8; 37]; // Non-aligned size
        rng.fill_random(&mut buf);

        // Should not be all zeros
        assert!(!buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_crc_random_encoder_encode_seed() {
        let mut rng = CrcRandomEncoder::new(42);
        let seed = 0x12345678_9ABCDEF0u64;
        let encoded = rng.encode_crc_seed(seed);

        // The encoding should be deterministic (same seed + RNG state = same result)
        let mut rng2 = CrcRandomEncoder::new(42);
        let encoded2 = rng2.encode_crc_seed(seed);
        assert_eq!(
            encoded, encoded2,
            "Same RNG state should produce same encoding"
        );
    }

    #[test]
    fn test_crc_random_encoder_encode_advances_state() {
        let mut rng1 = CrcRandomEncoder::new(42);
        let mut rng2 = CrcRandomEncoder::new(42);

        // encode_crc_seed calls next_u64 once (consuming 2 table entries)
        rng1.encode_crc_seed(0);
        rng2.next_u64(); // Same state advancement

        // After one encode, both should be at the same table position
        assert_eq!(rng1.next_u64(), rng2.next_u64());
    }

    // ─── System page size calculation tests ─────────────────────────

    #[test]
    fn test_system_page_size_minimum() {
        // Small data should give minimum page size of 0x400
        assert_eq!(get_system_page_size(8), 0x400);
        assert_eq!(get_system_page_size(100), 0x400);
        assert_eq!(get_system_page_size(200), 0x400);
    }

    #[test]
    fn test_system_page_size_large_data() {
        // Large enough data should produce a larger page size
        let size = get_system_page_size(10000);
        assert!(size >= 10000 * 2 / RS_SYSTEM_K as u64 * RS_N as u64);
        // Must be 0x20-aligned
        assert_eq!(size % PAGE_ALIGN_SIZE as u64, 0);
    }

    #[test]
    fn test_system_page_size_alignment() {
        // Result should always be 0x20-aligned or exactly 0x400
        for data_size in [8, 100, 500, 1000, 5000, 10000, 50000u64] {
            let ps = get_system_page_size(data_size);
            assert!(
                ps == 0x400 || ps % PAGE_ALIGN_SIZE as u64 == 0,
                "Page size {ps:#X} for data_size {data_size} not aligned"
            );
        }
    }

    // ─── Encode value tests ─────────────────────────────────────────

    #[test]
    fn test_encode_value_zero_shift() {
        assert_eq!(encode_value(0x1234, 0), 0x1234);
        // shift = 0 & 0x1F = 0 → no rotation
        assert_eq!(encode_value(0x1234, 0x20), 0x1234);
    }

    #[test]
    fn test_encode_value_shift_one() {
        // shift = 1 → rotate left by 1
        let result = encode_value(0x8000_0000_0000_0001, 1);
        assert_eq!(result, 0x0000_0000_0000_0003);
    }

    // ─── Writer structure tests ─────────────────────────────────────

    #[test]
    fn test_new_reserves_header_space() {
        let mut output = Cursor::new(Vec::new());
        let _writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        assert_eq!(output.position(), RESERVED_HEADER_SIZE as u64);
        assert_eq!(output.get_ref().len(), RESERVED_HEADER_SIZE);
        assert!(output.get_ref().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_handle_section_offset_always_zero() {
        let mut output = Cursor::new(Vec::new());
        let writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();
        assert_eq!(writer.handle_section_offset(), 0);
    }

    #[test]
    fn test_add_section_creates_pages() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        let data = vec![0xAA; 100];
        writer
            .add_section(&mut output, names::HEADER, &data)
            .unwrap();

        assert_eq!(writer.sections.len(), 1);
        assert_eq!(writer.sections[0].pages.len(), 1);
        assert_eq!(writer.page_records.len(), 1);
    }

    #[test]
    fn test_add_section_multi_page() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        // AcDb:Header has page_size 0x800 = 2048
        // Data larger than 2 pages
        let data = vec![0xBB; 0x800 * 2 + 100];
        writer
            .add_section(&mut output, names::HEADER, &data)
            .unwrap();

        assert_eq!(writer.sections.len(), 1);
        assert_eq!(writer.sections[0].pages.len(), 3);
        assert_eq!(writer.page_records.len(), 3);
    }

    #[test]
    fn test_section_page_sizes_match_page_records() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        // Force multiple AcDbObjects pages so we exercise real data-page bookkeeping.
        let data = vec![0x5A; 0xF800 * 2 + 321];
        writer
            .add_section(&mut output, names::ACDB_OBJECTS, &data)
            .unwrap();

        let section = &writer.sections[0];
        assert_eq!(section.pages.len(), writer.page_records.len());

        // Section map page_size = max decompressed page size (0xF800 for AcDbObjects).
        // Page record size = actual on-disk RS-encoded + aligned size.
        // They are intentionally different (matches AutoCAD behavior: spec §5.2).
        let expected_max = section.max_page_size;
        for page in &section.pages {
            assert_eq!(
                page.page_size, expected_max,
                "section map page_size should be max_page_size"
            );
            let _record = writer
                .page_records
                .iter()
                .find(|r| r.id == page.page_id)
                .expect("missing page record for section page");
        }
    }

    #[test]
    fn test_add_multiple_sections() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();
        writer
            .add_section(&mut output, names::CLASSES, &vec![0xBB; 200])
            .unwrap();

        assert_eq!(writer.sections.len(), 2);
        assert_eq!(writer.sections[0].name, names::HEADER);
        assert_eq!(writer.sections[1].name, names::CLASSES);
    }

    #[test]
    fn test_add_section_unknown_name_fails() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        let result = writer.add_section(&mut output, "AcDb:Nonexistent", &vec![0; 10]);
        assert!(result.is_err());
    }

    // ─── Section map building tests ─────────────────────────────────

    #[test]
    fn test_build_section_map_format() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();

        let map_data = writer.build_section_map().unwrap();

        // Should have: 0x40 header + UTF-16LE name + null term + page records
        // "AcDb:Header" = 11 chars → (11 + 1) × 2 = 24 bytes for name data
        // 1 page → 7 × 8 = 56 bytes
        // Total: 0x40 + 24 + 56 = 144 bytes
        assert_eq!(map_data.len(), 0x40 + 24 + 56);

        // Verify hash code at offset 0x18
        let hash = u64::from_le_bytes(map_data[0x18..0x20].try_into().unwrap());
        assert_eq!(hash, 0x32B803D9); // AcDb:Header hash code

        // Verify SectionNameLength at offset 0x20 is byte count (not char count)
        // "AcDb:Header" = 11 chars + 1 null = 12 × 2 = 24 bytes
        let name_len = u64::from_le_bytes(map_data[0x20..0x28].try_into().unwrap());
        assert_eq!(
            name_len, 24,
            "SectionNameLength should be byte count including null terminator"
        );
    }

    #[test]
    fn test_section_map_name_readable_by_reader() {
        // Verify that the section map name data can be parsed the same way
        // the reader does: read name_length bytes, decode as UTF-16LE, trim nulls
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::CLASSES, &vec![0xBB; 50])
            .unwrap();

        let map_data = writer.build_section_map().unwrap();

        // Read name_length from offset 0x20
        let name_len = u64::from_le_bytes(map_data[0x20..0x28].try_into().unwrap()) as usize;

        // Read name bytes starting at 0x40
        let name_bytes = &map_data[0x40..0x40 + name_len];
        let words: Vec<u16> = name_bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let name = String::from_utf16_lossy(&words)
            .trim_end_matches('\0')
            .to_string();
        assert_eq!(name, "AcDb:Classes");
    }

    #[test]
    fn test_build_page_map_format() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();

        let map_data = writer.build_page_map_ordered(100, 101, 0x400);

        // 1 data page entry + 2 page map entries + null terminator = 4 × 16 = 64 bytes
        assert_eq!(map_data.len(), 64);

        // First entry: the data page (id=1, comes first in file)
        let first_size = i64::from_le_bytes(map_data[0..8].try_into().unwrap());
        let first_id = i64::from_le_bytes(map_data[8..16].try_into().unwrap());
        assert_eq!(first_id, 1);
        assert!(first_size > 0);

        // Second entry: page map page (id=100, size=0x400)
        let pm_size = i64::from_le_bytes(map_data[16..24].try_into().unwrap());
        let pm_id = i64::from_le_bytes(map_data[24..32].try_into().unwrap());
        assert_eq!(pm_size, 0x400);
        assert_eq!(pm_id, 100);

        // Null terminator (size=0, id=0)
        let term_size = i64::from_le_bytes(map_data[48..56].try_into().unwrap());
        let term_id = i64::from_le_bytes(map_data[56..64].try_into().unwrap());
        assert_eq!(term_size, 0);
        assert_eq!(term_id, 0);
    }

    // ─── Full write test ────────────────────────────────────────────

    #[test]
    fn test_write_complete_file() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        // Add minimal sections
        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();
        writer
            .add_section(&mut output, names::CLASSES, &vec![0xBB; 50])
            .unwrap();
        writer
            .add_section(&mut output, names::SUMMARY_INFO, &vec![0; 10])
            .unwrap();
        writer
            .add_section(&mut output, names::PREVIEW, &vec![0xCC; 20])
            .unwrap();
        writer
            .add_section(&mut output, names::APP_INFO, &vec![0xDD; 30])
            .unwrap();
        writer
            .add_section(&mut output, names::AUX_HEADER, &vec![0; 50])
            .unwrap();
        writer
            .add_section(&mut output, names::ACDB_OBJECTS, &vec![0xEE; 300])
            .unwrap();
        writer
            .add_section(&mut output, names::HANDLES, &vec![0xFF; 100])
            .unwrap();

        writer.write_file(&mut output).unwrap();

        let data = output.into_inner();

        // File should start with "AC1021"
        assert_eq!(&data[0..6], b"AC1021");

        // File must be larger than the reserved header
        assert!(data.len() > RESERVED_HEADER_SIZE);

        // File header page at 0x80 should be exactly 0x400 bytes
        // (can verify it's not all zeros)
        assert!(!data[0x80..0x480].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_write_file_metadata_preview_addr() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();
        writer
            .add_section(&mut output, names::CLASSES, &vec![0xBB; 50])
            .unwrap();
        writer
            .add_section(&mut output, names::PREVIEW, &vec![0xCC; 20])
            .unwrap();

        writer.write_file(&mut output).unwrap();

        let data = output.into_inner();

        // Preview address at offset 0x0D should be non-zero
        let preview_addr = u32::from_le_bytes(data[0x0D..0x11].try_into().unwrap());
        assert!(preview_addr > 0, "Preview address should be non-zero");
    }

    #[test]
    fn test_file_header_page_has_check_data() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();

        writer.write_file(&mut output).unwrap();

        let data = output.into_inner();

        // Check data at 0x80 + 0x3D8 = 0x458 should not be all zeros
        let check_data = &data[0x458..0x480];
        assert!(
            !check_data.iter().all(|&b| b == 0),
            "Check data should be non-zero"
        );
    }

    #[test]
    fn test_file_header_copy_at_end() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();

        writer
            .add_section(&mut output, names::HEADER, &vec![0xAA; 100])
            .unwrap();

        writer.write_file(&mut output).unwrap();

        let data = output.into_inner();

        // The last 0x400 bytes should be a copy of the file header at 0x80
        let file_header = &data[0x80..0x480];
        let header_copy = &data[data.len() - FILE_HEADER_PAGE_SIZE..];
        assert_eq!(
            file_header, header_copy,
            "File header copy at end should match header at 0x80"
        );
    }

    // ─── RS data page encoding test ─────────────────────────────────

    #[test]
    fn stored_pages_are_raw_payload_with_rs_collision_guard() {
        // §19 H7: encoding-1 (stored) pages are the raw payload padded to
        // the 0x32 alignment — the author's convention (gold's raw path,
        // decode_r2007.c:868, memcpy's the payload). The page size must
        // never collide with the RS-coded form's size, or gold would pick
        // its RS path and de-interleave the raw payload into garbage.
        let rs_form = |size: usize| {
            let pesize = (size + 7) & !7;
            let blocks = (pesize + 251 - 1) / 251;
            (blocks * 255 + 31) & !31
        };
        for size in [
            0, 1, 7, 8, 82, 224, 225, 240, 248, 249, 250, 251, 252, 481, 500, 502, 503, 768,
            1296, 1502, 4097,
        ] {
            let data: Vec<u8> = (0..size).map(|i| (i * 73 + 11) as u8).collect();
            let page = encode_data_page(&data, 1, false);
            let aligned = align32(size);
            let expected_len = if aligned == rs_form(size) {
                aligned + PAGE_ALIGN_SIZE
            } else {
                aligned
            };
            assert_eq!(page.bytes.len(), expected_len, "size={size}");
            assert_eq!(&page.bytes[..size], &data, "size={size}");
            assert!(page.bytes[size..].iter().all(|&b| b == 0), "size={size}");
            assert_ne!(page.bytes.len(), rs_form(size), "size={size}");
            assert_eq!(page.compressed_size, size as u64);
            assert_eq!(page.uncompressed_size, size as u64);
            assert_eq!(page.crc, dwg_ac21_mirrored_crc64(0, size as u32, &data));
        }
    }

    #[test]
    fn compressed_pages_roundtrip_through_the_interleaved_rs_layout() {
        use crate::io::dwg::reed_solomon::reed_solomon_decode;

        // §19 H7: encoding-4 pages keep the interleaved RS(255,251)
        // layout — the only form gold's RS path (and this reader) decode.
        for size in [
            0, 1, 7, 8, 248, 249, 250, 251, 252, 500, 502, 503, 768, 4097,
        ] {
            let data: Vec<u8> = (0..size).map(|i| (i * 73 + 11) as u8).collect();
            let page = encode_data_page(&data, 4, true);
            let pesize = (size + 7) & !7;
            let blocks = (pesize + RS_DATA_K - 1) / RS_DATA_K;
            assert_eq!(page.bytes.len(), blocks * RS_N, "size={size}");
            let mut decoded = vec![0u8; blocks * RS_DATA_K];
            reed_solomon_decode(&page.bytes, &mut decoded, blocks, RS_DATA_K);
            assert_eq!(&decoded[..size], &data, "size={size}");
        }
    }

    #[test]
    fn system_pages_store_every_advertised_copy() {
        use crate::io::dwg::reed_solomon::reed_solomon_decode;

        let mut output = Cursor::new(Vec::new());
        let writer = DwgFileHeaderWriterAC21::new(DxfVersion::AC1021, &mut output).unwrap();
        for size in [64, 2048, 8192] {
            let data: Vec<u8> = (0..size).map(|i| (i * 73 + i / 17) as u8).collect();
            let page_size = get_system_page_size(size as u64) as usize;
            let (encoded, compressed_size, _, compressed_crc, _, repeats) =
                writer.encode_system_page(&data, page_size);
            let blocks = encoded.len() / RS_N;
            let mut decoded = vec![0; blocks * RS_SYSTEM_K];
            reed_solomon_decode(&encoded, &mut decoded, blocks, RS_SYSTEM_K);
            let stride = (compressed_size as usize + 7) & !7;
            assert!(repeats >= 2);
            let first = &decoded[..stride];
            assert_eq!(
                dwg_ac21_mirrored_crc64(
                    0,
                    compressed_size as u32,
                    &first[..compressed_size as usize]
                ),
                compressed_crc
            );
            for copy in decoded[..stride * repeats as usize].chunks_exact(stride) {
                assert_eq!(copy, first);
            }
        }
    }

    #[test]
    fn test_rs_encode_data_page_encoding1_size() {
        // §19 H7: stored pages are the raw payload padded to 0x32 (the
        // RS-coded form of 500 bytes is 768, so no collision guard fires).
        let data = vec![0xAB; 500];
        let encoded = encode_data_page(&data, 1, false).bytes;
        assert_eq!(encoded.len(), 512);
    }

    #[test]
    fn test_rs_encode_data_page_encoding4_size() {
        // encoding=4 also uses RS(255, 251) for data pages per spec §5.4
        let data = vec![0xAB; 500];
        let encoded = rs_encode_data_page_interleaved(&data, 4);

        // total_size = (500+7)&~7 = 504, factor = ceil(504/251) = 3
        // encoded = 3 × 255 = 765
        assert_eq!(encoded.len(), 3 * 255);
    }

    #[test]
    fn test_rs_encode_data_page_single_block() {
        // §19 H7: encoding=1 stores the raw payload (100 → align32 128;
        // the single-block RS form is 256, so no collision guard).
        let data = vec![0xCD; 100];
        let encoded = encode_data_page(&data, 1, false).bytes;
        assert_eq!(encoded.len(), 128);
        assert_eq!(&encoded[..100], &data);
    }

    #[test]
    fn test_rs_encode_data_page_encoding4_single_block() {
        // encoding=4 also uses RS(255, 251) for data pages per spec §5.4
        let data = vec![0xCD; 100];
        let encoded = rs_encode_data_page_interleaved(&data, 4);

        // total_size = (100+7)&~7 = 104, factor = ceil(104/251) = 1
        // encoded = 1 × 255 = 255
        assert_eq!(encoded.len(), 255);
    }

    #[test]
    fn test_rs_encode_data_page_roundtrip_encoding4() {
        // Verify that encoding → decoding recovers the original data (encoding=4)
        // All data pages use RS(255,251) per spec §5.4 / §5.13
        use crate::io::dwg::reed_solomon::reed_solomon_decode;

        let original = vec![0x42u8; 300];
        let encoded = rs_encode_data_page_interleaved(&original, 4);

        // Reader's decode parameters: all data pages use block_size=251
        let total_size = (300 + 7) & !7; // 304
        let factor = (total_size + 251 - 1) / 251; // ceil(304/251) = 2
        assert_eq!(encoded.len(), factor * 255);

        let mut decoded = vec![0u8; total_size];
        reed_solomon_decode(&encoded, &mut decoded, factor, 251);

        // First 300 bytes should match original
        assert_eq!(&decoded[..300], &original[..]);
    }

    #[test]
    fn test_rs_encode_data_page_roundtrip_encoding1() {
        // §19 H7: the stored page is the raw payload — the reader's
        // encoding-1 path reads the payload straight off the page start.
        let original = vec![0x77u8; 200];
        let encoded = encode_data_page(&original, 1, false).bytes;

        // align32(200) = 224; the RS form is 256, so no collision guard.
        assert_eq!(encoded.len(), 224);
        assert_eq!(&encoded[..200], &original[..]);
        assert!(encoded[200..].iter().all(|&b| b == 0));

        // The payload extraction is a plain prefix read.
        let decoded = &encoded[..original.len()];
        assert_eq!(decoded, &original[..]);
    }

    #[test]
    fn test_get_aligned_page_size() {
        assert_eq!(get_aligned_page_size(0x20), 0x20);
        assert_eq!(get_aligned_page_size(0x21), 0x40);
        assert_eq!(get_aligned_page_size(0x3F), 0x40);
        assert_eq!(get_aligned_page_size(0x40), 0x40);
        assert_eq!(get_aligned_page_size(1), 0x20);
    }
}
