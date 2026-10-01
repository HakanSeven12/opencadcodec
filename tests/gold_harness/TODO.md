# TODO.md — gold_harness open work

The distilled work surface for the gold-vs-silver harness, extracted
2026-09-30 from `IMPLEMENTATION.md` (the 8,144-line campaign record) and
cross-checked against the code. Sources cited per item. Nothing here is a
regression: all measured campaigns are closed at zero — corpus 280 files
0/0/0/0, genus gates PENDING-ZERO with `--strict`, every constructed
fixture MODELS with clean audits on both loaders, record-identity censuses
all-zero on every measured specimen.

Sections: **A. Open tasks** (agent code work, no fixtures needed) →
**B. Fixture-needed tasks** (require authored DWGs) → **C. Maintainer
decisions** (scope calls, not packets) → **D. Standing guardrails**
(re-verified periodically, not tasks).

---

## A. Open tasks — agent code work

### A2. MT crc_seed draw pinning (the one open algorithmic unknown)

The R2007_Header CRC random encoding: some header CRCs are stored as
UInt64s with 10 data bits spread evenly + 54 pseudo-random bits. The engine
(ODA §5 pseudocode) is implemented in-tree
(`src/io/dwg/file_headers/file_header_ac21.rs::CrcRandomEncoder` ~line 95,
with the exact constants), but **the generated sequence diverges from the
author's draws at the same seed+crc_seed — the first draw already differs**.

**INVESTIGATED 2026-09-30 (this session) — ground truth extracted and three
variants eliminated.** From `example_2007.dwg`'s `R2007_Header`:
`random_seed = 0x24cc9552adddcbdb`, `crc_seed = 0x0`, and the author's
recorded draws `sections_map_crc_seed = 0xa4825533cf3d200d`,
`pages_map_crc_seed = 0x06d124375b400495`,
`crc_seed_encoded = 0x719670845579d41b`. A faithful Python port of silver's
`CrcRandomEncoder` (LCG init on both 32-bit seed halves, MT-style recurrence
from `table[1]`, 0x80-entry padding consumption, no tempering, 10-bit
spread) reproduces silver's draws exactly — and **all three diverge from the
author's** (`534b…` vs `a482…` on the first). Two further init variants
(pure MT19937 `t[i]=f(t[i-1])+i` from `seed_lo`; LCG-01 then MT chain from
`t[1]`) also diverge. So the author's generator is none of: silver's
LCG+MT hybrid, standard MT19937-lo, or the LCG01-chain. The variant space
still open: MT tempering applied on read (silver reads raw table values),
a different twist offset (`0x18D`) or period (`0x270` vs `0x270`/`624`),
little-endian vs big-endian seed-byte split, and a different padding/index
walk.

- **Closes**: the 3-per-file R2007_Header derive family
  (`sections_map_crc_seed`, `pages_map_crc_seed`, `crc_seed_encoded`) —
  the author's values land for free once the draws match.
- **Ground truth**: every corpus R2007+ file supplies (seed, author's
  draws) on disk — **no fixture needed**. The extraction + variant harness
  is reproducible: read `R2007_Header` via gold `-O JSON`, port the encoder,
  diff the three draws.
- **Method**: widen the variant enumeration (tempering on/off, twist
  offset, index/wrap order, seed split) against `example_2007`'s three
  draws; the spec's §5.11 tables follow the engine exactly, so the
  discrepancy is in the variant, not the pseudocode. When a variant matches
  all three draws on `example_2007`, confirm on a second R2007 file before
  landing.
- **Sources**: `IMPLEMENTATION.md` §19.4.G (line 7188), the H8h-ext
  residual notes (lines 5422–5428, 5999–6000, 7257–7258); this session's
  variant-elimination evidence above.
- **Fixture**: none.

### A3. Full era-corpus record-identity censuses — **LANDED 2026-09-30 (this session)**

The era censuses were previously one file per era (the per-era constraints
specimen). The new instrument `analysis/era_corpus_census.py` (batch
wrapper: per file, `DWG_NO_ECHO=1 dwgrewrite` staging + the handle-keyed
size+CRC-16 census) ran the **full era corpora** — six eras, 117 files,
82,413 paired records. Result: **2007 fully clean (0/3,979)**;
2004/2010/2013 near-clean (15/2/24); 2000 at 375 (PolyLine2D-led); **2018
at 2,479 — the systematic wire divergence that is A1**. The census turned
A1 from "one specimen's residue" into "the 2018-era conventional-arm wire
divergence," and it is now the standing L4 gate for the full era corpora
(the honest next surface §19.5 named). Run:

