//! `AcDb:AcDsPrototype_1b` data-store section outline — the §19 H5a read
//! structure projection (gold's `AcDs` JSON shape).
//!
//! The section layout is byte-aligned (all fields whole RL/RLL/RS or fixed
//! byte strings), pinned against libredwg's `acds.spec` plus the corpus
//! gold JSONs: a 56-byte header (14 RL fields — `num_segidx` at offset 52
//! feeds the repeat and is suppressed in gold's JSON), the segment-index
//! table at `segidx_offset` (RLL offset + RL size per entry), then one
//! 48-byte segment header per non-zero index slot followed by a
//! type-specific body. Segment names map to gold's printed `type`
//! (segidx=0, datidx=1, _data_=2, schidx=3, schdat=4, search=5,
//! blob01=6, prvsav=7, freesp=8). Per-type bodies:
//!
//! - datidx: RL num_entries (suppressed) + RL di_unknown + entries
//!   (RL segidx, RL offset, RL schidx each; gold adds a sequential
//!   `index` when printing).
//! - schidx: RL num_props + RL si_unknown_1 + prop records (RL index,
//!   RL segidx, RL offset — index read from the wire) + RLL si_tag +
//!   RL num_prop_entries + RL si_unknown_2 + the prop_entries records
//!   (same wire shape).
//! - schdat: exactly one user-property header (gold's decoder
//!   hardcodes the count to 1: RL size + RL flags; the schema data
//!   that follows is never parsed or printed).
//! - search: RL num_search (suppressed) per record: RL schema_namidx +
//!   RL num_sortedidx + the RLL vector (printed even when empty) +
//!   RL num_ididxs + RL unknown + the ididxs slots (RL inner count;
//!   populated slots carry RLL handle + the RLL ididx vector, zero
//!   slots print as gold's `{}`).
//!
//! Zero-offset index slots print as gold's empty segment records `{}`;
//! gold's decoder also zeroes any offset that lands beyond the buffer,
//! which lowers such slots to empty records as well. Header or table
//! bound violations abort before the arrays (gold returns early from the
//! section read and projects the header only) — `parse_acds_section`
//! mirrors that by returning the header-only summary.
//!
//! The type-specific bodies (datidx/schidx/schdat/search) are read into
//! gold's TOP-LEVEL singleton structs (`_obj->datidx` etc. in acds.spec —
//! not per-segment storage): a later segment of the same type OVERWRITES
//! an earlier one, and gold's JSON prints the final block under every
//! segment of that type (pinned by Revolve_2018's two schdat segments:
//! both print the second segment's `{size: 8, flags: 0}` while the first
//! segment's own wire carries `{8, 1}`).

use crate::document::{
    DwgAcDsDataIndexEntry, DwgAcDsSchemaIndexProp, DwgAcDsSearchData, DwgAcDsSearchIdIdx,
    DwgAcDsSearchIdIdxs, DwgAcDsSegIdxEntry, DwgAcDsSegment, DwgAcDsSummary, DwgAcDsUProp,
};

