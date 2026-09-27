//! DWG rewrite binary for the silver round-trip harness.
//!
//! Usage:
//!   cargo run --bin dwgrewrite --features serde -- INPUT_DWG [OUTPUT_DWG] [--no-lz77]
//!
//! Reads a DWG file and writes it back out. The default output path is
//! `<input>_rt.dwg`. This binary intentionally performs *no* semantic
//! comparison; it just exercises the read -> write path.
//!
//! `--no-lz77` (diagnostics): write via `write_to_file_no_lz77`, which
//! bypasses the whole-file echo AND the objects-stream echo (the
//! diagnostic path passes the identity verdict as `false`) and stores
//! the pages uncompressed — the CONVENTIONAL emission, for
//! record-level study against the retained source with
//! `ac21_token_diff --our-rt`.

use std::path::PathBuf;

use acadrust::{CadDocument, DwgReader, DwgWriter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let no_lz77 = args.iter().any(|a| a == "--no-lz77");
    let paths: Vec<&String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    if paths.is_empty() {
        eprintln!("Usage: dwgrewrite INPUT_DWG [OUTPUT_DWG] [--no-lz77]");
        std::process::exit(1);
    }
    let input = PathBuf::from(paths[0]);
    let output = paths.get(1).map(|p| PathBuf::from(p.as_str())).unwrap_or_else(|| {
        let mut p = input.clone();
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "output".to_string());
        p.set_file_name(format!("{}_rt.dwg", stem));
        p
    });

    let mut reader = DwgReader::from_file(&input)?;
    let doc: CadDocument = reader.read()?;

    if no_lz77 {
        DwgWriter::write_to_file_no_lz77(&output, &doc)?;
    } else {
        DwgWriter::write_to_file(&output, &doc)?;
    }
    println!("Wrote {}", output.display());
    Ok(())
}
