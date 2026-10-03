# TODO.md — gold_harness open work

Revised 2026-10-03: finished items removed, the remainder written out
in plain language. The full history of everything closed here lives in
`NEXT_SESSION.md` (the session addenda) and `IMPLEMENTATION.md` (the
chronological campaign record).

## Where things stand

The large measured campaigns are closed and hold their zeros: parser
parity, the strict loader probes, the ACS/SH solid-history work, and
the §20 genus gates (pending-zero under `--strict`). The suite passes
1,618 tests with none failing, and the generation identity is stable at
`f2187565…` over 25,728 bytes. The genus cargo pin assertion is red
pre-existing (item A7).

The corpus counts 694 files in five populations (the R13/R14 era
admitted 2026-10-03 with C2). The established 401
libredwg-sourced files carry eight pre-existing fidelity rows — five
LIGHT `light_color` rows on the example_2004…2018 files and three
UNKNOWN_OBJ common-field rows on Wipeout_2004 and gh44 (item A5; both
verified pre-existing by bisection). The twenty b6_routes fixtures
read 0/0. The 221 golden_entities fixtures carry three rows:
Leader_AC1014's single row plus the Region_AC1012 and Solid3d_AC1012
ACIS rows (item A9). The six brep mints carry thirteen rows in one
family — the BREP record's R2013+ trailing region (also item A9).
The era population (the gold r14 dir, the two root examples, and the
41-file authored wave) measured its opening census at 103 rows:
example_r13 78, example_r14 20, the gold r14 Leader 4, and the
wave's Leader_r14 1 — while the other forty wave files and the gold
r14 Constraints/v read 0/0 (the era-census continuation's R13/R14
gates had already absorbed most of the previously recorded
89/153/149). The struct read key-gaps stand at 5,510 (the era files
carry the bulk). The genus cargo pin assertion is red pre-existing
(item A7).

Items closed since the last revision, removed from the list below:
B3 (the R13/R14 specimen wave is fully authored — 41 files parked
under `fixtures/r13_r14/`; admitting it is decision C2); B4 (the
ACSH_BREP_CLASS "external-only" verdict was
falsified on 2026-10-03 — the class mints from scratch with
`SOLIDHIST=1`, and the dataset landed in `fixtures/brep/`); B5 (the
dead rows were recorded terminal on 2026-10-02 after all three lever
routes measured dead); B6 (every authoring route for subcurve kinds 19
and 27 was measured dead across the b6_routes specimens — the kinds
remain attested nowhere, and any future specimen rides the
capture+replay net automatically, so nothing is owed until one
surfaces); C4 (the golden-entities campaign landed on 2026-10-03 and
its original work surface was demolished by the era-census
continuation — the one question it left open is C2); A4 (the pre-R13
mirroring task — C3 was decided 2026-10-03 to keep the outright
reject, so the task is dead until the maintainer reopens the
decision).

## A. Open tasks (agent code work)

### A5. The eight pre-existing corpus rows

Five LIGHT rows: gold reads `light_color` 5 on
example_2004/2007/2010/2013/2018 where silver reads null — the
leftover of the B1 raw-CMC twin family. Three UNKNOWN_OBJ rows on
Wipeout_2004 and gh44: the common-data conventions B1 left recorded
(`is_xdic_missing`, `ownerhandle`, `reactors`). The packet is a
read-side projection for the LIGHT twin's `light_color`, plus a
convention look at the two UNKNOWN_OBJ rows. Sources: the
H8h-ext-16 validation record and the B1 session records.

### A6. The `--no-lz77` DataStore re-emission blocker

The no-lz77 write arm dies on every 2013+ authored file at the
`AcDb:AcDsPrototype_1b` section with
`InvalidFormat("Unknown AC21 section: AcDb:AcDsPrototype_1b")`. The
failure is pre-existing (reproduced on the pre-B2 tree). Until it is
fixed, the layer-4 conventional-arm validation can only ride the
2007/2010 frames; the 2013/2018 typed emissions are covered by
unit-level bit arithmetic instead
(`tests/edge_action_param_subcurve.rs` is the working example). The
packet: skip or echo the DataStore section the way the default write
path does.

