# TODO.md — gold_harness open work

The distilled work surface for the gold-vs-silver harness, extracted
2026-09-30 from `IMPLEMENTATION.md`, re-revised 2026-10-01 after the
B1/B2 sessions and again 2026-10-02 through the day's arc (the
author-quads landing, the AutoCAD-liberation investigation and its
halt, the SweepSurf specimens + the A8 typed-NURB3D closure, the B5
dead-rows closure, the R13/R14 authoring-wave survey — the campaign
record now 9,078 lines) and refreshed 2026-10-03 (the era-census
packets: 2010 at full record identity, gh109_1/HatchG closed, the
A9 item distilled). Sources cited per item. The closed campaigns hold their zeros — parser parity, the
strict-load zero, the ACS/SH solid-history campaign, the §20 genus-gate
queue at PENDING-ZERO with `--strict`, every constructed fixture MODELS
with clean audits on both loaders, the generation identity
`f2187565…`/25,728 stable, the suite 1,618/0. The corpus is 401 files
(280 + the B1/B2 landings + the 2026-10-02 author quads + the eight
SweepSurf specimens): the forty
B2 fixtures, the sixteen author quads and the eight SweepSurf files
read 0/0 (the 401-file full run measured 2026-10-02: totals flat at
the pre-existing 8/8, zero per-file failures); the
**eight measured open rows are pre-existing B1-era residue** (verified
pre-existing — the LIGHT row bisected on the pre-B2 tree, the
UNKNOWN_OBJ rows B1's own recorded residue; see H8h-ext-16's
validation record): LIGHT.light_color ×5 on `example_2004…2018` +
UNKNOWN_OBJ common-fields ×3 on Wipeout_2004/gh44 — see A5. The genus
cargo pin assertion is red pre-existing too (A7).

Sections: **A. Open tasks** (agent code work, no fixtures needed) →
**B. Fixture-needed tasks** (require authored DWGs) → **C. Maintainer
decisions** (scope calls, not packets) → **D. Standing guardrails**
(re-verified periodically, not tasks).

---

## A. Open tasks — agent code work

### A4. AC1009-era behavior mirroring (if Tier 3 is accepted — see C3)

If the maintainer accepts the version-code-then-empty-decode surface for
pre-R13 files, the work is small: silver currently rejects pre-R13 DWG
versions at the version-map stage (`src/io/dwg/dwg_version.rs:49` returns
`UnsupportedVersion` for `AC1009` and older); gold *identifies* the version
codes and decodes zero entities (prints "This file's version code is: X",
SUCCESS with no content). Mirroring = report the version code and return an
empty document instead of the error. Blocked on the C3 decision.

### A5. The corpus's eight pre-existing rows (B1-era residue)

