# Zero-context prompt — TARGET ZERO held; the conventional arm at 12 residual records (H8h-ext-8: the Constraints 3E3 closure — the ASSOC2DCONSTRAINTGROUP node-region wire capture; the halt-doc count corrected 14→13→12)

> Campaign state 2026-09-28 (the halt after the H8h-ext-8 landing; the
> session continued the loop from the H8h-ext-7 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0,
> write-target 0** — verified after the landing). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d–H8h-ext-8 rows carry this arc's records), then this
> file top to bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-8)

1. **The Constraints 3E3 autopsy COMPLETED — via the era family**: the
   same drawing exists as r14/2000/2004/2007/2010/2013 `Constraints.dwg`
   specimens (found by a corpus scan for
   `Decode object ASSOC2DCONSTRAINTGROUP`). The pre-2007 records carry
   the node class names INLINE as TVs — `AcConstrainedCircle`,
   `AcConstrainedImplicitPoint`, `AcCenterPointConstraint`,
   `AcConstrainedBoundedLine`, `AcPointCurveConstraint`,
   `AcPointCoincidenceConstraint` — nine nodes, one name each, in walk
   order. THE REAL NODE WIRE: per node `[BLd id][RC status][TU
   class-name (AC1021: consumed from the record's TEXT stream in walk
   order; pre-2007: inline main TV)][class data arm][geometry handle
   refs]` — NOT gold's flat REPEAT (dwg2.spec 5682), whose own -v9 walk
   desyncs at node[1] and parks 5249 unknown bits.
2. **CROSS-ERA BIT-IDENTITY**: the circle node's 320-bit data region is
   bit-identical between the 2007 and 2000 records once the inline TV
   is discounted — the class arms are era-stable. Recovered forms: the
   circle `[BL 1][BL 1][BL 3][BL 0][3BD center (11.319546, 16.063856,
   0)][3BD (0,0,1)][3BD (1,0,0)][BD radius 2.147789][BD 0.0][BD 2π]`;
   the implicit points carry the `00FFFFFFFF` BLd -1 (point_idx)
   markers (three, one per ip node) + a curve_id BLd; the handle
   stream carries five node reads after the head's six
   (`[3E4][∅][3E5][∅][∅]` — the two ASSOCGEOMDEPENDENCY refs plus
   three nulls). THE AUTHORITY CLOSURE HOLDS: the ODA PDF has zero
   ConstraintGroup hits (grep-verified); libredwg's spec macros
   (`AcConstraintGeometry_fields` etc.) are defined but used by NO
   live block, and its model struct is base-only ("still in work").
3. **The fix (the H8d/persubent-tail/MultiLeader-tail wire-capture
   doctrine)**: the AC1021 reader retains the region verbatim — the
   main bits from the end of num_nodes to the record's main-data end,
   the per-node class-name TUs (bounded by what the text stream
   holds), and the handle bits from the drain position after the
   record's own head reads to the record end MINUS the author's
   closing 1s pad (a ≤7-bit trailing-1s scan; the merged writer
   re-creates the pad at close — the first landing attempt
   double-padded, the measured 666-vs-667 byte taught the trim). The
   writer re-emits the TUs into the text stream (walk order) + the raw
   bits + the raw handle tail; the naive modeled REPEAT stays the
   DXF/programmatic fallback (`nodes_wire_main` absent; the naive
   semantic walk kept for the model). Gated to AC1021 — the only
   dissected frame.
4. **Measured: Constraints 219/219 (0 divergent; streams 222306 =
   222306); the residual 12; 56/58 files at 100%.** THE HALT-DOC
   CORRECTION (honest bookkeeping, stash-rebuild verified): the
   ext-7 halt's "14" was actually 13 — its residual list carried the
   stale ext-6-era names h=352/h=541 for ATMOS (five names for
   "four"), and a direct re-measure of the halt's own committed HEAD
   shows ATMOS at 3 divergent (h=2/h=3/h=77 only): this landing's
   true delta is 13 → 12.
5. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0.
   Generation identity UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
   25,375 bytes, verified with AND without `--features serde` — the
   generator's programmatic groups take the naive fallback). The echo
   byte-identity held (circle cmp clean).

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
subcurve fields, the persubent tails, the SEQEND flag captures, and
the H8h-ext-8 constraint-group node captures are INCLUDED (real
state — an edit declines the echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 56 of the 58 AC1021 corpus files, with **12 named
residual records** in two classes (below).

## The residual 12 (each a named, byte-level-scoped packet)

- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
- **ATMOS (3 + 84 her-only)**: the controls h=2 16v15, h=3 20v15,
  h=77 96v40 — re-autopsy pending; the 84 her-only = the broken-map
  orphans (pre-existing, documented). (The ext-7 halt's list named
  h=352/h=541 too — both re-measure CLEAN at that halt's own HEAD;
  stale names from the ext-6 era, corrected in §19.2's H8h-ext-8
  row.)

## The remaining work (all optional — the target stays reached)

- **example_2007's 9 marginals + ATMOS's 3** — each an autopsy →
  census → rule → gates → re-survey packet in the H8h tradition.
- **The pre-2007 constraint-group conventional arms** (the
  r14/2000/2004/2010/2013 `Constraints.dwg` specimens): the
  H8h-ext-8 capture is AC1021-gated; those eras keep their current
  behavior (echo-covered on the corpus axes, ungated by the AC1021
  survey). A later packet could extend the capture (the inline-TV
  forms and the R2010+ framing need their own dissection).
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
  25,375 bytes (UNCHANGED through H8h-ext-8). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **56 files at 100%, 12 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  **Constraints 219/219**.
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
  subcurve wire, the non-assoc PersSubentManager, or the
  constraint-group node classes (the authority closure, commits
  `cc1a6c8` + the H8h-ext-8 grep); the ASSOC persubent variant HAS
  a gold spec block (the H8h-ext-6 re-parse follows it).
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
  as the DXF fallback only. **The pad lesson (H8h-ext-8)**: a
  wire capture that runs to the record end must TRIM the author's
  closing 1s pad (≤7 bits) — the writer re-creates it at close, so
  an untrimmed capture double-pads (+1 byte, the measured
  666-vs-667).

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use long timeouts on the bash tool for
the corpus — the full corpus takes ~4 minutes; the bash tool's
timeout parameter is capped at 120 s in practice — run the corpus
as a background process and read its logs).

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
# current state: 56 files at 100%, 12 divergent records total
# circle 211/211; PolyLine3D 218/218; Chamfer 207/207;
# Constraints 219/219

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
574a59e <docs> the post-H8h-ext-6 halt refresh (the prior halt head)
a1a1506 <feat> H8h-ext-7: the SEQEND captured-flags fix — the four
       polyline-family writers use the read-captured flag pair, the
       era convention demoted to the DXF fallback (15 -> 14; 55/58 at
       100%)
b5e16b5 <docs> the post-H8h-ext-7 halt refresh
<feat> H8h-ext-8: the Constraints 3E3 closure — the AC1021
       node-region wire capture for the ASSOC2DCONSTRAINTGROUP (the
       class-name TUs, the main bits, the handle tail; naive as the
       DXF fallback; Constraints 219/219; 13 -> 12)
<docs> the post-H8h-ext-8 halt refresh (this file)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-7
halt → the Constraints 3E3 autopsy continued (the started packet:
the matching prefix, the node[1] desync, the text-stream class
names) → the era-family discovery (r14/2000/2004/2010/2013
`Constraints.dwg` specimens; the pre-2007 records carry the node
class names INLINE as TVs — the real node wire pinned: per node
[id][status][class-name TU][class data arm][geometry handles]) →
the cross-era bit-identity proof (the circle's 320-bit data region
identical across eras) → the fix (the wire-capture doctrine:
names + main bits + handle tail, AC1021-gated, the naive REPEAT as
the DXF fallback) → the pad lesson (the first attempt's +1 byte =
the double-padded author's 1s pad; the ≤7-bit trailing-1s trim) →
the gates (serde 1602/0, gold_roundtrip, issue80, family smokes,
corpus 0/0/0/0, generation identity unchanged and
feature-independent) → the full survey (Constraints 219/219; the
residual 12; the halt-doc count corrected: the ext-7 "14" was 13 —
its ATMOS list carried the stale ext-6 names h=352/h=541,
stash-rebuild verified) → the docs (§19.2 H8h-ext-8 row; this halt
record). **The maintainer's loop instruction — "repeat process
until target = zero" — remains satisfied: the corpus is at zero
on every axis; the conventional arm is record-identical on 56 of
58 AC1021 files, and the residual 12 records are named,
byte-level-scoped packets in two classes.**
