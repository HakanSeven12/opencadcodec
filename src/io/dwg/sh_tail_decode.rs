//! Phase B tail decoders for the raw-retained solid-history node classes.
//!
//! The four raw-retained tails (`SolidHistorySweep::shsw_raw_tail` for
//! ACSH_SWEEP_CLASS and ACSH_EXTRUSION_CLASS, `SolidHistoryLoft::raw_tail`,
//! `SolidHistoryRevolve::raw_tail`) are captured verbatim by the reader
//! (Phase A); these decoders project the SEMANTIC content of those bits
//! into the typed tail views on the model. The classes are DEBUGGING
//! classes in gold — there is no oracle walk for their payloads — so
//! every anchor here is derived from the two non-oracle instruments
//! documented in IMPLEMENTATION.md §18.5:
//!
//! 1. Cross-specimen comparison: all four fixtures per family carry
//!    bit-identical tails across DWG 2007/2010/2013/2018, so the shapes
//!    below are version-portable statements about the wires themselves.
//! 2. The R2010 live-oracle fragment: where a tail entry overlaps values
//!    gold parses elsewhere (the 3DSOLID wireframe anchor read from the
//!    same modeler backing), the gold-parsed value names the semantics
//!    (Polysolid: direction unit components and the doubled segment end;
//!    Extrude: the (0, 0, 2.0) extrusion length vector; Loft: the 5.0
//!    top height with the two 90-degree draft angles).
//!
//! Write rule: the captured raw tail is the authority and re-emitted
//! verbatim; the decoders merely supply a typed view. A writer-token
//! re-encode exists for the raw LE64 spans (same bit length always):
//! `render_*_tail` splices the current model values into the stored bits
//! only where they differ from the decode, so a programmatic field edit
//! lands bit-locally while an untouched record stays byte-identical.

use crate::objects::{
    SolidHistoryLoftSection, SolidHistoryLoftTail, SolidHistoryProfileCall,
    SolidHistoryProfileCircle, SolidHistoryProfilePolyline, SolidHistoryRevolveTail,
    SolidHistorySweepTail,
};

/// A bit range within a tail, MSB-packed ([start, start+len)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub start: u32,
    pub len: u32,
}

/// MSB-packed read/write window over the captured tail bytes.
struct TailBits {
    bytes: Vec<u8>,
    bit_len: u32,
}

impl TailBits {
    fn new(bytes: &[u8], bit_len: u32) -> Self {
        Self {
            bytes: bytes.to_vec(),
            bit_len,
        }
    }

    fn get(&self, index: u32) -> Option<u8> {
        if index >= self.bit_len {
            return None;
        }
        Some((self.bytes[(index / 8) as usize] >> (7 - index % 8)) & 1)
    }

    fn code(&self, index: u32) -> Option<u8> {
        Some((self.get(index)? << 1) | self.get(index + 1)?)
    }

    /// Raw LE64 double at `start`: the eight wire bytes (each MSB-first on
    /// the DWG bit stream) are the double's little-endian bytes in
    /// transmission order (the same convention as `DwgBitReader::read_raw_double`).
    fn le64(&self, start: u32) -> Option<f64> {
        if start.checked_add(64)? > self.bit_len {
            return None;
        }
        let mut raw = [0u8; 8];
        for (j, byte) in raw.iter_mut().enumerate() {
            for k in 0..8 {
                // Wire bit (start + 8j) is the byte's MSB (weight 2^7).
                *byte |= self.get(start + (j as u32) * 8 + (7 - k as u32))? << k;
            }
        }
        Some(f64::from_bits(u64::from_le_bytes(raw)))
    }

    fn set_le64(&mut self, start: u32, value: f64) -> bool {
        if start.checked_add(64).map_or(true, |end| end > self.bit_len) {
            return false;
        }
        let raw = value.to_bits().to_le_bytes();
        for (j, byte) in raw.iter().enumerate() {
            for k in 0..8 {
                let bit = (byte >> k) & 1;
                let index = start + (j as u32) * 8 + (7 - k as u32);
                let byte_slot = &mut self.bytes[(index / 8) as usize];
                let mask = 1 << (7 - index % 8);
                if bit == 1 {
                    *byte_slot |= mask;
                } else {
                    *byte_slot &= !mask;
                }
            }
        }
        true
    }

    fn set_pair(&mut self, start: u32, code: u8) -> bool {
        if start.checked_add(2).map_or(true, |end| end > self.bit_len) {
            return false;
        }
        for (offset, bit) in [(0, code >> 1), (1, code & 1)] {
            let byte = &mut self.bytes[((start + offset) / 8) as usize];
            let mask = 1 << (7 - (start + offset) % 8);
            if bit == 1 {
                *byte |= mask;
            } else {
                *byte &= !mask;
            }
        }
        true
    }

    /// Byte-aligned plausible-double run starting at `start` (start must
    /// itself be byte aligned). Returns (positions, values) until the first
    /// implausible entry.
    fn aligned_run(&self, start: u32) -> (Vec<u32>, Vec<f64>) {
        debug_assert_eq!(start % 8, 0);
        let mut positions = Vec::new();
        let mut values = Vec::new();
        let mut pos = start;
        while let Some(value) = self.le64(pos) {
            if !plausible(value) {
                break;
            }
            positions.push(pos);
            values.push(value);
            pos += 64;
        }
        (positions, values)
    }
}

/// IEEE-plausibility gate used by the collectors: finite, not denormal
/// junk and inside a sane modeling range. `0.0` is accepted (raw zero
/// runs appear inside byte-aligned geometry blocks).
fn plausible(value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    let abs = value.abs();
    if abs == 0.0 {
        return true;
    }
    (1e-4..=1e12).contains(&abs)
}

/// Plausibility gate for raw BD ('00'-marked) doubles: a wire writer
/// encodes an exact `0.0` as the short BD form, so a raw-marked zero is
/// always a misaligned alias, not an entry.
fn plausible_raw(value: f64) -> bool {
    plausible(value) && value != 0.0
}

/// Decoded BD form with its bit span.
#[derive(Debug, Clone, Copy)]
struct BdEntry {
    value: f64,
    span: Span,
}

/// Read one BD at `pos`. Returns the entry or `None` when the form does
/// not fit the window. The reserved '11' code decodes as 0.0 (matching
/// the bitcode table) so callers decide whether it is still acceptable.
fn read_bd(bits: &TailBits, pos: u32) -> Option<BdEntry> {
    let code = bits.code(pos)?;
    match code {
        0b00 => {
            let value = bits.le64(pos + 2)?;
            Some(BdEntry {
                value,
                span: Span {
                    start: pos,
                    len: 66,
                },
            })
        }
        0b01 => Some(BdEntry {
            value: 1.0,
            span: Span { start: pos, len: 2 },
        }),
        0b10 => Some(BdEntry {
            value: 0.0,
            span: Span { start: pos, len: 2 },
        }),
        _ => Some(BdEntry {
            value: 0.0,
            span: Span { start: pos, len: 2 },
        }),
    }
}

/// Maximum entries collected in a head run (shape guard).
const MAX_HEAD: usize = 16;
/// Maximum raw BD entries collected (shape guard).
const MAX_RAWS: usize = 64;

/// Sweep/revolve shared head: BD shorts until the first raw/reserved
/// form. Returns the values with their spans and the end position.
fn read_head_shorts(bits: &TailBits, start: u32) -> (Vec<f64>, Vec<Span>, u32) {
    let mut values = Vec::new();
    let mut spans = Vec::new();
    let mut pos = Some(start);
    while let Some(position) = pos {
        if values.len() >= MAX_HEAD {
            break;
        }
        match bits.code(position) {
            Some(0b01) => {
                values.push(1.0);
                spans.push(Span {
                    start: position,
                    len: 2,
                });
                pos = Some(position + 2);
            }
            Some(0b10) => {
                values.push(0.0);
                spans.push(Span {
                    start: position,
                    len: 2,
                });
                pos = Some(position + 2);
            }
            _ => break,
        }
    }
    let end = spans.last().map_or(start, |span| span.start + span.len);
    (values, spans, end)
}

