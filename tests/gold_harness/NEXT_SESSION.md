# Zero-context prompt — TARGET ZERO held; the H8h-extension measured and 423 → 185 (the survey + four form rules)

> Campaign state 2026-09-27 (the halt after the H8h-ext landing; the
> session ran the review → land → verify → commit → push → continue
> loop from the H8h record-identity halt: **THE CORPUS STAYS AT ZERO
> ON EVERY AXIS: 280 files, read-fidelity 0, write-fidelity 0, read
> key-gap 0, write-target 0** — verified twice this session, before
> and after the landings). The ACS/SH campaign stays COMPLETE at
> 0/0. The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d–H8h + **H8h-ext** rows carry this arc's records),
> then this file top to bottom.

## The arc (2026-09-27, one session — the H8h extension)

1. **The halt-state verification** — the full battery green at the
   H8h state (serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0, corpus 280 files 0/0/0/0, generation
   identity `1a56bca0…` 25,375 bytes, push-state clean at
   `41259bb`).
2. **The survey instrument** — `tests/gold_harness/record_identity_survey.py`
   (permanent harness tool): per AC1021 file, the `DWG_NO_ECHO`
   conventional rewrite, the `AC21_DIFF_RAW_DIR`
   her/ours decompressed objects-stream dumps, the
   `DWG_RECORD_TRACE` maps, and a byte-wise walk of every record
   unit. ONE autopsy-grade discovery: the record-head MS is
   **15 data bits per 16-bit LE word** (bit 15 = continuation) —
   the survey's first 7-bit-per-byte parser was wrong; corrected
   (the crc16 window search over ±12-byte windows of identical
   records proved the domain [MS..span] beyond doubt; §19.4.A now
   carries the note). The circle anchor re-measured: 211/211.
3. **The measurement (58 AC1021 files)** — the "circle's two
   rules likely cover the class" hypothesis is FALSE: only
   14/58 (the simple ODA-authored named specimens) at 100%;
   **423 divergent records** initial, in a small set of clean
   classes.
4. **The autopsies** — gold's `-v9` walks + the JSON handle-code
   census (gold prints the raw ref codes): (a) the author writes
   **code-3 owner-class refs** where our writer used 4/5 —
   census-UNIVERSE (the 2027 fixtures, the 2024-authored
   example_2007, real-world ATMOS all print `[3, ..]`); (b) the
   table controls carry the **author's entry order plus null
   deleted-slot tails** (Box's BLOCK_CONTROL
   `entries[1]=(2.0.0) abs:0`; its DIMSTYLE_CONTROL
   `[Standard 432, Annotative 311, ISO-25 27, null]` vs our
   ascending 3-slot derivation); (c)–(g) the residual classes
   named below. NO genus conflicts: the 14 clean files have no
   SH-family records, and the captured-order echo reproduces
   their wire byte-for-byte.
5. **The landings (four rule-sets, ONE commit)**: (1)
   `AcDbShHistory.owner` graph ref SoftPointer(4) →
   **HardOwnership(3)**; (2) the eval-graph node `evalexpr` ref
   HardPointer(5) → **HardOwnership(3)**; (3) the **authored
   table-control entry slots** — `document.table_control_entries`
   (BTreeMap keyed by control handle, serde-skipped,
   fingerprint-excluded — the morehandles precedent) captured at
   DWG read for BLOCK/LTYPE/DIMSTYLE (order + nulls verbatim)
   and echoed by the writers under the **same-universe gate**
   (`authored_control_entries`: captured non-null set must equal
   the live table's handles; edited tables and DXF documents
   fall back to the model iteration); (4) the corrected survey
   tool. **423 → 185 divergent.**
6. **The gates** — serde 1602/0, gold_roundtrip ok, issue80 7/0,
   four family smokes 0/0, the full corpus 280 files 0/0/0/0
   (the echo arms untouched — the conventional arm serves
   edited documents and conversions only), generation identity
   **UNCHANGED** (`1a56bca0…`, 25,375 bytes, verified with AND
   without `--features serde` — the generator's programmatic
   documents never carry the touched fields).

## The final design (the one thing to understand — unchanged)