### A7. The genus cargo pin regeneration

The `genus_gates` cargo mirror asserts that a fresh extraction equals
the pinned expectations, and it is red pre-existing
(bisection-verified; the python `--strict` gates stay pending-zero
and the TOLERATED rows are unchanged). Per §20.3, regenerate
`config/genus_expectations.json` with `genus/genus_extract.py` and
review the drift — the seven-packet session of 2026-09-30 that last
moved the generation identity is the likely cause. The regeneration is
mechanical; the drift review is the maintainer's recorded verdict.

### A8. The composite (47) subcurve dissection

The NURB3D (42) half is closed: the grammar is named and
self-delimiting, the reader parses it typed (gated on the measured
constants, falling back to the capture+replay net on any deviation),
and the writer emits it without a version gate. What remains is the
47 composite — a delta-encoded polyline form carried by the
ExtrudePline, Extrude3DPoly, RevolvePline and LoftMixed quads plus the
2004/Surface.dwg records — which still replays verbatim. Naming its
fields is the remaining depth packet. Two optional enablers would
stretch the measured field set: a rational spline (weights ≠ 1) and a
CV-method spline. Both are safe to author — the parser's constant
gates fall back to the net, and the specimen would name the new
fields.

### A9. The era-census residue

The record-identity censuses (`analysis/record_size_census.py` against
a `DWG_NO_ECHO=1` rewrite) leave these surfaces, ranked by size:

1. The era population's real-world rows (the C2 campaign's opening
   surface): example_r13's 78 and example_r14's 20 read+write rows,
   plus the gold r14 Leader's 4 and the wave's Leader_r14 1. The
   per-record pair diff names the classes; the era-census
   continuation's gates already absorbed most of the previously
   recorded 89/153/149, so what remains is the true post-gate
   residue.
2. The golden R13 ACIS class — Region_AC1012 and Solid3d_AC1012 carry
   `acis_data` that gold re-serializes differently (the era-modeler SAT
   text). Two rows.
3. The BREP R2013+ trailing region — the brep dataset's thirteen rows:
   the `materials`, `has_revision_guid`, `revision_*` and `end_marker`
   fields that silver's typed ACSH_BREP_CLASS model does not emit yet.
   The two brep specimens are the clean per-version pair for a
   raw-remainder packet (the B2/A8 pattern: dissect, model typed, pin
   bit-exact).
4. The struct-axis key gaps — 5,510 read key-gaps, the bulk on the
   era files and the R13/R14 golden files (the key-shape class;
   untouched by the fidelity fixes).
5. The `class?` desync-mirror rows — eight on entities-2d and seven on
   entities-3d (the B1-era mojibake family).
6. PolyLine2D's last two POLYLINE_2D crc rows.
7. Surface.dwg's six rows (the §18 surface family) and gh44-error's
   eighteen (the pathological file: six LEADER records with a 2-bit
   slack between main data and handle stream, five crc-only HATCH
   rows, five type-57 rows, two DIMASSOC rows).
8. Leader_AC1014's single remaining row.

The standing era facts all hold and were re-verified on 2026-10-03:
gh209_1 166/166, gh109_1 664/664, HatchG 229/229, example_2000
750/750, example_2004 735/735, example_2010 536/536, example_2018
474/474, Constraints_2010 216/216, Constraints_2013 160/160.

## B. Fixture-needed tasks

None. The one question of this class — subcurve kinds 19 and 27 —
needs no fixture: every authoring route on this toolchain was
measured dead (the solid sweep and POLYSOLID emit no EdgeActionParam
records at all; the surface-mode sweep's path kind mirrors the path
entity's own curve kind; the sweep refuses infinite-line paths; the
constraint networks emit no edge params and refuse 3D curves), so no
AutoCAD or BricsCAD fixture can produce either kind. Both kinds stay
attested nowhere. If a real-world file carrying them ever surfaces
(from any source, any era), drop it into the fixtures tree with a
`.txt` provenance companion — the capture+replay net keeps it safe
from day one, and the typed-modeling packet opens then (the B2/A8
pattern). Until such a specimen appears, nothing is owed.