/// The FIRST '00'-framed plausible BD frame fully inside `[start, bound)`,
/// scanning at 2-bit offsets (the post-corner singles walk, §18.7). A
/// zero value under a raw marker is a misaligned alias, not an entry —
/// the same rule `scan_raws` applies.
fn first_plausible_frame(bits: &TailBits, start: u32, bound: u32) -> (Option<f64>, Option<Span>) {
    let mut pos = start;
    while pos + 66 <= bound {
        if bits.code(pos) == Some(0b00) {
            if let Some(value) = bits.le64(pos + 2) {
                if plausible_raw(value) {
                    return (
                        Some(value),
                        Some(Span {
                            start: pos,
                            len: 66,
                        }),
                    );
                }
            }
        }
        pos += 2;
    }
    (None, None)
}

/// Sweep-family frame entries: raw BD doubles found at '00' markers
/// between `start` and `bound` (exclusive), advancing 2 bits at a time
/// when a marker does not decode to a plausible raw double.
fn scan_raws(bits: &TailBits, start: u32, bound: u32) -> (Vec<f64>, Vec<Span>) {
    let mut values = Vec::new();
    let mut spans = Vec::new();
    let mut pos = start;
    while values.len() < MAX_RAWS && pos + 66 <= bound {
        if bits.code(pos) == Some(0b00) {
            if let Some(value) = bits.le64(pos + 2) {
                if plausible_raw(value) {
                    values.push(value);
                    spans.push(Span {
                        start: pos + 2,
                        len: 64,
                    });
                    pos += 66;
                    continue;
                }
            }
        }
        pos += 2;
    }
    (values, spans)
}

/// Byte-aligned plausible-double runs with at least `min_entries` members.
fn aligned_runs(bits: &TailBits, min_entries: usize, max_runs: usize) -> Vec<(Vec<u32>, Vec<f64>)> {
    let mut runs = Vec::new();
    let mut pos = 0u32;
    while pos + 64 <= bits.bit_len {
        if pos % 8 != 0 {
            pos += 1;
            continue;
        }
        let (positions, values) = bits.aligned_run(pos);
        if positions.len() >= min_entries {
            let next = positions.last().map_or(pos, |last| last + 64);
            runs.push((positions, values));
            if runs.len() >= max_runs {
                break;
            }
            // Resume after this run; `next` is byte aligned (the run
            // started byte aligned and advanced in 8-byte steps).
            pos = next;
        } else {
            pos += 8;
        }
    }
    runs
}

/// Decode spans + view payload of a sweep-family tail.
pub(crate) struct SweepTailInfo {
    pub direction: [f64; 3],
    pub direction_spans: [Span; 3],
    pub view: SolidHistorySweepTail,
    pub spine_spans: [Span; 6],
    pub option_spans: Vec<Span>,
    pub raw_spans: Vec<Span>,
    pub corner_spans: Vec<[Span; 2]>,
    pub record_constant_span: Option<Span>,
    pub post_corner_single_span: Option<Span>,
    pub segment_end_spans: Option<[Span; 2]>,
    pub profile_circle_spans: Option<ProfileCircleSpans>,
    pub profile_polyline_spans: Option<ProfilePolylineSpans>,
}

/// Spans of the extrusion profile CALL's circle body.
pub(crate) struct ProfileCircleSpans {
    pub center: [Span; 3],
    pub radius: Span,
    pub normal: [Span; 3],
}

/// The embedded-entity type code of the polyline profile CALL (the
/// `OBJ_LWPOLYLINE` constant in the object readers' common tables);
/// the circle code `PROFILE_CIRCLE` is defined at the revolve section.
const PROFILE_LWPOLYLINE: i64 = 77;

/// Scan for the extrusion profile CALL `[BL kind][BL bit-length][body]`
/// closing at `bit_len - 2` (the two trailing flag bits). Returns
/// (kind, bit_len, body_start). The pre-CALL region between the spine
/// extras and the CALL stays opaque (profile-dependent, value-stable
/// across the radius/height/taper differentials).
fn scan_profile_call(bits: &TailBits, from: u32) -> Option<(i64, i64, u32)> {
    let target = bits.bit_len.checked_sub(2)?;
    let mut pos = from;
    while pos < target {
        let (kind, after_kind) = read_bl(bits, pos)?;
        if kind != PROFILE_CIRCLE && kind != PROFILE_LWPOLYLINE {
            pos += 2;
            continue;
        }
        if let Some((len, body_start)) = read_bl(bits, after_kind) {
            if len > 0 && body_start.checked_add(len as u32)? == target {
                return Some((kind, len, body_start));
            }
        }
        pos += 2;
    }
    None
}

/// The polyline CALL body's value-spans (the §18 ExtrudeP walk),
/// parallel to `SolidHistoryProfilePolyline`'s raw fields.
pub(crate) struct ProfilePolylineSpans {
    pub flag_span: Span,
    pub num_points_span: Span,
    /// Per-vertex (x, y) value-spans, parallel to the points vector.
    pub point_spans: Vec<[Span; 2]>,
    pub bulge_spans: Vec<Span>,
    /// The body's trailing reserved pair (the '11' BD-terminator of
    /// the embedded window) — validated, never spliced.
    pub terminator_span: Span,
}

/// One MSB-first wire byte at `pos`.
fn read_bits_byte(bits: &TailBits, pos: u32) -> Option<u8> {
    let mut byte = 0u8;
    for k in 0..8u32 {
        let bit = bits.get(pos + (7 - k))?;
        byte |= bit << k;
    }
    Some(byte)
}

/// Read one BS at `pos` (all four bitcode forms).
fn read_bs(bits: &TailBits, pos: u32) -> Option<(i32, u32)> {
    match bits.code(pos)? {
        0b00 => {
            if pos.checked_add(18)? > bits.bit_len {
                return None;
            }
            let mut value = 0u16;
            for j in 0..2u32 {
                let byte = read_bits_byte(bits, pos + 2 + j * 8)?;
                value |= u16::from(byte) << (8 * j);
            }
            Some((value as i16 as i32, pos + 18))
        }
        0b01 => {
            if pos.checked_add(10)? > bits.bit_len {
                return None;
            }
            let byte = read_bits_byte(bits, pos + 2)?;
            Some((i32::from(byte), pos + 10))
        }
        0b10 => Some((0, pos + 2)),
        _ => Some((256, pos + 2)),
    }
}

