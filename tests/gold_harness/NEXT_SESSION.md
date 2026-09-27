# Zero-context prompt — the post-H8c halt: the encoder-ratio wall REFUTED (the content wall named; the instrument landed)

> Campaign state 2026-09-27 (the halt after the H8c landing; the session
> ran the review → update → commit → push → continue loop from the
> 3,576 post-H8b halt). **The ACS/SH campaign stays COMPLETE at 0/0:
> corpus 280 files, read 0, write 0, read key-gap 0, write-target
> key-gap 3,576 (UNCHANGED — H8c landed as the measurement instrument
> + the refutation record; the one encoder emission change the
> measurement motivated was REVERTED per the §19.3 zero-keeping rule
> after it measured +4 on the corpus).** The §19 structure campaign's
> READ axis stays ZERO corpus-wide. Read
> `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3 + §18.5–18.7
> in `IMPLEMENTATION.md` (§18.6 carries the full decode record), then
> §19.1–19.3 (§19.2's H8c row carries this landing's record), then
> this file top to bottom.

## The H8c verdict (this session's review finding — supersedes the prior halt's encoder theory)

**The prior halt's "AC21 LZ77 encoder ratio wall" is REFUTED.** The
halt's 1.16–1.87× figures were real measurements misattributed: they
compared OUR content (in her window boundaries) against HER content's
compression. Measured on IDENTICAL content — her own decompressed
bytes — our compressor BEATS the author's encoder on every tested
file's objects section:

- example_2007: ours 118,803 vs hers 121,989 (−2.6%)
- Box_2007: ours 25,589 vs hers 26,744 (−4.3%)
- circle.dwg: ours 25,126 vs hers 26,356 (−4.7%)

**The real wall is CONTENT, in two layers:**

1. **Layout**: our objects stream carries the same constituent masses
   in a different ORDER (circle: ours = [dense head][zero mass
   @16–40K][uniform ~35%-zeros mass to the tail]; hers = [sparse
   head][uniform mass @8–129K][dense pair 129–150K][zero mass
   @172–199K]; totals 199,227 vs 199,139 — the same bytes
   rearranged). Her window 0 slot is the EXACT rs_form of her ~9.5K
   comp (zero slack); our window-0 content compresses to ~17.9K —
   a −8,672 margin unclosable by any encoder work.
2. **Values**: record-level byte divergences from the first bytes
   (first divergence at byte 4–22 on the minis — head-record content
   including raw-double mantissa deltas, not bit-form deltas).

The mirror fit tolerance for 9–50K comp windows is one RS block
(~255 bytes, 0.5–1.8%): only near-byte-identity of the objects
stream can engage the mirror. **The "alternative unlock" the prior
halt recorded IS the real path: the layer-4 objects-stream parity**
(her record layout/order + her head-record byte values). The
uniform minis (Arc/ConstructionLine/Donut/Constraints/Ellipse/…)
all transpose the same two masses — one objects-parity fix should
move the whole class. ATMOS-DC22S fits every reachable window on
content alone (margins +256 to +17,600) — its decline is the
gap-entry class + our shorter stream, NOT encoding.

## The instrument (landed, committed)

`ac21_token_diff` — a new harness bin
(`tests/gold_harness/src/bin/ac21_token_diff.rs`, registered in
Cargo.toml):

- Extracts her on-disk comp per page (RS de-interleave, factor 1,
  RS(255,251)) and decompresses each page — replay-validating every
  walk byte-for-byte against `decompress_ac21`.
- Walks compressed streams with a token walker replicating the
  decoder state machine EXACTLY (the ctor 0x2X pseudo-op skip, the
  trailing-literal state machine, the chain-continuation
  `0xF_ → 0x0_` remap) — VERIFIED against gold C line-by-line
  (decode_r2007.c:142/320/361/431; the 31-pattern literal reorder
  table matches our Rust port byte-for-byte).
- `--our-rt RT_FILE`: the mirror-sim — reconstructs our rewrite's
  objects stream, slices it at HER window boundaries, scores the H8b
  fit per window (rs_form vs slot — the exact gate rule), and maps
  both streams' anatomy (per-8KB zero-density and aligned-equality).
- `AC21_DIFF_RAW_DIR=dir` dumps both reconstructed section streams
  for offline byte study.

Usage: `ac21_token_diff FILE.dwg --section AcDb:AcDbObjects
--our-rt OUR_RT.dwg` (build: `cargo build --bin ac21_token_diff
--features serde`).

## The 0xF_ chain-long finding (FOUND, VERIFIED, DEFERRED — do not re-litigate the deferral)

The census's one true encoder-grammar gap: chain-continuation long
matches should ride the author's remap form (opcode
`0xF0|len_field`, 3 bytes — the decoder masks 0xF_→0x0_ at exactly
that position, gold C decode_r2007.c:537-538) instead of our 4-byte
extended fallback. Her census: long(F) = 519 of 3,676 matches on
circle, 1,417 of 11,849 on example_2007. The emission change was
implemented + hermetic-tested + measured on the FULL corpus: it
moved R2007_Header 925→929 (net +4: 9 files gained a layout-coupled
row, 5 lost one — the pages_map_correction/header2_offset/offset
family reshuffling coincidences as our comp sizes crossed block
boundaries). §19.3 (struct moves only toward 0) → REVERTED; the
compressor stays at HEAD; 3,576 re-verified by a stash-run corpus.
**The change RELANDS with the objects-parity packet** — it is
REQUIRED there (her streams use the 0xF_ form; byte-identity demands
it) and every affected row closes anyway. The implementation is
recorded in §19.2's H8c row (the exact edit: `emit_chained_match`'s
nibble-0 branch emits `enc.bytes[0] |= 0xF0` instead of calling
`encode_extended`; the after-gap position keeps the class-0 form —
the prior halt's 0xF-misuse fix stands).

## The gold re-scan (this session — everything re-verified)

- Tree: `34f02f54` FROZEN, the tracked drift still ONLY
  `src/config.h.in` (the autoheader requote). DO NOT touch anything
  there.
- Oracle fingerprints (the corrected record holds): the 6,316-byte
  libtool wrapper `programs/dwgread` (md5
  `8dad57211b78f42e7594ba6c211cc0e5`) and the ELF
  `programs/.libs/dwgread` (md5
  `d852da1db0894b866e86042d4b26b91d`, 230,232 bytes), both mtime
  2026-09-15 20:56:22.
- Every pinned decode anchor re-verified at its line: rs_form @
  decode_r2007.c:692, decode_rs @:553 (byte-interleave, RS correction
  commented out), read_system_page @:590 (pesize =
  align8(comp)×repeat; bc = ceil(pesize/239); page_size =
  align8(bc×255)), read_data_page @:700 (k=251), the size-only
  RS/stored collision dispatch @:856, read_pages_map @:1099;
  decode.c:1432/1683; dwg2.spec:3984; dwg.spec:5446;
  r2004_file_header.spec:41-53; codepages.h.
- **NO `encode_r2007.c` exists** — gold has no R2007 writer; her
  on-disk streams remain the only author-encoder ground truth (the
  instrument above is the extraction path).

## H8d — the next packet: the objects-stream parity

The layer-4 doctrine applied to AC1021: make our rewritten
`AcDb:AcDbObjects` stream byte-identical to hers. Two named layers
from the anatomy:

1. **The record ORDER/mass layout**: our stream emits the section's
   records in a different arrangement than her author-time layout
   (the two-mass transposition). The candidate mechanisms: object
   emit order (her handle-map order vs our document order), the
   objfree-space/handles section coupling, or page-boundary
   alignment choices. The instrument's anatomy mode + raw dumps
   (`AC21_DIFF_RAW_DIR`) are the measurement path; the first-divergence
   token alignment (her stream vs ours over the same window) names
   the first record that moves.
2. **The head-record VALUES**: byte 4–22 divergences on every mini —
   the first objects record's fields (raw doubles incl. mantissa
   deltas). These are objects-section CONTENT fields (the §19
   OBJECTS axis is frozen at 0 on the READ side — our reader derives
   them 0/0 — so the divergences are in our WRITER's re-encoding of
   the same values: bit-form or value-form choices, e.g. BD raw vs
   short forms, or actual value drift in re-derived fields).

Acceptance = the mirror engage census (`AC21_MIRROR_DEBUG=1
python3 tests/gold_harness/run_roundtrip.py …` per file): every
engaged AC1021 file closes its FILEHEADER AC1021 + THUMBNAILIMAGE +
R2007 pages-map family rows automatically (the H8b machinery is IN
and stash-safe). The 0xF_ emission relands with this packet.

## The remaining queue (3,576 — unchanged by this halt)

- **R2007_Header 925**: the H8b mirror column is LIVE (engages when
  H8d lands); the content-coupled family (sections-map CRCs, six
  CRCs, file_size/header2, the 3-per-file MT-derive draws) + the
  pages-map crc_compressed rows stay open until byte-identity of the
  streams.
- **FILEHEADER 103** (= AC1021 82 + R2000 7 + fallback 14) and
  **THUMBNAILIMAGE 50**: close on the AC1021 side through H8d; the
  R2000 seeker pair needs flat-layout parity.
- **FileDepList 1,055 + SecondHeader 386 + AuxHeader 94**: the
  PARALLEL SESSION's rows (untracked probes `h7_probe1.sh`/
  `h7_rows.sh` in the repo root — not ours to commit).
- **R2004_Header 963 = 4 × 225 + 9 × 7**: accepted AC18 residue.

**Dead / no-path rows (do not re-litigate)**: `LoftD`; the SH revolve
option shorts; **BREP stays deferred** (external authentic
ACSH_BREP_CLASS specimen required).

## The standing facts

- The four raw-retained SH tails decode to typed views with the
  captured bits as the write authority (Phase B). Hermetic suite
  green after H8c (the compressor module at its HEAD 40; the probe
  bin is analysis-only, no lib change).
- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative.
- The generation identity is UNTOUCHED (`40ab5d356cf05a71333f208e16
  51daf`, 25,344 bytes — verified twice this session, including once
  WITH the since-reverted emission change: the generated file's
  AC1021 streams contain no chain-longs, so even the change left it
  identical).
- The gold tree sits at `34f02f54` with ONE tracked generated-file
  drift (`src/config.h.in`, autoheader requote) — the freeze rule
  stands.
- No parallel-board activity at this halt (main-only).

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use the tracked background process for
the corpus — the full corpus takes ~4 minutes).

```bash
# Environment (source this):
export PATH="$HOME/.cargo/bin:$PATH"
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
```

## Verification gate (the zero-keeping rule applies to 0/0)

```bash
# 1. Build gates
cargo test --features serde          # green at this halt
cargo test --features gold-harness --test gold_roundtrip