The only measured fidelity rows on the 377-file corpus, verified
pre-existing (the LIGHT row bisected — stashing `src/` alone reproduces
it on the pre-B2 tree; the UNKNOWN_OBJ rows are the B1 session's own
recorded residue):
**LIGHT.light_color ×5** — `example_2004/2007/2010/2013/2018`
read gold 5 vs silver null (the B1 raw-CMC twin family's leftover);
**UNKNOWN_OBJ ×3** on Wipeout_2004 + gh44 (the common-data convention
rows B1 left recorded: `is_xdic_missing`/`ownerhandle`/`reactors`).
A read-side projection packet for the LIGHT twin's `light_color` and a
convention look at the two UNKNOWN_OBJ rows. **Sources**: H8h-ext-16's
validation record; the B1 session records (the LIGHT raw-CMC twin).

### A6. The `--no-lz77` DataStore re-emission blocker

The conventional-write arm
(`DwgWriter::write_to_file_no_lz77`) dies on every 2013+ authored
file at the `AcDb:AcDsPrototype_1b` section
(`InvalidFormat("Unknown AC21 section: AcDb:AcDsPrototype_1b")` —
pre-existing, reproduced on the pre-B2 tree). Until it lands, the
layer-4 conventional-arm validation rides the 2007/2010 frames only;
the 2013/2018 typed emissions are validated by the unit-level bit
arithmetic (`tests/edge_action_param_subcurve.rs` is the working
example). A writer packet: skip-or-echo the DataStore section the way
the default path does. **Sources**: H8h-ext-16's validation record
(the no-lz77 note); the NEXT_SESSION 2026-10-01 addendum.

### A7. The genus cargo pin regeneration (§20.3 review step)

The `genus_gates` cargo mirror asserts fresh == pin and is red
pre-existing (bisect-verified; the python `--strict` gates stay
PENDING-ZERO — the TOLERATED rows unchanged). Per §20.3: regenerate
`config/genus_expectations.json` with `genus/genus_extract.py` and
**review the drift** — the seven-packet 2026-09-30 session chain that
last moved the generation identity is the likely cause. The
regeneration is mechanical; the drift review is the maintainer's
recorded verdict. **Sources**: `README.md` D-guardrail 4; §20.3.

### A8. The composite (47) named-fields dissection — the 42 half CLOSED
(2026-10-02, the typed Nurb3d model)

The NURB3D (42) dissection is DONE — the SweepSurfSpline quad (the
2026-10-02 surface-mode sweep path) was the differential that cracked
the form: the grammar is fully named and self-delimiting (see
`AssocNurb3dSubcurve` and the H8h-ext-16 A8 record), the reader parses
it typed (gated on the measured constants + exact closure, falling
back to the capture+replay net on any deviation), the writer emits it
era-ungated (the regions are bit-identical across 2007/2018 on every
specimen), and the bit-exact pins + the 220/220 conventional-arm
census hold it. What remains of this item: the **47 composite** — a
delta-encoded polyline/composite (the ExtrudePline/Extrude3DPoly/
RevolvePline/LoftMixed quads + the 2004/Surface.dwg records carry
it) — still replays verbatim; naming its fields is the remaining
depth packet. Optional enablers: a RATIONAL spline (weights ≠ 1) and
a CV-method spline would stretch the 42 weight fields beyond the
measured set (the current parser's constant gates would fall back to
the net — safe, and the specimen would name the new fields).
**Sources**: H8h-ext-16 (the 42 dissection record + the A8 closure);
fixtures/sh_history's ten B2 quads + the SweepSurf quads.

### A9. The era-census residue (the older eras' record-identity rows)

The conventional-arm record-identity censuses (`analysis/record_size_census.py`,
`DWG_NO_ECHO=1 target/debug/dwgrewrite` staging) — refreshed 2026-10-03 through
the "shrink TODO.md" EIGHT-packet session: **the 2000-era boilerplate family
COLLAPSED (PolyLine2D 225→2, entities-2d 49→8, entities-3d 49→7 — the TV
plain-form, the XRECORD wire codepage, the exact close-pad, and the dictionary
wire-text packets; the remaining rows are ALL the class? desync-mirror family
plus PolyLine2D's last 2 POLYLINE_2D crc rows)**; **the golden corpus collapsed
5825→122→32 fidelity rows** (the R13/R14 gates + the vertex chain + the
isbylayerlt retention + the per-family R13 gates: Point/Text/Viewport/Leader/
Face3d/Polyline2D/Polyline3D/Polyface/Insert/LWPolyline ALL 0/0 at R13/R14);
the maintainer's brep drop (the B4 ACSH_BREP_CLASS dataset, 6 re-authored
ATMOS twins) measured far below its README-recorded residue (AC1014/AC1015
0/0 — the session's TV-form/close-pad/dictionary packets absorbed the
era-modeler text class; AC1027 6, AC1032 7 remain). THE STANDING ERA FACTS
ALL HOLD (re-verified): gh209_1 166/166, gh109_1 664/664, HatchG 229/229,
example_2000 750/750, example_2004 735/735, example_2010 536/536, example_2018
474/474, Constraints_2010 216/216, Constraints_2013 160/160. THE REMAINING
ROWS, ranked: (a) **the R2010+ SEQEND plotstyle HANDLE** (1 row per R2004+
golden polyline — the seqend_flags capture extension); (b) **the ACIS
era-modeler text class** (Region/Solid3d_AC1012/1014's acis_data + the brep
residue's remaining rows — the Brep dataset is the per-version specimen set
for that packet); (c) the STRUCT-axis key-gaps (2631 — the R13/R14 golden
key-shape class); (d) the class? desync-mirror rows (entities-2d 8 +
entities-3d 7 — the B1-era mojibake family); (e) PolyLine2D's last 2;
(f) **Surface.dwg 6** (the §18 surface family) and **gh44-error 18** (the
pathological file: LEADER 6 — the 2-bit handle-stream slack; HATCH 5; type-57
5; DIMASSOC 2); (g) Leader_AC1014's last row. **Sources**: the 2026-10-03
era-census continuation (NEXT_SESSION.md, the file bottom); the corpus
report.json (the per-file rows).

---

## B. Fixture-needed tasks

Every fixture follows the conventions in `fixtures/README.md`: fresh
drawing, default template, layer 0, exactly ONE operation; SAVEAS per
target version; qualify with `dwgread -O JSON` (zero `Error` lines, target
class present, minimal object census); land with a sibling `.txt`
provenance companion. Either AutoCAD or BricsCAD may author (record which).

### B3. The R13/R14 specimen wave (the Tier 2 campaign enabler — see C2)

The R13/R14 parity campaign's specimen set is thin: three real-world
r14 DWGs (`r14/Constraints.dwg`, `r14/Leader.dwg`, `r14/v.dwg` — all
AC1014) measure 89/153/149 read+write fidelity diffs (attribution-
hard); `r13/` holds only `v.dxf` (no R13 DWG in the corpus at all);
`example_r13.dwg`/`example_r14.dwg` sit at gold's tree root, outside
the corpus dirs. **Toolchain measured 2026-10-02 (the maintainer's
report): AutoCAD saves DWG R14+; BricsCAD saves R13+ — both halves of
the enabler are authorable on demand** (BricsCAD is the R13 author;
either app for R14; record which per the fixtures README). The
corpus-driver's fixtures walk now carries the parked-campaign guard
(the r13/r14 substring, mirroring the gold-tree guard) so the wave
lands safely without touching the R2000-R2018 totals.

**The required-fixture list (the 2026-10-02 survey of gold's
2000-2018 backbone vs the era dirs; one op per file, minimal census,
`Stem_r13.dwg`/`Stem_r14.dwg`, dropped in
`fixtures/r13_r14/` — PARKED until C2 accepts the era extension):**

Wave 1 — the era-certain backbone (16 families × 2 eras = 32 files):
`Line`, `Circle`, `Arc`, `Point`, `Text`, `Polyline` (the heavy 2D
polyline — the R13/R14-native form), `PolyLine3D`, `Polygon`,
`Donut`, `Ellipse` (new in R13), `Spline` (new in R13), `Multiline`
(new in R13), `ConstructionLine` (XLINE, new in R13), `RAY` (new in
R13), `Dimension` (one linear), `Block` (one insert).

Wave 2 — the attempt set (era-introduction uncertain; a refusal is
recorded as data in the `.txt`, not a failure): `Leader` (R14-proven
by gold's own r14/Leader.dwg), `LWPolyline` (R13-or-R14
introduction), `MText` (R13+), `Hatch` (R14+), `Tolerance` (R13+).

NOT era-valid — do not author: `Helix` (2007+), the parametric
constraint manager (2010+; gold's same-named 2000-era file is an
early line+circle assoc demo, census-verified 2026-10-02), the whole
`sh_history`/ACSH surface family (2007+). Pre-R13 (r1.4-r12) stays
unauthorable by both tools — that tier's specimens remain
libredwg-sourced (see C3/A4).

### B4. ACSH_BREP_CLASS specimen — FALSIFIED 2026-10-03 (the from-scratch mint works; the dataset LANDED in `fixtures/brep/`)

**The "external-only" verdict is falsified by measurement**: the class
IS authorable from scratch, headless, in one script. The recipe (the
2026-10-03 mint discovery): `(setvar "SOLIDHIST" 1)` — THE switch; the
profile default 0 strips boolean history, and the D7 control proved
that default was the confound behind the original seven failures (with
0, even a plain two-box union saves as a bare `ACSH_HISTORY_CLASS`
root) — then a boolean UNION of a parametric BOX with any
raw-geometry solid (two doors measured: `MESH` → `CONVTOSOLID`, and
`ACISIN` of a self-authored SAT). The framework records the chain and
the raw operand's node IS `ACSH_BREP_CLASS`.

**The dataset** (`fixtures/brep/`, fully self-authored from a naked
`acadiso.dwt` seed — no legacy content in any chain): 6 versions minted
(R14/2000/2007/2010 at **0/0** silver pairs; 2013/2018 at 6/7 rows —
one coherent family: the BREP record's R2013+ trailing region
`materials`/`has_revision_guid`/`revision_*`/`end_marker`, the named
next dissection packet, the B2/A8 pattern). Every file: one `3DSOLID`
+ the exact four-node chain `ACSH_BOX_CLASS`+`ACSH_BREP_CLASS`+
`ACSH_BOOLEAN_CLASS`+`ACSH_HISTORY_CLASS`. The derailed `major` field
is container-coded (64 at R14/2000, 3528495168 at 2007–2013,
3545534528 at 2018 — double-sourced against the carrier re-emissions).

**The measured refusals** (recorded per the doctrine): 2004 drops the
whole SH object family on the downsave (a container property, same on
fresh content as on the carrier); R13 is closed on BOTH engines
(AutoCAD's save floor is R14; BricsCAD writes R13 and its
CONVTOSOLID even accepts polyface meshes where AutoCAD refuses them,
but its boolean modeler records NO operand history at all — a bare
HISTORY root at R13 and 2018 alike, no retention switch exists).

- **Sources**: the 2026-10-03 attempt + mint sessions (the recipe, the
  D7 control, the door measurements); `fixtures/brep/README.md` (the
  retention map + the recipe); `IMPLEMENTATION.md` §F2.3 Brep row
  (line 3365), §19.5 Tier 1 (line 7260).

### B5. Dead rows — CLOSED 2026-10-02 (recorded-dead; all three lever
routes measured)

