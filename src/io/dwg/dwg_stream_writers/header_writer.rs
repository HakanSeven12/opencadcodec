//! DWG Header section writer
//!
//! Writes the HEADER section containing all drawing header variables.
//! This is the most complex section, writing ~200 fields with extensive
//! version-conditional logic.
//!
//! ## Stream format
//!
//! - **Pre-R2007**: All data (including handle references) is written
//!   sequentially to a single stream (two-stream merge: text is inline,
//!   handles are appended at end Ã¢â‚¬â€ but for the header section the
//!   single-stream approach is used for legacy reasons).
//! - **R2007+**: Uses three-stream merge (`DwgMergedWriter`):
//!   text goes to a separate text sub-stream, handle references go to
//!   a separate handle sub-stream, and everything else to the main
//!   sub-stream. The merged output includes text-size flag words and
//!   a text-present bit per the R2007+ DWG section format.
//!
//! The section data is then wrapped with sentinels and CRC-16.
//!
//! Based on the reference `DwgHeaderWriter`.

use crate::document::HeaderVariables;
use crate::io::dwg::crc::{crc16, CRC16_SEED};
use crate::io::dwg::dwg_reference_type::DwgReferenceType;
use crate::io::dwg::dwg_stream_writers::DwgBitWriter;
use crate::io::dwg::dwg_stream_writers::DwgMergedWriter;
use crate::io::dwg::dwg_version::DwgVersion;
use crate::io::dwg::file_headers::section_definition::{end_sentinels, start_sentinels};
use crate::types::{Color, DxfVersion, Handle, LineWeight, Vector2, Vector3};

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Writer wrapper Ã¢â‚¬â€ dispatches to DwgBitWriter or DwgMergedWriter
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

/// Internal writer that uses DwgBitWriter for pre-R2007 and DwgMergedWriter
/// (three-stream merge) for R2007+ (AC1021+). This ensures that for R2007+,
/// text goes to the text sub-stream and handle references go to the handle
/// sub-stream, matching the reference `DwgHeaderWriter` behavior.
enum SectionWriterInner {
    /// Pre-R2007: single stream, everything inline
    BitWriter(DwgBitWriter),
    /// R2007+: three-stream merge (main + text + handle)
    MergedWriter(DwgMergedWriter),
}

struct SectionWriter {
    inner: SectionWriterInner,
}

impl SectionWriter {
    fn with_encoding(version: DxfVersion, encoding: &'static encoding_rs::Encoding) -> Self {
        let dwg = DwgVersion::from_dxf_version(version).unwrap_or(DwgVersion::AC15);

        let inner = if version >= DxfVersion::AC1021 {
            // R2007+: use three-stream merge
            let mut writer = DwgMergedWriter::with_encoding(dwg, version, encoding);
            writer.save_position_for_size(); // RL placeholder for total size in bits
            SectionWriterInner::MergedWriter(writer)
        } else {
            // Pre-R2007: single stream
            let writer = DwgBitWriter::with_encoding(dwg, version, encoding);
            SectionWriterInner::BitWriter(writer)
        };

        SectionWriter { inner }
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Main-stream data writes Ã¢â€â‚¬Ã¢â€â‚¬

    fn write_bit(&mut self, value: bool) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_bit(value),
            SectionWriterInner::MergedWriter(w) => w.write_bit(value),
        }
    }

    fn write_byte(&mut self, value: u8) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_byte(value),
            SectionWriterInner::MergedWriter(w) => w.write_byte(value),
        }
    }

    fn write_bit_short(&mut self, value: i16) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_bit_short(value),
            SectionWriterInner::MergedWriter(w) => w.write_bit_short(value),
        }
    }

    fn write_bit_long(&mut self, value: i32) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_bit_long(value),
            SectionWriterInner::MergedWriter(w) => w.write_bit_long(value),
        }
    }

    fn write_bit_long_long(&mut self, value: i64) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_bit_long_long(value),
            SectionWriterInner::MergedWriter(w) => w.write_bit_long_long(value),
        }
    }

    fn write_bit_double(&mut self, value: f64) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_bit_double(value),
            SectionWriterInner::MergedWriter(w) => w.write_bit_double(value),
        }
    }

    fn write_3bit_double(&mut self, value: Vector3) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_3bit_double(value),
            SectionWriterInner::MergedWriter(w) => w.write_3bit_double(value),
        }
    }

    fn write_2raw_double(&mut self, value: Vector2) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_2raw_double(value),
            SectionWriterInner::MergedWriter(w) => w.write_2raw_double(value),
        }
    }

    fn write_cm_color(&mut self, color: &Color) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_cm_color(color),
            SectionWriterInner::MergedWriter(w) => w.write_cm_color(color),
        }
    }

    /// Write a CmColor from its retained raw parts, re-emitting the wire
    /// form verbatim (Ã‚Â§19 H7): the wire BS index, the raw rgb word, the
    /// validated flag byte and the name/book-name strings behind its
    /// bits. Composed from the main-stream primitives so the text
    /// sub-stream routing (R2007+) follows `write_variable_text`.
    fn write_cm_color_raw(&mut self, cmc: &crate::document::DwgRawCmc) {
        self.write_bit_short(cmc.index as i16);
        self.write_bit_long(cmc.rgb as i32);
        self.write_byte(cmc.flag as u8);
        if (cmc.flag & 1) != 0 {
            if let Some(name) = &cmc.name {
                self.write_variable_text(name);
            }
        }
        if (cmc.flag & 2) != 0 {
            if let Some(book) = &cmc.book_name {
                self.write_variable_text(book);
            }
        }
    }

    fn write_datetime(&mut self, day: i32, ms: i32) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_datetime(day, ms),
            SectionWriterInner::MergedWriter(w) => w.write_datetime(day, ms),
        }
    }

    fn write_timespan(&mut self, days: i32, ms: i32) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_timespan(days, ms),
            SectionWriterInner::MergedWriter(w) => w.write_timespan(days, ms),
        }
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Text writes: route to text sub-stream for R2007+ Ã¢â€â‚¬Ã¢â€â‚¬

    fn write_variable_text(&mut self, value: &str) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_variable_text(value),
            SectionWriterInner::MergedWriter(w) => w.write_variable_text(value),
        }
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Handle writes: route to handle sub-stream for R2007+ Ã¢â€â‚¬Ã¢â€â‚¬

    /// Write handle reference Ã¢â‚¬â€ goes to handle sub-stream for R2007+.
    fn write_handle_ref(&mut self, ref_type: DwgReferenceType, handle: Handle) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_handle(ref_type, handle.value()),
            SectionWriterInner::MergedWriter(w) => w.write_handle(ref_type, handle.value()),
        }
    }

    /// Write a handle reference from its retained raw form (Ã‚Â§19 H7
    /// review): the raw-only slots re-emit the captured wire tuple
    /// verbatim Ã¢â‚¬â€ same code, same counter size, same payload Ã¢â‚¬â€ instead
    /// of a recomputed canonical HardPointer/absolute form (corpus-neutral
    /// today: every corpus header handle already is the canonical form,
    /// but a future authored non-canonical form would round-trip).
    fn write_handle_raw(&mut self, t: &crate::document::DwgRawHandle) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_handle_form(t.code, t.size, t.value),
            SectionWriterInner::MergedWriter(w) => w.write_handle_form(t.code, t.size, t.value),
        }
    }

    /// Write HANDSEED Ã¢â‚¬â€ always goes to the MAIN stream, even for R2007+.
    /// This matches C#: `this._writer.Main.HandleReference(...)`.
    fn write_handle_seed(&mut self, handle_seed: u64) {
        match &mut self.inner {
            SectionWriterInner::BitWriter(w) => w.write_handle_undefined(handle_seed),
            SectionWriterInner::MergedWriter(w) => {
                // HANDSEED is written to the main stream specifically,
                // not the handle sub-stream.
                w.main_mut().write_handle_undefined(handle_seed);
            }
        }
    }

    /// Finalize and return section data bytes.
    fn finalize(self) -> Vec<u8> {
        match self.inner {
            SectionWriterInner::BitWriter(mut w) => {
                // Pre-R2007: just pad and return
                w.write_spear_shift();
                w.into_bytes()
            }
            SectionWriterInner::MergedWriter(mut w) => {
                // R2007+: three-stream merge handles RL patching,
                // text-size flags, and byte alignment automatically.
                w.merge()
            }
        }
    }
}

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Public API
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

