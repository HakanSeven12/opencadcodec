//! The Â§20 rewrite-rejection bisect control: the SAME region entity
//! (read from the constructed Region.dwg) re-emitted in a FRESH
//! document (+LINE). If this file OPENS in BricsCAD, the rejection
//! poison lives in the READ-captured document state (the rewrite of
//! the read document); if it is REJECTED, the poison travels with
//! the entity itself (the captured region model).
//!
//! Usage: cargo run --example fresh_pair --features serde --
//!        <region.dwg> <outdir>

use std::fs;
use std::io::Cursor;

use opencadcodec::entities::EntityType;
use opencadcodec::{CadDocument, DwgReader, DwgWriter};

fn read_dwg(path: &str) -> CadDocument {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (src, outdir) = match args.as_slice() {
        [_, s, o] => (s.clone(), o.clone()),
        _ => {
            eprintln!("usage: fresh_pair <region.dwg> <outdir>");
            std::process::exit(2);
        }
    };
    let doc = read_dwg(&src);
    let region = doc
        .entities()
        .find_map(|e| match e {
            EntityType::Region(r) => Some(EntityType::Region(r.clone())),
            _ => None,
        })
        .expect("no region in the source file");

    // The FRESH document: with_version(AC1032), the same region, +LINE.
    let mut fresh = CadDocument::with_version(opencadcodec::types::DxfVersion::AC1032);
    fresh
        .add_entity(region)
        .expect("add region to the fresh document");
    let line = opencadcodec::entities::Line::from_points(
        opencadcodec::types::Vector3::new(0.0, 0.0, 0.0),
        opencadcodec::types::Vector3::new(1.0, 1.0, 0.0),
    );
    fresh.add_entity(EntityType::Line(line)).expect("add line");

    fs::create_dir_all(&outdir).unwrap();
    let out = format!("{outdir}/fresh_region_plus_line.dwg");
    DwgWriter::write_to_file(&out, &fresh).expect("write the fresh pair");
    println!("wrote {out}");

    // Both comparators from the same source:
    // (a) the read document + LINE (the known-REJECTED shape)
    let mut read_doc = read_dwg(&src);
    let line2 = opencadcodec::entities::Line::from_points(
        opencadcodec::types::Vector3::new(0.0, 0.0, 0.0),
        opencadcodec::types::Vector3::new(1.0, 1.0, 0.0),
    );
    read_doc.add_entity(EntityType::Line(line2)).expect("add line");
    let out2 = format!("{outdir}/read_region_plus_line.dwg");
    DwgWriter::write_to_file(&out2, &read_doc).expect("write the read pair");
    println!("wrote {out2}");
}
