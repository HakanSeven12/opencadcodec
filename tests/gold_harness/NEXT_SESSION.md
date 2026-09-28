# Zero-context prompt — TARGET ZERO held everywhere; THE NEXT WORK: implement §20, the genus gates (the constructed-content oracle — the designed fifth validation layer; the stale-claim reconciliation of the early doc sections landed this session)

> Campaign state 2026-09-28 (the halt after the documentation
> reconciliation; the record-identity campaigns are ALL CLOSED: **THE
> CORPUS STAYS AT ZERO ON EVERY AXIS: 280 files, read-fidelity 0,
> write-fidelity 0, read key-gap 0, write-target 0**; the AC1021 survey
> 58/58 files / 0 divergent records; the era censuses R2000 235/235,
> R2004 227/227, R2010 216/216, R2013 160/160). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. **THE ACTIVE WORK IS NOW §20 — the genus gates, a
> designed-but-unimplemented validation layer.** Read
> `tests/gold_harness/AGENTS.md` first, then §20 (all of it — the
> principle, the three gate families, the mechanics, the
> stability-baseline rule, the scope guard), then §19.4 (the
> external-mechanism reference) + §19.5 (the version-parity matrix),
> then §18.6 (the SAB/SH autopsy records the gates mechanize) +
> §F2.1–F2.3 (the fixture tree), then this file top to bottom.

## The arc (2026-09-28, the documentation session — the reconciliation)

1. **The IMPLEMENTATION.md review found six stale claims** in the early
   framing sections — residue of the document growing top-down from its
   2026-09-16 origins without a pass to reconcile the opening chapters
   with the landed state.
2. **All six fixed to reality**: (a) §1's goal statement — the "immediate
   blocker" (the long-resolved AcDbVisualStyle error) removed, the
   Phase-2 "planned" framing replaced with the landed state; (b) §5's
   day-one gap census — the covered-types list now states the final
   0/0 coverage (MTEXT, DIMENSION, TABLE, REGION, VIEWPORT, IMAGE,
   ATTDEF/ATTRIB, MLEADER all covered through the §8.1.6 packets and
   the §19 campaign), with the genuinely-out-of-loop types named
   (MESH, 3DFACE, SOLID-2D, TOLERANCE, WIPEOUT, XREF, OLE2, LIGHT,
   CAMERA, ARCDIMENSION); (c) §7's title — marked HISTORICAL (the
   campaign target completed 2026-09-20); (d) §8.1.6's queue pointer —
   the "not yet started" §19 arc replaced with the LANDED record; (e)
   §17 F1 — marked LANDED as §19's structure axis (H2/H3 landed the
   FILEHEADER/HEADER comparisons at zero); (f) §17 F2 — the fixture
   tree marked LANDED (the sh_history campaign, 156 files at 0/0), the
   remaining gaps named.
3. **The README** already carries the version-parity matrix (the
   File Version Support table with the per-code parity tiers) — current.
4. **The genus gates were scoped but NOT started** — the implementation
   is the next session's work; the full handover is below.

## THE NEXT WORK: implement §20 — the genus gates (the designed layer)