# 2. Family smokes (must stay 0/0)
python3 tests/gold_harness/run_roundtrip.py \
    tests/gold_harness/tests/sh_history/<FIXTURE>.dwg /tmp/smoke
# example_2007: use $GOLD_TESTDATA/example_2007.dwg
# (AC18 fixtures at write-target 4; Box_2007/example_2007 at 25 —
#  all AC1021 smokes hold their engage-pending numbers until H8d)
# AC21_MIRROR_DEBUG=1 prefixes any run to trace the gate decision.

# 3. Full corpus (280 files 0/0; read 0; write-target 3,576)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 40ab5d356cf05a71333f208e1651daf, 25,344 bytes

# 5. The H8c instrument (analysis-only)
cargo build --bin ac21_token_diff --features serde
./target/debug/ac21_token_diff "$GOLD_TESTDATA/2007/circle.dwg" \
    --section AcDb:AcDbObjects --our-rt <OUR_RT>.dwg
```

## Commit inventory (this halt — all PUSHED)

```
<feat> feat(harness): the H8c AC21 token-stream differential — the
       instrument + the encoder-ratio refutation (the wall is
       objects-stream content layout; the 0xF_ chain-long grammar
       gap found + deferred per zero-keeping; 3,576 unchanged)
<docs> docs(harness): the post-H8c halt refresh — the refutation
       record, the content-wall anatomy, the H8d objects-parity
       packet, the gold re-scan
