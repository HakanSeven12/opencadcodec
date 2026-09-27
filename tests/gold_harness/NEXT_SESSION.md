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
   (first divergence at byte 4–22 on the minis). **CORRECTED by the
   H8d opening measurement below: the byte-22 divergence is object
   0's TAIL (the 1s-padding byte), not head-record values — the
   records' data prefixes are byte-identical almost everywhere.**

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

## H8d — the next packet: the objects-stream parity (α slice VERIFIED this session)

The layer-4 doctrine applied to AC1021: make our rewritten
`AcDb:AcDbObjects` stream byte-identical to hers. The α slice
(record-level form parity) was implemented, measured, and is
FULLY VERIFIED — then reverted per zero-keeping with the exact
re-apply package below. The state of the wall:

**VERIFIED THIS SESSION (all on circle.dwg, traces + the
instrument, current-writer measurements):**

- **Handles are identical 211/211** (same order, same values) —
  no renumbering; no preservation work needed. (Two earlier
  theories refuted: renumbering, and "renumbers-stub" forms.)
- **The record skeleton**: `[MS][type][data][string stream
  R2007+][handle bits packed into main's tail][1s-pad to
  byte][crc16(0xC0C1, [MS..span]) after the span]` — the
  after-span CRC is IDENTICAL RULE both sides (our writer
  already correct); gold's "(addr+size−2)" CRC read is a
  phantom (no per-object CRC verdicts on the R2007 path).
- **The ownerhandle rule — 196/196 verified**: relative iff
  (rel_len < abs_len) OR (rel_len == abs_len AND |offset| <
  owner_value); her census = 78 absolutes + 118 relative + 10
  nulls. (NOT "always relative".)
- **The 1s final pad — mechanics**: the record's final partial
  byte is MAIN's closing shift (the handle sub-writer is ALWAYS
  byte-aligned at merge — the bits pack into main's tail
  mid-byte); the author pads with 1s (verified AC15/AC18/AC21
  samples all-ones).

**THE RE-APPLY PACKAGE (4 edits, ~30 minutes):**

1. `bit_writer.rs`: add `write_spear_shift_ones()` (the same
   loop as write_spear_shift with `write_bit(true)`).
2. `merged_writer.rs`: in merge_two_stream AND
   merge_three_stream, the FINAL `self.main.write_spear_shift()`
   → `write_spear_shift_ones()` (only the final one — the
   intermediate pads verified byte-identical).
3. `bit_writer.rs`: add `write_first_ref_handle(ref_type,
   reference, handle)` implementing the ownerhandle rule
   (rel_len/abs_len via handle_byte_count, tie → offset<value;
   relative → write_handle_relative, else write_handle).
4. `common.rs`: the entity path (entmode==0, r2007_plus, owner
   non-null → write_first_ref_handle) + the non-entity internal
   (r2007_plus non-null owner → write_first_ref_handle; the
   explicit relative_owner request paths keep
   write_handle_relative; nulls keep code-4) + a
   `write_first_ref_handle` delegate on merged_writer.

**Measured results of the α slice**: form census == hers
EXACTLY (4:88, 8:38, 12:75, 10:5); **206/211 records
BYTE-IDENTICAL** (from 6); stream totals IDENTICAL (199,139);
smokes 0/0 (circle, example_2007, Box_2007). Remaining record
rows — 5, AUTOPSIED (the α-era dumps, byte-8 windows at the
divergence): **the three LAYOUTs share ONE single-bit delta**
(bit 2 of a handles-tail byte: her `0x94`/`0x14` vs our
`0x90`/`0x10` at +227/+93/+93 — one common handle-form field,
likely one fix for all three); **obj 69 STYLE + obj 74
DIMSTYLE diverge beyond the head** (+7: her `0x12`/`0x52` vs
our `0x11`/`0x50`, and the following 8-16 bytes differ broadly
with IDENTICAL bitsize/positions — same-length different-form
field patterns in the head/EED/name region: a spec-ordered
field walk (the §8.1.6 recipe, bits.c semantics) names them;
expect one or two form rules).