/// Parse the polyline CALL body kind 77 through the embedded-LWPOLYLINE
/// grammar (the §18 ExtrudeP walk — the same field order as
/// `read_embedded_lwpolyline` in the object readers, raw-points arms):
/// `[BS flag][flag-gated BD fields][BL num_points][BL num_bulges
/// if flag & 16][BL num_vertex_ids if flag & 0x400][BL num_widths if
/// flag & 0x20][raw LE64 (x, y) pairs][BD bulges][BL vertex ids]
/// [BD (start, end) width pairs]` plus the corpus-observed trailing
/// reserved '11' pair that closes the consumed window. Requires the
/// parse to consume exactly `bit_len - 2` bits (the two in-window
/// flag bits follow, per the circle CALL convention).
fn parse_sweep_profile_polyline(
    bits: &TailBits,
    body_start: u32,
    call_len: i64,
) -> Option<(SolidHistoryProfilePolyline, ProfilePolylineSpans)> {
    let body_end = body_start.checked_add((call_len - 2) as u32)?;
    let mut pos = body_start;
    let mut view = SolidHistoryProfilePolyline::default();
    let mut spans = ProfilePolylineSpans {
        flag_span: Span { start: 0, len: 0 },
        num_points_span: Span { start: 0, len: 0 },
        point_spans: Vec::new(),
        bulge_spans: Vec::new(),
        terminator_span: Span { start: 0, len: 0 },
    };
    // [BS flag] — 512 closed on the corpus specimen.
    let (flag, next) = read_bs(bits, pos)?;
    view.flag = flag;
    spans.flag_span = Span {
        start: pos,
        len: next - pos,
    };
    pos = next;
    // The flag-gated optional doubles (the common LWPOLYLINE header).
    if flag & 0x4 != 0 {
        let entry = read_bd(bits, pos)?;
        let _ = entry.value;
        pos = entry.span.start + entry.span.len;
    }
    if flag & 0x8 != 0 {
        let entry = read_bd(bits, pos)?;
        let _ = entry.value;
        pos = entry.span.start + entry.span.len;
    }
    if flag & 0x2 != 0 {
        let entry = read_bd(bits, pos)?;
        let _ = entry.value;
        pos = entry.span.start + entry.span.len;
    }
    if flag & 0x1 != 0 {
        for _ in 0..3 {
            let entry = read_bd(bits, pos)?;
            pos = entry.span.start + entry.span.len;
        }
    }
    // [BL num_points]
    let (num_points, next) = read_bl(bits, pos)?;
    if !(0..=20000).contains(&num_points) {
        return None;
    }
    view.num_points = num_points;
    spans.num_points_span = Span {
        start: pos,
        len: next - pos,
    };
    pos = next;
    let mut num_bulges = 0i64;
    if flag & 0x10 != 0 {
        let (value, next) = read_bl(bits, pos)?;
        if !(0..=20000).contains(&value) {
            return None;
        }
        num_bulges = value;
        pos = next;
    }
    let mut num_vertex_ids = 0i64;
    if flag & 0x400 != 0 {
        let (value, next) = read_bl(bits, pos)?;
        if !(0..=20000).contains(&value) {
            return None;
        }
        num_vertex_ids = value;
        pos = next;
    }
    let mut num_widths = 0i64;
    if flag & 0x20 != 0 {
        let (value, next) = read_bl(bits, pos)?;
        if !(0..=20000).contains(&value) {
            return None;
        }
        num_widths = value;
        pos = next;
    }
    // The raw (x, y) vertex pairs — both components as plain LE64
    // doubles (the raw-points arm; no BD codes on the wire).
    for _ in 0..num_points {
        if pos.checked_add(128)? > bits.bit_len {
            return None;
        }
        let x = bits.le64(pos)?;
        let y = bits.le64(pos + 64)?;
        view.points.push([x, y]);
        spans.point_spans.push([
            Span {
                start: pos,
                len: 64,
            },
            Span {
                start: pos + 64,
                len: 64,
            },
        ]);
        pos += 128;
    }
    for _ in 0..num_bulges {
        let entry = read_bd(bits, pos)?;
        view.bulges.push(entry.value);
        spans.bulge_spans.push(entry.span);
        pos = entry.span.start + entry.span.len;
    }
    for _ in 0..num_vertex_ids {
        let (_, next) = read_bl(bits, pos)?;
        if next > bits.bit_len {
            return None;
        }
        pos = next;
    }
    for _ in 0..num_widths {
        for _ in 0..2 {
            let entry = read_bd(bits, pos)?;
            pos = entry.span.start + entry.span.len;
        }
    }
    // The corpus-observed trailing reserved pair (the '11' code, the
    // reserved BD form — the embedded window's terminator on the
    // ExtrudeP quad): validated, nameless.
    if bits.code(pos) == Some(0b11) {
        spans.terminator_span = Span { start: pos, len: 2 };
        pos += 2;
    }
    if pos != body_end {
        return None;
    }
    Some((view, spans))
}

/// Parse the circle CALL body `[center 3BD][radius BD][normal 3BD]`,
/// requiring it to consume exactly `bit_len - 2` bits (the two
/// in-window flag bits follow).
fn parse_sweep_profile_circle(
    bits: &TailBits,
    body_start: u32,
    call_len: i64,
) -> Option<(SolidHistoryProfileCircle, ProfileCircleSpans)> {
    let body_end = body_start.checked_add((call_len - 2) as u32)?;
    let mut pos = body_start;
    let mut center = [0.0f64; 3];
    let mut center_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        let entry = read_bd(bits, pos)?;
        center[axis] = entry.value;
        center_spans[axis] = entry.span;
        pos = entry.span.start + entry.span.len;
    }
    let radius = read_bd(bits, pos)?;
    let radius_span = radius.span;
    pos = radius.span.start + radius.span.len;
    let mut normal = [0.0f64; 3];
    let mut normal_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        let entry = read_bd(bits, pos)?;
        normal[axis] = entry.value;
        normal_spans[axis] = entry.span;
        pos = entry.span.start + entry.span.len;
    }
    if pos != body_end {
        return None;
    }
    Some((
        SolidHistoryProfileCircle {
            center,
            radius: radius.value,
            normal,
        },
        ProfileCircleSpans {
            center: center_spans,
            radius: radius_span,
            normal: normal_spans,
        },
    ))
}

