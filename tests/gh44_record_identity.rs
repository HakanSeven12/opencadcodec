//! The gh44-error record-identity packet (TODO A9 item 5, 2026-10-04):
//! the pathological 2013 specimen's 18 record-census rows, closed by
//! five per-record fixes — the LEADER handle-stream slack (6), the
//! HATCH EED block order (5), the LTYPE xref binding (4), the DIMASSOC
//! intsectobj ref code (2), the LTYPE 16A5 sequential-text/shapecode
//! (1). These pins hold the measured wire facts and the
//! conventional-rewrite survival (DWG_NO_ECHO) for each family.
//!
//! The specimen lives in the libredwg test-data tree (not the fixtures
//! corpus); the tests skip when GOLD_TESTDATA is unset or the file is
//! missing — the battery exports it.

use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

fn specimen() -> Option<CadDocument> {
    let data_dir = std::env::var_os("GOLD_TESTDATA")?;
    let path = std::path::Path::new(&data_dir).join("2013/gh44-error.dwg");
    if !path.exists() {
        eprintln!("gh44-error.dwg missing — skipping");
        return None;
    }
    let mut reader = DwgReader::from_file(&path).ok()?;
    reader.read().ok()
}

fn rewrite(document: &CadDocument) -> Option<CadDocument> {
    let bytes = DwgWriter::write_to_vec(document).ok()?;
    let mut reader = DwgReader::from_stream(Cursor::new(bytes));
    reader.read().ok()
}

#[test]
fn leader_handle_slack_pins() {
    // Her six LEADER records park an unparsed bit-group between the
    // walked main tail and the flag bit — nibble-aligning the RL
    // (1412/1348/1476/1524 all %4==0 where the packed emission wrote
    // 1410/1346/1474/1514). Five records pad two zero bits; 8774 parks
    // ten (0000000010). The capture is per-record author data — the
    // merge replays it only when the writer's main end matches the
    // captured walk end.
    let Some(document) = specimen() else { return };
    let expected: &[(u64, i64, u8, u16)] = &[
        (0x875B, 1409, 2, 0b00),
        (0x8760, 1409, 2, 0b00),
        (0x8765, 1345, 2, 0b00),
        (0x876A, 1473, 2, 0b00),
        (0x876F, 1409, 2, 0b00),
        (0x8774, 1513, 10, 0b0000100000),
    ];
    for (handle, walk_end, len, bits) in expected {
        let captured = document
            .handle_slack_by_handle
            .get(&acadrust::types::Handle::new(*handle))
            .unwrap_or_else(|| panic!("no slack captured for LEADER {handle:X}"));
        assert_eq!(captured, &(*walk_end, *len, *bits), "LEADER {handle:X}");
    }
    // The slack map is sparse — only records with a genuine gap enter it
    // (plus inert entries from under-reading walks, which the merge-time
    // walk-end guard disarms).
    assert!(document.handle_slack_by_handle.len() < 64);
}

#[test]
fn leader_slack_survives_the_conventional_rewrite() {
    let Some(document) = specimen() else { return };
    let before: Vec<_> = document
        .handle_slack_by_handle
        .iter()
        .map(|(h, v)| (h.value(), *v))
        .collect();
    assert!(!before.is_empty());
    std::env::set_var("DWG_NO_ECHO", "1");
    let Some(rewritten) = rewrite(&document) else {
        panic!("the conventional rewrite failed");
    };
    for (handle, (walk_end, len, bits)) in &before {
        let recaptured = rewritten
            .handle_slack_by_handle
            .get(&acadrust::types::Handle::new(*handle));
        assert_eq!(
            recaptured,
            Some(&(*walk_end, *len, *bits)),
            "the slack did not survive the rewrite for {handle:X}"
        );
    }
}