```
python3 tests/gold_harness/analysis/era_corpus_census.py \
  ~/work/libredwg/test/test-data/{2000,2004,2007,2010,2013,2018}
```

(GOLD_DWGREAD must point at the oracle; workdirs key by era+stem to avoid
the cross-era stem collision that mispaired the first run's `circle.dwg`.)

- **Sources**: §19.5 Tier 1 (the "honest next surface" note, lines
  7261–7264); the census table now lives under A1.
- **Fixture**: none — uses the existing corpus.

### A4. AC1009-era behavior mirroring (if Tier 3 is accepted — see C3)

If the maintainer accepts the version-code-then-empty-decode surface for
pre-R13 files, the work is small: silver currently rejects pre-R13 DWG
versions at the version-map stage (`src/io/dwg/dwg_version.rs:49` returns
`UnsupportedVersion` for `AC1009` and older); gold *identifies* the version
codes and decodes zero entities (prints "This file's version code is: X",
SUCCESS with no content). Mirroring = report the version code and return an
empty document instead of the error. Blocked on the C3 decision.

### A5. The 2018-era residual families (20 census rows, five packets) — **A1 CLOSED 2026-10-01; these are the named remainders**

A1 (the 2018-era conventional-arm wire divergence) landed 2026-10-01 as
capture-and-replay of the two authorial genus conventions — the
ownerhandle code choice (the ODA FileConverter genus always writes
absolute code-4; the AutoCAD genus writes §19 H8d's
relative-iff-shorter pick) and the record-close pad (a PER-AUTHOR
genus: the ODA FileConverter 2018 set — and her R2000 PolyLine2D —
pad zeros; the AutoCAD genus pads ones; the reader samples per
record, the document carries the majority, the writer replays). The
era census moved 2018 **2,479 → 20 divergent** (the 17 ODA
specimens fully record-identical) and also repaired 50 pre-2004
rows (2000: 375 → 325 — the close-pad replay's share: with the pad
sampler disabled pre-2007, PolyLine2D sits at 259, so the
ownerhandle share there is 1 record and the pad share 34). The era
table now:

| era | files | paired | divergent | shape |
|---|---|---|---|---|
| 2000 | 22 | 4,824 | 325 | PolyLine2D 225, entities-2d 50, entities-3d 50 (the pre-existing her-only/our-only rows unchanged) |
| 2004 | 21 | 4,710 | 15 | HatchG 2, Surface 13 |
| 2007 | 18 | 3,979 | **0** | clean (ATMOS's 84 her-only = the standing broken-map residual) |
| 2010 | 18 | 3,758 | 1 | gh209_1 1 |
| 2013 | 19 | 14,369 | 24 | gh44-error 23, gh109_1 1 |
| 2018 | 19 | 50,773 | **20** | Dynblocks 14, LiveSection1 2, Constraints 3, Leader 1 |

The 2018 remainders, each root-caused to a named family (none is the
A1 handle/pad genus):

1. **Dynblocks 12 — the dynamic-block action records**:
   BLOCKSTRETCHACTION (8) / BLOCKMOVEACTION (2) / BLOCKSCALEACTION (2),
   each +58 main bits (hdlsize −2) — one repeated field family our
   writer emits longer.
2. **Dynblocks 2 — ACDBASSOC2DCONSTRAINTGROUP blowup**: 2948→29650
   bytes (10×) on this specimen's node shape — the per-node repeat
   defect resurfacing on an unmodeled variant.
3. **Constraints 3 — PROXY_OBJECT objid byte-shrink**: 973→971 and
   284→283×2 (hdlsize −8 bits) — one handle form in the proxy handle
   stream our raw path emits shorter.
4. **Leader 1 — LTYPE dash shape_flag**: record 0x779; our write adds
   IS_SHAPE(0x4) where her file has 0 (+16 bits).
5. **LiveSection1 2 — SECTIONOBJECT/SECTION_MANAGER**: equal-size
   1-bit divergences (byte 11 `C4` vs `C5`, mid-record).

- **Instruments**: `analysis/era_corpus_census.py` (the standing L4
  gate) + `analysis/record_size_census.py` per file; each family opens
  as a per-record dissection packet (the H8h-ext-4 BD-walk method).
- **Gate**: zero-keeping workflow; the 2018 era census to zero.
- **Sources**: `IMPLEMENTATION.md` §20 thirteenth-continuation record
  (the A1 closure, the measurements, and the residual autopsies).
- **Fixture**: none — all specimens already in the corpus.

---

## B. Fixture-needed tasks

Every fixture follows the conventions in `fixtures/README.md`: fresh
drawing, default template, layer 0, exactly ONE operation; SAVEAS per
target version; qualify with `dwgread -O JSON` (zero `Error` lines, target
class present, minimal object census); land with a sibling `.txt`
provenance companion. Either AutoCAD or BricsCAD may author (record which).

### B1. The coverage-gap entity family — the §F2 remainder

Entity types with **neither a gold corpus file nor an authored specimen**;
each needs one minimal file per natively-supported version:

| Entity | Versions to author | Notes |
|---|---|---|
| MESH | the versions where the MESH object natively persists (verify per save by grepping the class in the qualified JSON) | the SUBDIVIDE/MESHSMOOTH family; per the §F2 rule, author only the versions the qualified JSON confirms |
| POLYFACEMESH | the versions where POLYFACEMESH natively persists (verify per save) | legacy mesh; plain `PFACE` command |
| 3DFACE | the versions where 3DFACE natively persists (verify per save) | the entity_verification canonical's constructed 3DFACE reads as gold's raw carrier (the cosmetic name-map gap) — a specimen anchors the typed name |
| SOLID (2D) | R2000–R2018 (all six) | the 2D SOLID command (not 3DSOLID) |
| TOLERANCE | R2000–R2018 (all six) | the `TOLERANCE` command |
| WIPEOUT | R2000–R2018 (all six) | `WIPEOUT`; lands an IMAGEDEF reactor chain |
| XREF | R2000–R2018 (all six) | one attached external reference; the referenced file must accompany the fixture or resolve at qualification time |
| OLE2 | R2000–R2018 (all six) | `INSERTOBJ` with a simple embedded object |
| LIGHT | R2007, R2010, R2013, R2018 | point light; the LIGHT object family is R2007+ |
| CAMERA | R2007, R2010, R2013, R2018 | the `CAMERA` command; R2007+ |
| ARCDIMENSION | R2000–R2018 (all six) | `DIMARC` on an arc |

Landing each moves the type from "OUT OF LOOP" to covered automatically
(the corpus driver collects `fixtures/**/*.dwg`; the
`entity_verification.py` KINDS list and the gen_all generator gain the
kind when silver-side construction lands). **Version discipline**: per
§F2, author only the versions where the entity natively persists — SAVEAS
per target, then verify each save by grepping the class in the qualified
JSON; emit no fixture for a version that dropped it. (The per-version
columns above for SOLID/TOLERANCE/WIPEOUT/XREF/OLE2/ARCDIMENSION reflect
the common-case expectation from the campaign record; MESH /
POLYFACEMESH / 3DFACE carry the verify-per-save form because the §F2 list
names no versions for them and native persistence differs by era.)

- **Sources**: `IMPLEMENTATION.md` §F2 (lines 3257–3278).
- **Version scope rule**: only versions where the entity natively persists
  (verify by grepping the class in the qualified JSON).

### B2. The unattested subcurve action types (17 / 19 / 23 / 42 / 27)

The ACDBASSOCEDGEACTIONPARAM subcurve region is wire-known only for
`action_type 11` (ARC — twelve BDs: center/normal/x-axis/radius/
start_angle/end_angle, reverse-engineered 2026-09-27, H8h-ext-4). The five
raw action-type values 17 / 19 / 23 / 42 / 27 have **no corpus
specimens** — their kinds and wire forms are unattested, and the
reader/writer leave their regions raw (`AssocSubcurveKind` in
`src/objects/associative.rs:955` already carries the kind ladder —
None / Arc / Ellipse / Line / LineSegment3d / Nurb3d / Curve3d — from the
type-map source; the five unattested numeric values presumably map onto
the five non-ARC/non-None rungs, but which value is which kind is
unmeasured).

- **Work (fixture + code)**: author SH-history solids whose profile edges
  exercise each missing kind. The known lever from the campaign record:
  **mode/section SH specimen variants** (the POLYSOLID-path segment kinds
  — line vs arc segments, splined profiles, ellipse/3D-polyline profiles
  for EXTRUDE/LOFT/REVOLVE/SWEEP) produce different EDGEACTIONPARAM
  `action_type` values; the authored quad must be bit-identical (the §18.7
  differential discipline), and the `.txt` records the exact command
  options. Start with: SWEEP along an arc path (expects 17 or 23),
  EXTRUDE of a splined closed profile, REVOLVE of a polyline with an arc
  segment, LOFT across mixed line/arc sections.
- **Versions**: 2007/2010/2013/2018 each (the SH-history genus is 2007+).
- **Then**: the per-action_type region dissection (the H8h-ext-4 BD-walk
  method) names each wire form; the reader/writer model it as
  `AssocArcSubcurve` did for kind 11.
- **Sources**: `IMPLEMENTATION.md` §19.5 Tier 1 (line 7256), the
  H8h-ext-3/4 records (lines 6170–6219).
- **Code surface**: `src/objects/associative.rs` (`AssocSubcurveKind`,
  `AssocEdgeActionParam.subcurve`), the DWG reader/writer at
  `src/io/dwg/dwg_stream_readers/object_reader/associative.rs:871` and the
  writer sibling.

### B3. More r14 specimens (the Tier 2 campaign enabler — see C2)

The R13/R14 parity campaign's specimen set is thin: three DWG files
(`r14/Constraints.dwg`, `r14/Leader.dwg`, `r14/v.dwg` — all AC1014) measure
89/153/149 read+write fidelity diffs; `r13/` holds only `v.dxf` (no R13 DWG
in the corpus at all). If C2 is accepted, more r14 (and any obtainable r13)
fixtures would qualify per the §F2.1 gates — the same
minimal-one-operation discipline, SAVEAS to R14 (AC1014) and R13 (AC1012)
where the toolchain allows.

### B4. ACSH_BREP_CLASS specimen — external-only (blocked on discovery)

`ACSH_BREP_CLASS` is not reachable through any user-facing AutoCAD op —
seven authored attempts (plain op, SLICE, single-op, foreign-body graft,
SOLIDEDIT face edit, real-template source) all produced parametric chains
or history-stripped plain solids. The class stays elided; silver tolerates
the one known carrier (`ATMOS-DC22S.dwg` — a real-world working drawing,
held as a probe specimen under `target/`, not a corpus fixture) at 0/0.
**Re-opens only if an authentic specimen surfaces** — the noted path is
legacy SAT-era import routes (an old SAT file imported and saved with
history intact). Not authorable on demand; no action until a specimen
exists.

- **Sources**: `IMPLEMENTATION.md` §F2.3 Brep row (line 3365),
  §19.5 Tier 1 (line 7260).

### B5. Dead rows — record, no fixture path (for completeness)

- **LoftD** (LOFT Settings → Ruled + draft fields): the authoring path
  does not exist in this AutoCAD release (maintainer-verified
  2026-09-24). The π/2 draft pair's naming path is the loft container walk
  or the LoftM raw tail (code work), not a fixture.
- **The SH revolve option shorts + flags**: the typed anchor is already
  in-corpus via RevolveM; the remaining option short rows are code work
  (container walk), not fixtures.

---

## C. Maintainer decisions (scope calls, not packets)

### C1. The §19.5 version-parity matrix — accept the current tier state?

- **Tier 1 (R2000–R2018)**: AT PARITY — the campaign's declared scope,
  complete (280-file corpus 0/0/0/0; record identity on all surveyed
  surfaces).
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
enabler: B3. **Sources**: §19.5 Tier 2 (lines 7266–7284).

### C3. Accept the pre-R13 mirror (A4) or keep the outright reject?

Silver's `UnsupportedVersion` reject is a defensible equivalent of gold's
identify-only behavior; the version-code-then-empty-decode surface mirrors
gold more literally but is a format-family addition, not a fidelity fix.
**Sources**: §19.5 Tier 3 (lines 7286–7301).

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