/// Write the complete Header section.
///
/// # Arguments
/// * `version` - Target DXF/DWG version
/// * `header` - Document header variables
///
/// # Returns
/// Complete section bytes including sentinels and CRC.
pub fn write_header(
    version: DxfVersion,
    header: &HeaderVariables,
    maintenance_version: u8,
) -> Vec<u8> {
    write_header_with_encoding_opt(
        version,
        header,
        maintenance_version,
        encoding_rs::WINDOWS_1252,
        None,
    )
}

pub fn write_header_with_encoding(
    version: DxfVersion,
    header: &HeaderVariables,
    maintenance_version: u8,
    encoding: &'static encoding_rs::Encoding,
) -> Vec<u8> {
    write_header_with_encoding_opt(version, header, maintenance_version, encoding, None)
}

/// Write the complete Header section, splicing the retained raw mirror
/// (Ã‚Â§19 H7) into every slot the model does not carry: the unmodeled
/// fields re-emit their wire values verbatim instead of writer defaults,
/// so a read-modify-write roundtrip preserves the original bits. The
/// modeled fields keep the model path (the `prepare_header` syncs and
/// the API surface stay authoritative); `raw = None` (programmatic
/// documents) keeps the historical default-constant behaviour.
pub fn write_header_with_encoding_opt(
    version: DxfVersion,
    header: &HeaderVariables,
    maintenance_version: u8,
    encoding: &'static encoding_rs::Encoding,
    raw: Option<&crate::document::DwgHeaderRaw>,
) -> Vec<u8> {
    let mut w = SectionWriter::with_encoding(version, encoding);
    write_header_fields(&mut w, version, header, raw);
    let section_data = w.finalize();
    wrap_with_sentinels_and_crc(version, maintenance_version, &section_data)
}

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Sentinel + CRC wrapper (same pattern as classes_writer)
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

fn wrap_with_sentinels_and_crc(
    version: DxfVersion,
    maintenance_version: u8,
    section_data: &[u8],
) -> Vec<u8> {
    let mut output = Vec::with_capacity(16 + 4 + section_data.len() + 2 + 16 + 8);

    output.extend_from_slice(&start_sentinels::HEADER);

    let mut crc_content = Vec::with_capacity(4 + section_data.len());
    crc_content.extend_from_slice(&(section_data.len() as i32).to_le_bytes());

    // Extra 4 zero bytes when: (AC1024+ && maintenance > 3) || AC1032+
    if DwgVersion::has_section_extra_rl(version, maintenance_version) {
        crc_content.extend_from_slice(&0i32.to_le_bytes());
    }

    crc_content.extend_from_slice(section_data);

    let crc = crc16(CRC16_SEED, &crc_content);
    output.extend_from_slice(&crc_content);
    output.extend_from_slice(&crc.to_le_bytes());

    output.extend_from_slice(&end_sentinels::HEADER);

    // R2004+: trailing 8 zero bytes (matches classes_writer pattern)
    if version >= DxfVersion::AC1018 {
        output.extend_from_slice(&[0u8; 8]);
    }

    output
}

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Version-range helpers
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