**The corpus measured the α slice: R2007_Header 925→927 (+2,
the coincidence-row mechanism — the 0xF_ lesson); read 0/0
held; every other key unchanged. Per §19.3 the writer edits
were REVERTED (git checkout of the three files) — the tree is
at the d935181 3,576-verified state. THE α SLICE RELANDS WITH
THE REMAINING LAYERS AS THE ONE UNIT:**

1. The 5 record autopsies (above) — POLISH, NOT BLOCKERS (see the
   placement re-scope below: the mirror path does not consume our
   emission).
2. **THE PLACEMENT LAYER — RE-SCOPED AFTER THE LOOP DIAGNOSIS
   (the maintenance review of 2026-09-27 found the sessions
   stuck in a measure→verify→revert loop: seven docs commits, one
   analysis feat, ZERO parity landings since H8b; 3,576
   unmoved)**. The loop's root cause was a framing error: the
   placement was scoped as "derive her allocator's rule" (big,
   risky surgery) when the codebase ALREADY HOLDS the pattern
   that solves it — RAW RETENTION + VERBATIM ECHO (the SH tails;
   the XrefManifest fallback IN THE H8b MIRROR ARM ITSELF:
   "the section re-emits her raw bytes verbatim"). Her physical
   layout is her editor's incremental-save allocation history —
   unmodelable by rule, and the mirror's own doctrine for
   unmodelable authored state is echo. THE IMPLEMENTATION
   (verified against the code this session):
   (a) READER: retain her reconstructed objects-section stream
       (the reader already rebuilds it while reading objects —
       the same reconstruction the instrument performs; one
       `Vec<u8>` on the document, the H8a-retention pattern);
   (b) WRITER: in `write_ac21_impl`'s MIRROR arm, pass her raw
       as the `objects` buffer — `Ac21MirrorBuffers.objects` is
       ALREADY an abstract `&[u8]` slot (dwg_writer.rs:2085);
       the conventional arm keeps our own emission untouched;
   (c) the gate then measures our comp == her comp per window →
       rs_form == her slot exactly → EVERY objects window fits →
       the 57 objects-decline files ENGAGE (ATMOS stays declined
       on the gap-entry class — legit).
   THE DEADLOCK RESOLUTION: the α slice's measured +2 was an
   artifact of partial parity while the files still took the
   CONVENTIONAL arm; the unit (α + raw-echo) flips those files
   to the mirror arm, where the conventional coincidences stop
   being measured. Land α + raw-echo TOGETHER; the generation
   identity RE-RECORDS (the conventional path changes — the
   gate says re-run when the writer changes; document the new
   md5 in the halt record).

**LAND AS A UNIT** (α + raw-echo together); acceptance = the
mirror engage census (`AC21_MIRROR_DEBUG=1 python3
tests/gold_harness/run_roundtrip.py …` per AC1021 file): every
engaged file closes its FILEHEADER AC1021 + THUMBNAILIMAGE +
R2007 pages-map family rows automatically (the H8b machinery is
IN and stash-safe). The 0xF_ emission relands with this packet.

**THE HONEST MOVEMENT FORECAST (recorded so the loop is judged
by results)**: the H8d landing is the FIRST row movement since
H8b — the engaged files close the bulk of R2007_Header 925 +
FILEHEADER's AC1021 82 + THUMBNAILIMAGE 50 ≈ **1,057 rows →
the total drops 3,576 → ~2,519**. THE CEILING, stated plainly:
the remaining ~2,498 rows are NOT this queue's to move —
FileDepList 1,055 + SecondHeader 386 + AuxHeader 94 = 1,535 are
the PARALLEL SESSION's rows, and R2004_Header 963 is accepted
AC18 residue (no active path). "Target = zero" for THIS queue
means the H8d family plus the follow-up polish (the 5 record
rows, the R2000 flat-layout pair, the fallback residue) — the
number's floor without the parallel session and an AC18
decision is ~2,498, and every halt record should say so
instead of implying 0 is reachable from this queue alone.