/// Decode a captured sweep/extrusion tail (ACSH_SWEEP_CLASS /
/// ACSH_EXTRUSION_CLASS). Returns `None` when the anchor layout does not
/// hold (tail shorter than a direction + the six-slot spine) — the
/// record then keeps its verbatim-only behavior.
pub(crate) fn decode_sweep_tail(bytes: &[u8], bit_len: u32) -> Option<SweepTailInfo> {
    if bit_len < 6 || bytes.len() * 8 < bit_len as usize {
        return None;
    }
    let bits = TailBits::new(bytes, bit_len);
    let mut pos = 0;
    let mut direction = [0.0; 3];
    let mut direction_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        // All four BD codes are legal for a direction component; the
        // reserved code decodes as 0.0 per the bitcode table.
        let entry = read_bd(&bits, pos)?;
        direction[axis] = entry.value;
        direction_spans[axis] = entry.span;
        pos += entry.span.len;
    }
    // The six sweep-option BDs, in the SweepOptions order (§18.7
    // ExtrudeT differential: the draft angle lands raw in slot 0; the
    // 1.0 default sits in the scale slot 4):
    // [draft_angle][draft_start_distance][draft_end_distance]
    // [twist_angle][scale_factor][align_angle].
    let mut spine = [0.0f64; 6];
    let mut spine_spans = [Span { start: 0, len: 0 }; 6];
    for slot in 0..6 {
        let entry = read_bd(&bits, pos)?;
        spine[slot] = entry.value;
        spine_spans[slot] = entry.span;
        pos = entry.span.start + entry.span.len;
    }
    // All-short extras after the named spine (the Polysolid sweep
    // carries two trailing zeros; extrusion tails stop at the reserved
    // pair that opens the pre-CALL region). The profile CALL chain is
    // probed at every step: the CALL's '01' BL marker is also a legal
    // short BD, so an extrusion tail whose CALL follows the spine
    // directly must not have it consumed as an extra.
    let mut extras: Vec<f64> = Vec::new();
    let mut option_spans: Vec<Span> = Vec::new();
    let mut extras_end = pos;
    let mut profile_chain = scan_profile_call(&bits, pos);
    while profile_chain.is_none() && extras.len() < MAX_HEAD {
        match bits.code(extras_end) {
            Some(0b01) => {
                extras.push(1.0);
                option_spans.push(Span {
                    start: extras_end,
                    len: 2,
                });
                extras_end += 2;
            }
            Some(0b10) => {
                extras.push(0.0);
                option_spans.push(Span {
                    start: extras_end,
                    len: 2,
                });
                extras_end += 2;
            }
            _ => break,
        }
        profile_chain = scan_profile_call(&bits, extras_end);
    }
    // Byte-aligned geometry runs bound the raw-marker scan: the sweep
    // profile corners are a run of >= 4 byte-aligned LE64 doubles, and no
    // raw BD frame entry can live inside one.
    let runs = aligned_runs(&bits, 4, 4);
    let raw_bound = runs.first().map_or(bit_len, |(positions, _)| positions[0]);
    let (raw_values, raw_spans) = scan_raws(&bits, extras_end, raw_bound);
    let mut view = SolidHistorySweepTail {
        draft_angle: Some(spine[0]),
        draft_start_distance: Some(spine[1]),
        draft_end_distance: Some(spine[2]),
        twist_angle: Some(spine[3]),
        scale_factor: Some(spine[4]),
        align_angle: Some(spine[5]),
        option_doubles: extras,
        raw_doubles: raw_values,
        profile_corners: Vec::new(),
        record_constant: None,
        post_corner_single: None,
        segment_end: None,
        profile: None,
    };
    let mut corner_spans = Vec::new();
    let mut record_constant_span = None;
    let mut post_corner_single_span = None;
    if let Some((positions, values)) = runs.first() {
        // Pair the (x, height) corner entries. A single trailing unpaired
        // entry is the record constant (the post-corner singles walk,
        // §18.7: 4.00024414192312 — bit-identical across every corpus
        // Polysolid profile and across the §18.7 confirmation quads);
        // more than one unpaired entry keeps the verbatim-only shape.
        let pairs = values.len() / 2;
        for pair in 0..pairs {
            let x = values[pair * 2];
            let height = values[pair * 2 + 1];
            view.profile_corners.push([x, height]);
            corner_spans.push([
                Span {
                    start: positions[pair * 2],
                    len: 64,
                },
                Span {
                    start: positions[pair * 2 + 1],
                    len: 64,
                },
            ]);
        }
        if values.len() % 2 == 1 {
            view.record_constant = values.last().copied();
            record_constant_span = Some(Span {
                start: positions[positions.len() - 1],
                len: 64,
            });
        }
    }
    // The post-corner single (§18.7's singles stream): the gap between
    // the corner block (incl. its record-constant tail entry) and the
    // segment-end block carries one '00'-marked BD frame — four zero
    // pad bits ahead of it, a short '10' pair after it. The FIRST
    // plausible frame fully inside the gap is the frame: earlier even
    // offsets alias into the zero pad (the scan-level "singles" of the
    // §18.7 P-probes — "2.0109", the "width single" 3.0/7.0, the
    // 8.06-class reads — all derive from the same zero-heavy bits and
    // never pin a span).
    if let (Some(first), Some(second)) = (runs.first(), runs.get(1)) {
        let gap_start = first.0.last().map_or(0, |p| p + 64);
        let gap_end = second.0[0];
        if let (Some(value), span) = first_plausible_frame(&bits, gap_start, gap_end) {
            view.post_corner_single = Some(value);
            post_corner_single_span = span;
        }
    }
    let mut segment_end_spans = None;
    if let Some((positions, values)) = runs.last() {
        if positions.len() >= 2 && values.len() >= 2 {
            // The final raw pair must end within the last 16 bits of the
            // tail (the capture window closes on the segment end).
            let last_end = positions[positions.len() - 1] + 64;
            if last_end.saturating_add(16) >= bit_len {
                let x = values[values.len() - 2];
                let y = values[values.len() - 1];
                view.segment_end = Some([x, y]);
                segment_end_spans = Some([
                    Span {
                        start: positions[positions.len() - 2],
                        len: 64,
                    },
                    Span {
                        start: positions[positions.len() - 1],
                        len: 64,
                    },
                ]);
            }
        }
    }
    // The extrusion profile CALL (§18.7 ExtrudeR/P evidence): sweep
    // tails carry none — their profile is the packed corners above.
    let mut profile_circle_spans = None;
    let mut profile_polyline_spans = None;
    if let Some((kind, call_len, body_start)) = profile_chain {
        let mut circle = None;
        let mut polyline = None;
        if kind == PROFILE_CIRCLE {
            if let Some((parsed, spans)) = parse_sweep_profile_circle(&bits, body_start, call_len) {
                circle = Some(parsed);
                profile_circle_spans = Some(spans);
            } else {
                // The chain closed but the body did not parse as a
                // circle: keep the CALL presence without the typed
                // circle fields (positional only).
                view.profile = Some(SolidHistoryProfileCall {
                    kind,
                    bit_len: call_len,
                    circle: None,
                    polyline: None,
                });
                return Some(SweepTailInfo {
                    direction,
                    direction_spans,
                    view,
                    spine_spans,
                    option_spans,
                    raw_spans,
                    corner_spans,
                    record_constant_span,
                    post_corner_single_span,
                    segment_end_spans,
                    profile_circle_spans: None,
                    profile_polyline_spans: None,
                });
            }
        } else if kind == PROFILE_LWPOLYLINE {
            // The §18 ExtrudeP walk: the kind-77 body decodes through
            // the embedded-LWPOLYLINE grammar; a body that does not
            // close the window exactly keeps the CALL positional.
            if let Some((parsed, spans)) =
                parse_sweep_profile_polyline(&bits, body_start, call_len)
            {
                polyline = Some(parsed);
                profile_polyline_spans = Some(spans);
            }
        }
        view.profile = Some(SolidHistoryProfileCall {
            kind,
            bit_len: call_len,
            circle,
            polyline,
        });
    }
    Some(SweepTailInfo {
        direction,
        direction_spans,
        view,
        spine_spans,
        option_spans,
        raw_spans,
        corner_spans,
        record_constant_span,
        post_corner_single_span,
        segment_end_spans,
        profile_circle_spans,
        profile_polyline_spans,
    })
}

/// Decode spans + view of a loft tail.
pub(crate) struct LoftTailInfo {
    pub view: SolidHistoryLoftTail,
    pub option_spans: Vec<Span>,
    pub raw_spans: Vec<Span>,
    /// The per-section field value-spans, parallel to
    /// `view.sections` (the §18 loft container walk). Raw frames
    /// only — an elided canonical field has no frame and no span;
    /// a future same-form splice target uses these spans.
    pub section_spans: Vec<LoftSectionSpans>,
    /// The draft-angle pair's value-spans (the final two frames).
    pub draft_angle_spans: [Option<Span>; 2],
}

/// The per-section field spans of the loft walk (parallel to
/// `SolidHistoryLoftSection`).
pub(crate) struct LoftSectionSpans {
    pub center: [Option<Span>; 2],
    pub height: Option<Span>,
    pub radius: Option<Span>,
}