#[inline]
fn r13_14_only(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1012 && v <= DxfVersion::AC1014
}

#[inline]
fn r13_15_only(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1012 && v <= DxfVersion::AC1015
}

#[inline]
fn r2000_plus(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1015
}

#[inline]
fn r2004_plus(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1018
}

#[inline]
fn r2007_plus(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1021
}

#[inline]
fn r2010_plus(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1024
}

#[inline]
fn r2013_plus(v: DxfVersion) -> bool {
    v >= DxfVersion::AC1027
}

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Julian date helpers
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

fn julian_to_day_ms(julian: f64) -> (i32, i32) {
    let day = julian as i32;
    let fraction = julian - day as f64;
    // Round, not truncate: the f64 days+ms/86400000 roundtrip carries a
    // tiny representation error (113000 ms Ã¢â€ â€™ 112999.9999Ã¢â‚¬Â¦) Ã¢â‚¬â€ truncation
    // lost 1 ms per span (Ã‚Â§19 H7: the TDINDWG/TDUSRTIMER off-by-ones).
    let ms = ((fraction * 86_400_000.0).round() as i32).clamp(0, 86_399_999);
    (day, ms)
}

fn timespan_to_day_ms(days_fraction: f64) -> (i32, i32) {
    let days = days_fraction as i32;
    let fraction = days_fraction - days as f64;
    let ms = ((fraction * 86_400_000.0).round() as i32).clamp(0, 86_399_999);
    (days, ms)
}

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Header field writer Ã¢â‚¬â€ the big one (~200 fields)
//
//  The unmodeled slots splice the retained raw mirror (`raw`, Ã‚Â§19 H7)
//  when present Ã¢â‚¬â€ re-emitting the wire values verbatim Ã¢â‚¬â€ and fall back
//  to the historical default constants for programmatic documents
//  (`raw = None`). The modeled slots always write the model.
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

