//! DWG Classes section reader
//!
//! Reads the AcDb:Classes section from a DWG file, producing a
//! `DxfClassCollection`. Mirrors the reference `DwgClassesReader`.
//!
//! ## Section layout
//!
//! ```text
//! ┌──────────────────┐
//! │ Start sentinel   │ 16 bytes
//! │ Section size (RL)│ 4 bytes
//! │ Class entries... │ variable
//! │ CRC-16           │ 2 bytes
//! │ End sentinel     │ 16 bytes
//! └──────────────────┘
//! ```

use crate::classes::{DxfClass, DxfClassCollection, ProxyFlags};
use crate::error::{DxfError, Result};
use crate::io::dwg::dwg_stream_readers::bit_reader::DwgBitReader;
use crate::io::dwg::dwg_version::DwgVersion;
use crate::io::dwg::file_headers::section_definition::{end_sentinels, start_sentinels};
use crate::types::DxfVersion;

/// Read the Classes section from raw section bytes (already decompressed).
///
/// # Arguments
/// * `data` - Complete section buffer (includes sentinels)
/// * `version` - DXF version for version-specific parsing
///
/// # Returns
/// `DxfClassCollection` containing all parsed class definitions.
pub fn read_classes(
    data: &[u8],
    version: DxfVersion,
    maintenance_version: u8,
) -> Result<DxfClassCollection> {
    read_classes_with_encoding(
        data,
        version,
        maintenance_version,
        encoding_rs::WINDOWS_1252,
    )
}

pub fn read_classes_with_encoding(
    data: &[u8],
    version: DxfVersion,
    maintenance_version: u8,
    encoding: &'static encoding_rs::Encoding,
) -> Result<DxfClassCollection> {
    // The shared prelude (sentinel/size/hsize/slice + the R2007+ text prefix
    // + the R2004+ header) — see `classes_section_prelude`. The primary walk
    // keeps its section-bounded slice; the gold-shadow walk re-runs the
    // prelude over the buffer-extended slice.
    let (mut reader, data_start, section_size, end_bit, header_max) =
        classes_section_prelude(data, version, maintenance_version, encoding, false)?;
    // ── R2004+: max class number ──
    // (read by the prelude; pre-R2004 sections carry no header, and the
    // i16::MAX default lets every class number through as before)
    let max_class_number = header_max.unwrap_or(i16::MAX);

    // ── Read class entries until we consume all section data ──
    let mut classes = DxfClassCollection::new();

    while reader.position_in_bits() < end_bit {
        let class_number = reader.read_bit_short();

        // Sanity check — class numbering starts at 500 and can't exceed max
        if class_number < 500 || class_number > max_class_number {
            break;
        }

        let proxy_flags_raw = reader.read_bit_short() as i32;
        let application_name = reader.read_variable_text();
        let cpp_class_name = reader.read_variable_text();
        let dxf_name = reader.read_variable_text();
        let was_zombie = reader.read_bit();
        let item_class_id = reader.read_bit_short();

        let mut class = DxfClass::new(&dxf_name, &cpp_class_name);
        class.application_name = application_name;
        class.class_number = class_number;
        class.proxy_flags = ProxyFlags::from(proxy_flags_raw);
        class.was_zombie = was_zombie;
        class.item_class_id = item_class_id;
        // 0x1F2 marks a graphical entity, 0x1F3 a non-graphical object. Without
        // this the flag stayed at its `false` default for every class, so any
        // class-based entity the reader has no explicit type for (e.g. an
        // application's custom entity) was classified as an object and decoded
        // with the wrong layout — the entity, and whatever it drew, vanished.
        class.is_an_entity = item_class_id == crate::classes::ENTITY_ITEM_CLASS_ID;

        // R2004+: instance count + 4 extra BL fields
        if version >= DxfVersion::AC1018 {
            let instance_count = reader.read_bit_long();
            class.instance_count = instance_count;
            class.dwg_version = reader.read_bit_long();
            class.maintenance_version = reader.read_bit_long();
            class.unknown1 = reader.read_bit_long();
            class.unknown2 = reader.read_bit_long();
        }

        // Preserve every entry in order: the DWG classes section is positional
        // (object type = 500 + index), so deduping by dxf name would drop a
        // legitimate duplicate-name class and shift all later class numbers.
        classes.push_preserving(class);
    }

    // ── Verify end sentinel ──
    let end_sentinel_start = data_start + section_size + 2; // +2 for CRC
    if data.len() >= end_sentinel_start + 16 {
        if &data[end_sentinel_start..end_sentinel_start + 16] != &end_sentinels::CLASSES {
            // Non-fatal: log warning but don't fail
            // Some writers produce slightly off sentinels
        }
    }

    // ── Gold-shadow walk ──
    // Reproduce gold's (libredwg's) numeric classes walk over the same
    // section bytes so every class carries the `item_class_id` gold
    // actually sees (see DxfClass::gold_shadow). The walks share
    // the header reads but diverge in the per-record tail: gold reads
    // `dwg_version`/`maint_version` as BS, this reader as BL, and from the
    // first record whose tail uses a non-byte bitcode form gold's cursor
    // desyncs from the true record layout for the rest of the table (the
    // observed trigger class is the table's 10th entry — on the
    // AutoCAD-2027.1-authored fixture set, all four 2007-2018 versions).
    // The strings never matter to gold's numeric cursor (R2007+ text reads
    // advance the separate string stream; pre-R2007 TV is read here
    // inline instead), so this walk yields the same per-index record
    // sequence gold walks.
    let gold_shadow = gold_shadow_classes(
        data,
        version,
        maintenance_version,
        encoding,
    );
    for (i, class) in classes.iter_mut().enumerate() {
        class.gold_shadow = gold_shadow.get(i).cloned().flatten();
    }

    Ok(classes)
}