**The unified whole-file echo** (`write_to_writer` in
`src/io/dwg/dwg_writer.rs`): when the document came from a
same-version DWG read, her whole on-disk file is retained
(`document.raw_ac21_tail`, serde-skipped, every container family),
and the document-state hash holds, the rewrite re-emits her bytes
VERBATIM. Everything else — the H8b/H7g mirrors, the conventional
arms, the prepare pipeline — serves edited documents and
conversions, unchanged.

**The document-state fingerprint** (`document_state_fingerprint` in
`src/io/dwg/mod.rs`): the sorted per-part hash of the semantic
inventory's visit plus the ten table control handles and the
retained metadata models. Captured at the END of the read; compared
BEFORE the prepare pipeline at the write entry. The
`table_control_entries` capture is EXCLUDED from the fingerprint
(the `dimstyle_morehandles` precedent) — wire-only state.

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm — the only path the H8h/H8h-ext form
rules affect — is record-identical to the author's stream on
circle (211/211) and on the 14 simple named specimens; on the
solid-history fixture genus it now carries ~2–3 residual records
per file (was 4–11), and **185 named residual records across the
58 AC1021 files** remain, each class byte-level-scoped (below).

## The residual taxonomy (185 records; each a named future packet)

- **(c) the SH-history DICTIONARY stub** (`size 70 vs 17`, every
  fixture): gold prints `numitems: 1, items:
  {"ACAD_PARALLEL_BACKGROUND": [2,0,0,0]}` — our emission drops
  the item whose target handle is NULL. Find where (reader
  capture or writer filter) and retain.
- **(d) the 3DSOLID/ACIS record-tail ref** (every fixture): `her
  06/64/03/19 vs our 08/84/04/21` — nibble-class/count deltas
  inside the SAT record's handle stream (e.g. her code-6
  vs our code-8 direction forms at +6018/+7740). Needs the
  bit-walk of the record tail; likely ONE ref-class rule.
- **(e) the M/CSurf assoc classes** (`+8 mutations`: the
  `+109/+82 (32v42)`, `+47/+43 (23v24)`, `+53v54`, the
  `2E6/2E7/2E9/2EA size XX vs 20` UNKNOWN bodies) — the
  ASSOCPATHACTIONPARAM/LOFTED-surface family forms + our
  stub emissions of the unmodeled assoc records.
- **(f) the fillet/chamfer UNKNOWN bodies** (`1463/1509 vs 28`)
  and the `29 vs 27` class on Loft3/C/H/R.
