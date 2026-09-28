# Zero-context prompt — TARGET ZERO held; the conventional arm at 14 residual records (H8h-ext-7: the SEQEND captured-flags fix; the era convention demoted to the DXF fallback)

> Campaign state 2026-09-28 (the halt after the H8h-ext-7 landing; the
> session continued the loop from the H8h-ext-6 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0, write-target
> 0** — verified after the landing). The ACS/SH campaign stays COMPLETE
> at 0/0. The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3
> + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's
> H8d–H8h-ext-7 rows carry this arc's records), then this file top to
> bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-7)

1. **The PolyLine3D 1C2 autopsy**: the divergent record is the
   polyline's SEQEND (type 6). Gold's -v9 walk mapped the record
   completely: her shadow_flags RC = 0x00 with NO shadow ref (Hdlsize
   25 = owner 8 + layer 16 + 1 pad bit); our record carried
   shadow_flags = 3 + the null shadow ref ([5,0,0,0], 8 bits) — the
   +1 byte and the 0x03-vs-0x00 delta.
2. **The root cause**: `seqend_era_flags()` — the "AutoCAD SEQEND
   convention, verified per owner across the example corpus
   (ex2000/2004/2007/2010/2013/2018)" returning (3, 0) for the R2007
   band. The convention is REAL for the example_* corpus but is a
   per-AUTHOR form, not an era law: the ODA-authored test-data
   PolyLine3D carries (0, 0). Our reader ALREADY captures the wire
   SEQEND's flag pair (pending.seqend_flags keyed by the owner,
   transferred to the polyline model with the seqend handle) — the
   writer ignored it.
3. **The fix**: the four polyline-family writers (Polyline2D/3D,
   PolyfaceMesh, PolygonMesh) use the captured pair when the read
   retained a wire SEQEND (`e.seqend_handle.is_some()`); the era
   convention is the DXF-built fallback only. The INSERT path
   (literal (0,0), no corpus divergence) untouched.
4. **Measured: 15 → 14 divergent records; 55/58 files at 100%.**
   PolyLine3D 218/218; Polyline/Polygon/example_* held (their captured
   (3,0) equals the era form — the switch is a no-op there).
5. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0.
   Generation identity UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
   25,375 bytes, verified with AND without `--features serde` — the
   generator's programmatic polylines take the era fallback).
6. **The next packet started**: Constraints `3E3` — the
   ASSOC2DCONSTRAINTGROUP record (ours 204 bytes LONGER, 870 vs 666;
   gold parks 5249 unknown bits in her record — its spec block is
   partial; our reader parses the group fully). The autopsy needs the
   -v9 walk + our writer's emission comparison — a fresh dissection.

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
subcurve fields, the persubent tails, and the SEQEND flag captures
are INCLUDED (real state — an edit declines the echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 55 of the 58 AC1021 corpus files, with **14 named
residual records** in three classes (below).

## The residual 14 (each a named, byte-level-scoped packet)

- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
- **ATMOS (4 + 84 her-only)**: the controls h=2 16v15, h=3 20v15,
  h=77 96v40, h=352 25642v36 (a 25KB ACIS mass we emit as 36 bytes),
  h=541 17v18 — re-autopsy pending; the 84 her-only = the
  broken-map orphans (pre-existing, documented).
- **Constraints (1)**: `3E3` — the ASSOC2DCONSTRAINTGROUP (666 vs
  870, OURS 204 bytes longer; gold parks 5249 unknown bits — the
  fresh dissection packet).

## The remaining work (all optional — the target stays reached)

- **The Constraints 3E3 packet** (the fresh start): gold's -v9 walk
  of the record + our writer's ConstraintGroup emission comparison;
  ours-longer suggests we emit a region the author omits (or a
  longer form of the nodes array). The ASSOC2DCONSTRAINTGROUP spec
  block is partial in gold (5249 unknown bits on her side).
- **example_2007's 9 marginals + ATMOS's 4** — each an autopsy →
  census → rule → gates → re-survey packet in the H8h tradition.
- **The unattested subcurve action types** (17=ELLIPSE, 19=LINE,
  23=LINESEG3D, 42=NURB3D, 27=CURVE3D): no corpus specimens.
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
  25,375 bytes (UNCHANGED through H8h-ext-7). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **55 files at 100%, 14 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218.
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
  closure, commit `cc1a6c8`); the ASSOC persubent variant HAS a
  gold spec block (the H8h-ext-6 re-parse follows it).
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). **The merged-stream
  frame math**: the RL (bitsize) = main content + text + 1 (the
  no-text flag); `main_remaining_bits()` is relative to the
  CONTENT end; the bit after the content is ALWAYS the flag —
  never a record field. The ASSOC persubent records end [BLs][B];
  the non-assoc records end flush. The BL forms: 00 = 4-byte LE,
  01 = 1 byte, 10 = 0 (2 bits); the BD forms: 00 = full 66-bit LE
  double, 01 = 1.0, 10 = 0.0. **The SEQEND lesson (H8h-ext-7)**:
  era-derived "conventions" verified on one corpus family are
  per-author forms — prefer the read capture, keep the convention
  as the DXF fallback only.

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
# current state: 55 files at 100%, 14 divergent records total
# circle 211/211; PolyLine3D 218/218; Chamfer 207/207

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
574a59e <docs> the post-H8h-ext-6 halt refresh (the prior halt head)
<feat> H8h-ext-7: the SEQEND captured-flags fix — the four
       polyline-family writers use the read-captured flag pair, the
       era convention demoted to the DXF fallback (15 -> 14; 55/58 at
       100%)
<docs> this halt refresh — the post-H8h-ext-7 record
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-6
halt → the PolyLine3D 1C2 autopsy (gold's -v9 walk: her shadow=0,
no ref; ours (3,0) + the null ref — the +1 byte) → the root cause
(`seqend_era_flags()` — the example-corpus convention mistaken for
an era law; the ODA-authored specimen carries (0,0)) → the fix (the
captured pair from pending.seqend_flags, era as the DXF fallback
only) → the gates (serde 1602/0, gold_roundtrip, issue80, family
smokes, corpus 0/0/0/0, generation identity unchanged and
feature-independent) → the full survey (15 → 14; 55/58 at 100% —
PolyLine3D 218/218) → the Constraints 3E3 autopsy started (the
ASSOC2DCONSTRAINTGROUP, ours 204 bytes longer — the fresh packet
map) → the docs (§19.2 H8h-ext-7 row; this halt record). **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied: the corpus is at zero on every axis; the
conventional arm is record-identical on 55 of 58 AC1021 files, and
the residual 14 records are named, byte-level-scoped packets in
three classes.**