/// Parse the decompressed AcDs section into gold's outline shape.
/// `None` only when even the 56-byte header does not fit (gold always
/// projects the header for a fetched section).
pub fn parse_acds_section(buf: &[u8]) -> Option<DwgAcDsSummary> {
    if buf.len() < 56 {
        return None;
    }
    let rl = |p: usize| -> u32 {
        u32::from_le_bytes(buf[p..p + 4].try_into().expect("fixed slice"))
    };
    let mut summary = DwgAcDsSummary {
        file_signature: rl(0),
        file_header_size: rl(4),
        unknown_1: rl(8),
        version: rl(12),
        unknown_2: rl(16),
        ds_version: rl(20),
        segidx_offset: rl(24),
        segidx_unknown: rl(28),
        schidx_segidx: rl(36),
        datidx_segidx: rl(40),
        search_segidx: rl(44),
        prvsav_segidx: rl(48),
        file_size: rl(52) as i32,
        segidx: Vec::new(),
        segments: Vec::new(),
    };
    let num_segidx = rl(32) as usize;
    let segidx_offset = summary.segidx_offset as usize;
    // At `segidx_offset` lives the index segment's own 48-byte header
    // (signature 0xD5AC, name "segidx" — consumed by spec's segments[0]
    // block); the entry table follows at +48. Gold's bounds checks abort
    // the arrays when the offset is past the buffer or the table
    // overruns the section (a header-only summary remains).
    if segidx_offset == 0
        || segidx_offset + 48 > buf.len()
        || num_segidx > (buf.len() - segidx_offset - 48) / 12
    {
        return Some(summary);
    }
    let table_at = segidx_offset + 48;
    let mut slots: Vec<(u64, u32)> = Vec::with_capacity(num_segidx);
    for i in 0..num_segidx {
        let base = table_at + i * 12;
        let offset = u64::from_le_bytes(buf[base..base + 8].try_into().expect("fixed slice"));
        let size = rl(base + 8);
        summary
            .segidx
            .push(DwgAcDsSegIdxEntry { index: i as u32, offset, size });
        slots.push((offset, size));
    }
    let mut shared = SharedTypeBlocks::default();
    for (i, (offset, _)) in slots.iter().enumerate() {
        // Gold's decoder zeroes beyond-buffer offsets, which print as
        // empty records — same as the zero-offset padding slots.
        if *offset == 0 || (*offset as usize) >= buf.len() {
            summary.segments.push(DwgAcDsSegment::default());
            continue;
        }
        let o = *offset as usize;
        let Some(seg) = parse_segment_header(buf, o, i as u32) else {
            summary.segments.push(DwgAcDsSegment::default());
            continue;
        };
        // The type body is read into gold's TOP-LEVEL singleton: a later
        // segment of the same type overwrites an earlier one.
        let b = o + 48;
        match seg.type_ {
            Some(1) => {
                let (di, entries) = parse_datidx_body(buf, b);
                shared.datidx_di_unknown = di;
                shared.datidx_entries = entries;
            }
            Some(3) => {
                let (u1, props, tag, u2, pe) = parse_schidx_body(buf, b);
                shared.si_unknown_1 = u1;
                shared.schidx_props = props;
                shared.si_tag = tag;
                shared.si_unknown_2 = u2;
                shared.schidx_prop_entries = pe;
            }
            Some(4) => shared.schdat_uprops = parse_schdat_body(buf, b),
            Some(5) => shared.search_search = parse_search_body(buf, b),
            _ => {}
        }
        summary.segments.push(seg);
    }
    // Attach gold's singleton semantics: every typed segment prints the
    // block of the LAST same-typed segment the decoder processed.
    for seg in &mut summary.segments {
        match seg.type_ {
            Some(1) => {
                seg.di_unknown = shared.datidx_di_unknown;
                seg.datidx_entries = shared.datidx_entries.clone();
            }
            Some(3) => {
                seg.si_unknown_1 = shared.si_unknown_1;
                seg.schidx_props = shared.schidx_props.clone();
                seg.si_tag = shared.si_tag;
                seg.si_unknown_2 = shared.si_unknown_2;
                seg.schidx_prop_entries = shared.schidx_prop_entries.clone();
            }
            Some(4) => seg.schdat_uprops = shared.schdat_uprops.clone(),
            Some(5) => seg.search_search = shared.search_search.clone(),
            _ => {}
        }
    }
    Some(summary)
}

