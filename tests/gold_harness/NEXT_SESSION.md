# Zero-context prompt — the post-H8a halt: the AC21 container-mirror queue at 3,576

> Campaign state 2026-09-26 (the halt after the H8a landing + this
> halt-refresh; the session ran the review → update → commit → push →
> continue loop from the 3,576 post-mirror halt). **The ACS/SH
> campaign stays COMPLETE at 0/0: corpus 280 files, read 0, write 0,
> read key-gap 0, write-target key-gap 3,576 (UNCHANGED — H8a is
> read-inert).** The §19 structure READ axis stays at ZERO
> corpus-wide. The H8b work (the AC21 container-shape WRITE mirror)
> is fully scoped, machine-grounded, and ready to implement — its
> complete emission contract lives in §19.2's "H8a" paragraph of
> `IMPLEMENTATION.md` and is restated below. Read
> `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3 + §18.5–18.7
> in `IMPLEMENTATION.md` (§18.6 carries the full decode record), then
> §19.1–19.3 (the structure campaign — §19.2's H7 row carries the
> H7a–H7g landings + the wall's residue map, and §19.2's H8a
> paragraph carries the AC21 mirror contract), then this file top to
> bottom.

## What the last session established (the load-bearing facts)

**The review pass (the halt-state verification + the gold re-scan)**:
the corpus re-ran clean (280 files 0/0, read key-gap 0, write-target
3,576, per-key census matching the halt record exactly: HEADER
138,021 @ 0/0, FILEHEADER write 103, R2004_Header 963, R2007_Header
925), `cargo test --features serde` stayed green, and
`gold-vs-silver` was verified PUSHED (remote `ls-remote` head ==
local HEAD). The gold re-scan re-verified every standing anchor
(LOFTEDSURFACE dwg2.spec:3984, LWPOLYLINE dwg.spec:5446,
r2004_file_header.spec:41-53, decode.c:1432/1683, codepages.h dual
pairs) and found ONE drift: `src/config.h.in` in the gold tree is
modified as a tracked file (an `autoheader`/`autoreconf` run
reformatted its quoting: `'foo'` → `` `foo' ``, "C89" → "C90",
`<stddef.h>` → `<sys/types.h>` size_t comment). It is a
generated-template drift, NOT a source-semantics change, and the
oracle binary is untouched (programs/dwgread mtime Sep 15 20:57,
md5 e01356f9e69839a61a5f4aa674ce53e5c recorded this session). DO
NOT touch it (the read-only-oracle rule holds); treat as the new
recorded state of the gold tree at `34f02f54`.

**The AC21 container analysis (the H8b design — machine-verified on
example_2007)**: gold's R2007 reader contracts are pinned from
`decode_r2007.c` itself (the −v9 prose is misleading; trust the code):
- `read_pages_map` (decode_r2007.c:1099): entries are (size RLL, id
  RLL); the PHYSICAL OFFSETS ARE A RUNNING SUM from 0x480 in
  map-listed order. The pages-map bytes are just (size, id) pairs.
- `read_data_section` (:856 dispatch): comp != uncomp → `read_data_page`
  ALWAYS (mandatory RS de-interleave: block_count = ceil(align8(comp)/
  251), block_count×255 read, trailing pad ignored); comp == uncomp &&
  page->size == rs_form(comp) → RS de-interleave of a STORED page;
  else → raw memcpy of uncomp bytes from page->offset (slack ignored).
- `rs_form(x)` = `align32(ceil(align8(x)/251)×255)` (decode_r2007.c:692).
- The sections-table record (silver's `DwgSectionInfo`/
  `DwgPageCrcInfo` parse order is gold-exact, verified by the 0/0
  corpus): per section (data_size, max_size, encrypted, hashcode,
  name_len, unknown, encoded, num_pages, UTF-16LE name), then per page
  7×RLL (offset-in-decomp-stream, frame, id, decomp, comp, checksum,
  crc). The −v9 "size" print = the FRAME field (== uncomp on full
  pages); comp prints only in the section data's own logs.
- The section-page ids are a CHOICE, not a structure: our writer's
  section map keeps OUR honest 7-field values; only the PAGES-map
  extent/id parity is load-bearing.