General fixture conventions, when new work does need one, are in
`fixtures/README.md`: fresh drawing, default template, one operation,
SAVEAS per target version, gold qualification with zero error lines,
and a `.txt` provenance companion.

## C. Maintainer decisions (recorded)

### C1. The §19.5 version-parity tier state — UPDATED 2026-10-03

Tier 1 (R2000–R2018) is at parity — the declared campaign scope,
complete, with record identity on every surveyed surface; the eight
measured open rows on the grown corpus are pre-existing B1-era
residue (A5), not regressions. Tier 2 (R13/R14) is now an ACCEPTED
parity campaign (C2, 2026-10-03): silver already reads and writes
both eras and gold re-reads silver's R14 output as valid AC1014; the
three gold-tree specimens measure 89/153/149 diffs (the opening
census), the parked-era scope guard in `run_corpus.py` is retired,
and the corpus admits the era dirs, the root examples, and the
41-file authored wave — the campaign's work surface is A9's queue
plus the era-parity rows the opening census names. Tier 3 (pre-R13)
stays UNSUPPORTED in silver, and gold decodes nothing there either
beyond version identification — with C3 decided (2026-10-03) to
keep silver's outright reject, the tier is closed as a recorded
non-goal until the maintainer reopens it.

### C2. The R13/R14 parity campaign — ACCEPTED 2026-10-03

The campaign is open. The fidelity harness now runs on the era
corpus (the scope guard retired 2026-10-03): the three gold-tree r14
files (Constraints/Leader/v), example_r13/example_r14 at the gold
root, and the 41-file authored wave under `fixtures/r13_r14/`. The
work order mirrors the landed H8 arc: record-count mismatches first
(the entities/tables desyncs from gold's R14 walk), then the
era-field projections (`isbylayerlt`, `linewt`, `plotstyle_flags`,
`ltype_flags` — R2000-era fields silver carries where gold's R13/R14
spec blocks lack them). **The opening census (2026-10-03, the first
694-file run): the era population measures 103 rows** — example_r13
78, example_r14 20, the gold r14 Leader 4, the wave's Leader_r14 1 —
**with the other forty wave files and the gold r14 Constraints/v at
0/0** (the era-census continuation's R13/R14 gates had already
absorbed most of the previously recorded 89/153/149). The campaign's
work surface is therefore example_r13/example_r14's real-world rows
plus the four-file residue — the A9 queue's era entries.

### C3. The pre-R13 behavior — DECIDED 2026-10-03: keep the outright reject

Silver's `UnsupportedVersion` reject stays. The version-code-then-
empty-decode mirroring (the former A4) is dead until further notice.
One fixture-side fact stays on record: AutoCAD 2027 opens the
r13/r14 corpus files but refuses r1.4 and cannot author below R14,
so pre-R13 specimens have no AutoCAD route at all — that tier's
specimens stay libredwg-sourced should the decision ever reopen.

## D. Standing guardrails (re-verified periodically; not tasks)

1. Full loader probes after any constructed-content change — README
   step 7b (`loaders/strict_load_probe.py`); the AutoCAD census is the
   acceptance signal.
2. Era censuses on the per-era constraints specimens after any
   writer-form change — `analysis/record_size_census.py` (the L4 gate
   in the zero-keeping workflow).
3. The generation identity canary after any writer change — `cargo
   run --example gen_all_entities_all_versions_dwg --features serde`
   plus `md5sum`; record the new value at the next halt (a tracked
   fact, not a frozen constant).
4. The genus pin refresh discipline — never hand-edit
   `config/genus_expectations.json`; regenerate with
   `genus/genus_extract.py` and review the drift (the `genus_gates`
   cargo mirror asserts fresh equals pin).