/// One 48-byte segment header. `None` when even the fixed header does
/// not fit (an empty record prints then). The type-specific bodies are
/// parsed separately: gold's spec reads them into TOP-LEVEL singletons
/// (`_obj->datidx`/`_obj->schidx`/`_obj->schdat`/`_obj->search` — not
/// per-segment storage), so every segment of a type prints the block of
/// the LAST segment of that type the decoder processed.
fn parse_segment_header(buf: &[u8], o: usize, slot: u32) -> Option<DwgAcDsSegment> {
    if o + 48 > buf.len() {
        return None;
    }
    let rl = |p: usize| -> u32 {
        buf.get(p..p + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("fixed slice")))
            .unwrap_or(0)
    };
    let rs = |p: usize| -> u32 {
        buf.get(p..p + 2)
            .map(|b| u16::from_le_bytes(b.try_into().expect("fixed slice")) as u32)
            .unwrap_or(0)
    };

    let signature = rs(o);
    let name_bytes = &buf[o + 2..o + 8];
    let name_len = name_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_bytes.len());
    let name = String::from_utf8_lossy(&name_bytes[..name_len]).into_owned();
    let type_ = match name.as_str() {
        "segidx" => 0,
        "datidx" => 1,
        "_data_" => 2,
        "schidx" => 3,
        "schdat" => 4,
        "search" => 5,
        "blob01" => 6,
        "prvsav" => 7,
        "freesp" => 8,
        // Gold's decoder logs an error and skips the segment body; the
        // header it already read still prints. Mirror an untyped (bare)
        // segment.
        _ => 0xffff_ffff,
    };
    let padding_bytes = &buf[o + 40..o + 48];
    Some(DwgAcDsSegment {
        index: Some(slot),
        signature: Some(signature),
        name: Some(name),
        type_: Some(type_),
        segment_idx: Some(rl(o + 8)),
        is_blob01: Some(rl(o + 12)),
        segsize: Some(rl(o + 16)),
        unknown_2: Some(rl(o + 20)),
        ds_version: Some(rl(o + 24)),
        unknown_3: Some(rl(o + 28)),
        data_algn_offset: Some(rl(o + 32)),
        objdata_algn_offset: Some(rl(o + 36)),
        padding: Some(String::from_utf8_lossy(padding_bytes).into_owned()),
        ..DwgAcDsSegment::default()
    })
}

/// The per-type bodies, read into gold's top-level singleton structs.
/// A later segment of the same type overwrites the earlier one.
#[derive(Default)]
struct SharedTypeBlocks {
    datidx_di_unknown: Option<u32>,
    datidx_entries: Option<Vec<DwgAcDsDataIndexEntry>>,
    si_unknown_1: Option<u32>,
    schidx_props: Option<Vec<DwgAcDsSchemaIndexProp>>,
    si_tag: Option<u64>,
    si_unknown_2: Option<u32>,
    schidx_prop_entries: Option<Vec<DwgAcDsSchemaIndexProp>>,
    schdat_uprops: Option<Vec<DwgAcDsUProp>>,
    search_search: Option<Vec<DwgAcDsSearchData>>,
}

/// datidx body: [RL num_entries (suppressed)][RL di_unknown]
/// [entries: RL segidx, RL offset, RL schidx]
fn parse_datidx_body(buf: &[u8], b: usize) -> (Option<u32>, Option<Vec<DwgAcDsDataIndexEntry>>) {
    let rl = |p: usize| -> u32 {
        buf.get(p..p + 4)
            .map(|x| u32::from_le_bytes(x.try_into().expect("fixed slice")))
            .unwrap_or(0)
    };
    let di_unknown = Some(rl(b + 4));
    let n = rl(b) as usize;
    let mut entries = None;
    if n > 0 && b + 8 <= buf.len() && n * 12 <= buf.len() - b - 8 {
        let mut v = Vec::with_capacity(n);
        for k in 0..n {
            let p = b + 8 + k * 12;
            v.push(DwgAcDsDataIndexEntry {
                index: k as u32,
                segidx: rl(p),
                offset: rl(p + 4),
                schidx: rl(p + 8),
            });
        }
        entries = Some(v);
    }
    (di_unknown, entries)
}