**The author's AC1021 layout (example_2007, machine-verified tiling —
all deltas 0)**: [0x80 metadata block][0x400 file-header page
@0x80][pages map page id 25 @0x480 (1024, correction
7)][pages-map COPY id 26 @0x880 (1024)][data pages 3..20 in ID
order: header-vars 160 @0xc80, preview 307,936 @0xd20 (comp
303,104, uncomp 302,255), thumb-adjacent small pages, summary 1440
@0x4c320, objects pages 8..14 (63,488/58,624/70,912/65,536/63,488/
69,120/remainder — IRREGULAR capacities, not 0xF800), …][sections map
id 21 (4864, correction 4) @0x6cb40][sections-map COPY id 22 (4864)
@0x6de40][header2 0x480-byte region (file-header copy 0x400) at
metadata-relative 453,824], file_size 456,000. pages_amount 22,
pages_maxid 26 (ids 23/24 unused). 13 sections in table order
([0] AppInfoHistory … [2] Preview id 4, [3] SummaryInfo id 6, [5]
Objects 7 pages, [11] Header id 3 (78 bytes), [12] the unnamed
0-page descriptor — gold "Invalid num_pages 0, skip"). The 0x80
FILEHEADER identity: summaryinfo_address 3200 (0xc80 = the
Header-vars page's offset), thumbnail_address 3360 (0xd20 = the
Preview page's offset), r2004_header_address 128, codepage 30,
dwg_version/app pair 33/29 — RETAIN the author's two addresses
verbatim under the mirror (dwg_file_header_summary already holds
them; they stay valid positions in the mirrored layout).

**H8a — the reader retention (landed this session)**:
`DwgAc21ContainerShape` on the document (`dwg_ac21_shape`, "
serde-skipped): `map_order` (the pages-map (id, size) entries in her
physical order, collected in `read_page_map_ac21`) + `sections` (per
author descriptor: name, encoding, data_size + per-page (offset,
frame, id, uncomp) with each id's on-disk size resolved from
`page_records`; composed in `read_file_header_ac21` after both maps
parse). Stored transfer-time beside `dwg_r2007_header` (gated on
`ac21_metadata.is_some()`). Sites: src/document.rs (structs after
`DwgR2007SystemHeader`), src/io/dwg/dwg_reader.rs (info fields +
`read_file_header_ac21` + the read_page_map_ac21 collector + the
transfer store), src/document/semantic_inventory.rs:166 (the
exhaustive destructure lists `dwg_ac21_shape: _`).

**H8b — the write mirror (THE next packet; emission contract)**:
1. In `write_ac21_impl` (dwg_writer.rs:1872), after the section
   buffers build, run `ac21_mirror_plan(document, buffers)`:
   same-origin + shape present; per author section: our buffer of
   that name exists and its len ≥ her last-page offset; per her page:
   our chunk (our_buffer[her_off .. next_off]) compresses with
   rs_form(our comp) ≤ her on-disk size (or comp == store-len within
   the collision rule); our section-name set == her table's non-empty
   names. Any fail → decline → conventional path (byte-identical
   fallback — stash-verified doctrine from H7g).
2. Emit sequentially from 0x480 in HER map_order: her first two
   entries are the PAGES-MAP system pages → build the pages-map
   bytes FIRST (her (size, id) pairs — byte-identical ⇒ encode with
   her correction factor; it lands hers), write at 0x480/0x880 with
   her sizes/ids; then her data pages in order (our chunk per page:
   comp < chunk → RS form of our stream (block_count×255 ≤ her size)
   + zero pad to her size; stored → rs_form(chunk) == her size ?
   emit the RS form (comp == uncomp declared; gold de-interleaves it
   correctly) : raw chunk + pad to her size); then the
   sections-map/2 system pages at the tail (OUR honest map bytes
   with per-page values (her offset, her frame, her id, OUR
   uncomp/comp/checksum/crc — all fields in the silver writer's
   existing 7-field order), encoded into HER on-disk sizes, ids
   21/22).
3. Metadata assembly (write_file): unchanged flows — under the
   mirrored layout pages_map_offset/map2_offset/ids/amount/maxid
   land her values naturally; the sections-map family stays OURS
   (honest); force NOTHING except the 0x80 addresses (the retained
   author values) + `set_mirror_ids`-style assertions under
   AC21_MIRROR_DEBUG.
4. Predicted closes (~−430): FILEHEADER 103 → 21 (AC1021 82 gone;
   R2000 7 + fallback 14 stay), THUMBNAILIMAGE 50 → 0, R2007_Header
   925 → ~450 (the pages-map family; the sections-map family + six
   CRCs + file_size/header2 stay), leaving the MT-derive family
   (3/file, the §5.11 instrument follow-up — see the standing AC21
   companion notes in §19.2's H7g row).
5. Decline classes expected (AC18-analogs): objects-overflow (the
   2007 Constraints/Dynblocks-style files), the FileDepList
   missing-content class (the parallel session's row re-engages them
   for free), and any name-set mismatch.

## The remaining queue (3,576) — the wall's residue map

- **R2007_Header 925**: the H8b mirror column (the pages-map family,
  ~12/file) + the content-coupled family (layout offsets, sizes,
  six CRCs, corrections — irreducible without whole-file byte
  identity) + the 3-per-file MT-derive family (§5.11).
- **FILEHEADER 103** (= AC1021 82 + R2000 7 + fallback 14): the
  AC1021 side closes through H8b; the R2000 seeker pair needs
  flat-layout parity.
- **THUMBNAILIMAGE 50**: closes through H8b (the preview page is a
  single page; the content is retained-raw since H5c).
- **FileDepList 1,055 + SecondHeader 386 + AuxHeader 94**: the
  PARALLEL SESSION's rows (untracked probes `h7_probe1.sh`/
  `h7_rows.sh` in the repo root — not ours to commit).
- **R2004_Header 963 = 4 × 225 + 9 × 7**: accepted AC18 residue.

**Dead / no-path rows (do not re-litigate)**: `LoftD`; the SH revolve
option shorts; **BREP stays deferred** (external authentic
ACSH_BREP_CLASS specimen required).

## The standing facts

- The four raw-retained SH tails decode to typed views with the
  captured bits as the write authority (Phase B). Hermetic suite:
  20 tests + module tests, all green after H8a.
- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative.
- **The generation identity should be untouched by H8a
  (`40ab5d356cf05a71333f208e1651daf`, 25,344 bytes — H8a changed no
  writer path; re-verify per the README's "The zero-keeping workflow"
  if H8b changes the writer).**
- The gold tree sits at `34f02f54` with ONE tracked generated-file
  drift (`src/config.h.in`, autoheader requote — recorded above; the
  oracle binary is UNCHANGED and the freeze rule stands).
- The anchors re-verified this session: the H7g set (dwg2.spec:3984,
  dwg.spec:5446, r2004_file_header.spec:41-53, decode.c:1432-1870,
  codepages.h) + the AC21 set (decode_r2007.c:692/:700-852/:856-:1099,
  out_json.c:2339-2376, the Dwg_R2007_Header struct in include/dwg.h).
- The parallel board showed NO peer posts at this halt (main-only).

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes [with `\$`], nested quotes and multi-word
grep alternations are all broken; ONE COMMAND PER LINE in script
files; `sleep` is capped at 120 s — use the tracked background
process for the corpus).

