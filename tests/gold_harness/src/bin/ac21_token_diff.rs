//! AC21 LZ77 token-stream differential — the §19 H8c instrument.
//!
//! Extracts the author's on-disk compressed streams per page (raw file
//! bytes + RS de-interleave), walks the token sequence they encode, then
//! compresses the same decompressed chunk with OUR compressor and walks
//! that stream too. Both walks replay through the exact decoder state
//! machine and are validated byte-for-byte against `decompress_ac21`
//! over the same bytes. The printed stats name the encoder divergence
//! classes (opcode-class selection, match-length/offset thresholds,
//! literal-run structure, in-chain long-match encoding).
//!
//! Usage:
//!   ac21_token_diff FILE.dwg [--section AcDb:AcDbObjects] [--pages N]
//!                    [--dump PREFIX] [--examples N]

use acadrust::io::dwg::compressor_ac21::compress_ac21;
use acadrust::io::dwg::decompressor_ac21::{decompress_ac21, decompress_copy_n_reordered};
use acadrust::io::dwg::dwg_reader::DwgReader;
use acadrust::io::dwg::reed_solomon::reed_solomon_decode;
use std::collections::BTreeMap;

const AC21_FILE_HEADER_SIZE: u64 = 0x480;
const RS_DATA_K: usize = 251;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Long,        // nibble 0 (len 19..50, off <= 4096)
    LongInChain, // nibble 0xF remapped to class 0 inside a chain
    Short,       // nibble 1 (len 3..18, off <= 8192)
    Extended,    // nibble 2, bit3 clear (len <= 255, 16-bit off)
    ExtendedLong, // nibble 2, bit3 set (len >= 256)
    Compact,     // nibble 3..14 (len 3..14, off <= 512)
}

