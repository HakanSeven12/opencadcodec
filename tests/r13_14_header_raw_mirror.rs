//! TODO A9 (2026-10-04): the R13/R14 header-variable raw mirror — the
//! struct-axis key-gap head.
//!
//! Gold's HEADER JSON (json_header_write → header_variables.spec) prints
//! the era-gated variable set per file: the R13/R14 wires carry
//! unknown_10, DIMSAV, BLIPMODE, ATTREQ/ATTDIA, WIREFRAME, DELOBJ,
//! DRAGMODE, OSMODE, COORDS, PICKSTYLE, the whole R13/R14 DIM block
//! (DIMTOL..DIMALTTD + DIMFIT + DIMUNIT + the DIMTXSTY handle) and the
//! DIMPOST/DIMAPOST/DIMBLK*_T text quintet — all VERSIONS (R_13b1,
//! R_14) in the spec. Silver's reader walked every slot (the typed
//! `HeaderVariables` stayed positioned) but the `DwgHeaderRaw` mirror
//! — the gold-JSON projection the structure census compares — was
//! never populated for them, so every R13/R14 corpus file ranked 49
//! HEADER key-gaps (the 5,439-leaf share of the 5,510 struct axis).
//! The reader now retains them raw; the writer's four no-model slots
//! (unknown_10, DIMSAV, WIREFRAME, DIMUNIT) replay the captured wire
//! value (§19 H7) instead of the hardcoded defaults. These pins hold
//! the measured gold values (the Line_AC1012/Line_AC1014 census,
//! 2026-10-04) and the conventional-rewrite survival.

use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

fn read_fixture(dir: &str, name: &str) -> Option<CadDocument> {
    let path = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/gold_harness/fixtures/golden_entities/"
    ))
    .join(dir)
    .join(name);
    if !path.exists() {
        eprintln!("fixture {name} missing — skipping");
        return None;
    }
    let mut reader = DwgReader::from_file(&path).expect("open fixture");
    Some(reader.read().expect("read fixture"))
}

fn raw_of(document: &CadDocument) -> acadrust::document::DwgHeaderRaw {
    document
        .dwg_header_raw
        .clone()
        .expect("the raw header mirror decodes")
}

#[test]
fn line_ac1014_header_raw_pins() {
    let Some(document) = read_fixture("AC1014", "Line_AC1014.dwg") else {
        return;
    };
    let raw = raw_of(&document);

    // The R13/R14-only mode bits and shorts (gold's values).
    assert_eq!(raw.unknown_10, Some(256)); // the '11' BS code
    assert_eq!(raw.dimsav, Some(0));
    assert_eq!(raw.blipmode, Some(0));
    assert_eq!(raw.attreq, Some(1));
    assert_eq!(raw.attdia, Some(1));
    assert_eq!(raw.wireframe, Some(0));
    assert_eq!(raw.delobj, Some(1));
    assert_eq!(raw.dragmode, Some(2));
    assert_eq!(raw.osmode, Some(37));
    assert_eq!(raw.coords, Some(1));
    assert_eq!(raw.pickstyle, Some(1));

    // The R13/R14 DIM block.
    assert_eq!(raw.dimtol, Some(0));
    assert_eq!(raw.dimlim, Some(0));
    assert_eq!(raw.dimtih, Some(0));
    assert_eq!(raw.dimtoh, Some(0));
    assert_eq!(raw.dimse1, Some(0));
    assert_eq!(raw.dimse2, Some(0));
    assert_eq!(raw.dimalt, Some(0));
    assert_eq!(raw.dimtofl, Some(1));
    assert_eq!(raw.dimsah, Some(0));
    assert_eq!(raw.dimtix, Some(0));
    assert_eq!(raw.dimsoxd, Some(0));
    assert_eq!(raw.dimaltd, Some(3));
    assert_eq!(raw.dimzin, Some(8));
    assert_eq!(raw.dimsd1, Some(0));
    assert_eq!(raw.dimsd2, Some(0));
    assert_eq!(raw.dimtolj, Some(0));
    assert_eq!(raw.dimjust, Some(0));
    assert_eq!(raw.dimfit, Some(3));
    assert_eq!(raw.dimupt, Some(0));
    assert_eq!(raw.dimtzin, Some(8));
    assert_eq!(raw.dimaltz, Some(0));
    assert_eq!(raw.dimalttz, Some(0));
    assert_eq!(raw.dimtad, Some(1));
    assert_eq!(raw.dimunit, Some(2));
    assert_eq!(raw.dimaunit, Some(0));
    assert_eq!(raw.dimdec, Some(2));
    assert_eq!(raw.dimtdec, Some(2));
    assert_eq!(raw.dimaltu, Some(2));
    assert_eq!(raw.dimalttd, Some(3));

    // The DIMTXSTY handle: gold prints [5, 1, 17, 17].
    let txsty = raw.dimtxsty.expect("DIMTXSTY decodes");
    assert_eq!((txsty.code, txsty.size, txsty.value, txsty.absolute), (5, 1, 17, 17));

    // The R13/R14 dimension text quintet.
    assert_eq!(raw.dimpost, Some(String::new()));
    assert_eq!(raw.dimapost, Some(String::new()));
    assert_eq!(raw.dimblk_t, Some(String::new()));
    assert_eq!(raw.dimblk1_t, Some(String::new()));
    assert_eq!(raw.dimblk2_t, Some(String::new()));
}

