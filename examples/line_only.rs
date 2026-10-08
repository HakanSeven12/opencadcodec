//! The Â§20 rewrite-rejection bisect, minimal suspect: a FRESH AC1032
//! document containing ONLY a LINE. If BricsCAD rejects this, the
//! conventional arm's LINE emission is fatal by itself; if it opens,
//! the poison is the combination (region + LINE).
//!
//! Usage: cargo run --example line_only --features serde -- <outdir>

use opencadcodec::entities::{EntityType, Line};
use opencadcodec::types::{DxfVersion, Vector3};
use opencadcodec::{CadDocument, DwgWriter};

fn main() {
    let outdir = std::env::args().nth(1).expect("usage: line_only <outdir>");
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let line = Line::from_points(Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 0.0));
    doc.add_entity(EntityType::Line(line)).expect("add line");
    std::fs::create_dir_all(&outdir).unwrap();
    let out = format!("{outdir}/line_only.dwg");
    DwgWriter::write_to_file(&out, &doc).expect("write");
    println!("wrote {out}");
}