```bash
# Environment (source this):
export PATH="$HOME/.cargo/bin:$PATH"
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
```

## Verification gate (the zero-keeping rule applies to 0/0)

```bash
# 1. Build gates
cargo test --features serde          # green at this halt (all suites ok)
cargo test --features gold-harness --test gold_roundtrip

# 2. Family smokes (must stay 0/0)
python3 tests/gold_harness/run_roundtrip.py \
    tests/gold_harness/tests/sh_history/<FIXTURE>.dwg /tmp/smoke
# example_2007: use $GOLD_TESTDATA/example_2007.dwg
# (AC18 fixtures at write-target 4; Box_2007/example_2007 at 25 until H8b)

# 3. Full corpus (280 files 0/0; read 0; write-target 3,576 at this halt)
python3 tests/gold_harness/run_corpus.py

# 4. Layer-4 byte-walk for writer re-encode paths
target/debug/dump_section_bytes <file> <A> <N>
```

## Commit inventory (this halt — all PUSHED)

```
<docs> docs(harness): the post-H8a halt refresh — the AC21 container-mirror
       contract, the H8a reader retention, the gold-tree drift record
<feat> feat(dwg): the H8a AC21 container reader retention — the AC1021
       author's page space retained (write-target 3,576 unchanged; read-inert)
16ee668 <review> ... (the prior halt series — see the previous NEXT_SESSION
       inventories: the H7g container-shape mirror, the code-page conflation
       split, the H7a-H7f landings, the §18 walks, the 2026-09-25/26 §19 arc)
```

**PUSH STATE (2026-09-26)**: the `gold-vs-silver` branch is PUSHED
through this halt; push after each landing per the maintainer's loop
instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (corpus
280 @ 0/0 at 3,576, tests green, remote synced) → the gold src
re-scan (anchors ✓, the config.h.in drift found and recorded) → the
required reading (§F2, §18.5-18.7, §19.1-19.3) → the AC21
container analysis (the −v9 page-space dump + gold decoder
archaeology incl. the dispatch contract, rs_form, the system-page
repeat factors, the metadata identities, the 0x80 convention — the
−v9 prose cross-checked numerically: tiling exact) → H8a (the
DwgAc21ContainerShape retention: structs, map-order collector,
shape composition, transfer store, the semantic_inventory
destructure fix) → gates (build green, hermetic green, the
example_2007/Box_2007 smokes 0/0 at 25, corpus 280 @ 0/0 at 3,576)
→ the halt refresh. The corpus held 280 @ 0/0 and the tests green
through every step; the write-target key-gap stays 3,576 — the H8b
landing is the next big move, and its whole emission contract is
recorded in §19.2's H8a paragraph.