/// schidx body: [RL num_props][RL si_unknown_1]
/// [props: RL index, RL segidx, RL offset]* [RLL si_tag]
/// [RL num_prop_entries][RL si_unknown_2]
/// [prop_entries: same wire shape]*
fn parse_schidx_body(
    buf: &[u8],
    b: usize,
) -> (Option<u32>, Option<Vec<DwgAcDsSchemaIndexProp>>, Option<u64>, Option<u32>, Option<Vec<DwgAcDsSchemaIndexProp>>) {
    let rl = |p: usize| -> u32 {
        buf.get(p..p + 4)
            .map(|x| u32::from_le_bytes(x.try_into().expect("fixed slice")))
            .unwrap_or(0)
    };
    let rll = |p: usize| -> u64 {
        buf.get(p..p + 8)
            .map(|x| u64::from_le_bytes(x.try_into().expect("fixed slice")))
            .unwrap_or(0)
    };
    let fit = |p: usize, len: usize| p <= buf.len() && len <= buf.len() - p;
    let si_unknown_1 = Some(rl(b + 4));
    let n = rl(b) as usize;
    let mut p = b + 8;
    let mut props = None;
    if n > 0 && fit(p, n * 12) {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(DwgAcDsSchemaIndexProp {
                index: rl(p),
                segidx: rl(p + 4),
                offset: rl(p + 8),
            });
            p += 12;
        }
        props = Some(v);
    }
    let (mut tag, mut si_unknown_2, mut prop_entries) = (None, None, None);
    if fit(p, 16) {
        tag = Some(rll(p));
        let n2 = rl(p + 8) as usize;
        si_unknown_2 = Some(rl(p + 12));
        p += 16;
        if n2 > 0 && fit(p, n2 * 12) {
            let mut v = Vec::with_capacity(n2);
            for _ in 0..n2 {
                v.push(DwgAcDsSchemaIndexProp {
                    index: rl(p),
                    segidx: rl(p + 4),
                    offset: rl(p + 8),
                });
                p += 12;
            }
            prop_entries = Some(v);
        }
    }
    (si_unknown_1, props, tag, si_unknown_2, prop_entries)
}

/// schdat body: exactly one user-property header on the wire (gold's
/// decoder hardcodes the count): [RL size][RL flags]. The schema payload
/// that follows is never parsed or printed.
fn parse_schdat_body(buf: &[u8], b: usize) -> Option<Vec<DwgAcDsUProp>> {
    if b + 8 > buf.len() {
        return None;
    }
    let rl = |p: usize| -> u32 {
        u32::from_le_bytes(buf[p..p + 4].try_into().expect("fixed slice"))
    };
    Some(vec![DwgAcDsUProp { size: rl(b), flags: rl(b + 4) }])
}