#[test]
fn hatch_eed_block_order_pins() {
    // Her five HATCH records carry the ACAD app's block FIRST
    // ([ACAD(25B), 16CA(17B)]); the remove-and-append reordered every
    // rewrite to [16CA, ACAD] — the keep-position fix preserves the
    // authored order.
    let Some(document) = specimen() else { return };
    for handle in [0x1C0Eu64, 0x1CDC, 0x1D26, 0x1E22, 0x1E47] {
        let hatch = document
            .entities()
            .find(|e| e.common().handle.value() == handle)
            .unwrap_or_else(|| panic!("HATCH {handle:X} missing"));
        let raw = &hatch.common().extended_data.raw_dwg_eed;
        assert!(
            raw.len() >= 2,
            "HATCH {handle:X}: expected the two authored EED blocks"
        );
        assert_eq!(raw[0].0, 0x12, "HATCH {handle:X}: the ACAD block must stay FIRST");
        assert_eq!(raw[0].1.len(), 25);
        assert_eq!(raw[1].0, 0x16CA, "HATCH {handle:X}: the second block's app");
        assert_eq!(raw[1].1.len(), 17);
    }
}

#[test]
fn ltype_xref_binding_pins() {
    // The pipe-named xref-dependent linetypes carry
    // is_xref_resolved 256 + a real xref handle (the block headers) where
    // the old emission wrote 0 + NULL on every rewrite.
    let Some(document) = specimen() else { return };
    // Two xref bindings: the -011 block header (BA16) and the -012 one
    // (BA6A), two linetypes each.
    for (handle, xref) in [
        (0xBA3Du64, 0xBA16u64),
        (0xBA3E, 0xBA16),
        (0xBA91, 0xBA6A),
        (0xBA92, 0xBA6A),
    ] {
        let ltype = document
            .line_types
            .iter()
            .find(|l| l.handle.value() == handle)
            .unwrap_or_else(|| panic!("LTYPE {handle:X} missing"));
        assert!(ltype.xref_dependent, "LTYPE {handle:X}: the dep bit");
        assert_eq!(
            ltype.xref_handle.value(),
            xref,
            "LTYPE {handle:X}: the xref binding"
        );
    }
    // An ordinary linetype keeps the NULL binding.
    let plain = document
        .line_types
        .iter()
        .find(|l| l.handle.value() == 0x16A5)
        .expect("LTYPE 16A5 missing");
    assert!(!plain.xref_dependent);
    assert!(plain.xref_handle.is_null());
}

#[test]
fn ltype_16a5_sequential_text_and_shapecode_pins() {
    // Gold assigns text-dash strings SEQUENTIALLY from the strings
    // area (dash_i walks the strings in order); the wire
    // complex_shapecode is the author's own value (4 where the
    // sequential position is 8 on this pathological specimen).
    let Some(document) = specimen() else { return };
    let ltype = document
        .line_types
        .iter()
        .find(|l| l.name == "SPRLINETYPE")
        .expect("the SPRLINETYPE record missing");
    assert_eq!(ltype.elements.len(), 6);
    let dash1 = ltype.elements[1].complex.as_ref().expect("dash 1 complex");
    let dash4 = ltype.elements[4].complex.as_ref().expect("dash 4 complex");
    assert_eq!(dash1.text(), Some("SPR"));
    assert_eq!(dash4.text(), Some("SPR"));
    assert_eq!(dash1.dwg_shape_number, Some(0));
    assert_eq!(dash4.dwg_shape_number, Some(4));
}

#[test]
fn dimassoc_intersection_objects_pin() {
    // The intsectobj vector: gold's dwg2.spec declares code 5, but the
    // authored wire carries code 4 (soft) — the ref-code lesson; the
    // model retains the handles and the writer emits SoftPointer.
    let Some(document) = specimen() else { return };
    use acadrust::objects::{AssociativeData, ObjectType};
    let mut found = 0;
    for object in document.objects.values() {
        let ObjectType::Associative(assoc) = object else {
            continue;
        };
        if let AssociativeData::DimensionAssociation(dim) = &assoc.data {
            for refs in &dim.references {
                for reference in refs {
                    if !reference.intersection_objects.is_empty() {
                        found += 1;
                    }
                }
            }
        }
    }
    assert!(found >= 2, "the DIMASSOC intersection objects missing (found {found})");
}