Both rows are terminal as recorded-dead; the naming they point at ran
to its evidence limit:

- **LoftD** (LOFT Settings → Ruled + draft fields): the authoring path
  does not exist in this AutoCAD release (maintainer-verified
  2026-09-24). The §18 container walk (2026-09-26) named the
  per-section `[center.x][center.y][height][radius]` runs and the
  trailing `[π/2, π/2]` draft pair provisionally
  (`SolidHistoryLoftTail`); the leading region, the inter-section gaps
  and the 14-bit trailer stay documented-verbatim (the conservative
  decode-fidelity rule).
- **The SH revolve option shorts + flags**: the grammar is CLOSED
  bit-exactly (§18.6); the six option shorts + two flags stay
  positionally named (`option_doubles`) — every corpus specimen
  carries zeros. The three lever routes are ALL measured dead:
  (1) the REVOLVE command exposes no draft/twist prompt in this
  release; (2) the GUI Properties surface for a revolved surface
  carries only the angle of revolution (maintainer-verified
  2026-10-02 — no draft/twist fields); (3) the ASSOC body's named
  pab-values carry only `RevolveAngle` (2007/2010; none at 2013+) —
  no semantic anchor for any option slot.

Re-opens only if a specimen with non-default values surfaces (a
real-world file, a future release's settings path, a different
authoring app) — the raw-retention/verbatim nets keep any such file
safe from day one.