impl Class {
    fn op_bytes(self) -> usize {
        match self {
            Class::Long | Class::LongInChain | Class::Short => 3,
            Class::Extended => 4,
            Class::ExtendedLong => 5,
            Class::Compact => 2,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Class::Long => "long",
            Class::LongInChain => "long(F)",
            Class::Short => "short",
            Class::Extended => "extended",
            Class::ExtendedLong => "extlong",
            Class::Compact => "compact",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LitKind {
    Declared, // literal-length opcode, >= 8
    Trailing, // attached to a match, 1..7
    Pseudo,   // the stream-start 0x2X pseudo-op's leading count
}

#[derive(Clone, Debug)]
enum Tok {
    Lit {
        count: u32,
        kind: LitKind,
        op_bytes: u32, // bytes spent on the length encoding itself
    },
    Match {
        length: u32,
        offset: u32,
        class: Class,
        first: bool, // first of a chain (directly after a literal run)
        trailing: u32,
    },
}

#[derive(Default)]
struct Agg {
    pages: usize,
    stored_pages: usize,
    comp_bytes: usize,
    out_bytes: usize,
    lit_bytes: usize,
    lit_op_bytes: usize,
    match_bytes: usize,
    match_op_bytes: usize,
    trailing_lit_bytes: usize,
    class_counts: BTreeMap<&'static str, (usize, usize)>, // (count, first_count)
    len_buckets: [usize; 6], // 3, 4-5, 6-8, 9-14, 15-50, 51+
    off_buckets: [usize; 6], // <=16, <=64, <=512, <=4096, <=8192, >8192
    trailing_hist: [usize; 8],
    lit_run_buckets: [usize; 6], // trailing 1-7, 8-15, 16-22, 23-63, 64-255, 256+
    matches: usize,
    literal_runs: usize,
}

impl Agg {
    fn add(&mut self, toks: &[Tok], comp_len: usize, out_len: usize) {
        self.pages += 1;
        self.comp_bytes += comp_len;
        self.out_bytes += out_len;
        for t in toks {
            match t {
                Tok::Lit { count, kind, op_bytes, .. } => {
                    self.lit_bytes += *count as usize;
                    self.lit_op_bytes += *op_bytes as usize;
                    self.literal_runs += 1;
                    match kind {
                        LitKind::Trailing => {
                            self.trailing_lit_bytes += *count as usize;
                            self.lit_run_buckets[0] += 1;
                        }
                        LitKind::Declared => {
                            let i = match count {
                                8..=15 => 1,
                                16..=22 => 2,
                                23..=63 => 3,
                                64..=255 => 4,
                                _ => 5,
                            };
                            self.lit_run_buckets[i] += 1;
                        }
                        LitKind::Pseudo => {}
                    }
                }
                Tok::Match { length, offset, class, first, trailing, .. } => {
                    self.matches += 1;
                    self.match_bytes += *length as usize;
                    self.match_op_bytes += class.op_bytes();
                    let e = self.class_counts.entry(class.name()).or_insert((0, 0));
                    e.0 += 1;
                    if *first {
                        e.1 += 1;
                    }
                    self.len_buckets[match length {
                        3 => 0,
                        4..=5 => 1,
                        6..=8 => 2,
                        9..=14 => 3,
                        15..=50 => 4,
                        _ => 5,
                    }] += 1;
                    self.off_buckets[match offset {
                        1..=16 => 0,
                        17..=64 => 1,
                        65..=512 => 2,
                        513..=4096 => 3,
                        4097..=8192 => 4,
                        _ => 5,
                    }] += 1;
                    self.trailing_hist[*trailing as usize] += 1;
                }
            }
        }
    }

    fn report(&self, label: &str) {
        println!("  [{label}] pages={} stored={} comp={} out={} ratio={:.4}",
            self.pages, self.stored_pages, self.comp_bytes, self.out_bytes,
            self.comp_bytes as f64 / self.out_bytes.max(1) as f64);
        println!(
            "    coverage: literals={} ({:.1}%) matches={} ({:.1}%)  \
lit_op_bytes={} match_op_bytes={} trailing_lit={}",
            self.lit_bytes,
            100.0 * self.lit_bytes as f64 / self.out_bytes.max(1) as f64,
            self.match_bytes,
            100.0 * self.match_bytes as f64 / self.out_bytes.max(1) as f64,
            self.lit_op_bytes,
            self.match_op_bytes,
            self.trailing_lit_bytes
        );
        println!(
            "    tokens: matches={} literal_runs={} avg_match_len={:.2}",
            self.matches,
            self.literal_runs,
            self.match_bytes as f64 / self.matches.max(1) as f64
        );
        let mut classes: Vec<_> = self.class_counts.iter().collect();
        classes.sort_by_key(|(_, (c, _))| std::cmp::Reverse(*c));
        let cstr = classes
            .iter()
            .map(|(n, (c, f))| format!("{n}:{c}(f{f})"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("    classes: {cstr}");
        println!("    len buckets [3|4-5|6-8|9-14|15-50|51+]: {:?}", self.len_buckets);
        println!("    off buckets [<=16|<=64|<=512|<=4096|<=8192|>8192]: {:?}", self.off_buckets);
        println!("    trailing hist [0..7]: {:?}", self.trailing_hist);
        println!("    lit runs [trail|8-15|16-22|23-63|64-255|256+]: {:?}", self.lit_run_buckets);
    }
}

/// Decode one match opcode starting at `si` (op byte already read into `op`).
/// Returns (class, length, offset, next_op) and advances `si`.
/// Replicates `read_instructions` exactly.
fn read_instructions(src: &[u8], si: &mut usize, op: &mut u32) -> (Class, u32, u32, u32) {
    match *op >> 4 {
        0 => {
            let mut len = (*op & 0x0F) + 0x13;
            let mut off = src[*si] as u32;
            *si += 1;
            let n = src[*si] as u32;
            *si += 1;
            len = ((n >> 3) & 0x10) + len;
            off = ((n & 0x78) << 5) + 1 + off;
            (Class::Long, len, off, n)
        }
        1 => {
            let len = (*op & 0x0F) + 3;
            let mut off = src[*si] as u32;
            *si += 1;
            let n = src[*si] as u32;
            *si += 1;
            off = ((n & 0xF8) << 5) + 1 + off;
            (Class::Short, len, off, n)
        }
        2 => {
            let mut off = src[*si] as u32;
            *si += 1;
            let b2 = src[*si] as u32;
            *si += 1;
            off = ((b2 << 8) & 0xFF00) | off;
            let len0 = *op & 7;
            if (*op & 8) == 0 {
                let n = src[*si] as u32;
                *si += 1;
                let len = (n & 0xF8) + len0;
                (Class::Extended, len, off, n)
            } else {
                off += 1;
                let b3 = src[*si] as u32;
                *si += 1;
                let mut len = (b3 << 3) + len0;
                let n = src[*si] as u32;
                *si += 1;
                len = ((n & 0xF8) << 8) + len + 0x100;
                (Class::ExtendedLong, len, off, n)
            }
        }
        _ => {
            let len = *op >> 4;
            let off0 = *op & 0x0F;
            let n = src[*si] as u32;
            *si += 1;
            let off = ((n & 0xF8) << 1) + off0 + 1;
            (Class::Compact, len, off, n)
        }
    }
}

/// Replicate `read_literal_length`: uses `op` (a nibble-0 byte) and
/// continuation bytes at `si`. Returns (length, op_bytes).
fn read_literal_length(src: &[u8], si: &mut usize, op: u32) -> (u32, u32) {
    let mut length = op + 8;
    let mut op_bytes = 1u32;
    if length == 0x17 {
        let mut n = src[*si] as u32;
        *si += 1;
        op_bytes += 1;
        length += n;
        if n == 0xFF {
            loop {
                n = src[*si] as u32;
                *si += 1;
                let n2 = src[*si] as u32;
                *si += 1;
                op_bytes += 2;
                n |= n2 << 8;
                length += n;
                if n != 0xFFFF {
                    break;
                }
            }
        }
    }
    (length, op_bytes)
}

/// Replicate the decompressor's `copy_literal` reordering into `out`.
fn replay_literal(src: &[u8], si: &mut usize, out: &mut Vec<u8>, length: u32) {
    let mut remaining = length;
    while remaining >= 32 {
        let s = *si;
        let d = out.len();
        out.resize(d + 32, 0);
        let src32 = &src[s..s + 32];
        // The decompressor's 32-byte block: 8-byte group reversal
        out[d..d + 4].copy_from_slice(&src32[24..28]);
        out[d + 4..d + 8].copy_from_slice(&src32[28..32]);
        out[d + 8..d + 12].copy_from_slice(&src32[16..20]);
        out[d + 12..d + 16].copy_from_slice(&src32[20..24]);
        out[d + 16..d + 20].copy_from_slice(&src32[8..12]);
        out[d + 20..d + 24].copy_from_slice(&src32[12..16]);
        out[d + 24..d + 28].copy_from_slice(&src32[0..4]);
        out[d + 28..d + 32].copy_from_slice(&src32[4..8]);
        *si += 32;
        remaining -= 32;
    }
    if remaining > 0 {
        let s = *si;
        let d = out.len();
        out.resize(d + remaining as usize, 0);
        decompress_copy_n_reordered(src, s, out.as_mut_slice(), d, remaining as usize);
        *si += remaining as usize;
    }
}

/// Walk one compressed stream, recording tokens and replaying the output.
/// Replicates `decompress_ac21_inner` + `copy_decompressed_chunks` exactly.
// The pending-literal state machine clears `match_len` on paths the borrow
// checker views as dead; the assignment mirrors the decoder's control flow.
#[allow(unused_assignments)]
fn walk_tokens(comp: &[u8]) -> Result<(Vec<Tok>, Vec<u8>), String> {
    if comp.is_empty() {
        return Err("empty stream".into());
    }
    // Pad like the real decoder (source +64) so inner reads never OOB.
    let mut padded = vec![0u8; comp.len() + 64];
    padded[..comp.len()].copy_from_slice(comp);
    let end = comp.len();
    let src = padded.as_slice();
    let mut toks = Vec::new();
    let mut out: Vec<u8> = Vec::new();

    let mut si = 0usize;
    let mut op = src[0] as u32;
    si += 1;
    let mut match_len: u32 = 0; // pending literal count from a match's trailing field
    let mut lit_op_bytes: u32 = 0;
    let mut lit_kind = LitKind::Declared;
    if si >= end {
        return Ok((toks, out));
    }
    if (op & 0xF0) == 0x20 {
        // stream-start pseudo-op: skip 4 bytes total, leading literal
        // count = low 3 bits of the 4th byte (its length/offset fields
        // are never applied by the decoder)
        si += 3;
        match_len = (src[si - 1] & 7) as u32;
        lit_op_bytes = 4;
        lit_kind = LitKind::Pseudo;
    }

    loop {
        if si >= end {
            break;
        }
        // Literal phase
        let count = if match_len == 0 {
            let (l, ob) = read_literal_length(src, &mut si, op);
            lit_op_bytes = ob;
            lit_kind = LitKind::Declared;
            l
        } else {
            match_len
        };
        toks.push(Tok::Lit { count, kind: lit_kind, op_bytes: lit_op_bytes });
        lit_op_bytes = 0;
        match_len = 0;
        lit_kind = LitKind::Trailing;
        replay_literal(src, &mut si, &mut out, count);
        if si >= end {
            break;
        }

        // Match chain block (copy_decompressed_chunks)
        op = src[si] as u32;
        si += 1;
        let (mut class, mut length, mut offset, mut next_op) = read_instructions(src, &mut si, &mut op);
        let mut first = true;
        loop {
            let trailing = next_op & 0x07;
            toks.push(Tok::Match { length, offset, class, first, trailing });
            if offset as usize <= out.len() {
                let start = out.len() - offset as usize;
                for i in 0..length as usize {
                    let b = out[start + i];
                    out.push(b);
                }
            } else {
                return Err(format!(
                    "match at {} references offset {} > out.len() {}",
                    out.len(),
                    offset,
                    out.len()
                ));
            }
            first = false;
            match_len = trailing;
            if match_len != 0 || si >= end {
                break;
            }
            op = src[si] as u32;
            si += 1;
            if (op >> 4) == 0 {
                break; // chain exit; the outer loop reads a literal length from op
            }
            let remapped = (op >> 4) == 15;
            if remapped {
                op &= 15;
            }
            let (c, l, o, n) = read_instructions(src, &mut si, &mut op);
            class = if remapped && c == Class::Long { Class::LongInChain } else { c };
            length = l;
            offset = o;
            next_op = n;
        }
    }
    Ok((toks, out))
}


/// gold's rs_form: decode_r2007.c:692 `page_size_if_rs_coded`.
fn page_size_if_rs_coded(len: usize) -> usize {
    let pesize = (len + 7) & !7;
    let block_count = (pesize + 251 - 1) / 251;
    (block_count * 255 + 31) & !31
}

/// One extracted page of one section: the declared frame fields,
/// the de-interleaved compressed stream and the decompressed chunk.
struct PageData {
    id: i64,
    offset: u64,
    uncomp: usize,
    comp_len: usize,
    chunk: Vec<u8>,
    comp_bytes: Vec<u8>,
}

/// All pages of one section, extracted and decompressed.
struct SectionData {
    name: String,
    encoding: u64,
    data_size: u64,
    pages: Vec<PageData>,
    slots: Vec<usize>, // the on-disk page sizes from the pages map, page order
}

fn extract_sections(
    file: &str,
    section_filter: Option<&str>,
) -> (String, Vec<SectionData>, usize) {
    let file_bytes = std::fs::read(file).expect("read file");
    let mut reader = DwgReader::from_file(file).expect("open DWG");
    let info = reader.read_file_header().expect("read file header");
    let mut out = Vec::new();
    for section in info.section_descriptors.iter() {
        if let Some(f) = section_filter {
            if section.name != f {
                continue;
            }
        }
        if section.page_count == 0 {
            continue;
        }
        let mut pages = Vec::new();
        let mut slots = Vec::new();
        for page in &section.pages {
            let comp = page.compressed_size as usize;
            let uncomp = page.decompressed_size as usize;
            let rec = info.page_records.get(&(page.page_number as i32));
            let (Some(&(_, on_disk)), true) = (rec, comp != 0 && uncomp != 0) else {
                continue;
            };
            slots.push(on_disk as usize);
            let chunk;
            let comp_len;
            let comp_bytes;
            if comp == uncomp {
                // stored page: raw content in the slot
                let start = AC21_FILE_HEADER_SIZE + rec.unwrap().0 as u64;
                chunk = file_bytes[start as usize..start as usize + uncomp].to_vec();
                comp_bytes = chunk.clone();
                comp_len = uncomp;
            } else {
                // encoding 4: RS(255,251) interleaved, factor 1
                let aligned = (comp + 7) & !7usize;
                let blocks = aligned.div_ceil(RS_DATA_K);
                let read_len = blocks * 255;
                let start = AC21_FILE_HEADER_SIZE + rec.unwrap().0 as u64;
                if start as usize + read_len > file_bytes.len() {
                    continue;
                }
                let encoded = &file_bytes[start as usize..start as usize + read_len];
                let mut comp_buf = vec![0u8; aligned];
                reed_solomon_decode(encoded, &mut comp_buf, blocks, RS_DATA_K);
                comp_bytes = comp_buf[..comp].to_vec();
                let mut padded = vec![0u8; comp_bytes.len() + 64];
                padded[..comp_bytes.len()].copy_from_slice(&comp_bytes);
                let mut out_buf = vec![0u8; uncomp + 64];
                decompress_ac21(&padded, 0, comp as u32, &mut out_buf);
                chunk = out_buf[..uncomp].to_vec();
                comp_len = comp_bytes.len();
            }
            pages.push(PageData {
                id: page.page_number,
                offset: page.offset,
                uncomp,
                comp_len,
                comp_bytes,
                chunk,
            });
        }
        out.push(SectionData {
            name: section.name.clone(),
            encoding: section.encoding,
            data_size: section.decompressed_size,
            pages,
            slots,
        });
    }
    (info.version_string.clone(), out, file_bytes.len())
}

/// The mirror-simulation: our stream sliced at her window boundaries,
/// compressed per slice, compared against her slots (the H8b gate).
fn mirror_sim(section: &SectionData, our_stream: &[u8], label: &str) {
    println!("  [mirror-sim {label}] our_stream={} her_data={}", our_stream.len(), section.data_size);
    for (i, page) in section.pages.iter().enumerate() {
        let start = page.offset as usize;
        let end = if i + 1 < section.pages.len() {
            section.pages[i + 1].offset as usize
        } else {
            our_stream.len()
        };
        if start > our_stream.len() || start >= end {
            println!(
                "    window {} id {}: our stream does not reach her boundary {}",
                i, page.id, page.offset
            );
            continue;
        }
        let end = end.min(our_stream.len());
        let chunk = &our_stream[start..end];
        let compressed = compress_ac21(chunk);
        let rs_form = page_size_if_rs_coded(compressed.len());
        let slot = section.slots.get(i).copied().unwrap_or(0);
        let her_rs = page_size_if_rs_coded(page.comp_len);
        let fit = rs_form <= slot;
        println!(
            "    window {} id {}: [{}..{}] our_comp={} her_comp={} rs_form={} slot={} her_rs={} -> {} (margin {})",
            i, page.id, start, end,
            compressed.len(), page.comp_len, rs_form, slot, her_rs,
            if fit { "FIT" } else { "OVER" },
            slot as i64 - rs_form as i64
        );
    }
}

/// Stream anatomy: byte-level comparison of our stream vs hers.
/// Dump both reconstructed section streams for offline byte study.
fn dump_raw(section: &SectionData, our_stream: &[u8], her_stream: &[u8], dir: &str) {
    let d = std::path::Path::new(dir);
    let _ = std::fs::create_dir_all(d);
    let base = section.name.replace(':', "_");
    std::fs::write(d.join(format!("{base}.her.bin")), her_stream).unwrap();
    std::fs::write(d.join(format!("{base}.ours.bin")), our_stream).unwrap();
    println!("  [raw] dumped {}/{}.her.bin ({} bytes) and {}.ours.bin ({} bytes)",
        dir, base, her_stream.len(), base, our_stream.len());
}

fn anatomy(_section: &SectionData, our_stream: &[u8], her_stream: &[u8], max_blocks: usize) {
    let n = our_stream.len().min(her_stream.len());
    let mut first_div = None;
    for i in 0..n {
        if our_stream[i] != her_stream[i] {
            first_div = Some(i);
            break;
        }
    }
    println!(
        "  [anatomy] ours={} hers={} first_diverge_at={:?} ({:.2}% into ours)",
        our_stream.len(),
        her_stream.len(),
        first_div,
        100.0 * first_div.unwrap_or(0) as f64 / our_stream.len().max(1) as f64
    );
    // Per-8KB density/equality map (first max_blocks blocks).
    const B: usize = 8192;
    let blocks = n / B;
    let show = blocks.min(max_blocks);
    println!("    per-8KB blocks (zeros% ours|her, equal% aligned):");
    for b in 0..show {
        let o = &our_stream[b * B..(b + 1) * B];
        let h = &her_stream[b * B..(b + 1) * B];
        let z = |s: &[u8]| s.iter().filter(|&&x| x == 0).count() as f64 / B as f64 * 100.0;
        let eq = o.iter().zip(h).filter(|(a, c)| a == c).count() as f64 / B as f64 * 100.0;
        println!(
            "      blk {:3} @{:7}: zeros ours {:4.1}% hers {:4.1}%  equal {:4.1}%",
            b,
            b * B,
            z(o),
            z(h),
            eq
        );
    }
    if blocks > show {
        println!("      ... ({} more blocks)", blocks - show);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut file = String::new();
    let mut section_filter: Option<String> = None;
    let mut our_rt: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--section" => {
                section_filter = Some(args[i + 1].clone());
                i += 2;
            }
            "--our-rt" => {
                our_rt = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                file = args[i].clone();
                i += 1;
            }
        }
    }
    if file.is_empty() {
        eprintln!("usage: ac21_token_diff FILE.dwg [--section NAME] [--our-rt RT_FILE]");
        std::process::exit(2);
    }

    let (version, her_sections, file_len) = extract_sections(&file, section_filter.as_deref());
    println!("file: {} ({} bytes, {}) sections={}", file, file_len, version, her_sections.len());

    let rt_sections = our_rt.as_ref().map(|rt| {
        let (_, ours, _) = extract_sections(rt, section_filter.as_deref());
        ours
    });

    for section in her_sections.iter() {
        println!(
            "== section {} encoding={} data={} pages={} comps={:?}",
            section.name,
            section.encoding,
            section.data_size,
            section.pages.len(),
            section.pages.iter().map(|p| p.comp_len).collect::<Vec<_>>()
        );

        // HER token stats for every encoding-4 page.
        let mut her = Agg::default();
        let mut her_stream: Vec<u8> = Vec::new();
        for page in &section.pages {
            her_stream.extend_from_slice(&page.chunk);
            if page.comp_len == page.uncomp {
                her.stored_pages += 1;
                continue;
            }
            let (toks, out) = walk_tokens(&page.comp_bytes)
                .unwrap_or_else(|e| panic!("her walk {} page {}: {}", section.name, page.id, e));
            assert_eq!(
                out, page.chunk,
                "her replay mismatch on {} page {}",
                section.name, page.id
            );
            her.add(&toks, page.comp_len, page.uncomp);
        }
        if her.pages > 0 {
            her.report("HER");
        }

        if let Some(rt) = &rt_sections {
            let our_sec = rt.iter().find(|s| s.name == section.name);
            match our_sec {
                Some(our_sec) => {
                    let mut our_stream = Vec::new();
                    for p in &our_sec.pages {
                        our_stream.extend_from_slice(&p.chunk);
                    }
                    // our own writer's compression per our own pages
                    let mut ours = Agg::default();
                    for page in &our_sec.pages {
                        if page.comp_len == page.uncomp {
                            ours.stored_pages += 1;
                            continue;
                        }
                        let (toks, out) = walk_tokens(&page.comp_bytes)
                            .unwrap_or_else(|e| panic!("rt walk {} page {}: {}", section.name, page.id, e));
                        assert_eq!(
                            out, page.chunk,
                            "rt replay mismatch on {} page {}",
                            section.name, page.id
                        );
                        ours.add(&toks, page.comp_len, page.uncomp);
                    }
                    if ours.pages > 0 {
                        ours.report("OUR-WRITER");
                    }
                    mirror_sim(section, &our_stream, "our stream in her windows");
                    anatomy(section, &our_stream, &her_stream, 24);
                    let raw = std::env::var("AC21_DIFF_RAW_DIR").ok();
                    if let Some(dir) = raw {
                        dump_raw(section, &our_stream, &her_stream, &dir);
                    }
                }
                None => println!("  [rt] section {} not present in our rewrite", section.name),
            }
        }
    }
}


