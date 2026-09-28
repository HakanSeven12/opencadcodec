# Zero-context prompt — TARGET ZERO held everywhere; THE NEXT WORK: work
# down the §20 genus-gate queue (the constructed-content oracle LANDED this
# session: genus_extract.py + genus_gates.py + the run_corpus additional
# sections + the cargo mirror; the day-one queue is ranked and live)

> Campaign state 2026-09-28 (the halt after the §20 implementation; the
> record-identity campaigns are ALL CLOSED: **THE CORPUS STAYS AT ZERO ON
> EVERY AXIS: 280 files, read-fidelity 0, write-fidelity 0, read key-gap
> 0, write-target 0** — re-verified this session WITH the genus sections
> attached as additional output; the AC1021 survey 58/58 / 0 divergent
> records; the era censuses R2000 235/235, R2004 227/227, R2010 216/216,
> R2013 160/160). The ACS/SH campaign stays COMPLETE at 0/0. The §19
> structure campaign's READ axis stays ZERO corpus-wide. **THE ACTIVE
> WORK IS NOW THE §20 GENUS-GATE QUEUE — the fifth validation layer is
> LANDED (§20.6), REVIEWED, and ONE PACKET CLOSED (the same day's G-C
> container campaign: `acds_genus_diffs` 11 rows → 0 — the constructed
> AcDs container now sits at the authored genus on every measured
> invariant, era-profiled for 2013/2018): the gates
> decode the constructed corpus silver-side,
> assert it against the authored-specimen genus pinned in
> genus_expectations.json, and emit the ranked sections
> sab_form_diffs (65 rows) / sh_genus_diffs (1 row) / acds_genus_diffs
> (0 rows — CLOSED). THE COUNTS ARE THE WORK QUEUE.** Read
> `tests/gold_harness/AGENTS.md` first, then §20 (all of it — now with
> the §20.6 landed state), then §19.4 + §19.5, then §18.6 + §F2.1–F2.3,
> then this file top to bottom.

## The arc (2026-09-28, the §20 implementation session)

1. **The genus gates implemented, verbatim in §20.3's shape**:
   `genus_extract.py` (the expectation extractor — decodes the
   156-file sh_history family silver-side, walks every SAB blob with a
   framing mirror of `sab.rs`, projects the three families into the
   pinned `genus_expectations.json`; fixtures-only pin so it
   regenerates identically everywhere; `--corpus-scan` folds in the
   ACIS-bearing gold-tree files on demand), `genus_gates.py` (the gate
   run — regenerates the constructed corpus: the gen_all canonical,
   its md5 recorded as a fact, plus the `genus_constructed` fixture
   family: one solid per SAB surface family, one region, one body, one
   `create_solid_history` tree; decodes, asserts, emits the ranked
   sections), the `run_corpus.py` integration (the sections as
   ADDITIONAL output; the four axes and the differ untouched; a genus
   pipeline failure degrades to an error note), and the env-gated cargo
   mirror `tests/genus_gates.rs` (presence-gated, skip-pass with a
   marker file; asserts a fresh extraction EQUALS the pin — expectation
   drift is itself reviewable — and the report well-formed; the counts
   are NOT asserted zero).