**Spec authority on file**: the ODA spec PDF
(`~/work/OpenDesign_Specification_for_.dwg_files.pdf`, 270pp;
the record-format chapter pp. 99–103; pypdf installed --user in
WSL python) + libredwg `bits.c` (`bit_read_H`, the H-code table:
6/8 = own±1, 10/12 = own±offset, 2–5 direct, resolved in
`dwg_resolve_handleref`).

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

## The zero-reachability audit (verified 2026-09-27, on the corpus itself)

**Zero is technically reachable with NO additional custom
examples** — the corpus exercises every remaining row family.
Verified per family:

- **FileDepList 1,055 — the §19.3 fixture ask is STALE**: the
  corpus files carry real content (circle.dwg: `num_features: 2`,
  comp 150; 273 files bear the section). The read axis is 0/0;
  the rows are write-side only (our writer emits an empty
  section). Closing = retain + emit from the document, verified
  against the corpus directly. No fixture needed.
- **SecondHeader 386**: all-missing — our writer skips the
  section on the 7 files that carry it; retention + emission
  closes it. Bounded.
- **AuxHeader 94**: value diffs with the read axis matching —
  retain the author values / fix the defaults. Bounded.
- **FILEHEADER's AC1021 rows are ADDRESS rows** (verified by
  classification: `thumbnail_address`/`summaryinfo_address` —
  e.g. Box_2007 gold=3360 vs ours=1280): they close
  automatically when the mirror engages (her container → her
  addresses). The R2000 pair is the same class for the flat
  layout (example_2000: gold=220 vs ours=557752). Bounded.
- **R2004_Header 963 — the ONLY genuine scope decision, and its
  row anatomy is now VERIFIED (2026-09-27 probe on the corpus
  report)**: the rows are ADDRESS/ID fields —
  `last_section_address` / `secondheader_address` /
  `section_map_address` (+ `last_section_id`) — across **147
  R2004+ files INCLUDING the R2010/R2013/R2018 containers**
  (Box_2010/2013/2018 all carry the same class). The key covers
  the whole AC18–AC28 container family. The rows close by the
  SAME mechanism as the AC1021 address rows: a container mirror
  (the H8a retention + H8b mirror arm + the objects raw-echo
  re-scope), because the addresses are pure layout functions of
  her container (incl. the objects section's size — the
  placement wall applies to AC18 too, and the raw-echo re-scope
  answers it the same way). The "4 × 225 + 9 × 7" decomposition
  in the old halts is STALE — the fresh scan shows 147 files ×
  3–4 address rows (~598 in the per-file read vs the census's
  963 leaf count; the delta = array-leaf counting — the packet's
  opening measurement reconciles it). Cost: a second
  H8-scale campaign over the R2004/R2010/R2013/R2018 container
  variants (our reader parses all of them — read axis 0/0 at
  every version; retention is plumbing). No new instruments, no
  custom examples; the corpus's 147 files are the surface.
  Reversing the acceptance is a maintainer call.
- **BREP is NOT in the 3,576** (a deferred row, separate) — the
  external-specimen ask does not gate this number.

The conditions for zero, stated plainly: (1) the H8d unit lands
(the first ~1,057); (2) the bounded packets (FileDepList,
SecondHeader, AuxHeader, the R2000 layout pair) — each a
retention/emission fix verified against the corpus; (3) the AC18
decision — without it the floor is 963; (4) the echo doctrine
extends to unmodelable authored state as needed (established
campaign precedent: the SH tails, the XrefManifest fallback) —
the alternative is a per-section byte-parity campaign for each
modeled section (feasible by the α method, slower).

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
