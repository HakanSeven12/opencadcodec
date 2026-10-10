//! The Â§20 candidate-6 payload-swap experiment: split the blame
//! between the SAB stream and the DWG wrapper.
//!
//! The constructed Region now mirrors the authored sheet on every
//! measured invariant (the sense chain ffff, the vertex int 2, the
//! era-profiled header, the BFS order) yet still NULL-BOXES in the
//! strict loader while the authored plain region MODELS. Two chimeras
//! triangulate the blocker:
//!
//!   swap_authored_wrapper.dwg  â€” the AUTHORED example_2018 document
//!     with its plain region's SAB replaced by the CONSTRUCTED
//!     region's SAB. Models => the constructed SAB stream is
//!     exonerated (the blocker lives in the wrapper/slot);
//!     null-boxes => the SAB stream itself is the blocker, below
//!     every measured invariant.
//!   swap_constructed_wrapper.dwg â€” the CONSTRUCTED Region.dwg
//!     document with its region's SAB replaced by the AUTHORED
//!     plain region's SAB. Models => the wrapper was never the
//!     problem (the SAB is the blocker); null-boxes => the
//!     wrapper/slot is the blocker.
//!
//! Usage: cargo run --example sab_swap --features serde --
//!        <authored.dwg> <constructed.dwg> <outdir>

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

fn first_region_sab(doc: &CadDocument) -> Vec<u8> {
    for e in doc.entities() {
        if let EntityType::Region(r) = e {
            if r.acis_data.is_binary && !r.acis_data.sab_data.is_empty() {
                return r.acis_data.sab_data.clone();
            }
        }
    }
    panic!("no binary region SAB found");
}

/// The wireframe-anchor coherence test (2026-09-30): the entity's
/// point_of_reference is written as the wireframe cache's 3BD anchor,
/// and the modeler may cross-check it against the SAB's actual
/// geometry. A swap that replaces only the SAB leaves the wrapper's
/// anchor pointing at the OLD geometry â€” an incoherent entity the
/// chimera then measures as a "wrapper blocker" that is really the
/// experiment's own artifact. The coherent swap carries the SAB's
/// owning entity's anchor with it.
fn first_region_point(doc: &CadDocument) -> opencadcodec::types::Vector3 {
    for e in doc.entities() {
        if let EntityType::Region(r) = e {
            if r.acis_data.is_binary && !r.acis_data.sab_data.is_empty() {
                return r.point_of_reference;
            }
        }
    }
    panic!("no binary region SAB found");
}

fn swap_first_region_sab(doc: &mut CadDocument, sab: &[u8], anchor: opencadcodec::types::Vector3) -> bool {
    for e in doc.entities_mut() {
        if let EntityType::Region(r) = e {
            if r.acis_data.is_binary && !r.acis_data.sab_data.is_empty() {
                r.acis_data.sab_data = sab.to_vec();
                r.point_of_reference = anchor;
                return true;
            }
        }
    }
    false
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (authored_path, constructed_path, outdir) = match args.as_slice() {
        [_, a, c, o] => (a.clone(), c.clone(), o.clone()),
        _ => {
            eprintln!("usage: sab_swap <authored.dwg> <constructed.dwg> <outdir>");
            std::process::exit(2);
        }
    };

    let authored = read_dwg(&authored_path);
    let constructed = read_dwg(&constructed_path);
    let authored_sab = first_region_sab(&authored);
    let constructed_sab = first_region_sab(&constructed);
    let authored_anchor = first_region_point(&authored);
    let constructed_anchor = first_region_point(&constructed);
    println!(
        "authored region SAB: {} bytes (anchor {:?}); constructed region SAB: {} bytes (anchor {:?})",
        authored_sab.len(),
        authored_anchor,
        constructed_sab.len(),
        constructed_anchor
    );

    fs::create_dir_all(&outdir).unwrap();

    // Chimera 1: the authored wrapper + the constructed SAB.
    let mut swap_a = read_dwg(&authored_path);
    assert!(swap_first_region_sab(&mut swap_a, &constructed_sab, constructed_anchor));
    let out_a = format!("{outdir}/swap_authored_wrapper.dwg");
    DwgWriter::write_to_file(&out_a, &swap_a).expect("write chimera 1");
    println!("wrote {out_a}");

    // Chimera 2: the constructed wrapper + the authored SAB.
    let mut swap_c = read_dwg(&constructed_path);
    assert!(swap_first_region_sab(&mut swap_c, &authored_sab, authored_anchor));
    let out_c = format!("{outdir}/swap_constructed_wrapper.dwg");
    DwgWriter::write_to_file(&out_c, &swap_c).expect("write chimera 2");
    println!("wrote {out_c}");

    // Control: the authored document REWRITTEN with the region
    // untouched â€” a trivial edit elsewhere (a LINE) forces the
    // conventional arm. If BricsCAD's census still finds the three
    // 3D entities, the rewrite path preserves visibility and the
    // swap's SAB-length change is what broke it; if the census
    // finds none, the readâ†’rewrite path loses 3D-entity visibility
    // generally.
    let mut control = read_dwg(&authored_path);
    let line = opencadcodec::entities::Line::from_points(
        opencadcodec::types::Vector3::new(0.0, 0.0, 0.0),
        opencadcodec::types::Vector3::new(1.0, 1.0, 0.0),
    );
    control
        .add_entity(opencadcodec::EntityType::Line(line))
        .expect("add line");
    let out_ctl = format!("{outdir}/rewrite_control.dwg");
    DwgWriter::write_to_file(&out_ctl, &control).expect("write control");
    println!("wrote {out_ctl}");
}
