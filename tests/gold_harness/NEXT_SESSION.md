# Zero-context prompt — TARGET ZERO held; the conventional arm at 15 residual records (H8h-ext-6: the persubent genus completed — the ASSOC re-parse + the passthrough tails)

> Campaign state 2026-09-27 (the halt after the H8h-ext-6 landing; the
> session continued the loop from the H8h-ext-5 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0, write-target
> 0** — verified after the landing). The ACS/SH campaign stays COMPLETE
> at 0/0. The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3
> + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's
> H8d–H8h-ext-6 rows carry this arc's records), then this file top to
> bottom.

## The arc (2026-09-27, the continuation session — H8h-ext-6)

1. **The ASSOC-variant re-parse**: the old `ASSOCPERSSUBENTMANAGER`
   reader skipped the gold spec's `unknown_bl1`/`unknown_bl2` (the two
   BLs between the markers and num_steps) and read BLs until the stream
   end + a final_flag bit — a value-level BL passthrough that was
   byte-SYMMETRIC for the loft records but desynced on Chamfer/Fillet
   2DE (the 2-bit code-11 branch's 256 → 30-byte garbage records).
   Re-parsed to the spec field order (bl1, bl2, num_steps, steps,
   num_subents, subents).
2. **The tail structure discovery**: the first re-parse attempt was 1
   byte short — the simple records end with ONE trailing B (the last
   content bit), but the LoftCSurf/LoftM 2DD records carry a variable
   BL run ([0,0,0,1,1,0]) before it. The old passthrough had consumed
   it all symmetrically.
3. **The passthrough tails (the landing)**: both variants now capture
   the post-subents region verbatim — the ASSOC variant: BLs until one
   bit remains + the trailing B; the non-assoc variant: BLs until the
   content end (flush, no B). This subsumes the H8h-ext-5 [BL][BL]
   tail (a short run) AND the Chamfer/Fillet 2DF **~1224-BL history
   blobs** (the 1509/1463-byte records — pure BL sequences ending
   flush).
4. **The frame math that settled the boundaries**: the RL (bitsize) =
   main content + text + 1 (the no-text flag); `main_remaining_bits()`
   is relative to the CONTENT end; the ASSOC records end [BLs][B], the
   non-assoc records end flush — the bit after the content is ALWAYS
   the flag, never a record field (the H8h-ext-5 lesson, now applied to
   both variants).
5. **Measured: 20 → 15 divergent records; 54/58 files at 100%.** The
   whole persubent genus byte-identical: Chamfer 207/207, Fillet
   207/207, Loft 207/207, LoftCSurf 221/221, LoftM 221/221; ATMOS
   dropped 5 → 4 (one of its records was persubent); circle 211/211
   held.
6. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
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
subcurve fields, and the persubent tail captures are INCLUDED (real
state — an edit declines the echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 54 of the 58 AC1021 corpus files, with **15 named
residual records** in four classes (below).

## The residual 15 (each a named, byte-level-scoped packet)

- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
- **ATMOS (4 + 84 her-only)**: the controls h=2 16v15, h=3 20v15,
  h=77 96v40, h=352 25642v36 (a 25KB ACIS mass we emit as 36 bytes),
  h=541 17v18 — re-autopsy after the persubent landing (one closed);
  the 84 her-only = the broken-map orphans (pre-existing,
  documented).
- **The singles**: Constraints `3E3` (666 vs 870); PolyLine3D `1C2`
  (16 vs 17 — OURS longer).

## The remaining work (all optional — the target stays reached)

- **The residual classes above** — each an autopsy → census →
  rule → gates → re-survey packet in the H8h tradition; the
  §19.2 H8h-ext-6 row carries the map and the method.
- **The unattested subcurve action types** (17=ELLIPSE, 19=LINE,
  23=LINESEG3D, 42=NURB3D, 27=CURVE3D): no corpus specimens — the
  twelve-BD ARC form is the only attested layout.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (the echo path never runs it for unedited roundtrips).
- **The dead/no-path rows**: `LoftD`; the SH revolve option shorts;
  **BREP stays deferred** (external authentic ACSH_BREP_CLASS
  specimen required).
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does); a DWG→DXF→DWG
  roundtrip of a tailed record would drop them — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-6). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **54 files at 100%, 15 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; LoftCSurf 221/221.
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
  closure, commit `cc1a6c8`); the ASSOC variant HAS a gold spec
  block (dwg2.spec — the H8h-ext-6 re-parse follows it).
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). **The merged-stream
  frame math (the H8h-ext-6 settlement)**: the RL (bitsize) = main
  content + text + 1 (the no-text flag); `main_remaining_bits()`
  is relative to the CONTENT end; the bit after the content is
  ALWAYS the flag — never a record field. The ASSOC persubent
  records end [BLs][B]; the non-assoc records end flush. The BL
  forms: 00 = 4-byte LE, 01 = 1 byte, 10 = 0 (2 bits); the BD
  forms: 00 = full 66-bit LE double, 01 = 1.0, 10 = 0.0.

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
# current state: 54 files at 100%, 15 divergent records total
# circle 211/211; Chamfer 207/207; Loft 207/207

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
5f4ced4 <docs> the post-H8h-ext-5 halt refresh (the prior halt head)
<feat> H8h-ext-6: the persubent genus completed — the ASSOC re-parse
       per the gold spec + the passthrough tails (20 -> 15; 54/58 at
       100%; the 2DF history blobs closed)
<docs> this halt refresh — the post-H8h-ext-6 record
```

**PUSH STATE (2026-09-27)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-5
halt → the Chamfer/Fillet autopsy (the 2DE records revealed as the
ASSOC variant with a garbage parse — the missing spec BLs) → the
re-parse landing and the tail discovery (the 1-byte-short first
attempt; the LoftCSurf/LoftM variable BL runs) → the passthrough
tails (the ASSOC [BLs][B] and the non-assoc flush endings; the 2DF
~1224-BL blobs) → the gates (serde 1602/0, gold_roundtrip,
issue80, family smokes, corpus 0/0/0/0, generation identity
unchanged and feature-independent) → the full survey (20 → 15;
54/58 at 100% — the whole persubent genus closed) → the docs (§19.2
H8h-ext-6 row; this halt record). **The maintainer's loop
instruction — "repeat process until target = zero" — remains
satisfied: the corpus is at zero on every axis; the conventional
arm is record-identical on 54 of 58 AC1021 files, and the residual
15 records are named, byte-level-scoped packets in four classes.**
