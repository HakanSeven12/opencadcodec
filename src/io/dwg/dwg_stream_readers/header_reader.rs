//! DWG Header section reader
//!
//! Reads the AcDb:Header section from a DWG file into `HeaderVariables`.
//! This is the inverse of `header_writer.rs`, reading ~200 fields with
//! extensive version-conditional logic.
//!
//! Based on the reference `DwgHeaderReader`.

use crate::document::HeaderVariables;
use crate::error::{DxfError, Result};
use crate::io::dwg::dwg_stream_readers::bit_reader::DwgBitReader;
use crate::io::dwg::dwg_stream_readers::merged_reader::DwgMergedReader;
use crate::io::dwg::dwg_version::DwgVersion;
use crate::io::dwg::file_headers::section_definition::start_sentinels;
use crate::types::{DxfVersion, Handle, LineWeight};

// ════════════════════════════════════════════════════════════════════════════
//  Version-range helpers (same as header_writer)
// ════════════════════════════════════════════════════════════════════════════

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

// ════════════════════════════════════════════════════════════════════════════
//  Julian date helpers
// ════════════════════════════════════════════════════════════════════════════

fn day_ms_to_julian(day: i32, ms: i32) -> f64 {
    day as f64 + (ms as f64 / 86_400_000.0)
}

fn day_ms_to_timespan(days: i32, ms: i32) -> f64 {
    days as f64 + (ms as f64 / 86_400_000.0)
}

// ════════════════════════════════════════════════════════════════════════════
//  Reader abstraction (pre-R2007 = inline, R2007+ = three-stream merge)
// ════════════════════════════════════════════════════════════════════════════

/// Abstraction over single-stream (pre-R2007) and merged (R2007+) reading.
///
/// For R2007+ the header section uses the three-stream merge format
/// (main + text + handle), matching the writer's `DwgMergedWriter`.
/// Text reads are routed to the text sub-stream and handle reads to the
/// handle sub-stream, preserving bit alignment of the main data.
enum SectionReaderInner {
    /// Pre-R2007: single stream, everything inline
    BitReader(DwgBitReader),
    /// R2007+: three-stream merge (main + text + handle)
    MergedReader(DwgMergedReader),
}

struct SectionReader {
    inner: SectionReaderInner,
}

impl SectionReader {
    fn with_encoding(
        data: Vec<u8>,
        version: DxfVersion,
        encoding: &'static encoding_rs::Encoding,
    ) -> Result<Self> {
        if version >= DxfVersion::AC1021 {
            // R2007+: three-stream merge.
            // The section data starts with an RL (total size in bits).
            // Create a DwgMergedReader in ThreeStream mode and set up
            // text/handle sub-streams from the RL.
            let mut merged = DwgMergedReader::new_with_encoding(data, version, 0, encoding);
            // Read the RL (total_size_bits) stored by save_position_for_size
            let total_size_bits = merged.main_mut().read_raw_long() as i64;
            merged.setup_text_and_handle(total_size_bits);
            Ok(SectionReader {
                inner: SectionReaderInner::MergedReader(merged),
            })
        } else {
            // Pre-R2007: single stream, everything inline
            let dwg = DwgVersion::from_dxf_version(version)?;
            let reader = DwgBitReader::with_encoding(data, dwg, version, encoding);
            Ok(SectionReader {
                inner: SectionReaderInner::BitReader(reader),
            })
        }
    }