- **(g) the gold-tree stragglers**: Leader/Constraints/example_2007
  VIEW_CONTROL (`27/18/22 vs 10` — the views live in gold's
  model but our control emits none), Constraints `3E3` (666 vs
  870), example_2007's marginal forms (`h=1F +10`, `h=176
  1726v1727`, `h=1A9 +32`, `h=37D`, `h=392/393` code-3-class +
  count deltas), PolyLine3D `1C2` (16 vs 17 — OURS longer),
  ATMOS's 64 + 84 (the ACAS/sat interior + her controls; its
  evalgraphs close under rule (1)/(2)).

## The remaining work (all optional — the target stays reached)

- **The residual classes (c)–(g) above** — each is a
  byte-level-scoped packet in the H8h tradition
  (autopsy → census → rule → gates → re-survey); the H8h-ext
  row in §19.2 (IMPLEMENTATION.md) carries the map.
- **The code-3 survey extension**: gold prints more `[3,`
  refs than the two landed (example_2007 h=392-class,
  `+67 (32 vs 52)` on the UNKNOWN records, ATMOS's stragglers) —
  a census sweep of gold's JSON ref codes vs our writer's
  choices on the residual records extends rule (1)/(2)'s
  pattern.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws —
  NOT attempted (a deep algorithmic study of gold's random.c MT
  variant; the echo path never runs it for unedited roundtrips,
  so it gates only edited-document re-emission quality, not
  rows).
- **The dead/no-path rows**: `LoftD`; the SH revolve option
  shorts; **BREP stays deferred** (external authentic
  ACSH_BREP_CLASS specimen required).
- **The parallel session's probes** (REMOVED at the 2026-09-27
  housekeeping — historical; their rows closed with the echo
  landings).

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `1a56bca0a56dff09511cec1659cfff1b`,
  25,375 bytes (UNCHANGED through H8h-ext — no fields the
  generator emits were touched). The generator builds and runs
  identically WITH or WITHOUT `--features serde`.
- The record-identity state: circle 211/211 (the H8h acceptance,
  re-verified); the 58-file survey: **14 files at 100%, 185
  divergent records residual, 0 her-only orphans except
  ATMOS's broken-map records (84) and the fillet/chamfer
  tail-els (1 per file class)**.
- The gold tree sits at `34f02f54` FROZEN with ONE tracked
  generated-file drift (`src/config.h.in`, autoheader requote) —
  the freeze rule stands. Oracle fingerprints unchanged: the
  6,316-byte libtool wrapper `programs/dwgread` (md5
  `8dad57211b78f42e7594ba6c211cc0e5`) and the ELF
  `programs/.libs/dwgread` (md5
  `d852da1db0894b866e86042d4b26b91d`, 230,232 bytes). NO
  `encode_r2007.c` exists — gold has no R2007 writer.
- Spec authority on file: the ODA spec PDF
  (`~/work/OpenDesign_Specification_for_.dwg_files.pdf`, 270pp) +
  libredwg `bits.c`.
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use long timeouts on the bash tool for
the corpus — the full corpus takes ~4 minutes).

```bash
# Environment (source this):
export PATH="$HOME/.cargo/bin:$PATH"
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
```

## Verification gate (the zero-keeping rule — all four axes)

```bash
# 1. Build gates
cargo test --features serde          # green at this halt (1602/0)
cargo test --features gold-harness --test gold_roundtrip
cargo test --features serde --test issue80   # the edit-survival gate

# 2. Family smokes (all four families, write-target 0)
python3 tests/gold_harness/run_roundtrip.py \
    <FIXTURE>.dwg /tmp/smoke
# AC21: $GOLD_TESTDATA/2007/circle.dwg
# R2000: $GOLD_TESTDATA/example_2000.dwg
# AC18: $GOLD_TESTDATA/example_2004.dwg
# R2018: $GOLD_TESTDATA/2018/Dynblocks.dwg

# 3. Full corpus (280 files; ALL FOUR AXES 0)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 1a56bca0a56dff09511cec1659cfff1b, 25,375 bytes
# (identical without --features serde)

# 4b. The record-identity survey (the H8h-extension instrument:
#     the 58-file AC1021 conventional-arm measurement)
python3 tests/gold_harness/record_identity_survey.py \
    "$GOLD_TESTDATA"/2007/*.dwg "$GOLD_TESTDATA"/example_2007.dwg \
    tests/gold_harness/tests/sh_history/*_2007.dwg
# current state: 14 files at 100%, 185 divergent records total
# circle re-verified: 211/211

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
41259bb <docs> the housekeeping sweep (the prior halt head)
<feat> H8h-ext: the AC1021 record-identity survey + the four
       form rules (the HardOwnership graph/evalexpr refs; the
       authored control-entries capture with same-universe
       echo; the MS frame corrected in the survey; 423 → 185)
<docs> this halt refresh — the post-H8h-ext record
```

**PUSH STATE (2026-09-27)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (battery
green at the H8h corpus-zero state) → the required reading → the
survey instrument (the MS frame discovery via the crc16 window
search — the record unit is [15-bit-word MS][span incl. pad][crc
over MS..span]; §19.4.A annotated) → the 58-file measurement
(the 423 initial divergence map; the hypothesis about circle's
two rules refuted — 14/58 files at 100%) → the autopsies (gold's
-v9 walks + the JSON ref-code census: the code-3 universe across
all three author genera; the null deleted-slot tails and
descending author orders on the Box/ExtrudeC sub-genera) → the
landing (two HardOwnership rules + the authored-slots capture
with the same-universe gate, the fingerprint untouched) → the
gates (serde 1602/0, gold_roundtrip, issue80, family smokes,
the corpus 0/0/0/0 twice, generation identity verified
feature-independent, no-serde build green) → the docs (§19.2
H8h-ext row; the §19.4.A MS note; this halt record). **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied: the corpus is at zero on every axis;
the conventional arm is record-identical on circle and the 14
simple-specimen class, and its residual on the solid-history
genera is now a named, byte-level-scoped queue of 185 records in
six classes.**