**Why it exists** (§20's opening): every existing gate compares silver
against GOLD ON THE SAME BYTES. Constructed content — documents silver
authors from scratch — has no gold counterpart, and the blindness
shipped real defects: the 2026-09-28 cylinder audit (a
factory-assembled `ACSH_HISTORY_CLASS` with a duplicated payload owner,
a dangling history_node_id, and the 1/0 version trio where every
authored specimen carries 33/427 — BricsCAD: "Duplicate ownership of
reference" + "Data stream is empty") and the SAB conic/quadric
short-width desync (cone 138 vs the authored 143; ellipse 109 vs
111/112 — "missing logical in restore file" ×17). Both passed the
corpus 280/0/0 the same morning.

**The principle** (§20.1): the corpus's authored specimens ARE the
genus reference. Silver is the ONLY complete decoder of the surfaces
gold cannot read (the SAB blobs, the SH node regions — gold's flat
REPEAT desyncs at node[1]). The gates decode the specimens
SILVER-side, extract their invariants, and assert silver's CONSTRUCTED
output against the result — mechanizing the manual autopsies the
campaigns ran by hand.

**The three gate families** (§20.2):

- **G-A — SAB form genus**: per-class record widths (width SETS —
  authored variance is legitimate, e.g. ellipse 111/112), the header
  triple, the magic/version pair, record-class ordering. Asserted on
  the gen_all canonical file's ACIS entities + a constructed-fixture
  family. The demonstrated catch: the cone-138/ellipse-109 short-width
  desync.
- **G-B — SH tree genus**: the topology invariants (the history
  payload owner resolves to a graph object, never the solid; payload
  owner != ownerhandle; the version trio 33/427; history_node_id
  resolves to a written node; the node's owner is the graph) + the
  scalar root fields. Asserted on constructed trees from
  `create_solid_history`. The demonstrated catch: the factory genus
  ranks on four invariants at once.
- **G-C — container genus**: the AcDs `jard`/`segidx` fields on
  CONSTRUCTED AcDs sections vs the authored-container invariants
  (ds_version, segidx position/ordering, num_segidx scale). The known
  day-one divergence: silver's ds_version=1 container vs the native 16
  (segidx-first, 91 entries) — the ranked queue for the container
  campaign.

**The mechanics** (§20.3 — the implementation plan):

1. `genus_extract.py` (NEW, `tests/gold_harness/`): runs `dwg2json` +
   the SAB/SH projections over the fixture specimens (the sh_history
   family + any ACIS-bearing corpus file), emits
   `genus_expectations.json` — per-class width SETS, topology
   invariants as predicates, container field values. Regenerable; the
   pinned copy diffed in CI so expectation drift is itself reviewable.
2. `genus_gates.py` (NEW): decodes the constructed corpus (the gen_all
   canonical + a small constructed-fixture family the codec emits: one
   solid per primitive family, one `create_solid_history` tree, one
   region), asserts against the expectations, emits ranked report
   sections — `sab_form_diffs`, `sh_genus_diffs`, `acds_genus_diffs` —
   in the corpus report's existing (type, field, count) shape.
3. `run_corpus.py` integration: the sections as ADDITIONAL output; the
   0/0 read/write-fidelity semantics and totals are UNTOUCHED (the
   differ stays frozen). **The gates land NONZERO on purpose: the
   initial counts ARE the work queue** (G-C's container divergence
   ranks day one), exactly as §19's audit matrix did.
4. The `cargo test --features gold-harness` mirror: an env-gated test
   alongside `gold_roundtrip.rs` (the corpus fixtures may be absent in
   some environments — gate on presence, skip cleanly).

**The stability-baseline rule** (§20.4 — the hardest lesson): where
gold's interpretation is known-wrong (the R2013+ 3DSOLID entity
stream — the 2026-09-25 verdict: gold's "clean" native decode carries
garbage revision fields; BricsCAD accepts what gold rejects), the
genus reference is NOT gold but the last BricsCAD-verified byte form
(the canonical-md5 class of baseline). A genus expectation for that
surface changes ONLY through a recorded strict-loader probe verdict,
never an oracle trace alone. The gates rank divergence; they do not
decide fatality.

**The scope guard** (§20.5): genus gates never modify the differ, the
normalizers, or the fidelity totals (AGENTS.md). Expectations come
from specimens, not hand-pinned magic numbers. A new genus arm
requires the same evidence bar as a codec census comment: specimen
lineage, the divergence it ranks, and the strict-loader verdict that
adjudicates it.

**Where the invariants live in the code** (the extraction surface):
the SAB writer `src/entities/acis/sab.rs` (the record forms the
authored specimens pin); the SH history assembly
(`create_solid_history` — the payload owner/version-trio invariants);
the AcDs container writer (`ds_version`/segidx — the §18 AcDs records);
the constructed-content entry points: the generator example
`examples/gen_all_entities_all_versions_dwg.rs` + the
programmatic-API constructors. The specimen corpus:
`tests/gold_harness/tests/sh_history/` (156 files, 39 per era
2007/2010/2013/2018) + the ACIS-bearing gold files (Box/Cone/
Cylinder/…, the §F2.3 table).

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
subcurve fields, the persubent tails, the SEQEND flag captures, the
constraint-group node captures, the persubent-id tail + stream-
presence captures, and the TABLECONTENT wire captures are INCLUDED.

**The honest framing, FINAL**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on EVERY AC1021 corpus file (58/58, 0 divergent) AND every
record of all four era specimens (the era censuses all-zero).

## The remaining work (after §20 lands)

- **The genus-gate work queue**: the initial NONZERO counts the gates
  emit are the queue (G-C's container divergence first); each closes
  through the §8.1.2 packet workflow with the strict-loader verdicts
  adjudicating.
- **The version-parity tiers outside the scope** (§19.5): R13/R14
  implemented-but-divergent (89/153/149 diffs on the three r14
  specimens); pre-R13 unsupported (gold decodes nothing there
  either). Both pathed in §19.5; maintainer decisions.
- **The era censuses across the full era corpora**: today's proofs
  cover the Constraints specimens (1 file/era).
- **The unattested subcurve action types** (17/19/23/42/27): no
  corpus specimens. **The MT-variant pinning (§19.4.G)**: the
  crc_seed draws. **The dead rows**: LoftD; the SH revolve shorts;
  BREP deferred. **The ATMOS 84 her-only orphans**: the broken-map
  pre-existing issue.
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does) — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-15). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey): **58/58
  files at 100%, 0 divergent records, 84 her-only orphans (ATMOS's
  broken map)**. circle 211/211; ExtrudeC 206/206; Box 207/207;
  Leader 245/245; Chamfer 207/207; Fillet 207/207; Loft 207/207;
  PolyLine3D 218/218; Constraints 219/219; ATMOS 340/340;
  example_2007 540/540. **The era censuses: R2000 235/235, R2004
  227/227, R2010 216/216, R2013 160/160.**
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
  subcurve wire, the non-assoc PersSubentManager, the
  constraint-group node classes, or the TABLECONTENT wire (the
  authority closure); the ASSOC persubent variant HAS a gold spec
  block; the 3DSOLID history_id AVAIL_BITS rule IS in gold's
  dwg.spec COMMON_3DSOLID; gold's pab/child_param declared ref
  codes (5/3) are WRONG vs the wire (4/4).