/// search body: [RL num_search (suppressed)] then per record
/// [RL schema_namidx][RL num_sortedidx][RLL sortedidx *]*
/// [RL num_ididxs][RL unknown][slots: [RL inner count]
/// [RLL handle][RL count][RLL ididx *]*]
fn parse_search_body(buf: &[u8], b: usize) -> Option<Vec<DwgAcDsSearchData>> {
    let rl = |p: usize| -> u32 {
        buf.get(p..p + 4)
            .map(|x| u32::from_le_bytes(x.try_into().expect("fixed slice")))
            .unwrap_or(0)
    };
    let rll = |p: usize| -> u64 {
        buf.get(p..p + 8)
            .map(|x| u64::from_le_bytes(x.try_into().expect("fixed slice")))
            .unwrap_or(0)
    };
    let fit = |p: usize, len: usize| p <= buf.len() && len <= buf.len() - p;
    let n = rl(b) as usize;
    let mut p = b + 4;
    if n == 0 {
        return None;
    }
    let mut records = Vec::new();
    for _ in 0..n {
        if !fit(p, 8) {
            break;
        }
        let schema_namidx = rl(p);
        let nsorted = rl(p + 4) as usize;
        p += 8;
        let mut sortedidx: Vec<i64> = Vec::new();
        if nsorted > 0 && fit(p, nsorted * 8) {
            for _ in 0..nsorted {
                sortedidx.push(rll(p) as i64);
                p += 8;
            }
        } else if nsorted > 0 {
            p = buf.len();
        }
        if !fit(p, 8) {
            records.push(DwgAcDsSearchData {
                schema_namidx,
                sortedidx,
                unknown: 0,
                ididxs: None,
            });
            break;
        }
        let nid = rl(p) as usize;
        let unknown = rl(p + 4);
        p += 8;
        let mut ididxs: Vec<DwgAcDsSearchIdIdxs> = Vec::new();
        if nid > 0 {
            for _ in 0..nid {
                if !fit(p, 4) {
                    ididxs.push(DwgAcDsSearchIdIdxs::default());
                    break;
                }
                let inner = rl(p) as usize;
                p += 4;
                if inner == 0 || !fit(p, inner * 20) {
                    ididxs.push(DwgAcDsSearchIdIdxs::default());
                    if inner > 0 {
                        break;
                    }
                    continue;
                }
                let mut ididx = Vec::with_capacity(inner);
                for _ in 0..inner {
                    if !fit(p, 12) {
                        break;
                    }
                    let handle = rll(p);
                    let cnt = rl(p + 8) as usize;
                    p += 12;
                    let mut values = Vec::new();
                    if cnt > 0 && fit(p, cnt * 8) {
                        for _ in 0..cnt {
                            values.push(rll(p));
                            p += 8;
                        }
                    } else if cnt > 0 {
                        p = buf.len();
                    }
                    ididx.push(DwgAcDsSearchIdIdx { handle, ididx: Some(values) });
                }
                ididxs.push(DwgAcDsSearchIdIdxs { ididx: Some(ididx) });
            }
        }
        records.push(DwgAcDsSearchData {
            schema_namidx,
            sortedidx,
            unknown,
            ididxs: (nid > 0).then_some(ididxs),
        });
    }
    (!records.is_empty()).then_some(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_rl(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_le_bytes());
    }

    /// Pack the 14 header RLs in wire order (num_segidx 9th, file_size
    /// last — the layout the corpus golds pin).
    fn push_header(
        v: &mut Vec<u8>,
        [sig, size, u1, version, u2, ds_version, segidx_off, segidx_unk, num_segidx, schidx,
         datidx, search, prvsav, file_size]: [u32; 14],
    ) {
        for x in [sig, size, u1, version, u2, ds_version, segidx_off, segidx_unk, num_segidx,
             schidx, datidx, search, prvsav, file_size]
        {
            push_rl(v, x);
        }
    }

    #[test]
    fn header_only_when_num_segidx_is_zero() {
        let mut buf = Vec::new();
        push_header(&mut buf, [0x64D5_AC52, 128, 2, 2, 0, 1, 0, 0, 0, 6, 4, 7, 0, 2816]);
        let s = parse_acds_section(&buf).expect("header parses");
        assert_eq!(s.file_size, 2816);
        assert_eq!(s.version, 2);
        assert_eq!(s.schidx_segidx, 6);
        assert_eq!(s.datidx_segidx, 4);
        assert_eq!(s.search_segidx, 7);
        assert!(s.segidx.is_empty());
        assert!(s.segments.is_empty());
    }

    #[test]
    fn datidx_and_empty_slot_parse() {
        // 56-byte header with num_segidx = 2, segidx_offset = 56:
        // the index segment's own 48-byte header at 56, the entry table
        // at 104: slot 0 zero (an empty record), slot 1 a datidx segment
        // built at 176.
        let mut buf = Vec::new();
        push_header(&mut buf, [1, 128, 2, 2, 0, 1, 56, 0, 2, 0, 0, 0, 0, 512]);
        // the index segment's 48-byte header (signature + name + fields).
        buf.extend_from_slice(&[0xAC, 0xD5]);
        buf.extend_from_slice(b"segidx");
        for _ in 0..8 {
            push_rl(&mut buf, 0);
        }
        buf.extend_from_slice(&[0x55; 8]);
        // segment-index table at 56 + 48 = 104.
        let segidx_at = buf.len();
        buf.extend_from_slice(&0u64.to_le_bytes());
        push_rl(&mut buf, 0);
        buf.extend_from_slice(&176u64.to_le_bytes());
        push_rl(&mut buf, 56);
        // pad to the segment start
        while buf.len() < 176 {
            buf.push(0);
        }
        // datidx segment header (48 bytes) at 176
        buf.extend_from_slice(&[0xAC, 0xD5]); // signature 0xD5AC
        buf.extend_from_slice(b"datidx"); // 6-byte name
        push_rl(&mut buf, 1); // segment_idx
        push_rl(&mut buf, 0); // is_blob01
        push_rl(&mut buf, 24); // segsize
        push_rl(&mut buf, 0); // unknown_2
        push_rl(&mut buf, 1); // ds_version
        push_rl(&mut buf, 0); // unknown_3
        push_rl(&mut buf, 0); // data_algn_offset
        push_rl(&mut buf, 3); // objdata_algn_offset
        buf.extend_from_slice(&[0x55; 8]); // padding
        // datidx body: 1 entry
        push_rl(&mut buf, 1); // num_entries (suppressed in JSON)
        push_rl(&mut buf, 0); // di_unknown
        push_rl(&mut buf, 3); // entry segidx
        push_rl(&mut buf, 8); // entry offset
        push_rl(&mut buf, 1); // entry schidx
        let _ = segidx_at;
        let s = parse_acds_section(&buf).expect("section parses");
        assert_eq!(s.segidx.len(), 2);
        assert_eq!(s.segidx[1].offset, 176);
        assert_eq!(s.segments.len(), 2);
        assert_eq!(s.segments[0].index, None); // zero-offset slot: `{}`.
        let seg1 = &s.segments[1];
        assert_eq!(seg1.name.as_deref(), Some("datidx"));
        assert_eq!(seg1.type_, Some(1));
        assert_eq!(seg1.padding.as_deref(), Some("UUUUUUUU"));
        assert_eq!(seg1.di_unknown, Some(0));
        let entries = seg1.datidx_entries.as_ref().expect("entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(
            (entries[0].index, entries[0].segidx, entries[0].offset, entries[0].schidx),
            (0, 3, 8, 1)
        );
    }

    #[test]
    fn bounds_violations_lower_to_header_only() {
        let mut buf = Vec::new();
        push_header(&mut buf, [1, 128, 2, 2, 0, 1, 9999, 0, 5, 0, 0, 0, 0, 10]);
        let s = parse_acds_section(&buf).expect("header parses");
        assert!(s.segidx.is_empty());
    }

    #[test]
    fn type_bodies_are_singletons_last_write_wins() {
        // Two schdat segments: both must print the SECOND one's uprops
        // (gold's top-level `_obj->schdat` is overwritten by each read).
        let mut buf = Vec::new();
        push_header(&mut buf, [1, 128, 2, 2, 0, 1, 56, 0, 2, 0, 0, 0, 0, 512]);
        // index segment header at 56 (48 bytes), table at 104.
        buf.extend_from_slice(&[0xAC, 0xD5]);
        buf.extend_from_slice(b"segidx");
        for _ in 0..8 {
            push_rl(&mut buf, 0);
        }
        buf.extend_from_slice(&[0x55; 8]);
        let seg1_at = 152usize;
        let seg2_at = 232usize;
        buf.extend_from_slice(&seg1_at.to_le_bytes()); // RLL offset
        push_rl(&mut buf, 40);
        buf.extend_from_slice(&seg2_at.to_le_bytes());
        push_rl(&mut buf, 40);
        while buf.len() < seg1_at {
            buf.push(0);
        }
        for (at, flags) in [(seg1_at, 1u32), (seg2_at, 0u32)] {
            while buf.len() < at {
                buf.push(0);
            }
            buf.extend_from_slice(&[0xAC, 0xD5]);
            buf.extend_from_slice(b"schdat");
            for _ in 0..8 {
                push_rl(&mut buf, 0);
            }
            buf.extend_from_slice(&[0x55; 8]);
            push_rl(&mut buf, 8); // uprops size
            push_rl(&mut buf, flags);
        }
        let s = parse_acds_section(&buf).expect("section parses");
        assert_eq!(s.segments.len(), 2);
        for seg in &s.segments {
            assert_eq!(seg.name.as_deref(), Some("schdat"));
            let uprops = seg.schdat_uprops.as_ref().expect("uprops");
            assert_eq!(uprops[0].flags, 0, "both print the LAST read block");
        }
    }
}