/// The §18 loft container walk over the raw frame stream: the
/// frames group into contiguous runs (a 2-bit gap between frames is
/// plain adjacency; a 4-bit gap is an elided canonical short field
/// between them; anything wider is a section boundary), the FINAL
/// 2-frame run names the draft-angle pair, and every earlier run
/// attributes its frames to the closed §18.6/§18.7 per-section
/// reading `[center.x][center.y][height][radius]`:
/// 4 frames → all four fields; 3 frames with the wide gap after the
/// second → `[cx][cy][elided z][r]` (LoftC's section 1); 2 frames →
/// `[z][r]` (the origin-section center elision: LoftR); 1 frame →
/// `[z]` (radius 1.0 elided too: LoftH/Loft3). Any group shape
/// outside the observed families leaves the whole tail positional
/// (nothing named, the verbatim bits stay the write authority).
fn walk_loft_sections(
    raw_values: &[f64],
    raw_spans: &[Span],
) -> (
    Vec<SolidHistoryLoftSection>,
    Vec<LoftSectionSpans>,
    [Option<f64>; 2],
    [Option<Span>; 2],
) {
    let sections = Vec::new();
    let section_spans = Vec::new();
    let mut draft_angles = [None, None];
    let mut draft_spans = [None, None];
    if raw_spans.is_empty() {
        return (sections, section_spans, draft_angles, draft_spans);
    }
    // Group the frame indices by gap width between consecutive frames
    // (value-span arithmetic: a contiguous frame starts marker-first, so
    // an adjacent next value sits at gap 2; an elided short widens it
    // to 4; wider gaps are section boundaries).
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current: Vec<usize> = vec![0];
    for i in 1..raw_spans.len() {
        let gap =
            raw_spans[i].start.saturating_sub(raw_spans[i - 1].start + raw_spans[i - 1].len);
        if gap > 4 {
            groups.push(std::mem::take(&mut current));
            current = vec![i];
        } else {
            current.push(i);
        }
    }
    groups.push(current);
    // The terminal 2-frame group is the draft pair; an unexpected
    // terminal shape (or no earlier group) keeps everything positional.
    let last = groups.last().cloned().unwrap_or_default();
    if last.len() != 2 || groups.len() < 2 {
        return (sections, section_spans, draft_angles, draft_spans);
    }
    for (slot, frame) in last.iter().enumerate() {
        draft_angles[slot] = Some(raw_values[*frame]);
        draft_spans[slot] = Some(raw_spans[*frame]);
    }
    groups.pop();
    // Attribute every section group; any unexpected shape aborts the
    // whole naming (the conservative decode-fidelity rule).
    let mut named: Vec<SolidHistoryLoftSection> = Vec::new();
    let mut named_spans: Vec<LoftSectionSpans> = Vec::new();
    for group in &groups {
        let spans: Vec<&Span> = group.iter().map(|i| &raw_spans[*i]).collect();
        match group.len() {
            4 if spans.windows(2).all(|w| w[1].start == w[0].start + w[0].len + 2) => {
                named.push(SolidHistoryLoftSection {
                    center: [Some(raw_values[group[0]]), Some(raw_values[group[1]])],
                    height: Some(raw_values[group[2]]),
                    radius: Some(raw_values[group[3]]),
                });
                named_spans.push(LoftSectionSpans {
                    center: [Some(*spans[0]), Some(*spans[1])],
                    height: Some(*spans[2]),
                    radius: Some(*spans[3]),
                });
            }
            3 if spans[1].start == spans[0].start + spans[0].len + 2
                && spans[2].start == spans[1].start + spans[1].len + 4 =>
            {
                // [cx][cy][elided z short][r] — the 4-bit gap carries
                // the canonical z=0.0 elision between the frames
                // (LoftC's section 1: (3, 4, z elided, 1.5)).
                named.push(SolidHistoryLoftSection {
                    center: [Some(raw_values[group[0]]), Some(raw_values[group[1]])],
                    height: None,
                    radius: Some(raw_values[group[2]]),
                });
                named_spans.push(LoftSectionSpans {
                    center: [Some(*spans[0]), Some(*spans[1])],
                    height: None,
                    radius: Some(*spans[2]),
                });
            }
            2 if spans[1].start == spans[0].start + spans[0].len + 2 => {
                // [z][r] — the origin-section center elision (LoftR:
                // (0, 0) elided as shorts in the leading region).
                named.push(SolidHistoryLoftSection {
                    center: [None, None],
                    height: Some(raw_values[group[0]]),
                    radius: Some(raw_values[group[1]]),
                });
                named_spans.push(LoftSectionSpans {
                    center: [None, None],
                    height: Some(*spans[0]),
                    radius: Some(*spans[1]),
                });
            }
            1 => {
                // [z] alone — the r=1.0 elision after the height frame
                // (LoftH: (0, 0, 7, 1 elided); Loft3's mid sections).
                named.push(SolidHistoryLoftSection {
                    center: [None, None],
                    height: Some(raw_values[group[0]]),
                    radius: None,
                });
                named_spans.push(LoftSectionSpans {
                    center: [None, None],
                    height: Some(*spans[0]),
                    radius: None,
                });
            }
            _ => {
                return (Vec::new(), Vec::new(), [None, None], [None, None]);
            }
        }
    }
    (named, named_spans, draft_angles, draft_spans)
}

pub(crate) fn decode_loft_tail(bytes: &[u8], bit_len: u32) -> Option<LoftTailInfo> {
    if bit_len < 2 || bytes.len() * 8 < bit_len as usize {
        return None;
    }
    let bits = TailBits::new(bytes, bit_len);
    let (options, option_spans, head_end) = read_head_shorts(&bits, 0);
    if options.is_empty() {
        return None;
    }
    let (raw_values, raw_spans) = scan_raws(&bits, head_end, bit_len);
    let (sections, section_spans, draft_angles, draft_angle_spans) =
        walk_loft_sections(&raw_values, &raw_spans);
    Some(LoftTailInfo {
        view: SolidHistoryLoftTail {
            option_doubles: options,
            sections,
            draft_angles,
            raw_doubles: raw_values,
        },
        option_spans,
        raw_spans,
        section_spans,
        draft_angle_spans,
    })
}

/// Decode spans + view of a revolve tail.
pub(crate) struct RevolveTailInfo {
    pub view: SolidHistoryRevolveTail,
    pub axis_point_spans: [Span; 3],
    pub axis_vector_spans: [Span; 3],
    pub angle_span: Option<Span>,
    pub option_spans: Vec<Span>,
    pub center_spans: Option<[Span; 3]>,
    pub radius_span: Option<Span>,
    pub normal_spans: Option<[Span; 3]>,
}

/// The embedded profile sub-entity's type code for a circle (the
/// `OBJ_CIRCLE` constant in the object readers' common tables; gold
/// records embedded sub-entities through a CALL:
/// [BL type][BL bit-length][body]).
const PROFILE_CIRCLE: i64 = 18;

/// Read one BL at `pos` (all four bitcode forms).
fn read_bl(bits: &TailBits, pos: u32) -> Option<(i64, u32)> {
    match bits.code(pos)? {
        0b00 => {
            if pos.checked_add(34)? > bits.bit_len {
                return None;
            }
            // Four wire bytes (each MSB-first), little-endian order -
            // the same convention as le64.
            let mut value = 0u32;
            for j in 0..4u32 {
                let mut byte = 0u8;
                for k in 0..8u32 {
                    let bit = bits.get(pos + 2 + j * 8 + (7 - k))?;
                    byte |= u8::from(bit) << k;
                }
                value |= u32::from(byte) << (8 * j);
            }
            Some((value as i32 as i64, pos + 34))
        }
        0b01 => {
            if pos.checked_add(10)? > bits.bit_len {
                return None;
            }
            // One RC byte, MSB-first on the wire.
            let mut byte = 0u8;
            for k in 0..8u32 {
                let bit = bits.get(pos + 2 + (7 - k))?;
                byte |= u8::from(bit) << k;
            }
            Some((byte as i64, pos + 10))
        }
        0b10 => Some((0, pos + 2)),
        _ => Some((256, pos + 2)),
    }
}

/// The embedded profile circle, parsed through the CALL grammar and
/// REQUIRED to account the tail exactly: [BL type == 18][BL len]
/// followed by [center 3BD][radius BD][normal 3BD] and two flag
/// bits, with the consumed span == len and the tail boundary at
/// bit_len. Return `None` when the chain does not close - the tail
/// then keeps its verbatim-only behavior.
struct ProfileCircle {
    center: [f64; 3],
    center_spans: [Span; 3],
    radius: f64,
    radius_span: Span,
    normal: [f64; 3],
    normal_spans: [Span; 3],
}