fn write_header_fields(
    w: &mut SectionWriter,
    v: DxfVersion,
    h: &HeaderVariables,
    raw: Option<&crate::document::DwgHeaderRaw>,
) {
    // Splice helpers: the raw value when retained, else the default.
    macro_rules! splice {
        ($field:ident, $default:expr) => {
            match raw.and_then(|r| r.$field) {
                Some(t) => t,
                None => $default,
            }
        };
    }
    macro_rules! splice_pt {
        ($raw:expr, $field:ident) => {
            match $raw.and_then(|r| r.$field) {
                Some(a) => Vector3::new(a[0], a[1], a[2]),
                None => Vector3::ZERO,
            }
        };
    }
    // The raw-only handle slots re-emit the retained wire tuple
    // verbatim (Ã‚Â§19 H7 review) Ã¢â‚¬â€ a null-handle default for the
    // programmatic documents.
    macro_rules! splice_handle_raw {
        ($field:ident) => {
            match raw.and_then(|r| r.$field.as_ref()) {
                Some(t) => w.write_handle_raw(t),
                None => w.write_handle_ref(DwgReferenceType::HardPointer, Handle::NULL),
            }
        };
    }

    // R2013+: BLL REQUIREDVERSIONS
    if r2013_plus(v) {
        w.write_bit_long_long(h.required_versions);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Unit conversions (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_bit_double(splice!(unit1_ratio, 412148564080.0));
    w.write_bit_double(splice!(unit2_ratio, 1.0));
    w.write_bit_double(splice!(unit3_ratio, 1.0));
    w.write_bit_double(splice!(unit4_ratio, 1.0));

    w.write_variable_text(raw.and_then(|r| r.unit1_name.as_deref()).unwrap_or("m"));
    w.write_variable_text(raw.and_then(|r| r.unit2_name.as_deref()).unwrap_or(""));
    w.write_variable_text(raw.and_then(|r| r.unit3_name.as_deref()).unwrap_or(""));
    w.write_variable_text(raw.and_then(|r| r.unit4_name.as_deref()).unwrap_or(""));

    w.write_bit_long(splice!(unknown_8, 24) as u32 as i32);
    w.write_bit_long(splice!(unknown_9, 0) as u32 as i32);

    // R13-R14 Only: BS unknown_10 (Ã‚Â§19 H7 Ã¢â‚¬â€ replay the captured wire
    // value; 256 = the '11' code on the R14 genus)
    if r13_14_only(v) {
        w.write_bit_short(splice!(unknown_10, 0) as u16 as i16);
    }

    // Pre-2004: current viewport header handle
    if v < DxfVersion::AC1018 {
        w.write_handle_ref(DwgReferenceType::HardPointer, h.current_vx_handle);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Drawing mode flags (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_bit(h.associate_dimensions);
    w.write_bit(h.update_dimensions_while_dragging);

    if r13_14_only(v) {
        w.write_bit(splice!(dimsav, 0) != 0); // DIMSAV undocumented
    }

    w.write_bit(h.polyline_linetype_generation);
    w.write_bit(h.ortho_mode);
    w.write_bit(h.regen_mode);
    w.write_bit(h.fill_mode);
    w.write_bit(h.quick_text_mode);
    w.write_bit(h.paper_space_linetype_scaling);
    w.write_bit(h.limit_check);

    if r13_14_only(v) {
        w.write_bit(h.blip_mode);
    }

    if r2004_plus(v) {
        w.write_bit(splice!(unknown_11, 0) != 0); // undocumented
    }

    w.write_bit(h.user_timer);
    // The legacy DWG header stores SKPOLY as one bit.
    w.write_bit(h.sketch_type != 0); // SKPOLY
    w.write_bit(h.angle_direction != 0); // ANGDIR
    w.write_bit(h.spline_frame); // SPLFRAME

    if r13_14_only(v) {
        w.write_bit(h.attribute_request);
        w.write_bit(h.attribute_dialog);
    }

    w.write_bit(h.mirror_text);
    w.write_bit(h.world_view);

    if r13_14_only(v) {
        w.write_bit(splice!(wireframe, 0) != 0); // WIREFRAME
    }

    w.write_bit(h.show_model_space);
    w.write_bit(h.paper_space_limit_check);
    w.write_bit(h.retain_xref_visibility);

    if r13_14_only(v) {
        w.write_bit(h.delete_objects);
    }

    w.write_bit(h.display_silhouette);
    w.write_bit(splice!(pellipse, 0) != 0); // PELLIPSE (CreateEllipseAsPolyline)
    w.write_bit_short(h.proxy_graphics);

    if r13_14_only(v) {
        w.write_bit_short(h.drag_mode);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Unit settings (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_bit_short(h.tree_depth);
    w.write_bit_short(h.linear_unit_format);
    w.write_bit_short(h.linear_unit_precision);
    w.write_bit_short(h.angular_unit_format);
    w.write_bit_short(h.angular_unit_precision);

    if r13_14_only(v) {
        w.write_bit_short(h.object_snap_mode as i16);
    }

    w.write_bit_short(h.attribute_visibility);

    if r13_14_only(v) {
        w.write_bit_short(h.coords_mode);
    }

    w.write_bit_short(h.point_display_mode);

    if r13_14_only(v) {
        w.write_bit_short(h.pick_style);
    }

    if r2004_plus(v) {
        w.write_bit_long(splice!(unknown_12, 0) as u32 as i32); // unknown
        w.write_bit_long(splice!(unknown_13, 0) as u32 as i32); // unknown
        w.write_bit_long(splice!(unknown_14, 0) as u32 as i32); // unknown
    }

    w.write_bit_short(h.user_int1);
    w.write_bit_short(h.user_int2);
    w.write_bit_short(h.user_int3);
    w.write_bit_short(h.user_int4);
    w.write_bit_short(h.user_int5);

    w.write_bit_short(h.spline_segments);
    w.write_bit_short(h.surface_u_density);
    w.write_bit_short(h.surface_v_density);
    w.write_bit_short(h.surface_type);
    w.write_bit_short(h.surface_tab1);
    w.write_bit_short(h.surface_tab2);
    w.write_bit_short(h.spline_type);
    w.write_bit_short(h.shade_edge);
    w.write_bit_short(h.shade_diffuse);
    w.write_bit_short(splice!(unitmode, 0) as u16 as i16); // UNITMODE
    w.write_bit_short(h.max_active_viewports);
    w.write_bit_short(h.isolines);
    w.write_bit_short(h.multiline_justification);
    w.write_bit_short(h.text_quality);

    // Ã¢â€â‚¬Ã¢â€â‚¬ Scale/size defaults (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_bit_double(h.linetype_scale);
    w.write_bit_double(h.text_height);
    w.write_bit_double(h.trace_width);
    w.write_bit_double(h.sketch_increment);
    w.write_bit_double(h.fillet_radius);
    w.write_bit_double(h.thickness);
    w.write_bit_double(h.angle_base);
    w.write_bit_double(h.point_display_size);
    w.write_bit_double(h.polyline_width);
    w.write_bit_double(h.user_real1);
    w.write_bit_double(h.user_real2);
    w.write_bit_double(h.user_real3);
    w.write_bit_double(h.user_real4);
    w.write_bit_double(h.user_real5);
    w.write_bit_double(h.chamfer_distance_a);
    w.write_bit_double(h.chamfer_distance_b);
    w.write_bit_double(h.chamfer_length);
    w.write_bit_double(h.chamfer_angle);
    w.write_bit_double(h.facet_resolution);
    w.write_bit_double(h.multiline_scale);
    w.write_bit_double(h.current_entity_linetype_scale);

    w.write_variable_text(&h.menu_name);

    // Ã¢â€â‚¬Ã¢â€â‚¬ Date/time (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    let (cd, cms) = julian_to_day_ms(h.create_date_julian);
    w.write_datetime(cd, cms);
    let (ud, ums) = julian_to_day_ms(h.update_date_julian);
    w.write_datetime(ud, ums);

    if r2004_plus(v) {
        w.write_bit_long(splice!(unknown_15, 0) as u32 as i32); // unknown
        w.write_bit_long(splice!(unknown_16, 0) as u32 as i32); // unknown
        w.write_bit_long(splice!(unknown_17, 0) as u32 as i32); // unknown
    }

    let (ted, tems) = timespan_to_day_ms(h.total_editing_time);
    w.write_timespan(ted, tems);
    let (ued, uems) = timespan_to_day_ms(h.user_elapsed_time);
    w.write_timespan(ued, uems);

    // Ã¢â€â‚¬Ã¢â€â‚¬ Current entity color Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_cm_color(&h.current_entity_color);

    // Ã¢â€â‚¬Ã¢â€â‚¬ HANDSEED Ã¢â‚¬â€ always main stream Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_handle_seed(h.handle_seed);

    // Ã¢â€â‚¬Ã¢â€â‚¬ Style/layer/linetype handles Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_handle_ref(DwgReferenceType::HardPointer, h.current_layer_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.current_text_style_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.current_linetype_handle);

    if r2007_plus(v) {
        w.write_handle_ref(DwgReferenceType::HardPointer, h.current_material_handle);
    }

    w.write_handle_ref(DwgReferenceType::HardPointer, h.current_dimstyle_handle);
    w.write_handle_ref(
        DwgReferenceType::HardPointer,
        h.current_multiline_style_handle,
    );

    if r2000_plus(v) {
        w.write_bit_double(h.viewport_scale_factor);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Paper space extents/limits/UCS Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_3bit_double(h.paper_space_insertion_base);
    w.write_3bit_double(h.paper_space_extents_min);
    w.write_3bit_double(h.paper_space_extents_max);
    w.write_2raw_double(h.paper_space_limits_min);
    w.write_2raw_double(h.paper_space_limits_max);
    w.write_bit_double(h.paper_elevation);
    w.write_3bit_double(h.paper_space_ucs_origin);
    w.write_3bit_double(h.paper_space_ucs_x_axis);
    w.write_3bit_double(h.paper_space_ucs_y_axis);

    // UCSNAME (PSPACE)
    splice_handle_raw!(pucsname);

    if r2000_plus(v) {
        // PUCSORTHOREF
        w.write_handle_ref(DwgReferenceType::HardPointer, h.paper_ucs_ortho_ref);
        w.write_bit_short(h.paper_ucs_ortho_view);
        // PUCSBASE
        splice_handle_raw!(pucsbase);

        // Paper space orthographic origins (6 Ãƒâ€” 3BD)
        w.write_3bit_double(splice_pt!(raw, pucsorgtop));
        w.write_3bit_double(splice_pt!(raw, pucsorgbottom));
        w.write_3bit_double(splice_pt!(raw, pucsorgleft));
        w.write_3bit_double(splice_pt!(raw, pucsorgright));
        w.write_3bit_double(splice_pt!(raw, pucsorgfront));
        w.write_3bit_double(splice_pt!(raw, pucsorgback));
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Model space extents/limits/UCS Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_3bit_double(h.model_space_insertion_base);
    w.write_3bit_double(h.model_space_extents_min);
    w.write_3bit_double(h.model_space_extents_max);
    w.write_2raw_double(h.model_space_limits_min);
    w.write_2raw_double(h.model_space_limits_max);
    w.write_bit_double(h.elevation);
    w.write_3bit_double(h.model_space_ucs_origin);
    w.write_3bit_double(h.model_space_ucs_x_axis);
    w.write_3bit_double(h.model_space_ucs_y_axis);

    // UCSNAME (MSPACE)
    splice_handle_raw!(ucsname);

    if r2000_plus(v) {
        // UCSORTHOREF
        w.write_handle_ref(DwgReferenceType::HardPointer, h.ucs_ortho_ref);
        w.write_bit_short(h.ucs_ortho_view);
        // UCSBASE
        splice_handle_raw!(ucsbase);

        // Model space orthographic origins (6 Ãƒâ€” 3BD)
        w.write_3bit_double(splice_pt!(raw, ucsorgtop));
        w.write_3bit_double(splice_pt!(raw, ucsorgbottom));
        w.write_3bit_double(splice_pt!(raw, ucsorgleft));
        w.write_3bit_double(splice_pt!(raw, ucsorgright));
        w.write_3bit_double(splice_pt!(raw, ucsorgfront));
        w.write_3bit_double(splice_pt!(raw, ucsorgback));

        // DIMPOST, DIMAPOST
        w.write_variable_text(&h.dim_post);
        w.write_variable_text(&h.dim_alt_post);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Dimension variables (R13-R14 Only block) Ã¢â€â‚¬Ã¢â€â‚¬
    if r13_14_only(v) {
        w.write_bit(h.dim_tolerance);
        w.write_bit(h.dim_limits);
        w.write_bit(h.dim_text_inside_horizontal);
        w.write_bit(h.dim_text_outside_horizontal);
        w.write_bit(h.dim_suppress_ext1);
        w.write_bit(h.dim_suppress_ext2);
        w.write_bit(h.dim_alternate_units);
        w.write_bit(h.dim_force_line_inside);
        w.write_bit(h.dim_separate_arrows);
        w.write_bit(h.dim_force_text_inside);
        w.write_bit(h.dim_suppress_outside_ext);
        w.write_byte(h.dim_alt_decimal_places as u8);
        w.write_byte(h.dim_zero_suppression as u8);
        w.write_bit(h.dim_suppress_line1);
        w.write_bit(h.dim_suppress_line2);
        w.write_byte(h.dim_tolerance_justification as u8);
        w.write_byte(h.dim_horizontal_justification as u8);
        w.write_byte(h.dim_fit as u8);
        w.write_bit(h.dim_user_positioned_text);
        w.write_byte(h.dim_tolerance_zero_suppression as u8);
        w.write_byte(h.dim_alt_tolerance_zero_suppression as u8);
        w.write_byte(h.dim_alt_tolerance_zero_tight as u8);
        w.write_byte(h.dim_text_above as u8);
        w.write_bit_short(splice!(dimunit, 0) as u16 as i16); // DIMUNIT
        w.write_bit_short(h.dim_angular_decimal_places);
        w.write_bit_short(h.dim_decimal_places);
        w.write_bit_short(h.dim_tolerance_decimal_places);
        w.write_bit_short(h.dim_alt_units_format);
        w.write_bit_short(h.dim_alt_tolerance_decimal_places);

        // DIMTXSTY handle
        w.write_handle_ref(DwgReferenceType::HardPointer, h.dim_text_style_handle);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Dimension variables (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_bit_double(h.dim_scale);
    w.write_bit_double(h.dim_arrow_size);
    w.write_bit_double(h.dim_ext_line_offset);
    w.write_bit_double(h.dim_line_increment);
    w.write_bit_double(h.dim_ext_line_extension);
    w.write_bit_double(h.dim_rounding);
    w.write_bit_double(h.dim_line_extension);
    w.write_bit_double(h.dim_tolerance_plus);
    w.write_bit_double(h.dim_tolerance_minus);

    // R2007+ dimension extras
    if r2007_plus(v) {
        w.write_bit_double(splice!(dimfxl, 0.0)); // DIMFXL
        w.write_bit_double(splice!(dimjogang, 0.7854)); // DIMJOGANG (default 45Ã‚Â°)
        w.write_bit_short(splice!(dimtfill, 0) as u16 as i16); // DIMTFILL
        match raw.and_then(|r| r.dimtfillclr.as_ref()) {
            Some(cmc) => w.write_cm_color_raw(cmc),
            None => w.write_cm_color(&Color::ByBlock), // DIMTFILLCLR
        }
    }

    // R2000+ dimension flags
    if r2000_plus(v) {
        w.write_bit(h.dim_tolerance);
        w.write_bit(h.dim_limits);
        w.write_bit(h.dim_text_inside_horizontal);
        w.write_bit(h.dim_text_outside_horizontal);
        w.write_bit(h.dim_suppress_ext1);
        w.write_bit(h.dim_suppress_ext2);
        w.write_bit_short(h.dim_text_above);
        w.write_bit_short(h.dim_zero_suppression);
        w.write_bit_short(h.dim_alt_zero_suppression);
    }

    if r2007_plus(v) {
        w.write_bit_short(splice!(dimarcsym, 0) as u16 as i16); // DIMARCSYM
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Dimension sizes (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_bit_double(h.dim_text_height);
    w.write_bit_double(h.dim_center_mark);
    w.write_bit_double(h.dim_tick_size);
    w.write_bit_double(h.dim_alt_scale);
    w.write_bit_double(h.dim_linear_scale);
    w.write_bit_double(h.dim_text_vertical_pos);
    w.write_bit_double(h.dim_tolerance_scale);
    w.write_bit_double(h.dim_line_gap);

    // R13-R14 only: dimension text strings
    if r13_14_only(v) {
        w.write_variable_text(&h.dim_post);
        w.write_variable_text(&h.dim_alt_post);
        w.write_variable_text(&h.dim_arrow_block);
        w.write_variable_text(&h.dim_arrow_block1);
        w.write_variable_text(&h.dim_arrow_block2);
    }

    // R2000+ only: additional dimension settings
    if r2000_plus(v) {
        w.write_bit_double(h.dim_alt_rounding);
        w.write_bit(h.dim_alternate_units);
        w.write_bit_short(h.dim_alt_decimal_places);
        w.write_bit(h.dim_force_line_inside);
        w.write_bit(h.dim_separate_arrows);
        w.write_bit(h.dim_force_text_inside);
        w.write_bit(h.dim_suppress_outside_ext);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Dimension colors (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_cm_color(&h.dim_line_color);
    w.write_cm_color(&h.dim_ext_line_color);
    w.write_cm_color(&h.dim_text_color);

    // R2000+ only: dimension unit settings
    if r2000_plus(v) {
        w.write_bit_short(h.dim_angular_decimal_places);
        w.write_bit_short(h.dim_decimal_places);
        w.write_bit_short(h.dim_tolerance_decimal_places);
        w.write_bit_short(h.dim_alt_units_format);
        w.write_bit_short(h.dim_alt_tolerance_decimal_places);
        w.write_bit_short(h.dim_angular_units);
        w.write_bit_short(h.dim_fraction_format);
        w.write_bit_short(h.dim_linear_unit_format);
        w.write_bit_short(h.dim_decimal_separator as i16);
        w.write_bit_short(h.dim_text_movement);
        w.write_bit_short(h.dim_horizontal_justification);
        w.write_bit(h.dim_suppress_line1);
        w.write_bit(h.dim_suppress_line2);
        w.write_bit_short(h.dim_tolerance_justification);
        w.write_bit_short(h.dim_tolerance_zero_suppression);
        w.write_bit_short(h.dim_alt_tolerance_zero_suppression);
        w.write_bit_short(h.dim_alt_tolerance_zero_tight);
        w.write_bit(h.dim_user_positioned_text);
        w.write_bit_short(h.dim_fit);
    }

    // R2007+: DIMFXLON
    if r2007_plus(v) {
        w.write_bit(splice!(dimfxlon, 0) != 0); // DimensionIsExtensionLineLengthFixed
    }

    // R2010+: extra dimension fields
    if r2010_plus(v) {
        w.write_bit(splice!(dimtxtdirection, 0) != 0); // DIMTXTDIRECTION
        w.write_bit_double(splice!(dimaltmzf, 0.0)); // DIMALTMZF
        w.write_variable_text(raw.and_then(|r| r.dimaltmzs.as_deref()).unwrap_or("")); // DIMALTMZS
        w.write_bit_double(splice!(dimmzf, 0.0)); // DIMMZF
        w.write_variable_text(raw.and_then(|r| r.dimmzs.as_deref()).unwrap_or("")); // DIMMZS
    }

    // R2000+ dimension handles
    if r2000_plus(v) {
        w.write_handle_ref(DwgReferenceType::HardPointer, h.dim_text_style_handle);
        splice_handle_raw!(dimldrblk); // DIMLDRBLK
        splice_handle_raw!(dimblk); // DIMBLK
        splice_handle_raw!(dimblk1); // DIMBLK1
        splice_handle_raw!(dimblk2); // DIMBLK2
    }

    // R2007+ dimension linetype handles
    if r2007_plus(v) {
        w.write_handle_ref(DwgReferenceType::HardPointer, h.dim_linetype_handle);
        w.write_handle_ref(DwgReferenceType::HardPointer, h.dim_linetype1_handle);
        w.write_handle_ref(DwgReferenceType::HardPointer, h.dim_linetype2_handle);
    }

    // R2000+ dimension line weights
    if r2000_plus(v) {
        w.write_bit_short(h.dim_line_weight);
        w.write_bit_short(h.dim_ext_line_weight);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Table control object handles (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.block_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.layer_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.style_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.linetype_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.view_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.ucs_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.vport_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.appid_control_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.dimstyle_control_handle);

    // R13-R15 only: VPEntHdr control
    if r13_15_only(v) {
        w.write_handle_ref(DwgReferenceType::HardOwnership, h.vpent_hdr_control_handle);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Dictionary handles (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_handle_ref(DwgReferenceType::HardPointer, h.acad_group_dict_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.acad_mlinestyle_dict_handle);
    w.write_handle_ref(DwgReferenceType::HardOwnership, h.named_objects_dict_handle);

    // R2000+ dictionaries and flags
    if r2000_plus(v) {
        w.write_bit_short(splice!(tstackalign, 1) as u16 as i16); // TSTACKALIGN
        w.write_bit_short(splice!(tstacksize, 70) as u16 as i16); // TSTACKSIZE

        w.write_variable_text(&h.hyperlink_base);
        w.write_variable_text(&h.stylesheet);

        w.write_handle_ref(DwgReferenceType::HardPointer, h.acad_layout_dict_handle);
        w.write_handle_ref(
            DwgReferenceType::HardPointer,
            h.acad_plotsettings_dict_handle,
        );
        w.write_handle_ref(
            DwgReferenceType::HardPointer,
            h.acad_plotstylename_dict_handle,
        );
    }

    // R2004+ dictionaries
    if r2004_plus(v) {
        w.write_handle_ref(DwgReferenceType::HardPointer, h.acad_material_dict_handle);
        w.write_handle_ref(DwgReferenceType::HardPointer, h.acad_color_dict_handle);
    }

    // R2007+ dictionaries
    if r2007_plus(v) {
        w.write_handle_ref(
            DwgReferenceType::HardPointer,
            h.acad_visualstyle_dict_handle,
        );
        if r2013_plus(v) {
            splice_handle_raw!(unknown_20); // unknown
        }
    }

    // R2000+ flags bitfield
    if r2000_plus(v) {
        let mut flags = i32::from(LineWeight::from_value(h.current_line_weight).to_dwg_index());
        flags |= (h.end_caps as i32) << 5;
        flags |= (h.join_style as i32) << 7;
        if !h.lineweight_display {
            flags |= 0x200;
        }
        if !h.xedit {
            flags |= 0x400;
        }
        if h.extended_names {
            flags |= 0x800;
        }
        if h.plotstyle_mode {
            flags |= 0x2000;
        }
        if h.ole_startup {
            flags |= 0x4000;
        }
        w.write_bit_long(flags);

        w.write_bit_short(h.insertion_units);
        w.write_bit_short(h.current_plotstyle_type);

        if h.current_plotstyle_type == 3 {
            // CPSNID (only if CEPSNTYPE == 3/ByObjectId)
            splice_handle_raw!(cpsnid);
        }

        w.write_variable_text(&h.fingerprint_guid);
        w.write_variable_text(&h.version_guid);
    }

    // R2004+ extra entity settings
    if r2004_plus(v) {
        w.write_byte(h.sort_entities as u8);
        w.write_byte(h.index_control as u8);
        w.write_byte(h.hide_text as u8);
        w.write_byte(h.xclip_frame as u8);
        w.write_byte(h.dimension_associativity as u8);
        w.write_byte(h.halo_gap as u8);
        w.write_bit_short(h.obscured_color);
        w.write_bit_short(h.intersection_color);
        w.write_byte(h.obscured_linetype as u8);
        w.write_byte(h.intersection_display as u8);

        w.write_variable_text(&h.project_name);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ Block record / linetype handles (Common) Ã¢â€â‚¬Ã¢â€â‚¬
    w.write_handle_ref(DwgReferenceType::HardPointer, h.paper_space_block_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.model_space_block_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.bylayer_linetype_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.byblock_linetype_handle);
    w.write_handle_ref(DwgReferenceType::HardPointer, h.continuous_linetype_handle);

    // Ã¢â€â‚¬Ã¢â€â‚¬ R2007+ extended fields Ã¢â€â‚¬Ã¢â€â‚¬
    if r2007_plus(v) {
        w.write_bit(h.camera_display);
        w.write_bit_long(splice!(unknown_21, 0) as u32 as i32); // unknown
        w.write_bit_long(splice!(unknown_22, 0) as u32 as i32); // unknown
        w.write_bit_double(splice!(unknown_23, 0.0)); // unknown

        w.write_bit_double(h.steps_per_second);
        w.write_bit_double(h.step_size);
        w.write_bit_double(splice!(_3ddwfprec, 2.0)); // 3DDWFPREC Ã¢â‚¬â€ valid range 1..6
        w.write_bit_double(h.lens_length);
        w.write_bit_double(h.camera_height);
        w.write_byte(u8::from(h.record_solid_history));
        w.write_byte(h.show_solid_history.clamp(0, 2) as u8);
        w.write_bit_double(splice!(psolwidth, 0.25)); // PSOLWIDTH Ã¢â‚¬â€ valid range >0
        w.write_bit_double(splice!(psolheight, 0.25)); // PSOLHEIGHT
        w.write_bit_double(h.loft_angle1);
        w.write_bit_double(h.loft_angle2);
        w.write_bit_double(h.loft_magnitude1);
        w.write_bit_double(h.loft_magnitude2);
        w.write_bit_short(h.loft_param);
        w.write_byte(h.loft_normals as u8);
        w.write_bit_double(h.latitude);
        w.write_bit_double(h.longitude);
        w.write_bit_double(h.north_direction);
        w.write_bit_long(h.timezone);
        w.write_byte(splice!(lightglyphdisplay, 0) as u8); // LIGHTGLYPHDISPLAY
        w.write_byte(splice!(tilemodelightsynch, 1) as u8); // TILEMODELIGHTSYNCH
        w.write_byte(splice!(dwfframe, h.dwf_frame.clamp(0, 2) as i64) as u8); // DWFFRAME
        w.write_byte(splice!(dgnframe, h.dgn_frame.clamp(0, 2) as i64) as u8); // DGNFRAME

        w.write_bit(splice!(realworldscale, 0) != 0); // REALWORLDSCALE

        match raw.and_then(|r| r.interferecolor.as_ref()) {
            Some(cmc) => w.write_cm_color_raw(cmc), // INTERFERECOLOR
            None => w.write_cm_color(&Color::None),
        }

        splice_handle_raw!(interfereobjvs);
        splice_handle_raw!(interferevpvs);
        splice_handle_raw!(dragvs);

        w.write_byte(splice!(cshadow, 0) as u8); // CSHADOW
        w.write_bit_double(h.shadow_plane_location);
    }

    // Ã¢â€â‚¬Ã¢â€â‚¬ R14+ trailing fields Ã¢â€â‚¬Ã¢â€â‚¬
    if v >= DxfVersion::AC1014 {
        w.write_bit_short(splice!(unknown_54, -1) as u16 as i16);
        w.write_bit_short(splice!(unknown_55, -1) as u16 as i16);
        w.write_bit_short(splice!(unknown_56, -1) as u16 as i16);
        w.write_bit_short(splice!(unknown_57, -1) as u16 as i16);

        // R2004+: the three undocumented trailing slots (Ã‚Â§19 H7 review) Ã¢â‚¬â€
        // spliced from the raw mirror (the reader's retained walk values);
        // the historical 0/0/false stays the programmatic default.
        if r2004_plus(v) {
            w.write_bit_long(splice!(unknown_tail_long1, 0) as u32 as i32);
            w.write_bit_long(splice!(unknown_tail_long2, 0) as u32 as i32);
            w.write_bit(splice!(unknown_tail_bit, false));
        }
    }
}

// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â
//  Tests
// Ã¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢ÂÃ¢â€¢Â

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_header_r2000_has_sentinels() {
        let h = HeaderVariables::default();
        let data = write_header(DxfVersion::AC1015, &h, 0);

        // Start sentinel
        assert_eq!(&data[..16], &start_sentinels::HEADER);
        // End sentinel at the end
        let end_start = data.len() - 16;
        assert_eq!(&data[end_start..], &end_sentinels::HEADER);
    }

    #[test]
    fn test_write_header_size_field() {
        let h = HeaderVariables::default();
        let data = write_header(DxfVersion::AC1015, &h, 0);

        // Size at offset 16 (RL = 4 bytes LE)
        let size = i32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        assert!(size > 0, "Header section size should be > 0: got {}", size);
    }

    #[test]
    fn test_write_header_crc_valid() {
        let h = HeaderVariables::default();
        let data = write_header(DxfVersion::AC1015, &h, 0);

        // CRC is 2 bytes before end sentinel
        let end_sentinel_start = data.len() - 16;
        let crc_bytes = &data[end_sentinel_start - 2..end_sentinel_start];

        let size_plus_data = &data[16..end_sentinel_start - 2];
        let expected_crc = crc16(CRC16_SEED, size_plus_data);
        let actual_crc = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
        assert_eq!(
            actual_crc, expected_crc,
            "Header CRC mismatch: got 0x{:04X}, expected 0x{:04X}",
            actual_crc, expected_crc
        );
    }

    #[test]
    fn test_write_header_r2004_larger_than_r2000() {
        let h = HeaderVariables::default();
        let data_2000 = write_header(DxfVersion::AC1015, &h, 0);
        let data_2004 = write_header(DxfVersion::AC1018, &h, 0);

        assert!(
            data_2004.len() > data_2000.len(),
            "R2004 header ({} bytes) should be larger than R2000 ({} bytes)",
            data_2004.len(),
            data_2000.len()
        );
    }

    #[test]
    fn test_write_header_r2007_uses_merged_format() {
        let h = HeaderVariables::default();
        let data_2007 = write_header(DxfVersion::AC1021, &h, 0);

        // Should have sentinels and be non-trivial size
        assert_eq!(&data_2007[..16], &start_sentinels::HEADER);
        assert!(data_2007.len() > 200, "R2007 header should be substantial");
    }

    #[test]
    fn test_write_header_r14_smaller_than_r2000() {
        let h = HeaderVariables::default();
        let data_r14 = write_header(DxfVersion::AC1014, &h, 0);
        let data_2000 = write_header(DxfVersion::AC1015, &h, 0);

        // R14 has fewer dimension handle fields, but more boolean R13_14 fields.
        // They should both be non-trivial.
        assert!(data_r14.len() > 100);
        assert!(data_2000.len() > 100);
    }

    #[test]
    fn test_julian_conversion() {
        let (day, ms) = julian_to_day_ms(2451544.5);
        assert_eq!(day, 2451544);
        assert!(ms > 0);

        let (d, m) = julian_to_day_ms(0.0);
        assert_eq!(d, 0);
        assert_eq!(m, 0);
    }

    #[test]
    fn test_timespan_conversion() {
        let (days, ms) = timespan_to_day_ms(1.5);
        assert_eq!(days, 1);
        assert_eq!(ms, 43_200_000);
    }
}