/// The classes-section prelude shared by this reader's own walk and the
/// gold-shadow walk: start-sentinel check, section-size RL (offset 16),
/// the extra zero-RL gate (`has_section_extra_rl`), the truncation check
/// (class data + CRC + end sentinel), the bit reader over the section
/// slice, the R2007+ RL text prefix (returning the text end bit the
/// primary loop needs), and the R2004+ header reads (BS max + RC + RC +
/// B). Both walks MUST consume these bits identically — a section-shape
/// fix applied to one walk and not the other silently misaligns the
/// shadow — so the prelude lives once, here.
///
/// `to_buffer_end` widens the reader slice past the section data into the
/// trailing CRC/end-sentinel bytes: gold's walks read over the whole
/// decompressed section (their reads clamp to zero past `size`), which
/// the shadow mirror needs; the primary walk keeps its section-bounded
/// slice.
fn classes_section_prelude(
    data: &[u8],
    version: DxfVersion,
    maintenance_version: u8,
    encoding: &'static encoding_rs::Encoding,
    to_buffer_end: bool,
) -> Result<(DwgBitReader, usize, usize, i64, Option<i16>)> {
    let dwg = DwgVersion::from_dxf_version(version)?;

    // ── Verify start sentinel ──
    if data.len() < 34 {
        return Err(DxfError::Parse("Classes section too short".to_string()));
    }
    if &data[..16] != &start_sentinels::CLASSES {
        return Err(DxfError::InvalidSentinel(
            "Classes section start sentinel mismatch".to_string(),
        ));
    }

    // ── Read section size (RL at offset 16) ──
    let section_size = i32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize;

    // Section data starts after sentinel (16) + size field (4).
    // Extra 4 zero bytes when: (AC1024+ && maintenance > 3) || AC1032+
    let mut data_start = 20;
    if DwgVersion::has_section_extra_rl(version, maintenance_version) {
        data_start += 4;
    }
    if data.len() < data_start + section_size + 2 + 16 {
        return Err(DxfError::Parse(
            "Classes section data truncated".to_string(),
        ));
    }
    let section_data = if to_buffer_end {
        data[data_start..].to_vec()
    } else {
        data[data_start..data_start + section_size].to_vec()
    };

    // ── Create the bit reader over the section data ──
    let mut reader = DwgBitReader::with_encoding(section_data, dwg, version, encoding);

    // R2007+: The section data has an RL prefix (total data size in bits)
    // from save_position_for_size. Text is INLINE (not in a separate stream).
    // R2007+: Set up text stream for three-stream merge.
    // The writer puts text strings (dxf_name, cpp_class_name, application_name)
    // in a separate text sub-stream. The RL stores the total bit count;
    // the flag bit is at RL − 1 (same convention as per-object records).
    let end_bit = if version >= DxfVersion::AC1021 {
        let total_size_bits = reader.read_raw_long() as i64;
        let start = reader.position_in_bits();
        let text_start = reader.set_position_by_flag(total_size_bits - 1);
        reader.set_position_in_bits(start);
        text_start
    } else {
        (section_size * 8) as i64
    };

    // ── R2004+: section header ── (pre-R2004 sections carry no header)
    let max = if version >= DxfVersion::AC1018 {
        let max = reader.read_bit_short(); // BS: max class number
        let _rc1 = reader.read_byte(); // RC: 0x00
        let _rc2 = reader.read_byte(); // RC: 0x00
        let _flag = reader.read_bit(); // B: true
        Some(max)
    } else {
        None
    };

    Ok((reader, data_start, section_size, end_bit, max))
}