fn try_profile_circle(bits: &TailBits, pos: u32) -> Option<ProfileCircle> {
    let (kind, after_type) = read_bl(bits, pos)?;
    if kind != PROFILE_CIRCLE {
        return None;
    }
    let (len, body_start) = read_bl(bits, after_type)?;
    if len <= 0 || body_start.checked_add(len as u32)? != bits.bit_len {
        return None;
    }
    let mut p = body_start;
    let mut center = [0.0f64; 3];
    let mut center_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        let entry = read_bd(bits, p)?;
        center[axis] = entry.value;
        center_spans[axis] = entry.span;
        p = entry.span.start + entry.span.len;
    }
    let radius = read_bd(bits, p)?;
    let radius_span = radius.span;
    p = radius.span.start + radius.span.len;
    let mut normal = [0.0f64; 3];
    let mut normal_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        let entry = read_bd(bits, p)?;
        normal[axis] = entry.value;
        normal_spans[axis] = entry.span;
        p = entry.span.start + entry.span.len;
    }
    // The CALL bit-length covers the circle body plus the two final
    // flag bits (the landed specimens' spans: A/R 144 = 142 body +
    // 2; the original 80 = 78 + 2).
    if p.checked_add(2)? != bits.bit_len {
        return None;
    }
    if p + 2 - body_start != len as u32 {
        return None;
    }
    Some(ProfileCircle {
        center,
        center_spans,
        radius: radius.value,
        radius_span,
        normal,
        normal_spans,
    })
}

pub(crate) fn decode_revolve_tail(bytes: &[u8], bit_len: u32) -> Option<RevolveTailInfo> {
    if bit_len < 14 || bytes.len() * 8 < bit_len as usize {
        return None;
    }
    let bits = TailBits::new(bytes, bit_len);
    let mut pos = 0u32;
    // Head: [axis_point 3BD][axis_vector 3BD] - the REVOLVEDSURFACE
    // twin's field order (axis_point, axis_vector, revolve_angle...).
    let mut axis_point = [0.0f64; 3];
    let mut axis_pt_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        let entry = read_bd(&bits, pos)?;
        axis_point[axis] = entry.value;
        axis_pt_spans[axis] = entry.span;
        pos = entry.span.start + entry.span.len;
    }
    let mut axis_vector = [0.0f64; 3];
    let mut axis_vec_spans = [Span { start: 0, len: 0 }; 3];
    for axis in 0..3 {
        let entry = read_bd(&bits, pos)?;
        axis_vector[axis] = entry.value;
        axis_vec_spans[axis] = entry.span;
        pos = entry.span.start + entry.span.len;
    }
    if pos >= bits.bit_len {
        return None;
    }
    let angle = read_bd(&bits, pos)?;
    let angle_span = angle.span;
    pos = angle.span.start + angle.span.len;
    // Options: all-short BD run between the angle and the profile
    // CALL; at every boundary attempt the CALL chain, which must
    // close the tail exactly (this handles the ambiguity that the
    // CALL's '01' marker is also a legal short BD 1.0).
    let mut options: Vec<f64> = Vec::new();
    let mut option_spans: Vec<Span> = Vec::new();
    let mut profile = try_profile_circle(&bits, pos);
    while profile.is_none() && options.len() < MAX_HEAD {
        match bits.code(pos) {
            Some(0b01) => {
                options.push(1.0);
                option_spans.push(Span { start: pos, len: 2 });
                pos += 2;
            }
            Some(0b10) => {
                options.push(0.0);
                option_spans.push(Span { start: pos, len: 2 });
                pos += 2;
            }
            _ => return None,
        }
        if pos.checked_add(2)? > bits.bit_len - 2 {
            return None;
        }
        profile = try_profile_circle(&bits, pos);
    }
    let profile = profile?;
    Some(RevolveTailInfo {
        view: SolidHistoryRevolveTail {
            axis_point: Some(axis_point),
            axis_vector: Some(axis_vector),
            revolve_angle: Some(angle.value),
            option_doubles: options,
            profile_center: Some(profile.center),
            profile_radius: Some(profile.radius),
            profile_normal: Some(profile.normal),
        },
        axis_point_spans: axis_pt_spans,
        axis_vector_spans: axis_vec_spans,
        angle_span: Some(angle_span),
        option_spans,
        center_spans: Some(profile.center_spans),
        radius_span: Some(profile.radius_span),
        normal_spans: Some(profile.normal_spans),
    })
}

/// Public per-family entry points used by the reader to populate the
/// model's tail views (spans are the writer's business, rebuilt at
/// write time from the same captured bits).
pub fn sweep_tail_view(bytes: &[u8], bit_len: u32) -> Option<SolidHistorySweepTail> {
    decode_sweep_tail(bytes, bit_len).map(|info| info.view)
}

pub fn loft_tail_view(bytes: &[u8], bit_len: u32) -> Option<SolidHistoryLoftTail> {
    decode_loft_tail(bytes, bit_len).map(|info| info.view)
}

pub fn revolve_tail_view(bytes: &[u8], bit_len: u32) -> Option<SolidHistoryRevolveTail> {
    decode_revolve_tail(bytes, bit_len).map(|info| info.view)
}

/// Same-form short-span splice: only 0.0 <-> 1.0 flips are expressible
/// in 2 bits; any other edit would need a length-shifting form change
/// (outside the Phase B write rule) and keeps the stored bits.
fn splice_short(bits: &mut TailBits, span: Span, value: f64) {
    if value == 0.0 {
        let _ = bits.set_pair(span.start, 0b10);
    } else if value == 1.0 {
        let _ = bits.set_pair(span.start, 0b01);
    }
}

