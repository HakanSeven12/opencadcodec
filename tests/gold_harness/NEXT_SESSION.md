# Zero-context prompt — TARGET ZERO held; the conventional arm at 20 residual records (H8h-ext-5: the PersSubentManager v2 tail; the assoc/non-assoc class distinction confirmed)

> Campaign state 2026-09-27 (the halt after the H8h-ext-5 landing; the
> session continued the loop from the H8h-ext-4 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0, write-target
> 0** — verified after the landing). The ACS/SH campaign stays COMPLETE
> at 0/0. The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3
> + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's
> H8d–H8h-ext-5 rows carry this arc's records), then this file top to
> bottom.

## The arc (2026-09-27, the continuation session — H8h-ext-5)

1. **The assoc/non-assoc distinction (the maintainer's hypothesis,
   CONFIRMED)**: the corpus carries TWO persubent-manager classes —
   `ACDBASSOCPERSSUBENTMANAGER` (the associative variant: gold has a
   spec block — class_version, unknowns 3/0/2, bl1/bl2, steps, subents,
   the v2 tail; our `AssocPersSubentManager` model; its records were
   already byte-identical) and `ACDBPERSSUBENTMANAGER` (the
   NON-associative variant: NO gold spec block, gold parks it raw; our
   `PersSubentManagerStatic` path; the divergent records). The loft
   files carry both (gold's warnings show the ASSOC at object 135 and
   the non-assoc at 136).
2. **The Loft 29-vs-27 autopsy**: her class region = [5 BLs:
   2, 0, 2, {2|1}, 1][num_steps=7 + steps [0,{1|2},{1|2},4,0,0,0]]
   [num_subents=0][BL(1)][BL({2|1})] — the walk ends exactly at the
   main-stream CONTENT end (bit 199; the RL = 200 = content + 1). The
   tail values: the first BL is 1, the second tracks the fourth header
   BL (= steps[2]).
3. **The +1-bit lesson (the first landing attempt)**: the initial tail
   read included a B field — but the bit after the content is the
   MERGED WRITER'S no-text flag (the finalization computes
   `total_bits = main + text + 1` and writes the flag bit itself), not
   a record field. The misread tail made every record +1 bit (bitsize
   201 vs 200; the old records' +1: content 179 → bitsize 180 was the
   same flag). THE RULE: a reverse-engineered tail must never include
   the flag bit — the walk's "end exactly at the main content end" is
   the boundary.
4. **The regression that found the gate (the second attempt)**: with
   the tail ungated, the ~25 count-0 records (e.g. ExtrudeC 2DC:
   associative_subent_count=0) captured garbage (the flag + handle
   bits) — a 25→48 regression, our records 22v27. THE GATE: the tail
   is present ONLY when `associative_subent_count != 0` (the loft
   records carry 1; the count-0 records end the content right after
   the steps).
5. **The landing**: `PersSubentManager.v2_tail: Option<(i32, i32)>`
   (serde-default; None for DXF-built records), the reader gated on
   `class_version == 2 && associative_subent_count != 0`, the writer
   emitting the two BLs when captured.
6. **Measured: 25 → 20 divergent records; 49/58 files at 100%.**
   Loft 207/207, Loft3 206/206, LoftC 211/211, LoftH 206/206,
   LoftR 206/206; the regressed genus restored (ExtrudeC 206/206,
   Box 207/207, RevolveA 207/207); circle 211/211 held.
7. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0.
   Generation identity UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
   25,375 bytes, verified with AND without `--features serde`).

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
BEFORE the prepare pipeline at the write entry. The wire-only
captures (`table_control_entries`, `dimstyle_morehandles`) are
EXCLUDED from the fingerprint; `xdic_by_handle`, the modeled
subcurve fields, and the PersSubentManager tail are INCLUDED (real
state — an edit declines the echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 49 of the 58 AC1021 corpus files, with **20 named
residual records** in five classes (below).

## The residual 20 (each a named, byte-level-scoped packet)

- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
- **ATMOS (5 + 84 her-only)**: the controls h=2 16v15, h=3 20v15,
  h=77 96v40, h=352 25642v36 (a 25KB ACIS mass we emit as 36
  bytes), h=541 17v18; the 84 her-only = the broken-map orphans
  (pre-existing, documented).
- **Chamfer/Fillet (4)**: h=2DE 22v26/30 + h=2DF 1509/1463v30 — the
  same non-assoc PersSubentManager class (the 22v30 record likely
  needs the v2 tail with different gates) + the fillet/chamfer
  UNKNOWN body (the unknown_bits mass our stub drops).
- **The singles**: Constraints `3E3` (666 vs 870); PolyLine3D `1C2`
  (16 vs 17 — OURS longer).

## The remaining work (all optional — the target stays reached)

- **The residual classes above** — each an autopsy → census →
  rule → gates → re-survey packet in the H8h tradition; the
  §19.2 H8h-ext-5 row carries the map and the method. NOTE for
  Chamfer/Fillet: the 2DE records are the same PersSubentManager
  class — check their associative_subent_count and whether a
  further tail form exists before treating them as unknown_bits.
- **The unattested subcurve action types** (17=ELLIPSE, 19=LINE,
  23=LINESEG3D, 42=NURB3D, 27=CURVE3D): no corpus specimens — the
  twelve-BD ARC form is the only attested layout.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (the echo path never runs it for unedited roundtrips).
- **The dead/no-path rows**: `LoftD`; the SH revolve option shorts;
  **BREP stays deferred** (external authentic ACSH_BREP_CLASS
  specimen required).

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-5). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **49 files at 100%, 20 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  ExtrudeCSurf 216/216; LoftCSurf 221/221; Loft 207/207.
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
  libredwg `bits.c`. NEITHER documents the ACDBASSOC* classes, the
  subcurve wire, or the non-assoc PersSubentManager (the authority
  closure, commit `cc1a6c8`).
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). **The merged
  writer's no-text flag bit**: the RL (bitsize) = main content +
  text + 1; the flag bit at the content end is NOT a record
  field — a reverse-engineered tail must end at the content end
  (the +1-bit lesson, H8h-ext-5). The BL forms: 00 = 4-byte LE,
  01 = 1 byte, 10 = 0 (2 bits). The BD forms: 00 = full 66-bit LE
  double, 01 = 1.0, 10 = 0.0.

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
# 84374e73ddcf1d6143877c4100b81e48, 25,375 bytes
# (identical without --features serde)

# 4b. The record-identity survey (the H8h-extension instrument:
#     the 58-file AC1021 conventional-arm measurement)
python3 tests/gold_harness/record_identity_survey.py \
    "$GOLD_TESTDATA"/2007/*.dwg "$GOLD_TESTDATA"/example_2007.dwg \
    tests/gold_harness/tests/sh_history/*_2007.dwg
# current state: 49 files at 100%, 20 divergent records total
# circle 211/211; ExtrudeCSurf 216/216; Loft 207/207

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
04d6815 <docs> the post-H8h-ext-4 halt refresh (the prior halt head)
<feat> H8h-ext-5: the PersSubentManager v2 tail — [BL][BL] gated on
       associative_subent_count != 0 (25 -> 20; 49/58 at 100%; the
       assoc/non-assoc class distinction confirmed)
<docs> this halt refresh — the post-H8h-ext-5 record
```

**PUSH STATE (2026-09-27)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-4
halt → the maintainer's assoc/non-assoc hypothesis verified against
gold's spec (the ASSOC block exists and its records were already
identical; the non-assoc has no block) → the Loft 29-vs-27 autopsy
(the 5-BL header, the 7 steps, the [BL][BL] tail ending at the
content end) → the +1-bit lesson (the merged writer's no-text flag
bit — the first attempt misread it as a record B field) → the
regression that found the gate (the count-0 records carry no tail;
the ungated capture pulled the flag + handle bits — 25→48, then
the gated fix restored them) → the gates (serde 1602/0,
gold_roundtrip, issue80, family smokes, corpus 0/0/0/0, generation
identity unchanged and feature-independent) → the full survey
(25 → 20; 49/58 at 100% — the whole loft genus closed) → the docs
(§19.2 H8h-ext-5 row; this halt record). **The maintainer's loop
instruction — "repeat process until target = zero" — remains
satisfied: the corpus is at zero on every axis; the conventional
arm is record-identical on 49 of 58 AC1021 files, and the residual
20 records are named, byte-level-scoped packets in five classes.**