2. **The day-one extraction is clean**: 136 SAB carriers (all four
   eras — the 2007/2010 in-entity SABs carry the 21200/21500
   flavors), 136 SH roots (uniform: the ACAD_EVALUATION_GRAPH
   interposition, 33/427, owner != ownerhandle, node ids resolve,
   node owner is the graph, the solid's history_handle links the
   root), 78 jard containers (ds_version 16/17, segidx-first 128,
   num_segidx 91/97, the two era tail patterns, the named-pointer slot
   types), ZERO extraction anomalies. The width sets carry the
   authored variance honestly: coedge {55, 60}, straight-curve
   {85, 103}, both terminators, four magic/version pairs.
 3. **The day-one queue (the initial counts — the designed nonzero)**:
    - **G-A (67 rows)**: the vertex short-width (34 vs the authored 39
      — the one topology class the conic/quadric completions did not
      cover), the missing asmheader, the `ACIS|700` flavor vs the
      authored 21200/21500/21800/22300 pairs, the header triple
      (0,1,0) vs the authored (0,2,*), the product strings
      ('acadrust'/'ACIS 7.0' vs 'Autodesk AutoCAD'/'ASM 232.6.0.65535
      NT'), the tolerance triple (spatial_resolution 10.0 vs the
      authored 1.0), the missing persubent-acadSolidHistory attribs,
      and the record-class ordering family (the writer's restore-file
      rank vs the authored order — with the point-rank rows its
      largest arm: the writer emits points FIRST, every authored
      specimen emits them LAST). **The completions' regression guard
      HOLDS: cone-surface 149, ellipse-curve 118, plane-surface 112,
      sphere-surface 122, torus-surface 130, straight-curve 85 all sit
      AT the authored genus.**
    - **G-B (1 row)**: the constructed-tree elide marker — the
      2646f05 contract (no ACSH records; the solid's history
      soft-pointer NULL) ranks as the tree's current genus divergence,
      adjudicated TOLERATED by the cylinder verdict; the topology
      invariants — including the solid-history-links-root arm — stay
      armed for any SH record that appears in a constructed decode.
      Tree-ABSENCE on a constructed solid is deliberately NOT ranked
      (the specimen family is selected for trees: 136 carriers = 136
      roots — selection bias, not genus).
    - **G-C (11 rows)**: ds_version 1 vs the authored 16/17, the
      segidx-last position vs the authored segidx-first 128,
      num_segidx 8 vs the authored 91/97, file_header_size 128 vs
      65664, the populated-slot tail pattern, the unpopulated prvsav
      slot — the ranked queue head, exactly as §20.2 predicted.
 4. **The review pass (the same day, the OCS lens — "review that the
    genus gates validate what OpenstudioCad constructs against the
    golden genus") found and fixed two instrument defects**: (a) the
    ordering genus computed `after ∩ keys` — a UNION — instead of
    always-after (`after − before`): variable pairs (coedge↔cone-
    surface, cone-surface↔plane-surface) were pinned as constraints
    BOTH ways (false positives against constructed orders the
    authored genus itself shows), and `point` — last in every
    authored first-appearance order — fell out of every constraint
    set, hiding the writer's points-first rank (the largest true
    ordering divergence) entirely; the fix recomputed after−before,
    regenerated the pin, and the corrected G-A queue grew 55→67 rows
    (false rows gone, the point-rank rows in). (b) the pin carried
    `solid_history_links_root: [True]` (the cylinder-audit
    dangling-link sibling: every authored root is named by its
    owning solid's history soft-pointer) but the gate never asserted
    it — the arm landed, armed for any constructed tree that
    survives save. Also verified in the pass: the constructed family
    exercises ONLY the public API surface (the OCS command path —
    the primitive builders, from_sat, add_entity,
    create_solid_history), and the unused-import residue was
    cleaned from both instruments.
 5. **The G-C container packet CLOSED (the first queue packet, the
    same day)**: `build_acds_prototype` gained the era profiles
    live-measured from the specimens (ds_version 16/17, segidx-FIRST
    at 128, the 91/97-row scales with the authored slot allocations,
    the named pointers, the populated prvsav — 48-byte header + zero
    body — and 2013's freesp, file_header_size 65664, unknown_1 8;
    the invented `_data_` id=3 thumbnail boilerplate dropped — the
    authored sections carry one `_data_` row). Two extractor arms
    extended (`unknown_1s`, `container_versions`), two regression
    tests pin both eras through construct → write → read-back with
    the SAB surviving. **`acds_genus_diffs`: 11 rows → 0.** The
    generation identity MOVED — the container bytes changed:
    `4265c04a19048e33295bc047cb0b2908`, 25,407 bytes (re-recorded
    here; an intended content change, never papered over).
 6. **The zero-keeping rule held through the G-C landing**: the full
    corpus re-ran at 280 files with ALL FOUR AXES 0 and the genus
    sections attached (the authored corpus rides the echo — the
    constructed container path is disjoint); the hermetic suites are
    green (52 segments, 0 failures, the two new era tests in); the
    genus cargo mirror green (fresh extraction == the regenerated
    pin); the four family smokes 0/0; the AC21 echo clean.
 7. **The vertex packet CLOSED (the second queue packet, the same
    day)**: the authored vertex's missing token — its role within
    its own edge (0 = start, 1 = end, 2 = both endpoints of a
    closed edge; the semantics pinned by a 580-vertex census: every
    authored vertex carries the 4-token payload, roles distribute
    216/268/96, the per-vertex edge-endpoint correlation is exact).
    `add_vertex` emits the role placeholder; `add_edge` fills it for
    the owning vertex records. **The vertex row's 28 occurrences
    gone: `sab_form_diffs` 67 → 66 rows, 521 → 493 occurrences.**
    The identity moved to
    `0e8b23cde1f2476198154cc317b16683`, 25,439 bytes. The suite
    green; the mirror green.
 8. **The asmheader packet CLOSED (the third queue packet, the same
    day)**: the authored SAB opens with
    `asmheader $-1 $-1 "232.6.0.65535"` (era-uniform, byte-identical
    across all 136 carriers). The record prepends at the SAB-write
    boundary (`prepend_asmheader`) with every wire pointer shifted
    by +1 — the SatDocument keeps the DXF-SAT body-at-0 convention
    (`new_body`'s recorded rule); the authored numbering is
    asmheader $0, body $1, …. Five SAB module tests updated to the
    shifted index convention. **The asmheader row's 11 occurrences
    gone: `sab_form_diffs` 66 → 65 rows, 493 → 482 occurrences.**
    The identity moved to
    `a8f227e09d60a17605043f9dedebc30c`, 25,439 bytes.

## THE NEXT WORK: work down the genus-gate queue (§8.1.2 packets, strict-loader verdicts)

The queue is ranked in `target/genus_gates/genus_report.md` (and in the
corpus report's §20 section). Each row closes through the packet
workflow: fix the constructed form, re-run
`python3 tests/gold_harness/genus_gates.py`, and let the counts fall; a
genus expectation for a surface where gold is known-wrong changes ONLY
through a recorded strict-loader probe verdict (§20.4 — the gates rank
divergence; they do not decide fatality; some rows may close as
TOLERATED with the row kept as the recorded state).

- **G-C CLOSED (the container campaign, same-day packet)**: the era
  profiles landed (§20.6); `acds_genus_diffs` 0. Guard: the two
  regression tests + the gate rows stay armed — any writer change that
  regresses ds_version, the segidx position, the row scale, the slot
  allocation, the pointers, prvsav, or the two extended header fields
  re-ranks immediately.
- **G-A NOW FIRST (the SAB form campaign)**: the asmheader emission
  (authored-uniform class), then the tolerance triple
  (spatial_resolution 10.0 vs the authored 1.0 — find the constructed
  default's source), then the ACIS-700
  flavor / product-string / header-triple family (a strict-loader
  question: the 2026-09-21 zero proved the ACIS-700 stance
  BricsCAD-ACCEPTED, so these rows may close TOLERATED; do NOT forge
  Autodesk identity stamps — the product strings are the author's
  identity), then the ordering family (the writer's
  `reorder_restore_file` rank vs the authored first-appearance genus),
  and the persubent-attrib class (investigate the tree-correlation
  with the `--corpus-scan` specimen evidence first — the fixture
  family is tree-selected, so uniformity may be selection bias).
  (The vertex short-width row CLOSED — the second packet, arc item
  7.)
- **G-B last (the tree)**: the elide marker closes only when a
  constructed tree passes a strict loader (the interposition + the
  33/427 trio + the node-id resolution are the authored genus to
  reproduce); until then the row stands as the recorded state.

## The final design (the one thing to understand — unchanged)

**The unified whole-file echo** (`write_to_writer` in
`src/io/dwg/dwg_writer.rs`): when the document came from a
same-version DWG read, her whole on-disk file is retained
(`document.raw_ac21_tail`, serde-skipped, every container family),
and the document-state hash holds, the rewrite re-emits her bytes
VERBATIM. Everything else — the H8b/H7g mirrors, the conventional
arms, the prepare pipeline — serves edited documents and conversions,
unchanged.

**The document-state fingerprint**
(`document_state_fingerprint` in `src/io/dwg/mod.rs`): the sorted
per-part hash of the semantic inventory's visit plus the ten table
control handles and the retained metadata models. Captured at the END
of the read; compared BEFORE the prepare pipeline at the write entry.
The wire-only captures (`table_control_entries`, `dimstyle_morehandles`)
are EXCLUDED from the fingerprint; `xdic_by_handle`, the modeled
subcurve fields, the persubent tails, the SEQEND flag captures, the
constraint-group node captures, the persubent-id tail + stream-
presence captures, and the TABLECONTENT wire captures are INCLUDED.

**The honest framing, FINAL**: for an unedited same-version roundtrip
the DWG writer is a byte-copy gated on a full-content hash. The
conventional arm is record-identical to the author's stream on EVERY
AC1021 corpus file (58/58, 0 divergent) AND every record of all four
era specimens (the era censuses all-zero).

## The remaining work (after the queue opens)

- **The genus-gate queue itself** (above) — the initial NONZERO
  counts are the queue; each closes through the §8.1.2 packet workflow
  with the strict-loader verdicts adjudicating.
- **The version-parity tiers outside the scope** (§19.5): R13/R14
  implemented-but-divergent (89/153/149 diffs on the three r14
  specimens); pre-R13 unsupported (gold decodes nothing there either).
  Both pathed in §19.5; maintainer decisions.
- **The era censuses across the full era corpora**: today's proofs
  cover the Constraints specimens (1 file/era).
- **The unattested subcurve action types** (17/19/23/42/27): no corpus
  specimens. **The MT-variant pinning (§19.4.G)**: the crc_seed draws.
  **The dead rows**: LoftD; the SH revolve shorts; BREP deferred.
  **The ATMOS 84 her-only orphans**: the broken-map pre-existing issue.
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does) — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative: all four axes 0, WITH the
  genus sections (`sab_form_diffs` 65 / `sh_genus_diffs` 1 /
  `acds_genus_diffs` 0 — the container campaign CLOSED) as
  additional output.
- The generation identity is `a8f227e09d60a17605043f9dedebc30c`,
  25,439 bytes (MOVED at the G-C, vertex, and asmheader packets —
  the constructed AcDs container, the vertex role token, and the
  asmheader record changed; intended content changes, re-recorded;
  the identity history: `84374e73…`/25,375 through the §20 landing →
  `4265c04a…`/25,407 at G-C → `0e8b23cd…`/25,439 at the vertex
  packet → `a8f227e0…`/25,439 at the asmheader packet). The
  generator builds and runs identically WITH or WITHOUT
  `--features serde`.
- The genus expectations pin
  (`tests/gold_harness/genus_expectations.json`) is FIXTURES-ONLY:
  regenerate with `python3 tests/gold_harness/genus_extract.py` and
  review the drift; the cargo mirror asserts fresh == pin.
- The record-identity state (the 58-file AC1021 survey): **58/58
  files at 100%, 0 divergent records, 84 her-only orphans (ATMOS's
  broken map)**. circle 211/211; ExtrudeC 206/206; Box 207/207;
  Leader 245/245; Chamfer 207/207; Fillet 207/207; Loft 207/207;
  PolyLine3D 218/218; Constraints 219/219; ATMOS 340/340;
  example_2007 540/540. **The era censuses: R2000 235/235, R2004
  227/227, R2010 216/216, R2013 160/160.**
- The gold tree sits at `34f02f54` FROZEN with ONE tracked
  generated-file drift (`src/config.h.in`, autoheader requote) — the
  freeze rule stands. Oracle fingerprints unchanged: the 6,316-byte
  libtool wrapper `programs/dwgread` (md5
  `8dad57211b78f42e7594ba6c211cc0e5`) and the ELF
  `programs/.libs/dwgread` (md5
  `d852da1db0894b866e86042d4b26b91d`, 230,232 bytes). NO
  `encode_r2007.c` exists — gold has no R2007 writer.
- Spec authority on file: the ODA spec PDF
  (`~/work/OpenDesign_Specification_for_.dwg_files.pdf`, 270pp) +
  libredwg `bits.c`. NEITHER documents the ACDBASSOC* classes, the
  subcurve wire, the non-assoc PersSubentManager, the
  constraint-group node classes, or the TABLECONTENT wire (the
  authority closure); the ASSOC persubent variant HAS a gold spec
  block; the 3DSOLID history_id AVAIL_BITS rule IS in gold's
  dwg.spec COMMON_3DSOLID; gold's pab/child_param declared ref codes
  (5/3) are WRONG vs the wire (4/4).
- The hermetic suites: serde green (the full suite at this halt), 
  gold_roundtrip green, genus_gates green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative; the survey's merge-trace positions are
  SPAN-relative. The record-head MS is 15 data bits per 16-bit LE
  word. **The merged-stream frame math**: RL = main + text + 16 + 1;
  main_data_end = RL − text − 17; the handle stream starts at the RL.
  **The pre-2007 TV wire**: BS(len+1) + chars + NUL. **The
  TwoStream frame**: the handle stream is BIT-CONTINUOUS at the RL.
  The BL forms: 00 = 4-byte LE, 01 = 1 byte, 10 = 0; the BD forms: 00
  = full 66-bit LE double, 01 = 1.0, 10 = 0.0. **The SAB walker
  facts** (genus_extract.py mirrors `sab.rs` framing): the
  ENTITY_TYPE 0x0D and SUBTYPE 0x0E tags carry a RAW 1-byte length
  (not a string tag); the ASM magic is 14 bytes + 1 trailing byte
  with the version u32 at offset 15 either way; the resfit double is
  optional; the End-of-* terminator carries no attribute pointer and
  no EOR. **The campaign lessons (the full set lives in the §19.2
  rows + §19.4)**: the SEQEND era-convention lesson; the pad lesson;
  the era-gate lesson; the presence lesson; the recompute lesson;
  the ref-code lesson; the normalize lesson; the form-rule lesson;
  the content-agnostic lesson; the reader-correctness proof; the
  accessor-gate pattern.

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use long timeouts on the bash tool for
the corpus — the full corpus takes ~4 minutes + ~2 for the genus
stage; the bash tool's timeout parameter is capped at 120 s in
practice — run the corpus as a background process and read its
logs).

```bash
# Environment (source this):
export PATH="$HOME/.cargo/bin:$PATH"
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
```

## Verification gate (the zero-keeping rule — all four axes; the genus sections are additional)

```bash
# 1. Build gates
cargo test --features serde          # green at this halt (the full suite)
cargo test --features gold-harness --test gold_roundtrip
cargo test --features gold-harness --test genus_gates   # the NEW mirror
cargo test --features serde --test issue80   # the edit-survival gate

# 2. Family smokes (all four families, write-target 0)
python3 tests/gold_harness/run_roundtrip.py \
    <FIXTURE>.dwg /tmp/smoke
# AC21: $GOLD_TESTDATA/2007/circle.dwg
# R2000: $GOLD_TESTDATA/example_2000.dwg
# AC18: $GOLD_TESTDATA/example_2004.dwg
# R2018: $GOLD_TESTDATA/2018/Dynblocks.dwg

# 3. Full corpus (280 files; ALL FOUR AXES 0 — the genus sections
#    are ADDITIONAL output; these totals must not move)
python3 tests/gold_harness/run_corpus.py
#    ... genus gates (the §20 work queue): sab_form 65 rows,
#        sh_genus 1 rows, acds_genus 0 rows (G-C CLOSED; the vertex
#        and asmheader rows closed at the second and third packets)

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# a8f227e09d60a17605043f9dedebc30c, 25,439 bytes (G-C + the vertex
# role token + the asmheader record moved it)
# (identical without --features serde)

# 4b. The record-identity survey (58/58, 0 divergent)
python3 tests/gold_harness/record_identity_survey.py \
    "$GOLD_TESTDATA"/2007/*.dwg "$GOLD_TESTDATA"/example_2007.dwg \
    tests/gold_harness/tests/sh_history/*_2007.dwg

# 4c. The era censuses (the handle-keyed -v9 comparison)
# R2000 235/235, R2004 227/227, R2010 216/216, R2013 160/160

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde

# 7. THE GENUS GATES (the §20 layer — LANDED)
python3 tests/gold_harness/genus_extract.py   # regenerate expectations
python3 tests/gold_harness/genus_gates.py     # the ranked report
# lands NONZERO on purpose — the counts are the work queue
```

## Commit inventory (this halt)

```
574a59e <docs> the post-H8h-ext-6 halt refresh (the prior halt head)
a1a1506 <feat> H8h-ext-7: the SEQEND captured-flags fix (15 -> 14)
b5e16b5 <docs> the post-H8h-ext-7 halt refresh
195ef3b <docs> the Constraints 3E3 dissection-state handover
d6b0431 <feat> H8h-ext-8: the Constraints 3E3 closure (13 -> 12)
d3d2258 <feat> H8h-ext-9: the ATMOS closure (12 -> 9)
0625f6c <feat> H8h-ext-10: the example_2007 first fruits (9 -> 5)
3dd3362 <feat> H8h-ext-11: the wire-exactness landings (5 -> 3)
000f3b2 <feat> H8h-ext-12: the zero-residual landing (3 -> 0)
d9bf5f3 <docs> the post-H8h-ext-12 maintenance review
6712327 <feat> H8h-ext-13: the pre-2007 constraint-group arms closed
dc3737a <feat> H8h-ext-14: the R2010/R2013 arms closed + the census
0f2bbfa <feat> H8h-ext-15: the era full-file record identity closed
2bafbe4 <docs> the post-H8h-ext-15 maintenance review
a99ccc6 <docs> the version-parity matrix as future work (§19.5)
274fcf4 <docs> the version-parity matrix in the README
2646f05 fix(dwg): the 2026-09-28 cylinder verdict - conic/quadric
       completions + the constructed-tree elide + §20 (the design)
da78f79 docs(harness): the IMPLEMENTATION.md review + the stale-claim
       reconciliation (§1 goal, §5 coverage, §7 title, §8.1.6 pointer,
       §17 F1/F2) + the genus-gates handover
ba12a2f <feat> §20 genus gates: genus_extract.py + genus_gates.py + the
       genus_constructed fixture-family bin + the pinned
       genus_expectations.json + the run_corpus additional sections +
       the tests/genus_gates.rs cargo mirror + §20.6 (the landed state)
       + the README step 7 + the AGENTS.md fifth-layer contract +
       this halt record
<fix> the genus-gate review (the OCS lens): the ordering genus
       recomputed as always-after (after − before, not the union that
       pinned variable pairs both ways and hid the points-first rank;
       the pin regenerated, G-A 55 -> 67 rows — false rows out, the
       point-rank rows in) + the solid-history-links-root arm gated +
       the unused-import residue cleaned + the docs reconciled
       (THIS SESSION'S REVIEW LANDING)
<fix> §20 G-C packet: the constructed AcDs container era-profiles —
       ds_version 16/17, segidx-first at 128, the 91/97-row scales,
       the authored slot allocations + pointers, the populated prvsav
       + 2013 freesp, file_header_size 65664, unknown_1 8, the
       invented thumbnail _data_ dropped; two extractor arms extended;
       two era regression tests — acds_genus_diffs 11 -> 0; the
       generation identity moved to 4265c04a... (25,407 bytes)
       (THIS SESSION'S G-C PACKET)
<fix> §20 G-A vertex packet: the authored vertex role token (0=start,
       1=end, 2=closed-edge; the 580-vertex census) — add_vertex emits
       the placeholder, add_edge fills the owning records' roles; the
       vertex row's 28 occurrences gone (sab_form 67 -> 66 rows,
       521 -> 493 occurrences); the identity moved to 0e8b23cd...
       (25,439 bytes)        (THIS SESSION'S VERTEX PACKET)
<fix> §20 G-A asmheader packet: the authored SAB's opening record
       (asmheader $-1 $-1 "232.6.0.65535", era-uniform 136/136)
       prepended at the SAB-write boundary with the +1 wire-pointer
       shift (the DXF-SAT body-at-0 convention preserved); five SAB
       module tests updated to the shifted indices; the asmheader
       row's 11 occurrences gone (sab_form 66 -> 65 rows,
       493 -> 482 occurrences); the identity moved to a8f227e0...
       (25,439 bytes) (THIS SESSION'S ASMHEADER PACKET)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the documentation
halt → §20 read in full + the extraction surface located (the SAB
walker facts pinned by live probing: the raw-length type tags, the
ASM header layout, the optional resfit) → the specimens decoded and
the genus measured (136 SAB carriers, 136 SH roots, 78 jard
containers, zero anomalies) → the constructed-fixture family built
(`genus_constructed`: Box/Sphere/Cylinder/Cone/Torus/Region/Body/
HistoryTree, the elide verified live) → the extractor + gates + pin
landed → the ordering-check inversion caught and fixed on the first
report → the corpus re-run at 280/0/0/0 with the sections attached →
the identity + echo + suites green → the halt record written → **the
review pass (the OCS lens)**: the ordering genus's union defect found
by probing the pin's constraint symmetry (variable pairs pinned both
ways = false positives; `point` dropped from every constraint set =
the writer's points-first rank invisible), recomputed as
after−before, the pin regenerated, the ungated
solid-history-links-root invariant armed, the corrected queue verified
(false rows gone, point-rank rows in), the docs reconciled. **The
maintainer's loop instruction — "repeat process until
target = zero" — remains satisfied on its own terms: the corpus is at
zero on every axis, the conventional arm is record-identical
everywhere surveyed, and the fifth layer's queue is now MEASURED,
REVIEWED, and ranked — the next sessions work it down packet by packet
with the strict loaders adjudicating.**