    // Delegate all read methods to the underlying reader
    fn read_bit(&mut self) -> bool {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_bit(),
            SectionReaderInner::MergedReader(r) => r.read_bit(),
        }
    }
    fn read_byte(&mut self) -> u8 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_byte(),
            SectionReaderInner::MergedReader(r) => r.read_byte(),
        }
    }
    fn read_bit_short(&mut self) -> i16 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_bit_short(),
            SectionReaderInner::MergedReader(r) => r.read_bit_short(),
        }
    }
    fn read_bit_long(&mut self) -> i32 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_bit_long(),
            SectionReaderInner::MergedReader(r) => r.read_bit_long(),
        }
    }
    fn read_bit_long_long(&mut self) -> i64 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_bit_long_long(),
            SectionReaderInner::MergedReader(r) => r.read_bit_long_long(),
        }
    }
    fn read_bit_double(&mut self) -> f64 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_bit_double(),
            SectionReaderInner::MergedReader(r) => r.read_bit_double(),
        }
    }
    fn read_3bit_double(&mut self) -> crate::types::Vector3 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_3bit_double(),
            SectionReaderInner::MergedReader(r) => r.read_3bit_double(),
        }
    }
    fn read_2raw_double(&mut self) -> crate::types::Vector2 {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_2raw_double(),
            SectionReaderInner::MergedReader(r) => r.read_2raw_double(),
        }
    }
    fn read_variable_text(&mut self) -> String {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_variable_text(),
            SectionReaderInner::MergedReader(r) => r.read_variable_text(),
        }
    }
    /// Read a handle reference retaining the wire form `(code, size,
    /// value, absolute)` (§19 H3 raw retention).
    fn read_handle_raw(&mut self) -> (u8, u8, u64, u64) {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_handle_raw(),
            SectionReaderInner::MergedReader(r) => r.read_handle_raw(),
        }
    }

    /// Read a handle form from the MAIN stream (not the handle sub-stream),
    /// retaining the wire tuple. Used for HANDSEED which is always written
    /// inline.
    fn read_handle_inline_raw(&mut self) -> (u8, u8, u64, u64) {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_handle_raw(),
            SectionReaderInner::MergedReader(r) => r.read_main_handle_raw(),
        }
    }

    /// Read a CmColor retaining the raw wire parts (§19 H3 raw retention).
    fn read_cm_color_raw(&mut self) -> crate::document::DwgRawCmc {
        match &mut self.inner {
            SectionReaderInner::BitReader(r) => r.read_cm_color_raw(),
            SectionReaderInner::MergedReader(r) => r.read_cm_color_raw(),
        }
    }

    fn read_datetime(&mut self) -> (i32, i32) {
        let day = self.read_bit_long();
        let ms = self.read_bit_long();
        (day, ms)
    }

    fn read_timespan(&mut self) -> (i32, i32) {
        let days = self.read_bit_long();
        let ms = self.read_bit_long();
        (days, ms)
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Public API
// ════════════════════════════════════════════════════════════════════════════

/// Read the complete Header section from raw bytes (including sentinels).
///
/// # Returns
/// `HeaderVariables` populated with all header variables, plus the
/// gold-JSON-mirror `DwgHeaderRaw` (§19 H3) retained from the same walk.
pub fn read_header(
    data: &[u8],
    version: DxfVersion,
    maintenance_version: u8,
) -> Result<(HeaderVariables, crate::document::DwgHeaderRaw)> {
    read_header_with_encoding(
        data,
        version,
        maintenance_version,
        encoding_rs::WINDOWS_1252,
    )
}

pub fn read_header_with_encoding(
    data: &[u8],
    version: DxfVersion,
    maintenance_version: u8,
    encoding: &'static encoding_rs::Encoding,
) -> Result<(HeaderVariables, crate::document::DwgHeaderRaw)> {
    // ── Verify start sentinel ──
    if data.len() < 36 {
        return Err(DxfError::Parse("Header section too short".to_string()));
    }
    if &data[..16] != &start_sentinels::HEADER {
        return Err(DxfError::InvalidSentinel(
            "Header section start sentinel mismatch".to_string(),
        ));
    }

    // ── Read section size ──
    let mut size_offset = 16;
    let section_size = i32::from_le_bytes([
        data[size_offset],
        data[size_offset + 1],
        data[size_offset + 2],
        data[size_offset + 3],
    ]) as usize;
    size_offset += 4;

    // Extra 4 zero bytes when: (AC1024+ && maintenance > 3) || AC1032+
    if DwgVersion::has_section_extra_rl(version, maintenance_version) {
        size_offset += 4;
    }

    let section_data = &data[size_offset..size_offset + section_size];

    let mut r = SectionReader::with_encoding(section_data.to_vec(), version, encoding)?;
    let mut h = HeaderVariables::default();
    let mut raw = crate::document::DwgHeaderRaw {
        version: version.as_str().to_string(),
        ..Default::default()
    };

    read_header_fields(&mut r, version, &mut h, &mut raw);

    Ok((h, raw))
}

// ════════════════════════════════════════════════════════════════════════════
//  Header field reader — the big one (~200 fields, inverse of writer)
// ════════════════════════════════════════════════════════════════════════════

fn read_header_fields(
    r: &mut SectionReader,
    v: DxfVersion,
    h: &mut HeaderVariables,
    raw: &mut crate::document::DwgHeaderRaw,
) {
    // R2013+: BLL REQUIREDVERSIONS (gold prints unsigned, PRIu64)
    if r2013_plus(v) {
        let t = r.read_bit_long_long();
        h.required_versions = t;
        raw.required_versions = Some(t as u64 as i64);
    }

    // ── Unit conversions (Common) ──
    raw.unit1_ratio = Some(r.read_bit_double());
    raw.unit2_ratio = Some(r.read_bit_double());
    raw.unit3_ratio = Some(r.read_bit_double());
    raw.unit4_ratio = Some(r.read_bit_double());

    raw.unit1_name = Some(r.read_variable_text());
    raw.unit2_name = Some(r.read_variable_text());
    raw.unit3_name = Some(r.read_variable_text());
    raw.unit4_name = Some(r.read_variable_text());

    raw.unknown_8 = Some(r.read_bit_long() as u32 as i64);
    raw.unknown_9 = Some(r.read_bit_long() as u32 as i64);

    // R13-R14 Only: BS unknown_10 (gold prints it unsigned: 256 = the
    // '11' code on the R14 genus)
    if r13_14_only(v) {
        raw.unknown_10 = Some(r.read_bit_short() as u16 as i64);
    }

    // Pre-2004: current viewport header handle
    if v < DxfVersion::AC1018 {
        let t = r.read_handle_raw();
        h.current_vx_handle = Handle::new(t.3);
        raw.vx_table_record = Some(t.into());
    }

    // ── Drawing mode flags (Common) ──
    let t = r.read_bit();
    h.associate_dimensions = t;
    raw.dimaso = Some(t as i64);
    let t = r.read_bit();
    h.update_dimensions_while_dragging = t;
    raw.dimsho = Some(t as i64);

    if r13_14_only(v) {
        raw.dimsav = Some(r.read_bit() as i64); // DIMSAV undocumented
    }

    let t = r.read_bit();
    h.polyline_linetype_generation = t;
    raw.plinegen = Some(t as i64);
    let t = r.read_bit();
    h.ortho_mode = t;
    raw.orthomode = Some(t as i64);
    let t = r.read_bit();
    h.regen_mode = t;
    raw.regenmode = Some(t as i64);
    let t = r.read_bit();
    h.fill_mode = t;
    raw.fillmode = Some(t as i64);
    let t = r.read_bit();
    h.quick_text_mode = t;
    raw.qtextmode = Some(t as i64);
    let t = r.read_bit();
    h.paper_space_linetype_scaling = t;
    raw.psltscale = Some(t as i64);
    let t = r.read_bit();
    h.limit_check = t;
    raw.limcheck = Some(t as i64);

    if r13_14_only(v) {
        let t = r.read_bit();
        h.blip_mode = t;
        raw.blipmode = Some(t as i64);
    }

    if r2004_plus(v) {
        raw.unknown_11 = Some(r.read_bit() as i64); // undocumented bit
    }

    let t = r.read_bit();
    h.user_timer = t;
    raw.usrtimer = Some(t as i64);
    let t = r.read_bit(); // SKPOLY
    h.sketch_type = if t { 1 } else { 0 };
    raw.skpoly = Some(t as i64);
    let t = r.read_bit(); // ANGDIR
    h.angle_direction = if t { 1 } else { 0 };
    raw.angdir = Some(t as i64);
    let t = r.read_bit(); // SPLFRAME
    h.spline_frame = t;
    raw.splframe = Some(t as i64);

    if r13_14_only(v) {
        let t = r.read_bit();
        h.attribute_request = t;
        raw.attreq = Some(t as i64);
        let t = r.read_bit();
        h.attribute_dialog = t;
        raw.attdia = Some(t as i64);
    }

    let t = r.read_bit();
    h.mirror_text = t;
    raw.mirrtext = Some(t as i64);
    let t = r.read_bit();
    h.world_view = t;
    raw.worldview = Some(t as i64);

    if r13_14_only(v) {
        raw.wireframe = Some(r.read_bit() as i64); // WIREFRAME
    }

    let t = r.read_bit();
    h.show_model_space = t;
    raw.tilemode = Some(t as i64);
    let t = r.read_bit();
    h.paper_space_limit_check = t;
    raw.plimcheck = Some(t as i64);
    let t = r.read_bit();
    h.retain_xref_visibility = t;
    raw.visretain = Some(t as i64);

    if r13_14_only(v) {
        let t = r.read_bit();
        h.delete_objects = t;
        raw.delobj = Some(t as i64);
    }

    let t = r.read_bit();
    h.display_silhouette = t;
    raw.dispsilh = Some(t as i64);
    raw.pellipse = Some(r.read_bit() as i64);
    let t = r.read_bit_short();
    h.proxy_graphics = t;
    raw.proxygraphics = Some(t as u16 as i64);

    if r13_14_only(v) {
        let t = r.read_bit_short();
        h.drag_mode = t;
        raw.dragmode = Some(t as u16 as i64);
    }

    // ── Unit settings (Common) ──
    let t = r.read_bit_short(); // TREEDEPTH (BSd)
    h.tree_depth = t;
    raw.treedepth = Some(t as i64);
    let t = r.read_bit_short();
    h.linear_unit_format = t;
    raw.lunits = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.linear_unit_precision = t;
    raw.luprec = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.angular_unit_format = t;
    raw.aunits = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.angular_unit_precision = t;
    raw.auprec = Some(t as u16 as i64);

    if r13_14_only(v) {
        let t = r.read_bit_short();
        h.object_snap_mode = t as i32;
        raw.osmode = Some(t as u16 as i64);
    }

    let t = r.read_bit_short();
    h.attribute_visibility = t;
    raw.attmode = Some(t as u16 as i64);

    if r13_14_only(v) {
        let t = r.read_bit_short();
        h.coords_mode = t;
        raw.coords = Some(t as u16 as i64);
    }

    let t = r.read_bit_short();
    h.point_display_mode = t;
    raw.pdmode = Some(t as u16 as i64);

    if r13_14_only(v) {
        let t = r.read_bit_short();
        h.pick_style = t;
        raw.pickstyle = Some(t as u16 as i64);
    }

    if r2004_plus(v) {
        raw.unknown_12 = Some(r.read_bit_long() as u32 as i64);
        raw.unknown_13 = Some(r.read_bit_long() as u32 as i64);
        raw.unknown_14 = Some(r.read_bit_long() as u32 as i64);
    }

    let t = r.read_bit_short(); // USERI1 (BSd)
    h.user_int1 = t;
    raw.useri1 = Some(t as i64);
    let t = r.read_bit_short(); // USERI2 (BSd)
    h.user_int2 = t;
    raw.useri2 = Some(t as i64);
    let t = r.read_bit_short(); // USERI3 (BSd)
    h.user_int3 = t;
    raw.useri3 = Some(t as i64);
    let t = r.read_bit_short(); // USERI4 (BSd)
    h.user_int4 = t;
    raw.useri4 = Some(t as i64);
    let t = r.read_bit_short(); // USERI5 (BSd)
    h.user_int5 = t;
    raw.useri5 = Some(t as i64);

    let t = r.read_bit_short();
    h.spline_segments = t;
    raw.splinesegs = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.surface_u_density = t;
    raw.surfu = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.surface_v_density = t;
    raw.surfv = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.surface_type = t;
    raw.surftype = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.surface_tab1 = t;
    raw.surftab1 = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.surface_tab2 = t;
    raw.surftab2 = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.spline_type = t;
    raw.splinetype = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.shade_edge = t;
    raw.shadeedge = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.shade_diffuse = t;
    raw.shadedif = Some(t as u16 as i64);
    raw.unitmode = Some(r.read_bit_short() as u16 as i64);
    let t = r.read_bit_short();
    h.max_active_viewports = t;
    raw.maxactvp = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.isolines = t;
    raw.isolines = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.multiline_justification = t;
    raw.cmljust = Some(t as u16 as i64);
    let t = r.read_bit_short();
    h.text_quality = t;
    raw.textqlty = Some(t as u16 as i64);

    // ── Scale/size defaults (Common) ──
    let t = r.read_bit_double();
    h.linetype_scale = t;
    raw.ltscale = Some(t);
    let t = r.read_bit_double();
    h.text_height = t;
    raw.textsize = Some(t);
    let t = r.read_bit_double();
    h.trace_width = t;
    raw.tracewid = Some(t);
    let t = r.read_bit_double();
    h.sketch_increment = t;
    raw.sketchinc = Some(t);
    let t = r.read_bit_double();
    h.fillet_radius = t;
    raw.filletrad = Some(t);
    let t = r.read_bit_double();
    h.thickness = t;
    raw.thickness = Some(t);
    let t = r.read_bit_double();
    h.angle_base = t;
    raw.angbase = Some(t);
    let t = r.read_bit_double();
    h.point_display_size = t;
    raw.pdsize = Some(t);
    let t = r.read_bit_double();
    h.polyline_width = t;
    raw.plinewid = Some(t);
    let t = r.read_bit_double();
    h.user_real1 = t;
    raw.userr1 = Some(t);
    let t = r.read_bit_double();
    h.user_real2 = t;
    raw.userr2 = Some(t);
    let t = r.read_bit_double();
    h.user_real3 = t;
    raw.userr3 = Some(t);
    let t = r.read_bit_double();
    h.user_real4 = t;
    raw.userr4 = Some(t);
    let t = r.read_bit_double();
    h.user_real5 = t;
    raw.userr5 = Some(t);
    let t = r.read_bit_double();
    h.chamfer_distance_a = t;
    raw.chamfera = Some(t);
    let t = r.read_bit_double();
    h.chamfer_distance_b = t;
    raw.chamferb = Some(t);
    let t = r.read_bit_double();
    h.chamfer_length = t;
    raw.chamferc = Some(t);
    let t = r.read_bit_double();
    h.chamfer_angle = t;
    raw.chamferd = Some(t);
    let t = r.read_bit_double();
    h.facet_resolution = t;
    raw.facetres = Some(t);
    let t = r.read_bit_double();
    h.multiline_scale = t;
    raw.cmlscale = Some(t);
    let t = r.read_bit_double();
    h.current_entity_linetype_scale = t;
    raw.celtscale = Some(t);

    let t = r.read_variable_text();
    h.menu_name = t.clone();
    raw.menu = Some(t);

    // ── Date/time (Common) ── gold's FIELD_TIMEBLL prints [days, ms] as
    // unsigned BL (PRIu32) — cast the 32-bit words unsigned.
    let (cd, cms) = r.read_datetime();
    h.create_date_julian = day_ms_to_julian(cd, cms);
    raw.tducreate = Some([cd as u32 as i64, cms as u32 as i64]);
    let (ud, ums) = r.read_datetime();
    h.update_date_julian = day_ms_to_julian(ud, ums);
    raw.tduupdate = Some([ud as u32 as i64, ums as u32 as i64]);

    if r2004_plus(v) {
        raw.unknown_15 = Some(r.read_bit_long() as u32 as i64);
        raw.unknown_16 = Some(r.read_bit_long() as u32 as i64);
        raw.unknown_17 = Some(r.read_bit_long() as u32 as i64);
    }

    let (ted, tems) = r.read_timespan();
    h.total_editing_time = day_ms_to_timespan(ted, tems);
    raw.tdindwg = Some([ted as u32 as i64, tems as u32 as i64]);
    let (ued, uems) = r.read_timespan();
    h.user_elapsed_time = day_ms_to_timespan(ued, uems);
    raw.tdusrtimer = Some([ued as u32 as i64, uems as u32 as i64]);

    // ── Current entity color ──
    let cmc = r.read_cm_color_raw();
    h.current_entity_color = if v >= DxfVersion::AC1018 {
        cmc.to_color()
    } else {
        cmc.index_color()
    };
    raw.cecolor = Some(cmc);

    // ── HANDSEED ──
    // HANDSEED is written to the main stream (not the handle sub-stream),
    // so we must read it inline from the main stream.
    let t = r.read_handle_inline_raw();
    h.handle_seed = t.3;
    raw.handseed = Some(t.into());

    // ── Style/layer/linetype handles ──
    let t = r.read_handle_raw();
    h.current_layer_handle = Handle::new(t.3);
    raw.clayer = Some(t.into());
    let t = r.read_handle_raw();
    h.current_text_style_handle = Handle::new(t.3);
    raw.textstyle = Some(t.into());
    let t = r.read_handle_raw();
    h.current_linetype_handle = Handle::new(t.3);
    raw.celtype = Some(t.into());

    if r2007_plus(v) {
        let t = r.read_handle_raw();
        h.current_material_handle = Handle::new(t.3);
        raw.cmaterial = Some(t.into());
    }

    let t = r.read_handle_raw();
    h.current_dimstyle_handle = Handle::new(t.3);
    raw.dimstyle = Some(t.into());
    let t = r.read_handle_raw();
    h.current_multiline_style_handle = Handle::new(t.3);
    raw.cmlstyle = Some(t.into());

    if r2000_plus(v) {
        let t = r.read_bit_double();
        h.viewport_scale_factor = t;
        raw.psvpscale = Some(t);
    }

    // ── Paper space extents/limits/UCS ──
    let t = r.read_3bit_double();
    h.paper_space_insertion_base = t;
    raw.pinsbase = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.paper_space_extents_min = t;
    raw.pextmin = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.paper_space_extents_max = t;
    raw.pextmax = Some([t.x, t.y, t.z]);
    let t = r.read_2raw_double();
    h.paper_space_limits_min = t;
    raw.plimmin = Some([t.x, t.y]);
    let t = r.read_2raw_double();
    h.paper_space_limits_max = t;
    raw.plimmax = Some([t.x, t.y]);
    let t = r.read_bit_double();
    h.paper_elevation = t;
    raw.pelevation = Some(t);
    let t = r.read_3bit_double();
    h.paper_space_ucs_origin = t;
    raw.pucsorg = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.paper_space_ucs_x_axis = t;
    raw.pucsxdir = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.paper_space_ucs_y_axis = t;
    raw.pucsydir = Some([t.x, t.y, t.z]);

    // PUCSNAME (PSPACE)
    raw.pucsname = Some(r.read_handle_raw().into());

    if r2000_plus(v) {
        let t = r.read_handle_raw();
        h.paper_ucs_ortho_ref = Handle::new(t.3);
        raw.pucsorthoref = Some(t.into());
        let t = r.read_bit_short();
        h.paper_ucs_ortho_view = t;
        raw.pucsorthoview = Some(t as u16 as i64);
        raw.pucsbase = Some(r.read_handle_raw().into());

        // Paper space orthographic origins (6 × 3BD)
        let t = r.read_3bit_double();
        raw.pucsorgtop = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.pucsorgbottom = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.pucsorgleft = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.pucsorgright = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.pucsorgfront = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.pucsorgback = Some([t.x, t.y, t.z]);
    }

    // ── Model space extents/limits/UCS ──
    let t = r.read_3bit_double();
    h.model_space_insertion_base = t;
    raw.insbase = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.model_space_extents_min = t;
    raw.extmin = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.model_space_extents_max = t;
    raw.extmax = Some([t.x, t.y, t.z]);
    let t = r.read_2raw_double();
    h.model_space_limits_min = t;
    raw.limmin = Some([t.x, t.y]);
    let t = r.read_2raw_double();
    h.model_space_limits_max = t;
    raw.limmax = Some([t.x, t.y]);
    let t = r.read_bit_double();
    h.elevation = t;
    raw.elevation = Some(t);
    let t = r.read_3bit_double();
    h.model_space_ucs_origin = t;
    raw.ucsorg = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.model_space_ucs_x_axis = t;
    raw.ucsxdir = Some([t.x, t.y, t.z]);
    let t = r.read_3bit_double();
    h.model_space_ucs_y_axis = t;
    raw.ucsydir = Some([t.x, t.y, t.z]);

    // UCSNAME (MSPACE)
    raw.ucsname = Some(r.read_handle_raw().into());

    if r2000_plus(v) {
        let t = r.read_handle_raw();
        h.ucs_ortho_ref = Handle::new(t.3);
        raw.ucsorthoref = Some(t.into());
        let t = r.read_bit_short();
        h.ucs_ortho_view = t;
        raw.ucsorthoview = Some(t as u16 as i64);
        raw.ucsbase = Some(r.read_handle_raw().into());

        // Model space orthographic origins (6 × 3BD)
        let t = r.read_3bit_double();
        raw.ucsorgtop = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.ucsorgbottom = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.ucsorgleft = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.ucsorgright = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.ucsorgfront = Some([t.x, t.y, t.z]);
        let t = r.read_3bit_double();
        raw.ucsorgback = Some([t.x, t.y, t.z]);

        // DIMPOST, DIMAPOST
        let t = r.read_variable_text();
        h.dim_post = t.clone();
        raw.dimpost = Some(t);
        let t = r.read_variable_text();
        h.dim_alt_post = t.clone();
        raw.dimapost = Some(t);
    }

    // ── Dimension variables (R13-R14 Only block) ──
    if r13_14_only(v) {
        let t = r.read_bit();
        h.dim_tolerance = t;
        raw.dimtol = Some(t as i64);
        let t = r.read_bit();
        h.dim_limits = t;
        raw.dimlim = Some(t as i64);
        let t = r.read_bit();
        h.dim_text_inside_horizontal = t;
        raw.dimtih = Some(t as i64);
        let t = r.read_bit();
        h.dim_text_outside_horizontal = t;
        raw.dimtoh = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_ext1 = t;
        raw.dimse1 = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_ext2 = t;
        raw.dimse2 = Some(t as i64);
        let t = r.read_bit();
        h.dim_alternate_units = t;
        raw.dimalt = Some(t as i64);
        let t = r.read_bit();
        h.dim_force_line_inside = t;
        raw.dimtofl = Some(t as i64);
        let t = r.read_bit();
        h.dim_separate_arrows = t;
        raw.dimsah = Some(t as i64);
        let t = r.read_bit();
        h.dim_force_text_inside = t;
        raw.dimtix = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_outside_ext = t;
        raw.dimsoxd = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_alt_decimal_places = t;
        raw.dimaltd = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_zero_suppression = t;
        raw.dimzin = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_line1 = t;
        raw.dimsd1 = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_line2 = t;
        raw.dimsd2 = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_tolerance_justification = t;
        raw.dimtolj = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_horizontal_justification = t;
        raw.dimjust = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_fit = t;
        raw.dimfit = Some(t as i64);
        let t = r.read_bit();
        h.dim_user_positioned_text = t;
        raw.dimupt = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_tolerance_zero_suppression = t;
        raw.dimtzin = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_alt_tolerance_zero_suppression = t;
        raw.dimaltz = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_alt_tolerance_zero_tight = t;
        raw.dimalttz = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dim_text_above = t;
        raw.dimtad = Some(t as i64);
        raw.dimunit = Some(r.read_bit_short() as u16 as i64); // DIMUNIT
        let t = r.read_bit_short();
        h.dim_angular_decimal_places = t;
        raw.dimaunit = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_decimal_places = t;
        raw.dimdec = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_tolerance_decimal_places = t;
        raw.dimtdec = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_units_format = t;
        raw.dimaltu = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_tolerance_decimal_places = t;
        raw.dimalttd = Some(t as u16 as i64);

        // DIMTXSTY handle
        let t = r.read_handle_raw();
        h.dim_text_style_handle = Handle::new(t.3);
        raw.dimtxsty = Some(t.into());
    }

    // ── Dimension variables (Common) ──
    let t = r.read_bit_double();
    h.dim_scale = t;
    raw.dimscale = Some(t);
    let t = r.read_bit_double();
    h.dim_arrow_size = t;
    raw.dimasz = Some(t);
    let t = r.read_bit_double();
    h.dim_ext_line_offset = t;
    raw.dimexo = Some(t);
    let t = r.read_bit_double();
    h.dim_line_increment = t;
    raw.dimdli = Some(t);
    let t = r.read_bit_double();
    h.dim_ext_line_extension = t;
    raw.dimexe = Some(t);
    let t = r.read_bit_double();
    h.dim_rounding = t;
    raw.dimrnd = Some(t);
    let t = r.read_bit_double();
    h.dim_line_extension = t;
    raw.dimdle = Some(t);
    let t = r.read_bit_double();
    h.dim_tolerance_plus = t;
    raw.dimtp = Some(t);
    let t = r.read_bit_double();
    h.dim_tolerance_minus = t;
    raw.dimtm = Some(t);

    // R2007+ dimension extras
    if r2007_plus(v) {
        raw.dimfxl = Some(r.read_bit_double()); // DIMFXL
        raw.dimjogang = Some(r.read_bit_double()); // DIMJOGANG
        raw.dimtfill = Some(r.read_bit_short() as u16 as i64); // DIMTFILL
        raw.dimtfillclr = Some(r.read_cm_color_raw()); // DIMTFILLCLR
    }

    // R2000+ dimension flags
    if r2000_plus(v) {
        let t = r.read_bit();
        h.dim_tolerance = t;
        raw.dimtol = Some(t as i64);
        let t = r.read_bit();
        h.dim_limits = t;
        raw.dimlim = Some(t as i64);
        let t = r.read_bit();
        h.dim_text_inside_horizontal = t;
        raw.dimtih = Some(t as i64);
        let t = r.read_bit();
        h.dim_text_outside_horizontal = t;
        raw.dimtoh = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_ext1 = t;
        raw.dimse1 = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_ext2 = t;
        raw.dimse2 = Some(t as i64);
        let t = r.read_bit_short();
        h.dim_text_above = t;
        raw.dimtad = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_zero_suppression = t;
        raw.dimzin = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_zero_suppression = t;
        raw.dimazin = Some(t as u16 as i64);
    }

    if r2007_plus(v) {
        raw.dimarcsym = Some(r.read_bit_short() as u16 as i64);
    }

    // ── Dimension sizes (Common) ──
    let t = r.read_bit_double();
    h.dim_text_height = t;
    raw.dimtxt = Some(t);
    let t = r.read_bit_double();
    h.dim_center_mark = t;
    raw.dimcen = Some(t);
    let t = r.read_bit_double();
    h.dim_tick_size = t;
    raw.dimtsz = Some(t);
    let t = r.read_bit_double();
    h.dim_alt_scale = t;
    raw.dimaltf = Some(t);
    let t = r.read_bit_double();
    h.dim_linear_scale = t;
    raw.dimlfac = Some(t);
    let t = r.read_bit_double();
    h.dim_text_vertical_pos = t;
    raw.dimtvp = Some(t);
    let t = r.read_bit_double();
    h.dim_tolerance_scale = t;
    raw.dimtfac = Some(t);
    let t = r.read_bit_double();
    h.dim_line_gap = t;
    raw.dimgap = Some(t);

    // R13-R14 only: dimension text strings
    if r13_14_only(v) {
        let t = r.read_variable_text();
        h.dim_post = t.clone();
        raw.dimpost = Some(t);
        let t = r.read_variable_text();
        h.dim_alt_post = t.clone();
        raw.dimapost = Some(t);
        let t = r.read_variable_text();
        h.dim_arrow_block = t.clone();
        raw.dimblk_t = Some(t);
        let t = r.read_variable_text();
        h.dim_arrow_block1 = t.clone();
        raw.dimblk1_t = Some(t);
        let t = r.read_variable_text();
        h.dim_arrow_block2 = t.clone();
        raw.dimblk2_t = Some(t);
    }

    // R2000+ only: additional dimension settings
    if r2000_plus(v) {
        let t = r.read_bit_double();
        h.dim_alt_rounding = t;
        raw.dimaltrnd = Some(t);
        let t = r.read_bit();
        h.dim_alternate_units = t;
        raw.dimalt = Some(t as i64);
        let t = r.read_bit_short();
        h.dim_alt_decimal_places = t;
        raw.dimaltd = Some(t as u16 as i64);
        let t = r.read_bit();
        h.dim_force_line_inside = t;
        raw.dimtofl = Some(t as i64);
        let t = r.read_bit();
        h.dim_separate_arrows = t;
        raw.dimsah = Some(t as i64);
        let t = r.read_bit();
        h.dim_force_text_inside = t;
        raw.dimtix = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_outside_ext = t;
        raw.dimsoxd = Some(t as i64);
    }

    // ── Dimension colors (Common) ──
    let cmc = r.read_cm_color_raw();
    h.dim_line_color = if v >= DxfVersion::AC1018 { cmc.to_color() } else { cmc.index_color() };
    raw.dimclrd = Some(cmc);
    let cmc = r.read_cm_color_raw();
    h.dim_ext_line_color = if v >= DxfVersion::AC1018 { cmc.to_color() } else { cmc.index_color() };
    raw.dimclre = Some(cmc);
    let cmc = r.read_cm_color_raw();
    h.dim_text_color = if v >= DxfVersion::AC1018 { cmc.to_color() } else { cmc.index_color() };
    raw.dimclrt = Some(cmc);

    // R2000+ only: dimension unit settings
    if r2000_plus(v) {
        let t = r.read_bit_short();
        h.dim_angular_decimal_places = t;
        raw.dimadec = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_decimal_places = t;
        raw.dimdec = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_tolerance_decimal_places = t;
        raw.dimtdec = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_units_format = t;
        raw.dimaltu = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_tolerance_decimal_places = t;
        raw.dimalttd = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_angular_units = t;
        raw.dimaunit = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_fraction_format = t;
        raw.dimfrac = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_linear_unit_format = t;
        raw.dimlunit = Some(t as u16 as i64);
        let t = r.read_bit_short(); // DIMDSEP
        h.dim_decimal_separator = t as u8 as char;
        raw.dimdsep = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_text_movement = t;
        raw.dimtmove = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_horizontal_justification = t;
        raw.dimjust = Some(t as u16 as i64);
        let t = r.read_bit();
        h.dim_suppress_line1 = t;
        raw.dimsd1 = Some(t as i64);
        let t = r.read_bit();
        h.dim_suppress_line2 = t;
        raw.dimsd2 = Some(t as i64);
        let t = r.read_bit_short();
        h.dim_tolerance_justification = t;
        raw.dimtolj = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_tolerance_zero_suppression = t;
        raw.dimtzin = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_tolerance_zero_suppression = t;
        raw.dimaltz = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.dim_alt_tolerance_zero_tight = t;
        raw.dimalttz = Some(t as u16 as i64);
        let t = r.read_bit();
        h.dim_user_positioned_text = t;
        raw.dimupt = Some(t as i64);
        let t = r.read_bit_short();
        h.dim_fit = t;
        raw.dimatfit = Some(t as u16 as i64);
    }

    // R2007+: DIMFXLON
    if r2007_plus(v) {
        raw.dimfxlon = Some(r.read_bit() as i64); // DimensionIsExtensionLineLengthFixed
    }

    // R2010+: extra dimension fields
    if r2010_plus(v) {
        raw.dimtxtdirection = Some(r.read_bit() as i64); // DIMTXTDIRECTION
        raw.dimaltmzf = Some(r.read_bit_double()); // DIMALTMZF
        raw.dimaltmzs = Some(r.read_variable_text()); // DIMALTMZS
        raw.dimmzf = Some(r.read_bit_double()); // DIMMZF
        raw.dimmzs = Some(r.read_variable_text()); // DIMMZS
    }

    // R2000+ dimension handles
    if r2000_plus(v) {
        let t = r.read_handle_raw();
        h.dim_text_style_handle = Handle::new(t.3);
        raw.dimtxsty = Some(t.into());
        raw.dimldrblk = Some(r.read_handle_raw().into()); // DIMLDRBLK
        raw.dimblk = Some(r.read_handle_raw().into()); // DIMBLK
        raw.dimblk1 = Some(r.read_handle_raw().into()); // DIMBLK1
        raw.dimblk2 = Some(r.read_handle_raw().into()); // DIMBLK2
    }

    // R2007+ dimension linetype handles
    if r2007_plus(v) {
        let t = r.read_handle_raw();
        h.dim_linetype_handle = Handle::new(t.3);
        raw.dimltype = Some(t.into());
        let t = r.read_handle_raw();
        h.dim_linetype1_handle = Handle::new(t.3);
        raw.dimltex1 = Some(t.into());
        let t = r.read_handle_raw();
        h.dim_linetype2_handle = Handle::new(t.3);
        raw.dimltex2 = Some(t.into());
    }

    // R2000+ dimension line weights
    if r2000_plus(v) {
        let t = r.read_bit_short();
        h.dim_line_weight = t;
        raw.dimlwd = Some(t as i16 as i64);
        let t = r.read_bit_short();
        h.dim_ext_line_weight = t;
        raw.dimlwe = Some(t as i16 as i64);
    }

    // ── Table control object handles (Common) ──
    let t = r.read_handle_raw();
    h.block_control_handle = Handle::new(t.3);
    raw.block_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.layer_control_handle = Handle::new(t.3);
    raw.layer_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.style_control_handle = Handle::new(t.3);
    raw.style_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.linetype_control_handle = Handle::new(t.3);
    raw.ltype_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.view_control_handle = Handle::new(t.3);
    raw.view_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.ucs_control_handle = Handle::new(t.3);
    raw.ucs_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.vport_control_handle = Handle::new(t.3);
    raw.vport_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.appid_control_handle = Handle::new(t.3);
    raw.appid_control_object = Some(t.into());
    let t = r.read_handle_raw();
    h.dimstyle_control_handle = Handle::new(t.3);
    raw.dimstyle_control_object = Some(t.into());

    // R13-R15 only: VPEntHdr control
    if r13_15_only(v) {
        let t = r.read_handle_raw();
        h.vpent_hdr_control_handle = Handle::new(t.3);
        raw.vx_control_object = Some(t.into());
    }

    // ── Dictionary handles (Common) ──
    let t = r.read_handle_raw();
    h.acad_group_dict_handle = Handle::new(t.3);
    raw.dictionary_acad_group = Some(t.into());
    let t = r.read_handle_raw();
    h.acad_mlinestyle_dict_handle = Handle::new(t.3);
    raw.dictionary_acad_mlinestyle = Some(t.into());
    let t = r.read_handle_raw();
    h.named_objects_dict_handle = Handle::new(t.3);
    raw.dictionary_named_object = Some(t.into());

    // R2000+ dictionaries and flags
    if r2000_plus(v) {
        raw.tstackalign = Some(r.read_bit_short() as u16 as i64); // TSTACKALIGN
        raw.tstacksize = Some(r.read_bit_short() as u16 as i64); // TSTACKSIZE

        let t = r.read_variable_text();
        h.hyperlink_base = t.clone();
        raw.hyperlinkbase = Some(t);
        let t = r.read_variable_text();
        h.stylesheet = t.clone();
        raw.stylesheet = Some(t);

        let t = r.read_handle_raw();
        h.acad_layout_dict_handle = Handle::new(t.3);
        raw.dictionary_layout = Some(t.into());
        let t = r.read_handle_raw();
        h.acad_plotsettings_dict_handle = Handle::new(t.3);
        raw.dictionary_plotsettings = Some(t.into());
        let t = r.read_handle_raw();
        h.acad_plotstylename_dict_handle = Handle::new(t.3);
        raw.dictionary_plotstylename = Some(t.into());
    }

    // R2004+ dictionaries
    if r2004_plus(v) {
        let t = r.read_handle_raw();
        h.acad_material_dict_handle = Handle::new(t.3);
        raw.dictionary_material = Some(t.into());
        let t = r.read_handle_raw();
        h.acad_color_dict_handle = Handle::new(t.3);
        raw.dictionary_color = Some(t.into());
    }

    // R2007+ dictionaries
    if r2007_plus(v) {
        let t = r.read_handle_raw();
        h.acad_visualstyle_dict_handle = Handle::new(t.3);
        raw.dictionary_visualstyle = Some(t.into());
        if r2013_plus(v) {
            raw.unknown_20 = Some(r.read_handle_raw().into()); // DICTIONARY_LIGHTLIST?
        }
    }

    // R2000+ flags bitfield
    if r2000_plus(v) {
        let flags = r.read_bit_long();
        raw.flags = Some(flags as u32 as i64);
        h.current_line_weight = LineWeight::from_dwg_index((flags & 0x1F) as u8).value();
        h.end_caps = ((flags >> 5) & 0x03) as i16;
        h.join_style = ((flags >> 7) & 0x03) as i16;
        h.lineweight_display = (flags & 0x200) == 0;
        h.xedit = (flags & 0x400) == 0;
        h.extended_names = (flags & 0x800) != 0;
        h.plotstyle_mode = (flags & 0x2000) != 0;
        h.ole_startup = (flags & 0x4000) != 0;

        let t = r.read_bit_short();
        h.insertion_units = t;
        raw.insunits = Some(t as u16 as i64);
        let t = r.read_bit_short();
        h.current_plotstyle_type = t;
        raw.cepsntype = Some(t as u16 as i64);

        if h.current_plotstyle_type == 3 {
            raw.cpsnid = Some(r.read_handle_raw().into()); // CPSNID
        }

        let t = r.read_variable_text();
        h.fingerprint_guid = t.clone();
        raw.fingerprintguid = Some(t);
        let t = r.read_variable_text();
        h.version_guid = t.clone();
        raw.versionguid = Some(t);
    }

    // R2004+ extra entity settings
    if r2004_plus(v) {
        let t = r.read_byte() as i16;
        h.sort_entities = t;
        raw.sortents = Some(t as i64);
        let t = r.read_byte() as i16;
        h.index_control = t;
        raw.indexctl = Some(t as i64);
        let t = r.read_byte() as i16;
        h.hide_text = t;
        raw.hidetext = Some(t as i64);
        let t = r.read_byte() as i16;
        h.xclip_frame = t;
        raw.xclipframe = Some(t as i64);
        let t = r.read_byte() as i16;
        h.dimension_associativity = t;
        raw.dimassoc = Some(t as i64);
        let t = r.read_byte() as i16;
        h.halo_gap = t;
        raw.halogap = Some(t as i64);
        let t = r.read_bit_short();
        h.obscured_color = t;
        raw.obscolor = Some(t as u16 as i64); // FIELD_BS — unsigned print
        let t = r.read_bit_short();
        h.intersection_color = t;
        raw.intersectioncolor = Some(t as u16 as i64); // FIELD_BS — unsigned print
        let t = r.read_byte() as i16;
        h.obscured_linetype = t;
        raw.obsltype = Some(t as i64);
        let t = r.read_byte() as i16;
        h.intersection_display = t;
        raw.intersectiondisplay = Some(t as i64);

        let t = r.read_variable_text();
        h.project_name = t.clone();
        raw.projectname = Some(t);
    }

    // ── Block record / linetype handles (Common) ──
    let t = r.read_handle_raw();
    h.paper_space_block_handle = Handle::new(t.3);
    raw.block_record_pspace = Some(t.into());
    let t = r.read_handle_raw();
    h.model_space_block_handle = Handle::new(t.3);
    raw.block_record_mspace = Some(t.into());
    let t = r.read_handle_raw();
    h.bylayer_linetype_handle = Handle::new(t.3);
    raw.ltype_bylayer = Some(t.into());
    let t = r.read_handle_raw();
    h.byblock_linetype_handle = Handle::new(t.3);
    raw.ltype_byblock = Some(t.into());
    let t = r.read_handle_raw();
    h.continuous_linetype_handle = Handle::new(t.3);
    raw.ltype_continuous = Some(t.into());

    // ── R2007+ extended fields ──
    if r2007_plus(v) {
        let t = r.read_bit();
        h.camera_display = t;
        raw.cameradisplay = Some(t as i64);
        raw.unknown_21 = Some(r.read_bit_long() as u32 as i64);
        raw.unknown_22 = Some(r.read_bit_long() as u32 as i64);
        raw.unknown_23 = Some(r.read_bit_double());

        let t = r.read_bit_double();
        h.steps_per_second = t;
        raw.stepspersec = Some(t);
        let t = r.read_bit_double();
        h.step_size = t;
        raw.stepsize = Some(t);
        raw._3ddwfprec = Some(r.read_bit_double()); // _3DDWFPREC
        let t = r.read_bit_double();
        h.lens_length = t;
        raw.lenslength = Some(t);
        let t = r.read_bit_double();
        h.camera_height = t;
        raw.cameraheight = Some(t);
        let t = r.read_byte();
        h.record_solid_history = t != 0;
        raw.solidhist = Some(t as i64);
        let t = r.read_byte() as i16;
        h.show_solid_history = t.clamp(0, 2);
        raw.showhist = Some(t as i64);
        raw.psolwidth = Some(r.read_bit_double());
        raw.psolheight = Some(r.read_bit_double());
        let t = r.read_bit_double();
        h.loft_angle1 = t;
        raw.loftang1 = Some(t);
        let t = r.read_bit_double();
        h.loft_angle2 = t;
        raw.loftang2 = Some(t);
        let t = r.read_bit_double();
        h.loft_magnitude1 = t;
        raw.loftmag1 = Some(t);
        let t = r.read_bit_double();
        h.loft_magnitude2 = t;
        raw.loftmag2 = Some(t);
        let t = r.read_bit_short();
        h.loft_param = t;
        raw.loftparam = Some(t as u16 as i64);
        let t = r.read_byte() as i16;
        h.loft_normals = t;
        raw.loftnormals = Some(t as i64);
        let t = r.read_bit_double();
        h.latitude = t;
        raw.latitude = Some(t);
        let t = r.read_bit_double();
        h.longitude = t;
        raw.longitude = Some(t);
        let t = r.read_bit_double();
        h.north_direction = t;
        raw.northdirection = Some(t);
        let t = r.read_bit_long();
        h.timezone = t;
        raw.timezone = Some(t as i64); // BLd (signed)
        raw.lightglyphdisplay = Some(r.read_byte() as i64);
        raw.tilemodelightsynch = Some(r.read_byte() as i64);
        raw.dwfframe = Some(r.read_byte() as i64);
        raw.dgnframe = Some(r.read_byte() as i64);

        raw.realworldscale = Some(r.read_bit() as i64); // REALWORLDSCALE

        raw.interferecolor = Some(r.read_cm_color_raw()); // INTERFERECOLOR

        raw.interfereobjvs = Some(r.read_handle_raw().into()); // INTERFEREOBJVS
        raw.interferevpvs = Some(r.read_handle_raw().into()); // INTERFEREVPVS
        raw.dragvs = Some(r.read_handle_raw().into()); // DRAGVS

        raw.cshadow = Some(r.read_byte() as i64); // CSHADOW
        let t = r.read_bit_double();
        h.shadow_plane_location = t;
        raw.shadowplanelocation = Some(t);
    }

    // ── R14+ trailing fields ──
    if v >= DxfVersion::AC1014 {
        raw.unknown_54 = Some(r.read_bit_short() as u16 as i64);
        raw.unknown_55 = Some(r.read_bit_short() as u16 as i64);
        raw.unknown_56 = Some(r.read_bit_short() as u16 as i64);
        raw.unknown_57 = Some(r.read_bit_short() as u16 as i64);

        // R2004+: three undocumented trailing slots gold does not emit;
        // retained raw (§19 H7 review) so the writer re-emits the wire
        // values verbatim instead of defaulting 0/0/false.
        if r2004_plus(v) {
            raw.unknown_tail_long1 = Some(r.read_bit_long() as u32 as i64);
            raw.unknown_tail_long2 = Some(r.read_bit_long() as u32 as i64);
            raw.unknown_tail_bit = Some(r.read_bit());
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::dwg::dwg_stream_writers::header_writer;

    #[test]
    fn test_header_roundtrip_r2000() {
        let original = HeaderVariables::default();
        let written = header_writer::write_header(DxfVersion::AC1015, &original, 0);
        let (read, raw) = read_header(&written, DxfVersion::AC1015, 0).unwrap();

        // Check sentinel verification worked
        assert_eq!(read.fill_mode, original.fill_mode);
        assert_eq!(read.ortho_mode, original.ortho_mode);
        assert_eq!(read.linear_unit_format, original.linear_unit_format);
        assert_eq!(read.angular_unit_format, original.angular_unit_format);
        assert!((read.linetype_scale - original.linetype_scale).abs() < 1e-10);
        assert!((read.text_height - original.text_height).abs() < 1e-10);
        assert!((read.dim_scale - original.dim_scale).abs() < 1e-10);
        assert!((read.dim_arrow_size - original.dim_arrow_size).abs() < 1e-10);

        // §19 H3: the raw mirror is version-gated by population.
        assert_eq!(raw.version, "AC1015");
        assert!(raw.tstackalign.is_some());
        assert!(raw.menu.is_some());
        assert!(raw.vx_table_record.is_some());
        assert!(raw.unknown_54.is_some());
        assert!(raw.cpsnid.is_none(), "CPSNID only read when CEPSNTYPE == 3");
        assert!(raw.cmaterial.is_none(), "CMATERIAL is R2007+");
        assert!(raw.cameradisplay.is_none(), "camera block is R2007+");
    }

    #[test]
    fn test_header_roundtrip_r2004() {
        let original = HeaderVariables::default();
        let written = header_writer::write_header(DxfVersion::AC1018, &original, 0);
        let (read, raw) = read_header(&written, DxfVersion::AC1018, 0).unwrap();

        assert_eq!(read.fill_mode, original.fill_mode);
        assert_eq!(read.sort_entities, original.sort_entities);
        assert_eq!(read.insertion_units, original.insertion_units);

        assert_eq!(raw.version, "AC1018");
        assert!(raw.unknown_11.is_some());
        assert!(raw.unknown_12.is_some());
        assert!(raw.unknown_15.is_some());
        assert!(raw.vx_table_record.is_none(), "VX_TABLE_RECORD is pre-R2004");
    }

    #[test]
    fn test_header_roundtrip_r2007() {
        // R2007+ uses three-stream merge (main + text + handle).
        // This test verifies the reader correctly splits the streams,
        // including TEXT values from the separate text sub-stream.
        let mut original = HeaderVariables::default();
        original.fingerprint_guid = "{TEST-GUID-1234}".to_string();
        original.version_guid = "{VERSION-GUID-5678}".to_string();
        original.current_layer_handle = Handle::new(98);
        let written = header_writer::write_header(DxfVersion::AC1021, &original, 0);
        let (read, raw) = read_header(&written, DxfVersion::AC1021, 0).unwrap();

        assert_eq!(raw.version, "AC1021");
        assert!(raw.cmaterial.is_some());
        assert!(raw.dimtfillclr.is_some());
        assert!(raw.cameradisplay.is_some());
        assert!(raw.interferecolor.is_some());
        assert!(raw.shadowplanelocation.is_some());

        // Verify numeric/boolean header variables
        assert_eq!(read.fill_mode, original.fill_mode);
        assert_eq!(read.ortho_mode, original.ortho_mode);
        assert_eq!(
            read.linear_unit_format, original.linear_unit_format,
            "LUNITS should survive roundtrip"
        );
        assert_eq!(read.angular_unit_format, original.angular_unit_format);
        assert!(
            (read.text_height - original.text_height).abs() < 1e-10,
            "TEXTSIZE should survive roundtrip: got {} expected {}",
            read.text_height,
            original.text_height
        );
        assert!((read.linetype_scale - original.linetype_scale).abs() < 1e-10);
        assert_eq!(
            read.attribute_visibility, original.attribute_visibility,
            "ATTMODE should survive roundtrip"
        );
        assert!(
            (read.current_entity_linetype_scale - original.current_entity_linetype_scale).abs()
                < 1e-10,
            "CELTSCALE should survive roundtrip"
        );
        assert_eq!(read.insertion_units, original.insertion_units);
        assert_eq!(read.spline_segments, original.spline_segments);
        assert_eq!(read.sort_entities, original.sort_entities);
        // Verify TEXT values survive the three-stream roundtrip (these go in the
        // separate text sub-stream in R2007+, not inline in main).
        assert_eq!(
            read.fingerprint_guid, original.fingerprint_guid,
            "FINGERPRINTGUID should survive three-stream roundtrip"
        );
        assert_eq!(
            read.version_guid, original.version_guid,
            "VERSIONGUID should survive three-stream roundtrip"
        );
        assert_eq!(read.current_layer_handle, original.current_layer_handle);
    }

    #[test]
    fn test_header_roundtrip_r2010() {
        let mut original = HeaderVariables::default();
        original.current_layer_handle = Handle::new(98);
        let written = header_writer::write_header(DxfVersion::AC1024, &original, 0);
        let (read, raw) = read_header(&written, DxfVersion::AC1024, 0).unwrap();

        assert_eq!(raw.version, "AC1024");
        assert!(raw.dimtxtdirection.is_some());
        assert!(raw.dimaltmzf.is_some());
        assert!(raw.unknown_20.is_none(), "unknown_20 is R2013+");

        assert_eq!(read.fill_mode, original.fill_mode);
        assert_eq!(read.linear_unit_format, original.linear_unit_format);
        assert!((read.text_height - original.text_height).abs() < 1e-10);
        assert_eq!(read.attribute_visibility, original.attribute_visibility);
        assert_eq!(read.current_layer_handle, original.current_layer_handle);
    }

    #[test]
    fn test_header_bad_sentinel_fails() {
        let mut bad_data = vec![0u8; 50];
        bad_data[..16].fill(0xFF);
        let result = read_header(&bad_data, DxfVersion::AC1015, 0);
        assert!(result.is_err());
    }
}