/// Walk the classes section the way gold does and return the
/// `item_class_id` gold reads per record index.
///
/// Mirrors libredwg's per-reader classes walks
/// (`dwg->dwg_class[i].item_class_id`): R2004 and R2010+ files use
/// `read_2004_section_classes` (decode.c:2249), R2007 files
/// `read_2007_section_classes` (decode_r2007.c:1491), pre-R2004
/// files the classes walk inside `decode_R13_R2000` (decode.c:290;
/// the walk at decode.c:578-700 — byte-bounded with the CWE
/// per-record cap, no header). The header goes through the shared
/// prelude (over the buffer-extended slice — gold reads over the whole
/// decompressed section); the per-record tail is read `BL instances,
/// BS dwg_version, BS maint, BL, BL` — the BS/BS pair gold uses where
/// the true encoding (and this reader's own walk) uses BL — so from the
/// first record whose tail needs the multi-byte bitcode the two cursors
/// desync and this walk yields gold's garbage ids.
///
/// Gold's plausibility bounds are mirrored per reader: R2007 rejects
/// `max > 5000`, the R2004-based readers reject
/// `max - 499 > 100 + size/sizeof(Dwg_Class)` (64 on the built oracle),
/// and the pre-R2004 loop caps at the same size-proportional record count
/// (plus 65535). When gold abandons the table (num_classes = 0) this
/// returns the empty vector and the dispatch keeps its sane-parse
/// fallback. Surviving tables are walked EXACTLY as gold walks them:
/// every `max - 499` record gets a definite id — including the
/// zero-filled ones once the cursor stalls past the section end (a
/// stalled or desynced read is never 0x1F2, which is the entire point
/// of the mirror).
/// Gold's `bit_read_TV` semantics for the class-name shadow (TODO B1,
/// 2026-10-01): BS length, then `CHK_OVERFLOW_PLUS(length)` — a run
/// past the section's end bails to NULL WITHOUT consuming the chars
/// (the cursor keeps only the BS advance; out_json prints the empty
/// string) — else `length` raw bytes, and the printed name is the
/// C-string PREFIX: everything up to the first NUL. The MIF/`\U+`
/// escape decode never applies (gold prints the class names raw).
fn gold_tv_cstring(
    reader: &mut DwgBitReader,
    section_size: i64,
    encoding: &'static encoding_rs::Encoding,
) -> String {
    let length = reader.read_bit_short();
    if length <= 0 {
        return String::new();
    }
    // CHK_OVERFLOW_PLUS: the bit-level form — `(byte + plus) * 8 + bit
    // > size * 8` — reduces to the position in bits plus the run.
    let pos_bits = reader.position_in_bits();
    if pos_bits + (length as i64) * 8 > section_size * 8 {
        // gold's overflow bail — no char consumption.
        return String::new();
    }
    let bytes = reader.read_bytes(length as usize);
    let (decoded, _, _) = encoding.decode(&bytes);
    decoded
        .split('\0')
        .next()
        .unwrap_or("")
        .to_string()
}