- The hermetic suites: serde green (1602/0 + issue80 7/0),
  gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative; the survey's merge-trace positions are
  SPAN-relative. The record-head MS is 15 data bits per 16-bit LE
  word. **The merged-stream frame math**: RL = main + text + 16 +
  1; main_data_end = RL − text − 17; the handle stream starts at
  RL. **The pre-2007 TV wire**: BS(len+1) + chars + NUL. **The
  TwoStream frame**: the handle stream is BIT-CONTINUOUS at the RL.
  The BL forms: 00 = 4-byte LE, 01 = 1 byte, 10 = 0; the BD forms:
  00 = full 66-bit LE double, 01 = 1.0, 10 = 0.0. **The campaign
  lessons (the full set lives in the §19.2 rows + §19.4)**: the
  SEQEND era-convention lesson; the pad lesson (trim the author's
  1s pad, re-create it explicitly); the era-gate lesson (gate
  workarounds to their era); the presence lesson (capture wire
  presence per-record — Some(NULL) vs None, has_strings 0 vs 1);
  the recompute lesson (no model-API semantics on DWG-read values);
  the ref-code lesson (declared codes are guesses — the wire is
  the authority); the normalize lesson (pop every new wire-capture
  field in the compare paths); the form-rule lesson (the H8d rule
  holds on all eras); the content-agnostic lesson (retain
  undecodable regions verbatim); the reader-correctness proof (the
  reader's own parse reveals the author's convention); the
  accessor-gate pattern (pad at access time, keep the model
  faithful).

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

## Verification gate (the zero-keeping rule — all four axes; §20 must not move them)

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

# 3. Full corpus (280 files; ALL FOUR AXES 0 — the genus sections
#    are ADDITIONAL output; these totals must not move)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 84374e73ddcf1d6143877c4100b81e48, 25,375 bytes
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

# 7. THE GENUS GATES (the new §20 layer — after implementation)
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
<docs> the IMPLEMENTATION.md review + the stale-claim reconciliation
       (§1 goal, §5 coverage, §7 title, §8.1.6 pointer, §17 F1/F2 —
       all six fixed to the landed reality) + this halt record
<feat> §20 genus gates: genus_extract.py + genus_gates.py + the
       run_corpus additional sections + the cargo test mirror
       (THE NEXT SESSION'S WORK — the design is §20; the handover
       above)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the ext-15
maintenance review → the IMPLEMENTATION.md review (the six stale
claims found: §1's dead blocker + "planned" Phase 2, §5's day-one gap
census, §7's "Current" title, §8.1.6's "not yet started" pointer,
§17's F1/F2 future-work framing of landed work) → all six fixed to
reality (the landed §19 state stated where the pre-§19 world was
described) → the genus-gates implementation scoped (§20 read in
full; the extraction surface, the three gate families, the
mechanics, the stability-baseline rule located in the code) → the
handover written (this file) for the next zero-context session. **The
maintainer's loop instruction — "repeat process until target = zero"
— remains satisfied: the corpus is at zero on every axis, the
conventional arm is record-identical everywhere surveyed, and the
next layer (§20, the genus gates) is fully designed with the
implementation plan recorded above.**
