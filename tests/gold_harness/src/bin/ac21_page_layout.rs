//! AC21 page-layout dump — the A2 consumption-model instrument.
//!
//! Prints every data section's per-page RS geometry (compressed size,
//! aligned size, RS blocks, tail pad bytes) plus the system-page maps
//! and the file-header block pad, so the author's pre-metadata encoder
//! consumption (the draw start offsets measured by the A2 hunt) can be
//! regressed against the structural pad sizes.
//!
//! Usage: ac21_page_layout FILE.dwg

use acadrust::io::dwg::dwg_reader::DwgReader;
use acadrust::io::dwg::reed_solomon::reed_solomon_decode;

const RS_DATA_K: usize = 251;
const RS_SYSTEM_K: usize = 239;
const AC21_FILE_HEADER_SIZE: u64 = 0x480;

fn main() {
    let file = std::env::args().nth(1).unwrap_or_default();
    if file.is_empty() {
        eprintln!("usage: ac21_page_layout FILE.dwg");
        std::process::exit(2);
    }
    let mut reader = DwgReader::from_file(&file).expect("open DWG");
    let info = reader.read_file_header().expect("read file header");

    let mut total_data_pad = 0usize;
    let mut total_data_pad_words = 0usize;

    let file_bytes = std::fs::read(&file).expect("read file");

    for section in info.section_descriptors.iter() {
        if section.page_count == 0 {
            println!("== section {} (no pages)", section.name);
            continue;
        }
        println!(
            "== section {} encoding={} pages={}",
            section.name,
            section.encoding,
            section.pages.len()
        );
        for page in &section.pages {
            let comp = page.compressed_size as usize;
            let uncomp = page.decompressed_size as usize;
            println!(
                "   page id={} checksum={:#018x} crc={:#018x} comp={comp} uncomp={uncomp}",
                page.page_number, page.checksum, page.crc
            );
            if comp == 0 || uncomp == 0 {
                continue;
            }
            if comp == uncomp {
                println!("   page {} STORED uncomp={uncomp}", page.page_number);
                continue;
            }
            let aligned = (comp + 7) & !7usize;
            let blocks = aligned.div_ceil(RS_DATA_K);
            let pad = blocks * RS_DATA_K - aligned;
            let pad_u32 = pad.div_ceil(4);
            total_data_pad += pad;
            total_data_pad_words += pad_u32;
            // The RS-block tail: her on-disk pad bytes. RS-decode the
            // page and print the [aligned..blocks*251) tail.
            let read_len = blocks * 255;
            let rec = info.page_records.get(&(page.page_number as i32));
            let Some(&(offset, _)) = rec else {
                println!(
                    "   page {} comp={} aligned={} blocks={} pad={} (no slot)",
                    page.page_number, comp, aligned, blocks, pad
                );
                continue;
            };
            let start = (AC21_FILE_HEADER_SIZE + offset as u64) as usize;
            if start + read_len > file_bytes.len() {
                println!("   page {} (slot out of range)", page.page_number);
                continue;
            }
            let encoded = &file_bytes[start..start + read_len];
            let mut full = vec![0u8; blocks * RS_DATA_K];
            reed_solomon_decode(encoded, &mut full, blocks, RS_DATA_K);
            let tail = &full[aligned..];
            let hex: Vec<String> = tail.iter().map(|b| format!("{b:02x}")).collect();
            println!(
                "   page {} pad={} padhex={}",
                page.page_number,
                pad,
                hex.join("")
            );
        }
    }
    println!(
        "TOTAL data-page pad bytes={} pad_u32={}",
        total_data_pad, total_data_pad_words
    );
    println!(
        "page_records count={} file_header_size=0x480",
        info.page_records.len()
    );

    // The system (map) pages: every page record not part of a data
    // section — the section-map/page-map copies and their second
    // copies. RS(255,239) system pages; their tails are the candidate
    // walk-consumption sink.
    use std::collections::BTreeSet;
    let mut data_ids: BTreeSet<i32> = BTreeSet::new();
    for section in info.section_descriptors.iter() {
        for page in &section.pages {
            data_ids.insert(page.page_number as i32);
        }
    }
    let mut sorted_records: Vec<(i32, (i64, i64))> = info
        .page_records
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();
    sorted_records.sort_by_key(|(id, _)| *id);
    for (id, (offset, size)) in sorted_records {
        if data_ids.contains(&id) {
            continue;
        }
        // A system page: 8-byte [checksum|crc] prefix then the RS
        // area. The payload size is unknown here; try to bound the RS
        // blocks from the on-disk size: blocks = (size - 8)/255.
        if size < 8 {
            continue;
        }
        let blocks = ((size - 8) / 255) as usize;
        if blocks == 0 {
            continue;
        }
        let start = (AC21_FILE_HEADER_SIZE + offset as u64) as usize;
        let read_len = blocks * 255;
        if start + 8 + read_len > file_bytes.len() {
            continue;
        }
        let encoded = &file_bytes[start + 8..start + 8 + read_len];
        let mut full = vec![0u8; blocks * RS_SYSTEM_K];
        reed_solomon_decode(encoded, &mut full, blocks, RS_SYSTEM_K);
        println!(
            "   SYSPAGE id={} on_disk={} blocks={} header8={:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            id,
            size,
            blocks,
            file_bytes[start],
            file_bytes[start + 1],
            file_bytes[start + 2],
            file_bytes[start + 3],
            file_bytes[start + 4],
            file_bytes[start + 5],
            file_bytes[start + 6],
            file_bytes[start + 7],
        );
        // Payload dump: 16-byte rows with offsets, capped at 4600 bytes
        // (covers the 19-block section-map slot fills).
        for off in (0..full.len().min(4600)).step_by(16) {
            let upto = (off + 16).min(full.len());
            let row: Vec<String> = full[off..upto]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("     {:04x}: {}", off, row.join(""));
        }
    }
}