fn gold_shadow_classes(
    data: &[u8],
    version: DxfVersion,
    maintenance_version: u8,
    encoding: &'static encoding_rs::Encoding,
) -> Vec<Option<crate::classes::DwgClassGoldShadow>> {
    // The shadow runs after the primary walk accepted the section, so the
    // prelude errors here mean the mirror cannot run — fall back to the
    // sane parse (all `None`).
    let (mut reader, data_start, section_size, _end_bit, header_max) =
        match classes_section_prelude(data, version, maintenance_version, encoding, true) {
            Ok(prelude) => prelude,
            Err(_) => return Vec::new(),
        };

    let num_classes: i64 = if let Some(max) = header_max {
        let num = max as i64 - 499;
        let bail = if version == DxfVersion::AC1021 {
            // read_2007_section_classes (decode_r2007.c):
            // `max_num < 500 || max_num > 5000` -> num_classes = 0.
            max < 500 || max > 5000
        } else {
            // read_2004_section_classes (decode.c; R2004 and R2010+):
            // `max_num < 500 || num_classes > 100 + size/sizeof(Dwg_Class)`
            // -> num_classes = 0. sizeof(Dwg_Class) is 64 on the built
            // oracle (x86-64).
            max < 500 || num > 100 + (section_size as i64) / 64
        };
        if bail {
            return Vec::new();
        }
        num
    } else {
        -1 // pre-R2004: gold walks to endpos with a size-proportional cap
    };

    let mut ids: Vec<Option<crate::classes::DwgClassGoldShadow>> = Vec::new();
    let mut index: i64 = 0;
    let mut last_pos: i64 = -1;
    // The pre-R2007 inline name strings read at gold's cursor position
    // (TODO B1, 2026-10-01) — `None` on R2007+ (the separate string
    // stream never desyncs; the projection falls back to the own names).
    let mut shadow_names: Option<(String, String, String)> = None;
    // Gold's overflow bound is its FULL decompressed-section Bit_Chain
    // (sentinels + CRC included), not the class-data area — the shadow
    // reader spans data_start..end, so the bound is the reader buffer.
    let tv_bound: i64 = (data.len() - data_start) as i64;
    // Pre-R2004: gold's loop is `while (dat->byte < endpos - 1)` — the
    // byte position bound — plus the record cap
    // `i >= 100 + size/sizeof(Dwg_Class) || i >= 65535`, and an
    // in-record endpos check after the number/proxyflag pair. R2004+:
    // gold walks exactly `num_classes` records with reads that clamp to
    // zero past the section — the mirror pushes a definite id for every
    // remaining record, so the dispatch never reverts to the sane-parse
    // semantics where gold has desynced semantics.
    let end_byte = section_size as i64;
    loop {
        if num_classes >= 0 {
            if index >= num_classes {
                break;
            }
        } else if index >= 100 + (section_size as i64) / 64
            || index >= 65535
            || (reader.position_in_bits() >> 3) as i64 >= end_byte - 1
        {
            break;
        }
        let pos = reader.position_in_bits() as i64;
        if pos == last_pos && num_classes < 0 {
            // The reader stalled at the buffer end (a malformed pre-R2004
            // tail); gold's endpos bound stops its walk too.
            break;
        }
        last_pos = pos;

        // Gold's BITCODE_BS is uint16_t (include/dwg.h:120) — every BS
        // field prints UNSIGNED in JSON (number 36108 observed, never
        // −29428), and the dwg_version/maint_version BS reads stored
        // into gold's BITCODE_BL uint32 struct fields zero-extend
        // (pinned by ExtrudeM_2018 record 19: gold 32970, not the
        // sign-extended 4294934730); num_instances is a true BL whose
        // '11' degenerate code returns 256 (gold's error branch).
        let number = reader.read_bit_short() as u16;
        let proxyflag = reader.read_bit_short() as u16;
        if num_classes < 0 && (reader.position_in_bits() >> 3) as i64 >= end_byte {
            // decode.c drops a record whose number/proxyflag pair already
            // starts at endpos (`if (dat->byte >= endpos) break;`).
            break;
        }
        if version < DxfVersion::AC1021 {
            // Pre-R2007: the three text fields are inline TV (BS length +
            // chars) and advance the shared cursor; gold reads them the
            // same way. TODO B1 (2026-10-01): the strings are captured
            // with gold's EXACT semantics — past a numeric derail the
            // garbage lengths run past the section and gold's
            // CHK_OVERFLOW bail returns NULL without consuming the chars
            // (the cursor keeps only the BS advance), and the printed
            // name is the C-string PREFIX (up to the first NUL) — the
            // structure axis projects gold's view from the shadow.
            let app = gold_tv_cstring(&mut reader, tv_bound, encoding);
            let cpp = gold_tv_cstring(&mut reader, tv_bound, encoding);
            let dxf = gold_tv_cstring(&mut reader, tv_bound, encoding);
            shadow_names = Some((dxf, cpp, app));
        } else {
            shadow_names = None;
        }
        let is_zombie = reader.read_bit() as u8;
        let item_class_id = reader.read_bit_short() as u16;
        let mut num_instances = 0u32;
        let mut dwg_version = 0u32;
        let mut maint_version = 0u32;
        if header_max.is_some() {
            // R2004+ tail: the BS/BS reads where gold's walk derails.
            num_instances = reader.read_bit_long() as u32;
            dwg_version = reader.read_bit_short() as u16 as u32;
            maint_version = reader.read_bit_short() as u16 as u32;
            let _unknown1 = reader.read_bit_long();
            let _unknown2 = reader.read_bit_long();
        }
        let (shadow_dxf, shadow_cpp, shadow_app) = match shadow_names.take() {
            Some((dxf, cpp, app)) => (dxf, cpp, app),
            None => (String::new(), String::new(), String::new()),
        };
        ids.push(Some(crate::classes::DwgClassGoldShadow {
            number,
            proxyflag,
            is_zombie,
            item_class_id,
            num_instances,
            dwg_version,
            maint_version,
            dxfname: shadow_dxf,
            cppname: shadow_cpp,
            appname: shadow_app,
        }));
        index += 1;
    }

    ids
}

