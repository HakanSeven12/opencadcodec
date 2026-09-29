//! The §20 second-poison bisect: rewrite example_2018 with only a
//! SUBSET of its entities to localize which entity's read→rewrite
//! emission poisons the file for the strict loaders.
//!
//! Modes:
//!   first N  — keep the first N entities (document order), drop the rest
//!   3d       — keep only the 3DSOLID/REGION/BODY entities
//!   drop3d   — keep everything EXCEPT the 3D entities
//!
//! Usage: cargo run --example entity_subset --features serde --
//!        <src.dwg> <out.dwg> <mode> [param]

use std::fs;
use std::io::Cursor;

use acadrust::entities::EntityType;
use acadrust::{CadDocument, DwgReader, DwgWriter};

fn read_dwg(path: &str) -> CadDocument {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn is_3d(e: &EntityType) -> bool {
    matches!(e, EntityType::Solid3D(_) | EntityType::Region(_) | EntityType::Body(_))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (src, out, mode, param) = match args.as_slice() {
        [_, s, o, m] => (s.clone(), o.clone(), m.clone(), String::new()),
        [_, s, o, m, p] => (s.clone(), o.clone(), m.clone(), p.clone()),
        _ => {
            eprintln!("usage: entity_subset <src.dwg> <out.dwg> <first|3d|drop3d> [N]");
            std::process::exit(2);
        }
    };

    let mut doc = read_dwg(&src);
    let handles: Vec<_> = doc.entities().map(|e| e.common().handle).collect();
    let total = handles.len();

    // Compute the keep-set upfront (no closure borrowing during removal).
    let keep_set: std::collections::HashSet<acadrust::types::Handle> = match mode.as_str() {
        "3d" => handles
            .iter()
            .copied()
            .filter(|h| doc.get_entity(*h).map(is_3d).unwrap_or(false))
            .collect(),
        "drop3d" => handles
            .iter()
            .copied()
            .filter(|h| doc.get_entity(*h).map(|e| !is_3d(e)).unwrap_or(false))
            .collect(),
        "first" => {
            let n: usize = param.parse().expect("first needs N");
            handles.iter().take(n).copied().collect()
        }
        _ => panic!("unknown mode {mode}"),
    };

    let mut removed = 0usize;
    for h in handles {
        if !keep_set.contains(&h) {
            doc.remove_entity(h);
            removed += 1;
        }
    }
    println!(
        "{src}: {total} entities, kept {}, removed {removed} -> {out}",
        total - removed
    );
    DwgWriter::write_to_file(&out, &doc).expect("write");
    println!("wrote {out}");
}