37cc8ad <docs> / 06c12a7 <feat> ... (the prior halt series — see the
       previous NEXT_SESSION inventories: the H8b mirror, the H8a
       retention, the H7g mirror, the H7a-H7f landings, the §18
       walks)
```

**PUSH STATE (2026-09-27)**: the `gold-vs-silver` branch is PUSHED
through this halt; push after each landing per the maintainer's loop
instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (battery:
serde green, gold_roundtrip green, the halt's 3,576 confirmed by
the stash-run corpus later in the session) → the required reading
(AGENTS, §F2, §18.5-18.7, §19.1-19.3) → the instrument built (the
token walker replicating the decoder state machine, replay-
validated per page; the mirror-sim; the anatomy mode) → the
refutation measured (our compressor beats hers on her own content
on every tested file; the halt's figures re-derived as
layout measurements) → the content wall named (the two-mass
transposition + the head-record value divergences; the ATMOS
content-fits census) → the 0xF_ gap found + implemented + tested +
corpus-measured (+4 → REVERTED per §19.3; the finding recorded, the
reland named) → the gold re-scan (tree + fingerprints + every
anchor + no-encoder confirmation) → the battery (serde green,
gold_roundtrip green, smokes 0/0, generation identity twice) →
the halt refresh. The H8d objects-parity packet is fully scoped
above with its instrument; the H8b mirror machinery stays LIVE and
stash-safe, waiting on the content.
