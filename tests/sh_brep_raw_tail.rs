//! TODO A9 head (2026-10-04): the ACSH_BREP_CLASS raw-remainder form.
//!
//! AutoCAD-authored SH-BREP records (the `fixtures/brep` mints) carry a
//! modeler-geometry head whose wire version sits outside {1, 2} (0 on
//! the 2007–2013 mints, 38438 on the 2018 one). Gold's unstable-class
//! decode (dwg2.spec 3054, ACTION_3DSOLID → DECODE_3DSOLID) reads NO
//! body there and walks the COMMON_3DSOLID tail straight from the
//! modeler blob's first bits (dwg_spec_shared.h 471: wireframe block →
//! acis_empty_bit → materials when version > 1 → the R2013b revision
//! block), leaving the record's ~45 KB body un-walked between the tail
//! and the handle stream. The reader mirrors that walk (typed,
//! projection-only) and captures the whole tail verbatim as the write
//! authority — the re-emission is bit-exact, so a conventional rewrite
//! is record-identical (the record census: 153/153 and 141/141 on the
//! two R2013+ mints, size+CRC). These pins hold the measured gold
//! values (the `-v9` dissection, 2026-10-04) and the rewrite survival.

use acadrust::objects::{DynamicBlockData, ObjectType, SolidHistoryOperation};
use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

fn read_fixture(name: &str) -> Option<CadDocument> {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/gold_harness/fixtures/brep/"
    ))
    .join(name);
    if !path.exists() {
        eprintln!("fixture {name} missing — skipping");
        return None;
    }
    let mut reader = DwgReader::from_file(&path).expect("open fixture");
    Some(reader.read().expect("read fixture"))
}

/// The document's single ACSH_BREP_CLASS record's Brep model.
fn brep_of(document: &CadDocument) -> Option<acadrust::entities::solid3d::AcisData> {
    for object in document.objects.values() {
        if let ObjectType::DynamicBlock(block) = object {
            if block.dxf_name != "ACSH_BREP_CLASS" {
                continue;
            }
            if let DynamicBlockData::SolidHistoryNode(SolidHistoryOperation::Brep(value)) =
                &block.data
            {
                return Some(value.acis_data.clone());
            }
        }
    }
    None
}

#[test]
fn brep_ac1027_raw_tail_pins() {
    let Some(document) = read_fixture("Brep_AC1027.dwg") else {
        return;
    };
    let acis = brep_of(&document).expect("the SH-BREP record decodes");

    // The wire head: version 0 (gold's [BS 70] read), unknown 0, and the
    // leading acis_empty bit 0 — the raw-remainder form marker.
    assert_eq!(acis.raw_wire_version, Some(0));
    assert!(!acis.raw_wire_unknown);
    assert!(!acis.raw_wire_acis_empty);

    // Gold's COMMON_3DSOLID walk over the body's first bits: no
    // wireframe cache, no materials (version 0), then the R2013b
    // revision block — gold's -v9 values pinned.
    assert!(!acis.wireframe_data_present);
    assert!(!acis.acis_empty_bit);
    assert!(acis.materials.is_empty());
    let revision = &acis.revision;
    assert!(!revision.has_guid);
    assert_eq!(revision.major, 3269251716);
    assert_eq!(revision.minor1, 256);
    assert_eq!(revision.minor2, 0);
    assert_eq!(
        revision.bytes,
        [0x4F, 0x28, 0xCD, 0x2D, 0x8C, 0xA5, 0x0A, 0xA0]
    );
    assert_eq!(revision.end_marker, 0);

    // The un-walked modeler blob: the record's whole tail (from just
    // after the version BS to the handle stream) captured verbatim.
    assert_eq!(acis.raw_tail_bit_len, 361604);
    assert_eq!(acis.raw_tail.len(), (361604 + 7) / 8);
}

#[test]
fn brep_ac1032_raw_tail_pins() {
    let Some(document) = read_fixture("Brep_AC1032.dwg") else {
        return;
    };
    let acis = brep_of(&document).expect("the SH-BREP record decodes");

    // The wire head: version 38438 (u16; the model's i16 wraps to
    // -27098 — the writer re-emits the same 16 bits), unknown 1.
    assert_eq!(acis.raw_wire_version, Some(-27098));
    assert!(acis.raw_wire_unknown);
    assert!(!acis.raw_wire_acis_empty);

    // Gold's walk reads a garbage wireframe cache and a 177-entry
    // materials array out of the body's first bits (every handle read
    // overflows the exhausted handle stream — gold's own trace), then
    // the revision block. Pinned to gold's -v9 values.
    assert!(acis.wireframe_data_present);
    assert!(acis.wireframe_point_present);
    assert_eq!(acis.wireframe_isolines as u32, 2589040220);
    assert!(!acis.wireframe_isoline_present);
    assert!(acis.acis_empty_bit);
    assert_eq!(acis.materials.len(), 177);
    let first = &acis.materials[0];
    assert_eq!(first.array_index, 0);
    assert_eq!(first.absolute_reference, 77);
    assert_eq!(first.material_handle, None);
    let second = &acis.materials[1];
    assert_eq!(second.array_index, 22300);
    assert_eq!(second.absolute_reference, 0);
    assert_eq!(second.material_handle, None);

    let revision = &acis.revision;
    assert!(!revision.has_guid);
    assert_eq!(revision.major, 101187584);
    assert_eq!(revision.minor1, 0);
    assert_eq!(revision.minor2 as u16, 40962);
    assert_eq!(revision.bytes, [0, 0, 0, 0x02, 0xA0, 0, 0, 0]);
    assert_eq!(revision.end_marker, 32778);

    // The un-walked blob after the 177-entry walk.
    assert_eq!(acis.raw_tail_bit_len, 361588);
    assert_eq!(acis.raw_tail.len(), (361588 + 7) / 8);
}

#[test]
fn brep_raw_tail_survives_the_conventional_rewrite() {
    // The echo arm re-emits the whole file verbatim; the CONVENTIONAL arm
    // (edited documents) is where the raw tail's write authority matters.
    std::env::set_var("DWG_NO_ECHO", "1");
    for (name, version, bit_len) in [
        ("Brep_AC1027.dwg", 0i16, 361604u32),
        ("Brep_AC1032.dwg", -27098, 361588),
    ] {
        let Some(document) = read_fixture(name) else {
            return;
        };
        let before = brep_of(&document).expect("the SH-BREP record decodes");
        assert_eq!(before.raw_wire_version, Some(version));
        assert_eq!(before.raw_tail_bit_len, bit_len);

        let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
        let decoded = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read rewrite");
        let after = brep_of(&decoded).expect("the SH-BREP record survives");

        // The write authority round-trips bit-exact: head, tail blob, and
        // the typed projection values all survive the rewrite.
        assert_eq!(after.raw_wire_version, before.raw_wire_version);
        assert_eq!(after.raw_wire_unknown, before.raw_wire_unknown);
        assert_eq!(after.raw_wire_acis_empty, before.raw_wire_acis_empty);
        assert_eq!(after.raw_tail_bit_len, before.raw_tail_bit_len);
        assert_eq!(after.raw_tail, before.raw_tail);
        assert_eq!(after.revision.major, before.revision.major);
        assert_eq!(after.revision.minor1, before.revision.minor1);
        assert_eq!(after.revision.minor2, before.revision.minor2);
        assert_eq!(after.revision.bytes, before.revision.bytes);
        assert_eq!(after.revision.end_marker, before.revision.end_marker);
        assert_eq!(after.materials.len(), before.materials.len());
    }
}