/// Re-encode a sweep family tail from the model. Returns the bytes to
/// write: bit-identical to the stored tail when no decoded field was
/// programmatically modified, with only the edited raw spans changed
/// otherwise (the tail — and the record — never changes length).
pub(crate) fn render_sweep_tail(
    bytes: &[u8],
    bit_len: u32,
    direction: [f64; 3],
    view: Option<&SolidHistorySweepTail>,
) -> Option<Vec<u8>> {
    let info = decode_sweep_tail(bytes, bit_len)?;
    let mut bits = TailBits::new(bytes, bit_len);
    for (axis, component) in direction.iter().enumerate() {
        let span = info.direction_spans[axis];
        let raw = span.len == 66;
        if component != &info.direction[axis] {
            if raw {
                // A raw BD span covers the 2-bit marker plus the LE64
                // value; the payload bits start two past the marker.
                let _ = bits.set_le64(span.start + 2, *component);
            } else {
                splice_short(&mut bits, span, *component);
            }
        }
    }
    if let Some(view) = view {
        // The six named spine slots (same-form splices: raw spans take
        // any f64, short spans only the 0.0/1.0 pair flips).
        for (slot, (new, old)) in [
            (view.draft_angle, info.view.draft_angle),
            (view.draft_start_distance, info.view.draft_start_distance),
            (view.draft_end_distance, info.view.draft_end_distance),
            (view.twist_angle, info.view.twist_angle),
            (view.scale_factor, info.view.scale_factor),
            (view.align_angle, info.view.align_angle),
        ]
        .into_iter()
        .enumerate()
        {
            if let (Some(new), Some(old)) = (new, old) {
                if new != old {
                    let span = info.spine_spans[slot];
                    splice_bd_field(&mut bits, span, span.len == 66, new);
                }
            }
        }
        splice_short_run(&mut bits, &info.option_spans, &view.option_doubles, &info.view.option_doubles);
        splice_raw_run(&mut bits, &info.raw_spans, &view.raw_doubles, &info.view.raw_doubles);
        slice_pairs(&mut bits, &info.corner_spans, &view.profile_corners, &info.view.profile_corners);
        // The record constant and the post-corner single: same-form LE64
        // entries, spliced only when a programmatic edit differs from the
        // decoded bits (§18.7 — authored file state by default).
        if let (Some(new), Some(old), Some(span)) =
            (view.record_constant, info.view.record_constant, info.record_constant_span)
        {
            if new != old {
                let _ = bits.set_le64(span.start, new);
            }
        }
        if let (Some(new), Some(old), Some(span)) = (
            view.post_corner_single,
            info.view.post_corner_single,
            info.post_corner_single_span,
        ) {
            if new != old {
                // Raw BD frame: the marker sits at span.start, the value
                // two bits past it.
                let _ = bits.set_le64(span.start + 2, new);
            }
        }
        if let (Some(new), Some(old), Some(spans)) = (
            view.segment_end,
            info.view.segment_end,
            info.segment_end_spans,
        ) {
            if new != old {
                let _ = bits.set_le64(spans[0].start, new[0]);
                let _ = bits.set_le64(spans[1].start, new[1]);
            }
        }
        // The polyline profile fields of the §18 ExtrudeP walk: a
        // vertex edit lands bit-locally in its own 64-bit frame-half
        // (the raw point components; the flag/counts are structural).
        if let (Some(spans), Some(new), Some(old)) = (
            &info.profile_polyline_spans,
            view.profile.as_ref().and_then(|call| call.polyline.as_ref()),
            info.view.profile.as_ref().and_then(|call| call.polyline.as_ref()),
        ) {
            for (index, (new_point, old_point)) in
                new.points.iter().zip(old.points.iter()).enumerate()
            {
                if let Some(span_pair) = spans.point_spans.get(index) {
                    for axis in 0..2 {
                        if new_point[axis] != old_point[axis] {
                            let _ = bits.set_le64(span_pair[axis].start, new_point[axis]);
                        }
                    }
                }
            }
        }
        // The profile circle fields (same-form per component; the CALL's
        // kind/bit-length are structural and never spliced).
        if let (Some(spans), Some(new), Some(old)) = (
            &info.profile_circle_spans,
            view.profile.as_ref().and_then(|call| call.circle.as_ref()),
            info.view.profile.as_ref().and_then(|call| call.circle.as_ref()),
        ) {
            for axis in 0..3 {
                if new.center[axis] != old.center[axis] {
                    let span = spans.center[axis];
                    splice_bd_field(&mut bits, span, span.len == 66, new.center[axis]);
                }
            }
            if new.radius != old.radius {
                let span = spans.radius;
                splice_bd_field(&mut bits, span, span.len == 66, new.radius);
            }
            for axis in 0..3 {
                if new.normal[axis] != old.normal[axis] {
                    let span = spans.normal[axis];
                    splice_bd_field(&mut bits, span, span.len == 66, new.normal[axis]);
                }
            }
        }
    }
    Some(bits.bytes)
}

pub(crate) fn render_loft_tail(
    bytes: &[u8],
    bit_len: u32,
    view: Option<&SolidHistoryLoftTail>,
) -> Option<Vec<u8>> {
    let info = decode_loft_tail(bytes, bit_len)?;
    if view.is_none() {
        return Some(bytes.to_vec());
    }
    let view = view?;
    let mut bits = TailBits::new(bytes, bit_len);
    splice_short_run(&mut bits, &info.option_spans, &view.option_doubles, &info.view.option_doubles);
    splice_raw_run(&mut bits, &info.raw_spans, &view.raw_doubles, &info.view.raw_doubles);
    // The §18 container walk's named fields: same-form LE64 splices for
    // the per-section frames and the draft angles (a programmatic edit
    // of a named field lands bit-locally in its own frame — identical
    // bits to the positional raw run, never a length change).
    for ((new, old), spans) in view
        .sections
        .iter()
        .zip(info.view.sections.iter())
        .zip(info.section_spans.iter())
    {
        for axis in 0..2 {
            if let (Some(new), Some(old)) = (new.center[axis], old.center[axis]) {
                if new != old {
                    if let Some(span) = spans.center[axis] {
                        let _ = bits.set_le64(span.start, new);
                    }
                }
            }
        }
        if let (Some(new), Some(old), Some(span)) = (new.height, old.height, spans.height) {
            if new != old {
                let _ = bits.set_le64(span.start, new);
            }
        }
        if let (Some(new), Some(old), Some(span)) = (new.radius, old.radius, spans.radius) {
            if new != old {
                let _ = bits.set_le64(span.start, new);
            }
        }
    }
    for slot in 0..2 {
        if let (Some(new), Some(old), Some(span)) = (
            view.draft_angles[slot],
            info.view.draft_angles[slot],
            info.draft_angle_spans[slot],
        ) {
            if new != old {
                let _ = bits.set_le64(span.start, new);
            }
        }
    }
    Some(bits.bytes)
}

pub(crate) fn render_revolve_tail(
    bytes: &[u8],
    bit_len: u32,
    view: Option<&SolidHistoryRevolveTail>,
) -> Option<Vec<u8>> {
    let info = decode_revolve_tail(bytes, bit_len)?;
    if view.is_none() {
        return Some(bytes.to_vec());
    }
    let view = view?;
    let mut bits = TailBits::new(bytes, bit_len);
    // Axis pair: same-form splices per component (raw spans take any
    // f64; short spans only the 0.0/1.0 pair flips).
    for (spans, new, old) in [
        (&info.axis_point_spans, view.axis_point, info.view.axis_point),
        (&info.axis_vector_spans, view.axis_vector, info.view.axis_vector),
    ] {
        if let (Some(new), Some(old)) = (new, old) {
            for axis in 0..3 {
                if new[axis] != old[axis] {
                    let span = spans[axis];
                    splice_bd_field(&mut bits, span, span.len == 66, new[axis]);
                }
            }
        }
    }
    // The sweep angle.
    if let (Some(new), Some(old), Some(span)) =
        (view.revolve_angle, info.view.revolve_angle, info.angle_span)
    {
        if new != old {
            splice_bd_field(&mut bits, span, span.len == 66, new);
        }
    }
    // The options run.
    splice_short_run(
        &mut bits,
        &info.option_spans,
        &view.option_doubles,
        &info.view.option_doubles,
    );
    // The embedded profile circle.
    if let (Some(spans), Some(new), Some(old)) = (
        &info.center_spans,
        &view.profile_center,
        &info.view.profile_center,
    ) {
        for axis in 0..3 {
            if new[axis] != old[axis] {
                let span = spans[axis];
                splice_bd_field(&mut bits, span, span.len == 66, new[axis]);
            }
        }
    }
    if let (Some(new), Some(old), Some(span)) =
        (view.profile_radius, info.view.profile_radius, info.radius_span)
    {
        if new != old {
            splice_bd_field(&mut bits, span, span.len == 66, new);
        }
    }
    if let (Some(spans), Some(new), Some(old)) = (
        &info.normal_spans,
        &view.profile_normal,
        &info.view.profile_normal,
    ) {
        for axis in 0..3 {
            if new[axis] != old[axis] {
                let span = spans[axis];
                splice_bd_field(&mut bits, span, span.len == 66, new[axis]);
            }
        }
    }
    Some(bits.bytes)
}

/// Splice one BD field: raw spans take any f64 (bit-exact in their 64
/// value bits); short spans flip only the same-form 0.0/1.0 pair (a
/// form change would shift the stream — outside the Phase B rule).
fn splice_bd_field(bits: &mut TailBits, span: Span, raw: bool, value: f64) {
    if raw {
        let _ = bits.set_le64(span.start + 2, value);
    } else if value == 0.0 {
        let _ = bits.set_pair(span.start, 0b10);
    } else if value == 1.0 {
        let _ = bits.set_pair(span.start, 0b01);
    }
}