- **Sources**: `IMPLEMENTATION.md` §18.6/§18.7 (the closure records)
  + the B5 consult record (2026-10-02, the ASSOC-body probe + the
  maintainer's GUI confirmation).

### B6. The unattested subcurve kinds 19 (Line) / 27 (Curve3d) — blocked
on specimen discovery

The B2 census closed the reachable ladder: 11/17/23/42/47 are modeled
(typed) or netted (verbatim capture+replay on same-version writes).
**19 and 27 exist in no corpus file and no authoring lever the B2
campaign traversed** — the measured profile-kind map: circle→11,
ellipse→17, line→23, closed/open spline→42, helix→42, 2D/3D pline→47,
LOFT→11/47, REVOLVE→47, POLYSOLID→no EdgeActionParam records at all,
and **SWEEP is unauthorable headless** (accoreconsole's path prompt
cannot be driven — the selection dialect has no view aperture). The
GUI sweep route was measured 2026-10-02: the maintainer-authored
**SweepHelix quad lands as a fixture and carries ZERO
EdgeActionParam records** (solid ACSH_SWEEP_CLASS genus — the sweep
family does not produce them in solid mode any more than POLYSOLID
does), removing the GUI-sweep from the candidate routes. The
**surface-mode sweep was measured 2026-10-02 (the SweepSurfLine /
SweepSurfSpline quads)**: the surface network DOES carry subcurve
records (two per drawing — unlike the solid-mode zero) — but the
path's kind MIRRORS THE PATH ENTITY'S OWN CURVE KIND (line path →
23, spline path → 42; the circle profile → 11 in both), so 19/27
remain unattested and the surface-mode sweep route closes too. The
kind map now reads: the edge param's kind = the source entity's
curve kind (circle→11, ellipse→17, line→23, spline→42, pline→47),
stable across every measured era (all four versions of every quad).
Any future specimen rides the capture+replay net
from day one, so the modeling packet is pure upside when one surfaces.
**The 2026-10-03 b6_routes review measured every remaining route
dead**: the constraint-network probes (DimConstr — the dimensional
DCLINEAR/DCRADIUS network; GConstrNet — tangent/parallel; ConstrSmooth
— GCSMOOTH between two 2D splines, the strongest kind-27 candidate;
GConstrXline — an XLINE in a GCPARALLEL/GCPERPENDICULAR network, the
kind-19 candidate) ALL produce **zero EdgeActionParam records** —
their ASSOC class lists carry only NETWORK/2DCONSTRAINTGROUP/
GEOMDEPENDENCY (+ the dimensional family's DIMDEPENDENCYBODY/
VALUEDEPENDENCY/VARIABLE); the infinite-line routes are REFUSED by
geometry (SweepSurfXline/Ray: "This entity cannot be a sweeping
path") and the 3D-curve constraint route by the manager's own domain
(ConstrHelix: the picker accepts "line/straight polyline segment/arc/
curved polyline segment" only). **19/27 remain attested nowhere;
B6 is the pure watch** — no authoring route exists on this toolchain;
the one weak remaining candidate is an era conversion of an existing
carrier (the kind is era-stable in every measured quad). Sources: the
b6_routes companions (the refusal records verbatim); the 2026-10-03
b6 review (NEXT_SESSION.md's era-census addendum, the b6 paragraph).

**The concrete specimen recipes (the 2026-10-02 consult; author at
2010/2013/2018 — the constraint-manager eras — or 2007+ for the
sweep route; drop in `fixtures/b6_routes/`, IN-SCOPE, one op per
file, qualify per §F2.1; a refusal is recorded as data, not a
failure):**

1. `SweepSurfXline_<v>` — SWEEP with MOde=Surface, a CIRCLE profile,
   an XLINE (construction line) as the path. The kind-19 hypothesis:
   the bounded LINE path gave 23 (LineSeg3d) — the INFINITE line may
   give 19 (Line). If the app refuses an infinite path, record the
   refusal text verbatim in the `.txt` (the route's negative datum).
2. `DimConstr_<v>` — DIMCONSTRAINT: a linear constraint on a LINE
   plus a radius constraint on a CIRCLE (the dimensional network,
   distinct from gold's plain geometric-constraint files, which the
   B2 census measured negative).
3. `GConstrNet_<v>` — GCONSTRAIN: tangent + parallel constraints
   between a LINE and a CIRCLE (weak — gold's plain files carry no
   EdgeActionParam; the network variant is the untested lever).

- **Sources**: `IMPLEMENTATION.md` H8h-ext-16 (the full-corpus census +
  the authoring-lever negative map) + its 2026-10-02 review notes (the
  SweepHelix solid-mode + SweepSurfLine/SweepSurfSpline surface-mode
  sweep data); the ten B2 quads and the six author quads in
  `fixtures/sh_history/` (the negative-evidence levers are landed).

---

## C. Maintainer decisions (scope calls, not packets)

### C1. The §19.5 version-parity matrix — accept the current tier state?

- **Tier 1 (R2000–R2018)**: AT PARITY — the campaign's declared scope,
  complete (record identity on all surveyed surfaces). The corpus has
  since grown 280 → 377 files (the B1/B2 fixture landings); the eight
  measured open rows are B1-era residue on pre-existing files, verified
  and recorded (see A5) — not Tier-1 campaign regressions.
- **Tier 2 (R13/R14)**: IMPLEMENTED, NOT AT PARITY — silver reads/writes
  them (gold re-reads our R14 output as valid AC1014); the three specimens
  measure 89/153/149 diffs (record-count mismatches + era-field
  divergence). Currently excluded by a recorded scope guard in
  `run_corpus.py::in_scope_files`.
- **Tier 3 (pre-R13)**: UNSUPPORTED in silver; gold decodes nothing there
  either (version identification only).

### C2. Accept an R13/R14 parity campaign?

Mirrors the landed H8 arc: the fidelity harness already runs on the three
files (the 89–153 rows are the opening census); record-count mismatches
(the entities/tables desyncs from gold's R14 walk — DIMSTYLE 28 extra vs
10 missing, IMAGE 15 vs 5) come first, then the era-field projections
(`isbylayerlt`, `linewt`, `plotstyle_flags`, `ltype_flags` — R2000-era
fields silver carries where gold's R14 spec block lacks them). Fixture
enabler: B3 — **COMPLETE as of 2026-10-02 (both eras authorable:
BricsCAD R13+, AutoCAD R14+); the specimen wave is accumulating
PARKED under `fixtures/r13_r14/` (the corpus driver's parked-campaign
guard keeps it out of the R2000-R2018 totals until this decision
flips the scope)**. **Sources**: §19.5 Tier 2 (lines 7266–7284).

### C3. Accept the pre-R13 mirror (A4) or keep the outright reject?

Silver's `UnsupportedVersion` reject is a defensible equivalent of gold's
identify-only behavior; the version-code-then-empty-decode surface mirrors
gold more literally but is a format-family addition, not a fidelity fix.
**Sources**: §19.5 Tier 3 (lines 7286–7301). One 2026-10-02 measurement
bears on the fixture side of this tier: AutoCAD 2027 opens gold's r13/r14
corpus files but refuses r1.4 ("cannot read old format drawing",
ErrorStatus=53) and cannot author below R14 — pre-R13 fixtures have no
AutoCAD route at all; that tier's specimens stay libredwg-sourced.

### C4. The AutoCAD-liberation question — REOPENED 2026-10-03 (the golden-entities campaign LANDED)

The 2026-10-02 investigation (the record: IMPLEMENTATION.md's afternoon
note) measured AutoCAD 2027 as a fixture engine: authoring R14→2018 (the
SAVEAS menu; no R13/pre-R13), opening r2.10/r10/r11/r13/r14 but not r1.4
(ErrorStatus=53), prompt-free headless generation via `entmake`
(probe-validated), and the MCP ecosystem as wrappers over those same
channels (no new primitive; nothing wire-level). The 2026-10-02 halt
("continue on the libredwg test-files") was REVERSED by the
2026-10-03 maintainer call: "generate per version supported in cadcodec
all supported entities separately to replace libredwg test-files and
beyond". **THE CAMPAIGN LANDED THE SAME DAY** (the maintainer's
authoring session): `fixtures/golden_entities/` — 30 families × 8
versions (AC1012..AC1032), **221 fixtures + 8 Body refusals + 10
version-gated cells**, every `.dwg` with its `.scr` replication record
and `.txt` provenance (engine, seed, op, save token, qualification,
gold census); the engines per the measured map (R13 = BricsCAD —
AutoCAD's save floor is R14; R14+ = AutoCAD core console); the
version gates measured in-campaign (HATCH is R14+ — the R13 downsave
explodes it; MULTILEADER is AC1021+; MESH is AC1024+); the LWPolyline
R13 downsave datum (the light polyline converts to the era-native
heavy POLYLINE_2D — the B3 two-class datum); the BODY refusals
consistent on both engines (INTERFERE non-functional headless —
the B4 no-user-facing-lever class).

**THE REVIEW VERDICT (the 642-file corpus run, 2026-10-03)**: the
established 401 UNMOVED at 8/8; the b6 population all twenty 0/0
(including both arc families — the BricsCAD A/B SweepSurfArcB with
its gold-UNKNOWN_ENT surface reads clean on the differ axes); **the
golden population: SIX ERAS AT 0/0** (AC1015/1018/1021/1024/1027/
1032 — every family except the polyline pair; the maintainer's
authored corpus validates the codec per-era at zero) — **the R13/R14
eras carry the ~104-row shared boilerplate family on EVERY family**
(the §19.5 Tier-2 state, uniform across all 52 AC1012/AC1014 files —
one shared root class, not per-entity divergence; the outliers:
Dimension 161, Polyface 131, Insert 125, Polyline2D 120/123,
LWPolyline-R13 123, Viewport 121, Leader 112/111, Text 113) **plus
the polyline family's cross-era rows** (Polyline2D 3→7 by era,
Polyline3D 1 on 2007+, Polyface-2000 3 — the known 2000-era class
at the minimal-file scale). The new corpus baseline: **642 files,
5825/5825 fidelity + 2581 read key-gaps** — the totals now carry the
C4 work surface; the established-zero rule holds per-population.

**THE C4 WORK SURFACE (the campaign's queue, ranked)**: (a) **the
R13/R14 boilerplate family** — the ~104-row uniform class across all
52 golden R13/R14 files (one root, the 2000-era boilerplate family's
dissection method applies: the per-record pair diff names the class;
§19.5's Tier-2 "implemented-but-divergent" gets its first named
root); (b) the polyline family's minimal-file rows (the known
Polyline2D class — the golden files give it a per-era minimal
specimen set); (c) **C2's era-extension decision** (the parked
r13_r14 B3 wave's corpus admission — now the same question as the
golden R13/R14 population: the corpus already carries the golden
files in-scope by the maintainer's intent, so C2 reduces to whether
the r13_r14 wave adds coverage beyond the golden set).

- **Sources**: the golden_entities README (the matrix, the engines,
  the gates, the corpus note — the stems are in-scope by design);
  the 2026-10-03 landing record (NEXT_SESSION.md's era-census
  addendum, the golden-entities section); the corpus report.json
  (the per-file rows).

---

## D. Standing guardrails (re-verified periodically; not tasks)

These are covered-state surfaces that decay silently if never re-run:

1. **Full loader probes after any constructed-content change** — README
   step 7b (`loaders/strict_load_probe.py`); the AutoCAD census is the
   acceptance signal (docs/ARCHITECTURE.md §4).
2. **Era censuses on the per-era constraints specimens** after any
   writer-form change — `analysis/record_size_census.py` (the L4 gate in
   the zero-keeping workflow).
3. **The generation identity canary** after any writer change —
   `cargo run --example gen_all_entities_all_versions_dwg --features serde`
   + `md5sum`; record the new value at the next halt (a tracked fact, not
   a frozen constant).
4. **The genus pin refresh discipline** — never hand-edit
   `config/genus_expectations.json`; regenerate with
   `genus/genus_extract.py` and review the drift (the `genus_gates` cargo
   mirror asserts fresh == pin).