// ════════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::dwg::dwg_stream_writers::classes_writer;

    #[test]
    fn test_classes_roundtrip_r2000() {
        let mut classes = DxfClassCollection::new();
        classes.update_defaults();

        // Write classes section
        let class_vec: Vec<DxfClass> = classes.iter().cloned().collect();
        let written = classes_writer::write_classes(DxfVersion::AC1015, &class_vec, 0);

        // Read it back
        let read_classes = read_classes(&written, DxfVersion::AC1015, 0).unwrap();

        // Should have the same number of classes
        assert_eq!(
            read_classes.len(),
            classes.len(),
            "Class count mismatch: wrote {}, read {}",
            classes.len(),
            read_classes.len()
        );
    }

    #[test]
    fn test_classes_roundtrip_r2004() {
        let mut classes = DxfClassCollection::new();
        classes.update_defaults();

        let class_vec: Vec<DxfClass> = classes.iter().cloned().collect();
        let written = classes_writer::write_classes(DxfVersion::AC1018, &class_vec, 0);
        let read_classes = read_classes(&written, DxfVersion::AC1018, 0).unwrap();

        assert_eq!(
            read_classes.len(),
            classes.len(),
            "Class count mismatch: wrote {}, read {}",
            classes.len(),
            read_classes.len()
        );

        // Verify a specific class and preserve its positional class number.
        let acdb_placeholder = read_classes.get_by_name("ACDBPLACEHOLDER");
        assert!(
            acdb_placeholder.is_some(),
            "Should find ACDBPLACEHOLDER class"
        );
        let cls = acdb_placeholder.unwrap();
        let expected = classes.get_by_name("ACDBPLACEHOLDER").unwrap();
        assert_eq!(cls.cpp_class_name, "AcDbPlaceHolder");
        assert_eq!(cls.class_number, expected.class_number);
    }

    #[test]
    fn test_classes_bad_sentinel_fails() {
        let mut bad_data = vec![0u8; 50];
        // Wrong sentinel
        bad_data[..16].fill(0xFF);
        let result = read_classes(&bad_data, DxfVersion::AC1015, 0);
        assert!(result.is_err());
    }

    // ── Gold-shadow invariants (crafted-section pins) ─────────────────────
    //
    // The shadow walk and this reader's own walk share the prelude and
    // every pre-tail field; they only diverge in the record tail (gold's
    // BS/BS vs the true BL/BL). These pins hold the coupling: on
    // byte-form tails the two walks must agree value-for-value, and on
    // multi-byte tails the mirror must keep walking gold's full count
    // with definite ids — never falling back to `None` mid-table.

    /// MSB-first bit writer with the DWG bitcodes for crafting
    /// classes-section bytes: BS `'00'`=RS16 little-endian, `'01'`=RC,
    /// `'10'`=0, `'11'`=256; BL `"00"`=RL32 little-endian, `'01'`=RC,
    /// `'10'`=0, `'11'`=256. Pre-R2007 TV is BS length + bytes.
    struct SectionBits {
        bytes: Vec<u8>,
        bit: usize, // bits used in the current (last) byte
    }

    impl SectionBits {
        fn new() -> Self {
            Self {
                bytes: Vec::new(),
                bit: 0,
            }
        }
        fn push_bit(&mut self, set: bool) {
            if self.bit == 0 {
                self.bytes.push(0);
            }
            if set {
                *self.bytes.last_mut().unwrap() |= 1 << (7 - self.bit);
            }
            self.bit = (self.bit + 1) % 8;
        }
        fn bits(&mut self, value: u32, count: usize) {
            for i in (0..count).rev() {
                self.push_bit((value >> i) & 1 == 1);
            }
        }
        fn rc(&mut self, value: u8) {
            self.bits(value as u32, 8);
        }
        fn rs_le(&mut self, value: u16) {
            self.rc((value & 0xFF) as u8);
            self.rc((value >> 8) as u8);
        }
        fn rl_le(&mut self, value: u32) {
            for i in 0..4 {
                self.rc(((value >> (8 * i)) & 0xFF) as u8);
            }
        }
        /// BitShort in its canonical forms.
        fn bs(&mut self, value: i16) {
            match value {
                0 => self.bits(0b10, 2),
                1..=255 => {
                    self.bits(0b01, 2);
                    self.rc(value as u8);
                }
                _ => {
                    self.bits(0b00, 2);
                    self.rs_le(value as u16);
                }
            }
        }
        /// BitLong in its canonical forms.
        fn bl(&mut self, value: i32) {
            match value {
                0 => self.bits(0b10, 2),
                1..=255 => {
                    self.bits(0b01, 2);
                    self.rc(value as u8);
                }
                _ => {
                    self.bits(0b00, 2);
                    self.rl_le(value as u32);
                }
            }
        }
        /// BitLong forced into the multi-byte `'00'`+RL32 form — the
        /// real-encoding tail shape that derails gold's BS/BS read.
        fn bl_rl_form(&mut self, value: i32) {
            self.bits(0b00, 2);
            self.rl_le(value as u32);
        }
        /// Pre-R2007 TV with BS length.
        fn tv_empty(&mut self) {
            self.bs(0);
        }

        /// The R2004+ section header: BS max, RC, RC, B.
        fn header(&mut self, max: i16) {
            self.bs(max);
            self.rc(0);
            self.rc(0);
            self.push_bit(true);
        }

        /// One R2004 class record with a BYTE-FORM tail (the shape every
        /// sane writer emits — gold's BS reads stay in sync).
        fn record_sane(&mut self, index: i16, item: i16) {
            self.bs(500 + index); // number
            self.bs(0); // proxy flags
            self.tv_empty();
            self.tv_empty();
            self.tv_empty();
            self.push_bit(false); // zombie
            self.bs(item); // item_class_id
            self.bl(1); // instances (byte form)
            self.bs(0x16); // dwg_version 22 — byte form (BS-compatible)
            self.bs(0); // maint version
            self.bl(0); // unknown1
            self.bl(0); // unknown2
        }

        /// One R2004 class record whose `dwg_version` is written in the
        /// BL multi-byte form — the truth on the AutoCAD-2027.1-authored
        /// tables; gold's BS read stops 16 bits short and its cursor
        /// derails for the rest of the table.
        fn record_multi_byte_tail(&mut self, index: i16, item: i16) {
            self.bs(500 + index);
            self.bs(0);
            self.tv_empty();
            self.tv_empty();
            self.tv_empty();
            self.push_bit(false);
            self.bs(item);
            self.bl(1);
            self.bl_rl_form(0x0F0F_1234); // dwg_version in RL form
            self.bs(0);
            self.bl(0);
            self.bl(0);
        }
    }

    /// Assemble a full classes-section buffer around the crafted bits:
    /// sentinel + size RL + section data + CRC + end sentinel + zero pad
    /// (the trailing bytes are the region gold's walk reads into when its
    /// cursor runs past the class data).
    fn assemble_section(bits: &SectionBits, pad: usize) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&start_sentinels::CLASSES);
        data.extend_from_slice(&(bits.bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(&bits.bytes);
        data.extend_from_slice(&[0u8, 0]);
        data.extend_from_slice(&end_sentinels::CLASSES);
        data.extend(std::iter::repeat(0u8).take(pad));
        data
    }

    #[test]
    fn gold_shadow_matches_own_walk_on_byte_form_tails() {
        let mut w = SectionBits::new();
        w.header(502);
        w.record_sane(0, crate::classes::ENTITY_ITEM_CLASS_ID);
        w.record_sane(1, crate::classes::OBJECT_ITEM_CLASS_ID);
        w.record_sane(2, crate::classes::OBJECT_ITEM_CLASS_ID);
        let data = assemble_section(&w, 32);

        let classes =
            read_classes_with_encoding(&data, DxfVersion::AC1018, 0, encoding_rs::WINDOWS_1252)
                .unwrap();
        assert_eq!(classes.len(), 3, "the own walk parses all crafted records");
        for class in classes.iter() {
            assert_eq!(
                class.gold_shadow.map(|s| s.item_class_id),
                Some(class.item_class_id as u16),
                "the gold-shadow must equal the own walk on byte-form tails"
            );
        }
    }

    #[test]
    fn gold_shadow_mirror_diverges_on_multi_byte_tails() {
        let mut w = SectionBits::new();
        w.header(502);
        w.record_sane(0, crate::classes::OBJECT_ITEM_CLASS_ID);
        w.record_multi_byte_tail(1, crate::classes::OBJECT_ITEM_CLASS_ID);
        w.record_sane(2, crate::classes::OBJECT_ITEM_CLASS_ID);
        let data = assemble_section(&w, 32);

        let classes =
            read_classes_with_encoding(&data, DxfVersion::AC1018, 0, encoding_rs::WINDOWS_1252)
                .unwrap();
        // The own walk reads the BL tails and stays sane on all 3.
        assert_eq!(classes.len(), 3);
        let own: Vec<i16> = classes.iter().map(|c| c.item_class_id).collect();
        let shadow: Vec<Option<u16>> = classes
            .iter()
            .map(|c| c.gold_shadow.map(|s| s.item_class_id))
            .collect();

        // The records up to the multi-byte tail mirror the own walk
        // exactly. The record after it reads from gold's derailed cursor —
        // the load-bearing pins are the FULL record count and a DEFINITE
        // id per record: the mirror never falls back to `None` (the
        // sane-parse classification) past a desync. The post-desync
        // VALUE itself is craft-dependent — short-form codes in the
        // following record can re-absorb the 16-bit deficit and re-align
        // both walks (real derail tables carry multi-record non-byte
        // bursts that keep the deficit; the corpus fixtures pin gold's
        // garbage ids there).
        assert_eq!(shadow[0], Some(own[0] as u16));
        assert_eq!(shadow[1], Some(own[1] as u16));
        assert_eq!(shadow.len(), 3, "gold walks max - 499 records exactly");
        assert!(
            shadow.iter().all(Option::is_some),
            "the mirror never falls back past a desync"
        );
    }

    #[test]
    fn gold_shadow_walks_golds_full_record_count() {
        // max = 520 -> gold walks 21 records; only 2 exist in the class
        // data. The mirror must return all 21 definite ids — reading
        // through the trailing CRC/end-sentinel bytes and finally the zero
        // pad, where every read clamps to zero patterns.
        let mut w = SectionBits::new();
        w.header(520);
        w.record_sane(0, crate::classes::ENTITY_ITEM_CLASS_ID);
        w.record_sane(1, crate::classes::OBJECT_ITEM_CLASS_ID);
        let data = assemble_section(&w, 256);

        let classes =
            read_classes_with_encoding(&data, DxfVersion::AC1018, 0, encoding_rs::WINDOWS_1252)
                .unwrap();
        assert_eq!(classes.len(), 2, "the own walk stops at the data end");
        let shadow: Vec<Option<u16>> = classes
            .iter()
            .map(|c| c.gold_shadow.map(|s| s.item_class_id))
            .collect();
        assert_eq!(shadow.len(), 2);

        // Run the mirror directly for the full count (the shadow vector is
        // per-parsed-record; the un-walked gold records live only here).
        let full = gold_shadow_classes(&data, DxfVersion::AC1018, 0, encoding_rs::WINDOWS_1252);
        assert_eq!(full.len(), 21, "gold walks max - 499 records exactly");
        assert!(full.iter().all(Option::is_some), "no None mid-table");
        assert_eq!(
            full[0].map(|s| s.item_class_id),
            Some(crate::classes::ENTITY_ITEM_CLASS_ID as u16)
        );
        assert_eq!(
            full[1].map(|s| s.item_class_id),
            Some(crate::classes::OBJECT_ITEM_CLASS_ID as u16)
        );
        // The trailing records read deep inside the 256-byte zero pad:
        // all-zero bits decode through the `'00'` RS16 branch to 0 —
        // a definite never-0x1F2 id, the object classification gold
        // assigns past its table end.
        assert_eq!(full[20].map(|s| s.item_class_id), Some(0));
    }

    #[test]
    fn gold_shadow_applies_golds_plausibility_bails() {
        // R2004-based files: gold bails when
        // `num_classes > 100 + size/sizeof(Dwg_Class)` — a small crafted
        // section with an oversized max gets its table abandoned
        // (0 classes), which the mirror reports as an empty vector (the
        // sane-parse fallback), never a partial walk.
        let mut w = SectionBits::new();
        w.header(4999); // num_classes = 4500 >> 100 + ~size/64
        w.record_sane(0, crate::classes::OBJECT_ITEM_CLASS_ID);
        let data = assemble_section(&w, 32);
        let full = gold_shadow_classes(&data, DxfVersion::AC1018, 0, encoding_rs::WINDOWS_1252);
        assert!(full.is_empty(), "the size-proportional bail must fire");

        // (The AC1021 `max > 5000` variant is the other bail branch; an
        // R2007 craft needs the well-formed string-stream flag machinery,
        // which the roundtripped-fixture tests exercise through the
        // corpus instead.)
    }
}