#[test]
fn line_ac1012_header_raw_pins() {
    let Some(document) = read_fixture("AC1012", "Line_AC1012.dwg") else {
        return;
    };
    let raw = raw_of(&document);

    // The R13 genus: unknown_10 reads 0 here (the R14 files carry 256),
    // OSMODE carries the author's live snap setting.
    assert_eq!(raw.unknown_10, Some(0));
    assert_eq!(raw.osmode, Some(4149));
    assert_eq!(raw.attdia, Some(1));
    assert_eq!(raw.attreq, Some(1));
    assert_eq!(raw.dragmode, Some(2));
    assert_eq!(raw.coords, Some(1));
    assert_eq!(raw.pickstyle, Some(1));
    assert_eq!(raw.delobj, Some(1));

    // The DIM block (era-uniform defaults on the golden set).
    assert_eq!(raw.dimfit, Some(3));
    assert_eq!(raw.dimunit, Some(2));
    assert_eq!(raw.dimtad, Some(1));
    assert_eq!(raw.dimaltd, Some(3));
    assert_eq!(raw.dimtzin, Some(8));
    assert_eq!(raw.dimzin, Some(8));
    assert_eq!(raw.dimtofl, Some(1));
    assert_eq!(raw.dimaltu, Some(2));
    assert_eq!(raw.dimalttd, Some(3));
}

#[test]
fn header_raw_survives_the_conventional_rewrite() {
    // The echo arm re-emits the whole file verbatim; the CONVENTIONAL arm
    // (edited documents) is where the new splices' write authority
    // matters — unknown_10/DIMSAV/WIREFRAME/DIMUNIT have no typed-model
    // slot, so the captured raw value is the only carrier.
    std::env::set_var("DWG_NO_ECHO", "1");
    for (dir, name, unknown_10) in [
        ("AC1014", "Line_AC1014.dwg", 256i64),
        ("AC1012", "Line_AC1012.dwg", 0),
    ] {
        let Some(document) = read_fixture(dir, name) else {
            return;
        };
        let before = raw_of(&document);
        assert_eq!(before.unknown_10, Some(unknown_10));

        let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
        let decoded = DwgReader::from_stream(Cursor::new(bytes))
            .read()
            .expect("read rewrite");
        let after = raw_of(&decoded);

        // The R13/R14-only slots round-trip through the conventional arm.
        assert_eq!(after.unknown_10, before.unknown_10);
        assert_eq!(after.dimsav, before.dimsav);
        assert_eq!(after.wireframe, before.wireframe);
        assert_eq!(after.dimunit, before.dimunit);
        assert_eq!(after.blipmode, before.blipmode);
        assert_eq!(after.attreq, before.attreq);
        assert_eq!(after.attdia, before.attdia);
        assert_eq!(after.delobj, before.delobj);
        assert_eq!(after.dragmode, before.dragmode);
        assert_eq!(after.osmode, before.osmode);
        assert_eq!(after.coords, before.coords);
        assert_eq!(after.pickstyle, before.pickstyle);
        assert_eq!(after.dimfit, before.dimfit);
        assert_eq!(after.dimtol, before.dimtol);
        assert_eq!(after.dimtad, before.dimtad);
        assert_eq!(after.dimtxsty.is_some(), before.dimtxsty.is_some());
        assert_eq!(after.dimblk_t, before.dimblk_t);
        assert_eq!(after.dimblk1_t, before.dimblk1_t);
        assert_eq!(after.dimblk2_t, before.dimblk2_t);
    }
}