fn slice_pairs(
    bits: &mut TailBits,
    spans: &[[Span; 2]],
    new: &[[f64; 2]],
    old: &[[f64; 2]],
) {
    for (index, (spans_pair, new_pair)) in spans.iter().zip(new).enumerate() {
        let old_pair = old.get(index).copied().unwrap_or([f64::NAN, f64::NAN]);
        if new_pair != &old_pair {
            let _ = bits.set_le64(spans_pair[0].start, new_pair[0]);
            let _ = bits.set_le64(spans_pair[1].start, new_pair[1]);
        }
    }
}

fn splice_short_run(
    bits: &mut TailBits,
    spans: &[Span],
    new: &[f64],
    old: &[f64],
) {
    for (index, value) in new.iter().enumerate() {
        let old_value = old.get(index);
        if Some(value) == old_value {
            continue;
        }
        if let Some(span) = spans.get(index) {
            splice_short(bits, *span, *value);
        }
    }
}

fn splice_raw_run(
    bits: &mut TailBits,
    spans: &[Span],
    new: &[f64],
    old: &[f64],
) {
    for (index, value) in new.iter().enumerate() {
        let old_value = old.get(index);
        if Some(value) == old_value {
            continue;
        }
        if let Some(span) = spans.get(index) {
            let _ = bits.set_le64(span.start, *value);
        }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    /// Minimal synthetic extrusion tail in the confirmed grammar:
    /// [3BD direction with raw z] [six spine BDs: 0,0,0,0,1.0,0]
    /// [CALL: BL 18][BL 16][circle (0,0,0) r 1.0-short, normal
    /// (0,0,1)] [2 in-window flag bits][2 trailing bits].
    fn synthetic_sweep_tail() -> (Vec<u8>, u32) {
        let mut bits: Vec<u8> = Vec::new();
        fn pair(bits: &mut Vec<u8>, code: u8) {
            bits.push(code >> 1);
            bits.push(code & 1);
        }
        fn rc(bits: &mut Vec<u8>, value: u8) {
            pair(bits, 0b01);
            for bit in (0..8).rev() {
                bits.push((value >> bit) & 1);
            }
        }
        pair(&mut bits, 0b10); // BD 0.0 (direction x)
        pair(&mut bits, 0b10); // BD 0.0 (direction y)
        // direction z: raw BD with LE64 2.0.
        pair(&mut bits, 0b00);
        for byte in 2.0f64.to_bits().to_le_bytes() {
            for bit in (0..8).rev() {
                bits.push((byte >> bit) & 1);
            }
        }
        // The six spine slots: draft, start, end, twist, scale, align.
        pair(&mut bits, 0b10); // draft_angle 0.0
        pair(&mut bits, 0b10); // draft_start_distance 0.0
        pair(&mut bits, 0b10); // draft_end_distance 0.0
        pair(&mut bits, 0b10); // twist_angle 0.0
        pair(&mut bits, 0b01); // scale_factor 1.0
        pair(&mut bits, 0b10); // align_angle 0.0
        // The profile CALL: [BL 18][BL 16][circle body][2 flags].
        rc(&mut bits, 18);
        rc(&mut bits, 16);
        pair(&mut bits, 0b10); // center x 0.0
        pair(&mut bits, 0b10); // center y 0.0
        pair(&mut bits, 0b10); // center z 0.0
        pair(&mut bits, 0b01); // radius 1.0 (short)
        pair(&mut bits, 0b10); // normal x 0.0
        pair(&mut bits, 0b10); // normal y 0.0
        pair(&mut bits, 0b01); // normal z 1.0
        pair(&mut bits, 0b00); // in-window flag bits
        pair(&mut bits, 0b10); // trailing bits
        let bit_len = bits.len() as u32;
        let mut bytes = vec![0u8; (bit_len as usize + 7) / 8];
        for (index, bit) in bits.iter().enumerate() {
            if *bit == 1 {
                bytes[index / 8] |= 1 << (7 - index % 8);
            }
        }
        (bytes, bit_len)
    }

    #[test]
    fn unmodified_renders_stay_verbatim() {
        let (bytes, bit_len) = synthetic_sweep_tail();
        let info = decode_sweep_tail(&bytes, bit_len).expect("synthetic tail decodes");
        assert_eq!(info.view.scale_factor, Some(1.0));
        let call = info.view.profile.as_ref().expect("the CALL decodes");
        assert_eq!(call.kind, 18);
        assert_eq!(call.bit_len, 16);
        let rendered = render_sweep_tail(
            &bytes,
            bit_len,
            info.direction,
            Some(&info.view),
        )
        .expect("render works");
        assert_eq!(rendered, bytes, "an untouched model must re-emit verbatim");
    }

    #[test]
    fn direction_edits_splice_only_their_span() {
        let (bytes, bit_len) = synthetic_sweep_tail();
        let rendered = render_sweep_tail(&bytes, bit_len, [0.0, 0.0, 3.0], None)
            .expect("render works");
        // Only the direction-z raw value span moved (bits [6..70),
        // bytes 0..=8); bit length is unchanged.
        assert_eq!(rendered.len(), bytes.len());
        let differing: Vec<usize> = rendered
            .iter()
            .zip(bytes.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(index, _)| index)
            .collect();
        assert!(!differing.is_empty());
        assert!(differing.iter().all(|index| (0..9).contains(index)));
        // Re-decode: the new direction reads back through the spans.
        let info = decode_sweep_tail(&rendered, bit_len).unwrap();
        assert_eq!(info.direction, [0.0, 0.0, 3.0]);
    }

    #[test]
    fn spine_and_profile_edits_flip_only_same_form_pairs() {
        let (bytes, bit_len) = synthetic_sweep_tail();
        let info = decode_sweep_tail(&bytes, bit_len).unwrap();
        // Same-form short flips: draft 0.0 -> 1.0 (bits [70..72),
        // byte 8) and scale 1.0 -> 0.0 (bits [78..80), byte 9).
        let mut view = info.view.clone();
        view.draft_angle = Some(1.0);
        view.scale_factor = Some(0.0);
        let rendered = render_sweep_tail(&bytes, bit_len, info.direction, Some(&view))
            .expect("render works");
        assert_eq!(rendered.len(), bytes.len());
        let differing: Vec<usize> = rendered
            .iter()
            .zip(bytes.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(differing, vec![8, 9]);
        // Form-changing values on short spans keep the stored bits
        // (the Phase B write rule: no length-shifting re-encode), and
        // the profile radius short (1.0 -> 2.5) keeps them too.
        let mut view = info.view.clone();
        view.draft_angle = Some(0.7);
        view.profile
            .as_mut()
            .unwrap()
            .circle
            .as_mut()
            .unwrap()
            .radius = 2.5;
        let rendered = render_sweep_tail(&bytes, bit_len, info.direction, Some(&view))
            .expect("render works");
        assert_eq!(rendered, bytes);
        // The profile radius short CAN flip to 0.0 (bits [108..110),
        // byte 13).
        let mut view = info.view.clone();
        view.profile
            .as_mut()
            .unwrap()
            .circle
            .as_mut()
            .unwrap()
            .radius = 0.0;
        let rendered = render_sweep_tail(&bytes, bit_len, info.direction, Some(&view))
            .expect("render works");
        let differing: Vec<usize> = rendered
            .iter()
            .zip(bytes.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(differing, vec![13]);
    }

    #[test]
    fn render_without_decode_stays_verbatim() {
        // A head with a reserved pair anchors nothing: the tail keeps
        // its verbatim-only behavior (no view, no splices).
        let bytes = vec![0b1100_0000, 0, 0, 0];
        assert!(decode_sweep_tail(&bytes, 32).is_none());
    }
}
