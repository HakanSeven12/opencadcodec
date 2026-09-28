# Gold-vs-Silver Roundtrip Harness — Unified Implementation Plan

Status: Phase 2–5 complete; `AcDbVisualStyle` blocker **fixed** (2026-09-17);
EntityCommon storage-only field gap **closed** (2026-09-17) — LINE/CIRCLE entity
diffs are zero on all six versions for read and write fidelity. Phase 6 fix loop
continues on table-record storage fields and object representation gaps.
Location: `tests/gold_harness/IMPLEMENTATION.md` — the single source of truth.

> This file is the only authoritative plan for the gold-vs-silver harness.
> Earlier superseded planning documents have been removed; this file is
> self-contained.

---

## 1. Goal

Close the read/write fidelity gap between **LibreDWG (gold oracle)** and
**cadcodec (`acadrust` crate, silver)** for a representative subset of the
LibreDWG `test/test-data` corpus, using a repeatable automated harness, then
drive an autonomous fix loop until the silver non-header output matches gold
across the covered corpus.

"Roundtrip" verifies **both** cadcodec's reader and writer: cadcodec reads a
gold DWG, re-encodes it, and the result is re-decoded — with LibreDWG acting as
the independent oracle on both the original and the rewritten file.

The header is tested **laxly** (out of strict-diff scope — only entity/object
non-header parts must match exactly). The loop is **bounded to the test-data
corpus** and must respect the **R2000–R2018 version scope**. **Phase 2
(planned 2026-09-25, §19): the laxly-tested header itself comes under a
second, separately-gated structure axis — the OBJECTS zero stays frozen
while the 17 structure keys get their own drive to 0, with a
whole-structure audit matrix proving every section is diffed or
excluded-with-reason.**

The immediate blocker is an AutoCAD `AcDbVisualStyle` "Object improperly read"
error on rewritten DWGs (see §7).

---

## 2. Confirmed decisions

| Decision | Choice |
|---|---|
| Comparison oracle | JSON field-level diff (libredwg `dwgread -O JSON` vs acadrust serde JSON) |
| Roundtrip meaning | Verify both cadcodec reader and writer; libredwg validates both ends |
| Environment | WSL — build libredwg from source; run cadcodec via `cargo` |
| Version scope | R2000–R2018 (AC1015, AC1018, AC1021, AC1024, AC1027, AC1032) |
| Silver JSON source | Reuse cadcodec's `serde` feature + a dump bin that re-includes serde-skipped `EntityCommon` fields under `_common_dwg[handle]` |
| Fix-loop coverage | Bounded to the test-data corpus; gap entities logged as out-of-loop |
| Loop convergence target | Zero non-header, non-ignored field diffs across the covered corpus |
| Version-scope guard | Gate each fix by the gold spec's version predicate; verify all 6 versions |
| Safety / regression gate | Per-fix checkpoint: `git commit` if available, else record in `report.md`; corpus non-regression + `cargo test` green |
| Convergence reachability | Curated `ignore_fields.toml` of gold-only derived/verbatim fields, **frozen** during the loop |

---

## 3. Architecture

```mermaid
flowchart LR
    ORIG["orig.dwg"]

    subgraph SGOLD["LibreDWG — gold (read-only oracle, WSL)"]
        DR1["dwgread -O JSON"]
        DR2["dwgread -O JSON"]
    end

    subgraph SSILVER["cadcodec / acadrust — silver"]
        R1["DwgReader.read()"]
        DOC[("CadDocument")]
        W["DwgWriter"]
        R2["DwgReader.read()"]
    end

    subgraph SHARNESS["gold_harness — normalize + diff_fields"]
        D1["1. read fidelity<br/>(gold_orig vs silver_orig)"]
        D2["2. write fidelity<br/>(gold_orig vs gold_rt)"]
        D3["3. internal consistency<br/>(silver_orig vs silver_rt — currently stubbed)"]
        REP["reports<br/>(per-file + corpus)"]
    end

    %% gold reads the original
    ORIG --> DR1 --> GO["gold_orig.json"]

    %% silver reads the original, dumps it, and rewrites it
    ORIG --> R1 --> DOC
    DOC -- "serde dump" --> SO["silver_orig.json"]
    DOC --> W --> RT["rt.dwg"]

    %% both sides read the rewrite
    RT --> DR2 --> GR["gold_rt.json"]
    RT --> R2 --> SR["silver_rt.json"]

    %% the three diffs per file
    GO --> D1
    SO --> D1
    GO --> D2
    GR --> D2
    SO --> D3
    SR --> D3

    D1 --> REP
    D2 --> REP
    D3 --> REP
```

Three diffs per file:

1. **Read fidelity:** `gold_orig.json` vs `silver_orig.json` → finds fields
   cadcodec's *reader* drops/misparses.
2. **Write fidelity:** `gold_orig.json` vs `gold_rt.json` (libredwg reads
   cadcodec's rewritten DWG) → finds fields cadcodec's *writer* omits/corrupts,
   validated by the gold reader.
3. **Internal consistency:** `silver_orig.json` vs `silver_rt.json` → cadcodec
   read→write→read self-check (complements `tests/roundtrip.rs` with real gold
   files). **Currently stubbed** to zero in `run_roundtrip.py`; wire up when
   needed.

### Alignment and handle comparison

- **Alignment**: records are paired by `(type, ordinal-within-type)`, not by
  raw handle, because handles are reassigned on rewrite.
- **Handle comparison**: handle-valued fields are compared by the resolved
  target type (`handle → record_type`), not by raw id. The differ builds a
  `handle_value → type` map on each side independently and substitutes the
  target type before comparing.
- **Strict mode**: `GOLD_HARNESS_STRICT=1` asserts zero `missing_in_silver`
  diffs.

#### Handle-design alternatives considered and rejected

- **Compare raw handle ids**: rejected — cadcodec intentionally reassigns
  handles on rewrite, producing false positives on every roundtrip.
- **Align 1:1 by silver `entity_index` / gold `handle`**: rejected — handles
  are not stable across read→write→read and silver's `entity_index` is keyed
  by the silver-assigned id.
- **Normalize handles to symbolic names ("layer 0", "model space")**: too
  brittle; the type-resolved approach is deterministic.
- **Ignore all handle-valued fields**: rejected — handle topology is part of
  the non-header structure the loop must preserve.

#### Known limitation

The handle resolver cannot distinguish two objects of the **same** type that
are both valid targets (e.g. a `LINE` pointing at layer handle 16 vs handle
99, both resolving to `LAYER`). Layer identity is semantically preserved, but
a swap to a different layer of the same type would be missed. A future
improvement could resolve handles to `(type, name)` when a name table is
available.

---

## 4. Scope boundary

Gold `dwgread -O JSON` emits one top-level object with keys (in order):
`created_by`, `FILEHEADER`, `HEADER`, `CLASSES` (R13+), `OBJECTS` (array),
`THUMBNAILIMAGE` (opt), and optional section records (`ObjFreeSpace`,
`SecondHeader`, `Template`, `AuxHeader`, `R2004_Header`/`R2007_Header`,
`SummaryInfo`, `VBAProject`, `AppInfo`, `AppInfoHistory`, `FileDepList`,
`Security`, `RevHistory`, `AcDs`).

- **IN scope:** the top-level **`OBJECTS` array only** (entities and non-entity
  objects). Compare per-object fields defined by `dwg.spec`, `dwg2.spec`,
  `common_entity_data.spec`, `common_entity_handle_data.spec`,
  `common_object_handle_data.spec`, and `acds.spec`.
- **OUT of scope (header; lax — dropped by the normalizer):** every other
  top-level key listed above.

---

## 5. Corpus coverage

Inventoried `test/test-data` at commit `34f02f54…`. Per-version `.dwg` counts:
2000=23, 2004=21, 2007=18, 2010=18, 2013=19, 2018=19, plus top-level
`example_*.dwg`/`sample_*.dwg`.

- **Covered entity types (loop CAN fix):** LINE, CIRCLE, ARC, ELLIPSE, TEXT,
  LWPOLYLINE, POLYLINE_2D, POLYLINE_3D, SPLINE, HATCH (2004 `HatchG`), MLINE,
  LEADER, HELIX, XLINE, RAY, POINT, 3DSOLID (2000 `Cone`), SURFACE (2004),
  UNDERLAY (2004), MATERIAL (2004), geometric/dimensional constraints, dynamic
  blocks (2018 `Dynblocks`), LIVESECTION (2018), plus broad mixed coverage via
  2000 `entities-2d`/`entities-3d` and `example_*`/`sample_*`.
- **Coverage gaps (loop CANNOT fix — no gold file):** MTEXT, DIMENSION (all
  kinds), TABLE, REGION, dedicated static INSERT/BLOCK, MLEADER/MULTILEADER,
  VIEWPORT, IMAGE, MESH/POLYFACEMESH, 3DFACE, SOLID (2D), ATTDEF/ATTRIB,
  TOLERANCE, WIPEOUT, XREF, OLE2, LIGHT, CAMERA, ARCDIMENSION. These are
  logged as `no gold coverage — out of loop`.
- **Richest dirs:** 2000 (widest entity set) and 2007 (best DXF companions).
  2010/2013 are near-minimal.

---

## 6. What is implemented (Phases 0–5 status)

### Phase 0–1 — Sources and gold oracle (WSL)

- libredwg and cadcodec cloned under `~/work/`. libredwg built with
  `sh ./autogen.sh && ./configure --disable-bindings --disable-docs && make -j`.
- `programs/dwgread -O JSON` smoke-tested on `test/test-data/2000/Line.dwg`.
- Path to `dwgread` exported via `GOLD_DWGREAD`; corpus via `GOLD_TESTDATA`.

### Phase 2 — Silver JSON dump (`tests/gold_harness/src/bin/dwg2json.rs`)

- Reads a DWG into `CadDocument`, serializes with `serde`, and re-injects the
  serde-skipped `EntityCommon` round-trip fields under `_common_dwg[handle]`.
- **Field count has grown from 12 to 17** since the original plan; current
  set: `linetype_handle`, `graphic_data`, `color_book_handle`,
  `face_visual_style_handle`, `edge_visual_style_handle`, `material_flags`,
  `material_handle`, `shadow_flags`, `plotstyle_flags`, `plotstyle_handle`,
  `linetype_flags`, `entity_mode`, `has_ds_data`, `prev_entity_handle`,
  `next_entity_handle`, `nolinks`, `z_are_zero`.
- Also emits `Block`/`BlockEnd` delimiter entities collected via
  `block_records` (not exposed by `CadDocument::entities()`).
- Raw byte blobs (`graphic_data`, `raw_acds_data`) stay excluded — verbatim
  storage, not comparable structure.

### Silver rewrite binary (`tests/gold_harness/src/bin/dwgrewrite.rs`)

- Minimal read → write binary that targets the same DWG version as the source.

### Phase 3 — Normalizer + differ (harness core)

- `normalize_gold.py`: keeps only the top-level `OBJECTS` array, drops
  header/section metadata, flattens points/handles/colors.
- `normalize_silver.py`: emits entities, objects, and table records; resolves
  layer names to handles; maps variant/field names to gold conventions (see
  §9 mapping table).
- `diff_fields.py`: aligns by `(type, ordinal)` and compares handle-valued
  fields by resolved target type. Reports `missing_in_silver`, `wrong_value`,
  `extra_in_silver`, `count_mismatch`.
- `ignore_fields.toml`: curated list of gold-only derived/verbatim fields,
  expanded after the first real run to include raw/verbatim object fields
  (`raw_data`, `cloning_flags`, `entries`, `items`, `xdata`, …) and now
  **frozen** for the loop.

### Phase 4 — Roundtrip driver

- `run_roundtrip.py <file.dwg> [workdir]`: orchestrates the three diffs per
  file. Default workdir `target/gold_harness/`.
- `run_corpus.py`: batch driver over the in-scope corpus; aggregates
  `report.json`/`report.md` ranking `(entity_type, field)` divergences.
  Default workdir `target/gold_harness_corpus/`.
- Internal-consistency diff (#3) is **stubbed** (reported as 0) — see §3.

### Phase 5 — Cargo test integration

- `tests/gold_roundtrip.rs` gated behind the `gold-harness` feature; shells
  out to `run_roundtrip.py` for a representative subset (`2000/Line.dwg`,
  `2000/circle.dwg`). Default assertion: harness runs and none of the
  prohibited `EntityCommon` storage-only fields (`z_is_zero`, `ltype_flags`,
  `prev_entity`, `next_entity`, `nolinks`) appear in any diff. Strict
  zero-`missing_in_silver` mode is opt-in via `GOLD_HARNESS_STRICT=1`.
- `tests/visualstyle_dwg_roundtrip.rs`: regression test that builds a
  `VisualStyle` with a full 24-element pre-R2010 property bag, round-trips it
  through R2000 DWG, and asserts the recovered properties match.

### Differences from the original plan (kept for the record)

| Plan text | Implementation | Rationale |
|---|---|---|
| Silver dump re-includes skipped fields inline in the entity record. | Skipped fields live in `_common_dwg[handle]`, not merged into the entity object. | Avoids polluting the public serde schema; the normalizer merges them before diffing. |
| `ignore_fields.toml` starts with a curated list. | List was expanded after the first real run to include raw/verbatim object fields. | Without these ignores, object-level representation noise drowned out real entity gaps. Now frozen. |
| Phase 5 test "asserts zero `missing_in_silver` for representative subset". | Test asserts the harness runs cleanly; strict mode gated by `GOLD_HARNESS_STRICT=1`. | Representative files still show missing common fields that the fix loop must close; making the default test fail would break `cargo test`. |
| Normalizer reshapes polylines into child `VERTEX_*` records or folds them. | Not implemented. | `Line.dwg`/`circle.dwg` have no expanded vertices; deferred until a polyline fixture is processed. |
| `ignore_fields.toml` is written once and never edited. | Edited during harness construction; now frozen. | Refinement happened before the loop boundary. |
| Silver bins live under `examples/` or `src/bin/`. | Bins live under `tests/gold_harness/src/bin/` and are registered as Cargo `[[bin]]` targets. | Keeps the harness self-contained. |
| Workdirs default to `/tmp`. | Workdirs default to `target/gold_harness*/`. | Repo-local, easy to inspect. |

### Recent code fixes (part of the harness loop)

- `src/io/dwg/dwg_writer.rs`: class-table filtering now retains live
  `ClassObject` instances (e.g. `ACDBSECTIONVIEWSTYLE`,
  `ACDBDETAILVIEWSTYLE`) after `retain_legacy_dwg_classes()`, so their type
  codes are not mis-emitted as `500` (`ACDBDICTIONARYWDFLT`).

---

## 7. Current baseline and blockers

### How to start cold (read this first in a fresh session)

A zero-context agent begins here. Read in order, on demand:

1. **This file** (`tests/gold_harness/IMPLEMENTATION.md`) — the single source
   of truth. The self-contained operating manual is §8.1 (§8.1.0 environment →
   §8.1.1 context budget → §8.1.2 packet → §8.1.3 decision tree → §8.1.4 fix
   recipe → §8.1.6 work queue → §8.1.6a coverage audit → §8.1.7 prohibitions).
2. **The work queue** §8.1.6 — the next packet is named there with its full
   diagnosis. Per-packet cold-start briefs are written as `NEXT_<PACKET>.md`
   beside this file when a packet needs one; when a packet completes, the brief
   is folded back into its §8.1.6 DONE entry and the `NEXT_<PACKET>.md` file is
   deleted (the queue is the single source of truth).
3. **Verify the environment** with §8.1.0 before editing.

**Current corpus baseline (post ownerhandle/SCALE/reactors/c_prop33/vertexids/
BLOCK_HEADER/VPORT/VIEWPORT/VIEW/LTYPE/POINT/LAYOUT/MATERIAL/*_CONTROL/color/
LINE-POINT color/INSERT/ELLIPSE/MTEXT/SOLID/3DFACE/LAYER-flag0-ltype/DIMASSOC
packets + audit fixes + CMC color-method fix + TABLESTYLE/MLINESTYLE/
MLEADERSTYLE/DICTIONARYWDFLT/IMAGE/ATTDEF/LAYOUT/CONTROL/LEADER/TEXT/HATCH/
SPLINE/WIPEOUT + INSERT.block_header + UNKNOWN_OBJ naming + MTEXT R2018 +
gold-spec audit + MULTILEADER + 3DSOLID + ACSH_HISTORY + EVALUATION_GRAPH +
BLOCK_HEADER/controls + viewstyles + mtext.style + underlays + dimensions +
ASSOC family + polyline vertex emission + xdic family + layer visualstyle +
mid-rank batch (has_ds_data/SORTENTSTABLE/APPID/DIMASSOC.ref/
IMAGEDEF_REACTOR) + elevation/linewt projections + R2013+ AcDs
3DSOLID-family writer payload + pre-R2007 is_xref_ref wire bit +
fabricated-Standard-DIMSTYLE removal + 3DSOLID prologue-divergence
projection + MTEXT R2018 redundant extents/ignore_attachment + DIMENSION
family block handle + pre-R2013 classes-verbatim roundtrip +
unmodeled-class (UNKNOWN_OBJ/ENT) projections + UNKNOWN_ENT
_common_dwg graphic-data strip + U2 dynamic-block-class retype map +
unknown_bits raw-remainder side channel + text-style map/ATTDEF trio +
INSERT attrib/seqend chain + MLINE projection + POLYLINE_3D/GROUP/PFACE/
VIEWPORT-named_ucs + ASSOC actionbody/path retype family + RAY/XLINE/
IMAGEDEF/HATCH degenerates/VX family/LIGHT/RASTERVARIABLES/MULTILEADER +
ARC_DIMENSION graphic + HELIX family/LEADER/TOLERANCE dimstyle/MATERIAL
rgb + UNKNOWN-family handle-code unstamp + raw-linewt fidelity
reader/writer/normalizer + LAYOUT.viewports (2026-09-20 fourth + fifth +
sixth batches) + PROXY_OBJECT raw-window/objids capture (2026-09-20
seventh batch, `fa2cb0a`) + DIMSTYLE_CONTROL.morehandles reader capture
+ writer echo (2026-09-20 eighth batch, `b08d346`) + LEADER R2000-pair
family fix (2026-09-20 ninth batch, `b6e6e92`) + MULTILEADER attach
trio — R2010 tail gate + raw retention (2026-09-20 tenth batch,
`45382ec`) + UNKNOWN-family dropped-records/typing fixes (TABLECONTENT
retype, 2026-09-20 eleventh batch, `b123b4c`) + VERTEX_MESH dropped
records (TS1, 2026-09-20 twelfth batch, `fca5367`) + ACSH_CONE_CLASS
retype (2026-09-20 thirteenth batch, `fad3042`) + WIPEOUT/IMAGE
imagedefreactor wire codes (2026-09-20 fourteenth batch, `543f877`) +
SEQEND real-handle retention (2026-09-20 fifteenth batch, `689b14d`) +
VIEWPORT.status_flag raw retention (2026-09-20 sixteenth batch,
`263ab5f`) + LEADEROBJECTCONTEXTDATA retype (2026-09-20 seventeenth
batch, `9f06d89`) + TOLERANCE field-name set (2026-09-20 eighteenth
batch, `a7e451b`) + smalls batch: MTEXT columns / GROUP.name /
VISUALSTYLE BS sign / 3DFACE z_is_zero / MLINE CLOSED (2026-09-20
nineteenth batch, `257895a`) + TRACE/SOLID wire-type split (twentieth
batch, `91e72a3`) + MESH/PDFUNDERLAY graphic_data pops (twenty-first
batch, `e90fb77`) + SORTENTSTABLE.ents emission (twenty-second batch,
`ab0e02f`) + SURFACE family retypes + PLANESURFACE projection
(twenty-third batch, `b2955a7`) + the big-family projection wave
(FIELD/FIELDLIST, PLOTSETTINGS, GEODATA, Underlay PDFUNDERLAY+
PDFDEFINITION, MESH, PLANESURFACE reader tail, SHAPE/TEXT-family raw
dataflags, OLE2FRAME raw blob, encr_sat_data per-block retention,
seqend-flag truth + common plotstyle-order — batches 24-32,
`82f0225`) + the Dynblocks/PolyLine2D/LWPOLYLINE/SEQEND/LAYOUTPRINTCONFIG
wave (raw LwPolyline flag + vertexids, SPLINE ctrl_pts collapse shape,
BLOCK_HEADER xref_pname + lazy dupe strip, POLYLINE_2D parent projection,
LAYOUTPRINTCONFIG retype, kid linetype inheritance — batches 33-38,
`078c119`) + the SECTION trio retype (SECTIONOBJECT/SECTION_MANAGER/
SECTION_SETTINGS — thirty-ninth batch, `5a25bf9`), 2026-09-20):**
read-fidelity **38**, write-fidelity **30** (from 479/455 at the
morning baseline) — the 1 000 milestone was passed at 943/804, the
below-100 interim milestone was reached on 2026-09-20, the below-50
band on 2026-09-20 evening, and the **campaign target is now 0 on
both sides**
(raised 2026-09-20: land every remaining family, heavy pockets included)
— and the two sides MIRROR
family-for-family (the classes-verbatim fix un-masked real gaps whose rows
used to be cancelled by symmetric counterfeit garbage; the UNKNOWN
projections then took −308/−324; U2 then took −719/−720; the unknown_bits
side channel then took −448/−448; the style-map/chain/MLINE batch then took
−327/−319; the POLYLINE_3D/GROUP/PFACE/Associative-retype batch then took
−478/−493; the RAY/XLINE/IMAGEDEF/HATCH/VX/LIGHT batch then took −338/−336;
the HELIX/dimstyle/MATERIAL-rgb batch then took −172/−172; the UNKNOWN
handle-code unstamp then took −37/−3; the raw-linewt + LAYOUT.viewports
batch then took −46/−9; the PROXY_OBJECT raw-window/objids batch then took
−13/−13; the DIMSTYLE_CONTROL.morehandles capture then took −104/0; the
LEADER R2000-pair family then took −24/−24; the MULTILEADER attach-trio
batch then took −12/−12 — MULTILEADER rows zero on both sides; the
UNKNOWN-family dropped-records batch then took −12/−12; the VERTEX_MESH
dropped-records batch then took −12/−12; the ACSH_CONE_CLASS retype
then took −4/−4; the WIPEOUT/IMAGE imagedefreactor code fix then took
−12 write rows; the SEQEND real-handle retention then took −18 write
rows; the VIEWPORT.status_flag raw retention then took −17 read
rows; the LEADEROBJECTCONTEXTDATA retype then took −24/−24; the
TOLERANCE field-name batch then took −63/−63; the smalls batch
(MTEXT columns, GROUP.name, VISUALSTYLE sign, 3DFACE z_is_zero,
MLINE CLOSED) then took −17/−62; the TRACE/SOLID wire-type split
then took −53/−53; the MESH/PDFUNDERLAY graphic_data pops then took
−5/−5; the SORTENTSTABLE.ents emission then took −4/−5; the SURFACE
family retypes + PLANESURFACE projection then took −17/−18; the
2026-09-20 evening session then took **−426/−414** across three
committed waves — read **479 → 53**, write **455 → 41**, **103 of 124
files at 0/0** — the big-family projection wave (batches 24-32,
`82f0225`: FIELD/FIELDLIST name+dotted-value union + [0]*n childval,
PLOTSETTINGS full 33-field map + plot_flags bits + enum inversion,
GEODATA R2010 projection with obsolete/civil pops, PDFUNDERLAY field
map + the UnderlayDefinition retype-order fix + reactor retention,
MESH unknown_b1/b2 raw trailing bits + [0]*n edges + per-edge crease,
PLANESURFACE modeler/u/v reader alignment + fixed-15 banner split +
acis_empty_bit/isolines from retained wire data, SHAPE slot-map (size=
scale, relative_x_scale=width_factor), the TEXT/ATTRIB/ATTDEF raw
dataflags byte retained end-to-end with the dwg_set_dataflags
alignment==insertion semantics, OLE2FRAME raw 315 KB blob retention,
encr_sat_data per-block raw retention + verbatim echo, the
entity-common plotstyle/material pair-order truth (common_entity_data
.spec 507-522: the old reader mislabeled them inside an isochronous
12-bit window), the per-SEQEND wire flag retention (the flags-3-null
families are LibreDWG-authored data, DWG-native chains 0), POLYLINE_3D
flag from the captured bit dict, POLYLINE_PFACE vertex vector SINCE
R2004a, DIMENSION class_version SINCE(R_2010b), MTEXT \U+XXXX decode);
the Dynblocks/PolyLine2D wave (batches 33-38, `078c119`: LwPolyline
raw wire flag retention with VERTEXIDCOUNT emission including the
normalize_gold 4-int raw-handle-tuple mirror, SPLINE ctrl_pts [0]*n
per-point collapse, BLOCK_HEADER xref_pname from xref_path + lazy
anonymous dupe strip, POLYLINE_2D parent projection + chain fields,
LAYOUTPRINTCONFIG retype of the Extended wrapper, kid/SEQEND writer
linetype inheritance); the SECTION trio retype (batch 39, `5a25bf9`:
SECTION_MANAGER/SECTION_SETTINGS/SECTIONOBJECT from the ClassObject
and Extended wrappers); the constraint-flat-nodes + Helix era gates
(batch 40, `c20e6fe`: ASSOC2DCONSTRAINTGROUP mirrored gold's flat
per-node REPEAT in reader AND writer — the root-node/class-registry
shape was a misparse that lost the node count and produced garbage
signed BLs; HELIX knotparam/splineflags gated SINCE(R_2013b)); the
2026-09-20 late-night session then took **−38/−28** across three
committed waves to the campaign floor — read **38 → 0**, write
**30 → 2**, **123 of 124 files at 0/0** — the gh109_1 zero-out
(batch 41, `707e7a8`: the RAPIDRT rapid-order gold shadow — gold reads
the base has_predefined bit at AC1027 BEFORE the rapid seven, so its
cursor enters the rapid block one bit late and every rapid value is a
deterministic misparse; the reader re-walks the just-read bits in
gold's order and retains RapidRtGoldShadow with u32 BL fields (gold
prints FORMAT_BL %u), normalize_silver types ClassObject data.
RapidRtRenderSettings and projects base+shadow — the parallel
UNKNOWN_OBJ count rows re-balance with the retype; plus add_wire_entry:
SortEntitiesTable.add_entry deduped wire pairs by entity handle which
folded the four dead slots (ent 0, sort 919/1219/928/1027) of gh109_1's
39-pair SORTENTSTABLE into one entry with the LAST duplicate's sort
kept — 39 pairs append verbatim now); the chain-ordinal wave (batch 42,
`604f956`: Vertex2D.wire_handle reader retention — the parent+1+i kid
synthesis guess mispaired whenever a foreign block-chain record
interleaves the kid run (2000/PolyLine2D's LINE@512 between poly@511
and kids 513/514 poisoned BLOCK_HEADER.last_entity, LINE.handle and
POLYLINE_2D.next_entity from one root), Vertex3DPolyline.reactor_
handles retention + writer kid echo (assoc networks register individual
vertices — the single-registered-kid rows across example_2000..2018),
rebuild_block_membership keeps the raw owner for class entities owned
by non-block objects (CAcLayoutPrintConfig → DICTIONARY 856 with the
NULL model-space chain gold has), VIEW/VPORT VIEWMODE composed from
the wire's FIELD_4BITS nibble (bits.c composes MSB-first: first-read
bit is value 8) instead of the ucs bools, VIEW.is_camera_plottable
gated R2007+, DIMENSION_ANG2LN.xline2end_pt mapping fix (the arm
sourced first_point instead of definition_point — dwg.spec
DWG_ENTITY(DIMENSION_ANG2LN) wire order), INSERT-kid ATTRIB xdicobj_
handle threading, the synthesized VERTEX_MESH last-vertex prev =
previous kid (gold 2000/TS1 539: prev {8→538}), VIEWPORT.vport_entity_
handle end-to-end retention of the pre-R2004 wire slot replacing the
vx-table-lookup fabrication, R2013+ table-entry has_ds_data projected
from dwg_data_store_handles (Pass 1 captures the NonEntityCommonData
bit)); plus the deep-gate arms (batch 43, `e0c6f93`:
normalize_entity_for_comparison clears the new vertex raw-retention
fields on both sides — writer-allocated kid handles on constructed
documents are bookkeeping, not payload semantics). **The Surface
zero-out then landed the last pocket (batch 44, `9b1738c`): 2004/
Surface.dwg 0/2 → 0/0 — the ASSOCPLANESURFACEACTIONBODY embedded
PAB/SAB handle-pull pair (assocdep/pbsab_status) via gold's per-object
dat-end clamps in the pre-2007 merged reader + the SAB-slot projection
in the normalizer + the writer's no-slot echo of a truncated
sab.assocdep (full recipe in the §8.1.6a DONE entry). Batches seven
through forty-four each carry their full recipes in their commit
messages and §8.1.6).
gh44-error.dwg
stays out of scope (explicit guard in `run_corpus.in_scope_files`,
`246e60a`; the new `-nan` shim in normalize_gold had briefly re-included
it, inflating totals to 7689/7559).
`cargo test --features serde` = 47 ok test segments (roundtrip 97/0,
1312 units) on 2026-09-21; `cargo test
--features gold-harness --test gold_roundtrip` = ok. Update these numbers after
each packet lands.

**Campaign state (2026-09-21, post-`9b1738c`: CAMPAIGN TARGET REACHED —
read 0 / write 0 on the full 124-file corpus, 124 of 124 files at
0/0, residues empty):** the last pocket (2004/Surface.dwg, 0/2 rt,
ASSOCPLANESURFACEACTIONBODY.assocdep + .pbsab_status) was zeroed as
batch 44 (`9b1738c`, 2026-09-21) — the dossier decode from the
2026-09-20 fold held up on every point except one measured
refinement: gold decodes an EXPLICIT (5,0) null-form slot as a real
handle (the [5,0,0,0] tuple print), NOT the [0,0] overflow form —
the overflow-refuse print and the valid-null print differ, so the
writer echoes a truncated record's missing sab.assocdep slot by
OMITTING it instead of writing a fabricated (5,0) form. The full
recipe is the §8.1.6a DONE entry; traces + byte dumps remain
regenerable via `pk19_55_surf_dossier.sh`. The campaign is closed:
further sessions are verification patrols (§8.1.6's queue is empty) —
run the corpus after any codec change and treat any resurfacing row
with the §8.1 per-packet workflow.

Done-packet recipes for batches 7-44 live in §8.1.6 below and in the
commit messages of `fa2cb0a..9b1738c`.
### Baseline after EntityCommon closure (2026-09-17)

- **LINE entity diffs: 0** on all six versions (2000–2018), both read fidelity
  (`diff_orig`) and write fidelity (`diff_rt`). Same for CIRCLE on 2000.
- Corpus totals are dominated by **object/table-record** representation gaps
  (2000/Line.dwg read-fidelity is now 602 after the BLOCK_HEADER packet; it was
  ~2.6k at EntityCommon closure), not entity fields.
- Strict mode (`GOLD_HARNESS_STRICT=1`) still fails on the representative
  subset due to **table-record storage fields** and object representation
  gaps, not entities.

### EntityCommon storage-only fields — CLOSED (2026-09-17)

The six target fields (`prev_entity`, `next_entity`, `nolinks`, `ltype_flags`,
`plotstyle_flags`, `z_is_zero`) were already read and written correctly by the
core codec; the gaps were in the dump/normalizer projection:

| Fix | Where | Detail |
|---|---|---|
| `material_flags`/`shadow_flags` gated `SINCE R_2007a` | `dwg2json.rs` | Emitted as `Option`, `None` before AC1021 (was unconditional `0` → `extra_in_silver` on R2000/R2004) |
| `has_ds_data` gated `SINCE R_2013` | `dwg2json.rs` | `None` before AC1027 |
| `is_xdic_missing` derived | `dwg2json.rs` | `Some(xdictionary_handle.is_none())` when ≥ AC1018 (gold `SINCE R_2004a`) |
| `has_full/face/edge_visualstyle` derived | `dwg2json.rs` | `Some(handle.is_some())` when ≥ AC1024 (gold `SINCE R_2010b`) |
| `entity_mode` → `entmode` | `normalize_silver.py` | Gold field name; ignored on both sides via `ignore_fields.toml` |
| `linetype` name dropped; `linetype_handle` → `ltype` | `normalize_silver.py` | Gold has no name field, only the conditional handle |
| `plotstyle_handle` → `plotstyle`, `material_handle` → `material`, `face/edge_visual_style_handle` → `face/edge_visualstyle`, `full_visual_style_handle` → `full_visualstyle` | `normalize_silver.py` | Gold handle names from `common_entity_handle_data.spec` |
| `color_name` skipped when `None` | `normalize_silver.py` | Gold only emits it when the color carries a name |
| `ownerhandle` only when `entity_mode == 0` | `normalize_silver.py` | Gold emits it for entities only when `entmode == 0` (polyline sub-entities) |
| Color hash `{index, rgb:"000000", flag}` collapses to `index` | `normalize_gold.py` | R2004+ ENC JSON emits derived keys alongside `index` |

### Known issues and blockers

| Issue | Status | Notes |
|---|---|---|
| `AcDbDictionaryWithDefault` mis-emitted as type 500 | **Fixed** | Class-table retention fix in `src/io/dwg/dwg_writer.rs` |
| `ACDBSECTIONVIEWSTYLE` / `ACDBDETAILVIEWSTYLE` missing after rewrite | **Fixed** | Same class-table retention fix |
| `AcDbVisualStyle` "Object improperly read" | **Fixed** (2026-09-17) | `bd2007_45` was read/written unconditionally in the pre-R2010 branch; gold gates it `SINCE R_2007a`. R2000/R2004 rewrites carried 8 extra bytes per VisualStyle, desynchronizing the object stream. Fixed by gating on `r2007_plus()` in both `read_visual_style` and `write_visual_style`. Confirmed: gold `dwgread` now decodes R2000/R2004 rewrites with exit 0 and no VisualStyle errors; R2007 behavior unchanged. |
| EntityCommon storage-only field gaps | **Fixed** (2026-09-17) | See table above. LINE/CIRCLE entity diffs now zero on all versions. |
| Uniform table-record storage fields (`ownerhandle`, `is_xref_ref`, `is_xref_resolved`, `is_xref_dep`, `xref`, `unknown`, `is_xdic_missing` R2004+, `has_ds_data` R2013+) | **Fixed** (2026-09-17; xdic updated `3cff463` 2026-09-19) | These are uniform across ordinary table records and derived in `normalize_silver.py` from silver state: `ownerhandle` = the table's control-object handle, xref bits = `1/0/0`, `xref` = null handle, `unknown` = 0 (APPID only), `is_xdic_missing`/`xdicobjhandle` = looked up per record in the document-level `xdic_by_handle` map (silver's `xdictionary_handle` storage, `xdicobjhandle` on all versions when present, the R2004+ bit otherwise; under `xdictionary_handle.is_none()` before `3cff463` — wrong for records that own an xdic), `has_ds_data` = 0 (R2013+, objects). No codec changes needed for the read side. APPID/TEXTSTYLE/VIEW/UCS table records are now diff-free. |
| Table-record payload fields | **Open** | Per-record semantic data, not uniform: DIMSTYLE ~70 `DIM*` vars (largest), LTYPE dash patterns (`dashes`, `numdashes`, `pattern_len`), VPORT view params (`VIEWCTR`, `VIEWDIR`, `BACKZ`, `FRONTZ`, `GRID*`, `SNAP*`, `UCS*`, …), BLOCK_HEADER topology (`xdicobjhandle`, `base_pt`, `xref_pname`, `block_entity`, `entities`, `endblk_entity`, …), LAYER `plotstyle`/`linewt`. These require reader storage and/or per-type normalizer mapping — one packet per table type. |
| Object representation gaps | **In progress** | VISUALSTYLE, MATERIAL, DIMSTYLE, SCALE, XRECORD, DICTIONARYVAR, and LWPOLYLINE are mapped in `normalize_silver.py`. Corpus totals on `Line.dwg` dropped from ~2500 to ~880; Polyline.dwg from ~995 to ~880. `ownerhandle` handle-code semantics is done (see §8.1.6). Residual: CMC absent-color encoding, `reactors` storage gap. Remaining per-type: LAYOUT, MLEADERSTYLE, BLOCK_HEADER topology, LTYPE dash patterns, VPORT view params. See §8.1.6. |
| Write-fidelity diff compares silver_rt, not gold_rt | **Open** (found 2026-09-17) | `run_roundtrip.py` builds `diff_rt = diff(gold_rt_norm, silver_rt_norm)` although §3 documents write fidelity as `gold_orig` vs `gold_rt`. Consequences: (a) the differ's two-sided handle-code check is unreachable (silver emits `code=None` on every path), so acadrust's writer flattening all ownerhandle codes to 4 (gold_rt `{4:214}` vs gold_orig `{4:94, 8:40, 12:78, 10:5}`) is invisible to the harness; (b) "write fidelity" currently measures gold-vs-silver on the *rewritten* file, not what the writer changed. Wiring the documented gold_orig-vs-gold_rt diff would activate code comparison and catch code-flattening regressions. |
| Corpus per-file workdir collisions | **Open** (found 2026-09-17) | `run_corpus.py` keys workdirs by `path.stem`; same-named files across version dirs (e.g. six `Line.dwg`) overwrite each other, and the report aggregation re-reads the surviving diff once per result entry — colliding stems are counted multiple times with the last version's content. Totals are stable run-to-run, but per-`(type,field)` counts are distorted. Fix: key workdirs by `{version}/{stem}`. |
| Plain `cargo test` fails to compile `examples/entity_atlas.rs` | **Open** (pre-existing; confirmed on 6d375d1, 2026-09-17) | The example imports `serde_json` unconditionally while `serde_json` is optional behind the `serde` feature. Add `required-features = ["serde"]` to the `[[example]]` entry (or gate the import). `cargo test --features serde` = 1556/0 is unaffected. |

---

## 8. Phase 6 — Autonomous fix loop (specification)

Run after Phases 0–5 are in place, under an **implementation-capable agent**
(file edits + `cargo` + subprocess), NOT in plan mode. Git mutations optional.

**Corpus (bounded):** in-scope version dirs
`{2000,2004,2007,2010,2013,2018}/*.dwg` plus top-level
`example_{2000,2004,2007,2010,2013,2018}.dwg` and `sample_{2000,2018}.dwg`.
Entity types with no gold file are written to `report.md` under
`OUT OF LOOP (no gold coverage)` and are NOT worked on.

**Convergence target:** for every covered file,
`normalize(gold_orig) ≡ normalize(silver_orig)` on the non-header,
**non-ignored** field set, AND `normalize(gold_orig) ≡ normalize(gold_rt)` on
the same set. Header keys are excluded from the strict diff (lax header).

**Loop body (repeat until converged or all blocked):**

1. **Baseline.** Run the Phase 4 driver over the whole covered corpus. Produce
   `report.json`/`report.md` ranking `(entity_type, gold_field, dwg_version)`
   divergences by frequency and by a fixed priority: read-fidelity first, then
   write-fidelity, then internal consistency. Geometry-correctness fields
   (points, radii, angles, counts) rank above cosmetic/default fields.
2. **Pick top offender.** Choose the highest-ranked `(entity_type,
   gold_field)` not already marked blocked.
3. **Consult gold.** Open `dwg.spec`/`dwg2.spec` at that
   `DWG_ENTITY(...)`/`DWG_OBJECT(...)` block (and `common_entity_data.spec` /
   `common_entity_handle_data.spec` for common fields). Record the exact
   `FIELD_*` macro, bitcode type, and `PRE/VERSIONS/SINCE/UNTIL/LATER_VERSIONS`
   gating.
4. **Fix silver, version-gated.** Edit the matching cadcodec `read_<entity>`
   in `object_reader/entities.rs` and its mirror in
   `object_writer/entities.rs`. Add/correct the field using the same bitcode
   primitives (`read_bit_double_with_default`, `read_bit_extrusion`,
   `read_bit_thickness`, `read_bit`, …) and gate it behind the SAME version
   predicate gold uses (`r2000_plus()`, `r2004_plus()`, `r2007_plus()`, …).
   **Do NOT modify version dispatch, decompression, CRC, or the section map** —
   only the per-field read/write bodies.
5. **Verify across all 6 versions.** Re-run the driver over the FULL covered
   corpus, not just the failing file, to catch cross-version regressions.
   Confirm the target field now matches and the corpus-wide non-header diff
   count strictly decreased.
6. **Regression gate + checkpoint.** Require: (a) `cargo test` green (existing
   suite, harness feature off), (b) `cargo test --features serde` green,
   (c) corpus non-header diff count strictly decreased. If all pass,
   checkpoint the fix (`git commit` if git mutations are available, else
   record in `report.md`). If any check fails, revert and mark the offender
   blocked with the reason.
7. **Blocked handling.** If an offender cannot converge after **5 fix
   attempts** (or requires touching out-of-bounds code: version dispatch,
   decompressor, CRC), mark it `BLOCKED: <reason>` in `report.md` and move on.
   Revisit blocked items only after all non-blocked offenders are exhausted.
8. **Stop conditions.** Stop when (a) corpus-wide non-header, non-ignored diff
   count == 0 (SUCCESS), or (b) every remaining offender is BLOCKED (PARTIAL).
   Emit final `report.md` listing fixed fields, blocked fields, and
   out-of-loop entity types.

**Loop invariants the agent must preserve:**

- Never edit gold (libredwg) — it is a read-only oracle input.
- Never weaken the differ/normalizer to make a diff pass — no adding
  tolerances, no dropping fields, and **no editing `ignore_fields.toml`**
  during the loop.
- Never change the header strictness — header stays lax throughout.
- Each checkpoint must leave the build green; the working tree must never be
  left broken.

**Version-scope guard (explicit):** a fix is only valid if it is gated to the
exact DWG versions gold specifies. Confirm for each touched field that the
field is read/written under the identical version predicate in both reader and
writer, and that no other version's behavior changed (verified by step 5).

### First fix-loop target — DONE (2026-09-17)

~~Scope the first fix-loop pass to entity `EntityCommon` storage-only fields:
`prev_entity`, `next_entity`, `nolinks`, `ltype_flags`, `plotstyle_flags`,
`z_is_zero`.~~ Closed — see §7. Next targets, in priority order:

1. **Table-record storage fields** (`ownerhandle`, `is_xref_ref`,
   `is_xref_resolved`, `is_xref_dep`, `xref`, `unknown`) on APPID, LAYER,
   LTYPE, STYLE, DIMSTYLE, … — these block `GOLD_HARNESS_STRICT=1`.
2. **Object representation gaps** ranked by corpus frequency (VISUALSTYLE,
   MATERIAL, DIMSTYLE, LAYOUT, SCALE, MLEADERSTYLE, DICTIONARYVAR, …).

---

## 8.1 Subagent execution guide (128K context)

This section is a self-contained operating manual for an implementation agent
with a **128K-token context window** running the Phase 6 loop. It assumes the
agent sees ONLY this section plus whatever it reads on demand — not the whole
plan, not whole source files. Follow the context-budget rules strictly; the
two spec files and the two big Rust files together exceed the window.

### 8.1.0 Environment (verify once per session)

```bash
cd ~/work/cadcodec
source "$HOME/.cargo/env"
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
python3 tests/gold_harness/check_env.py   # must print "Environment looks good"
cargo build --features serde --bins        # must succeed before any edit
```

Repos: silver = `~/work/cadcodec` (Rust crate `acadrust`), gold =
`~/work/libredwg` (C, read-only oracle). **Never edit anything under
`~/work/libredwg`.**

**Corpus + artifacts (where things land):**
- Run the full corpus (125 files, all six versions): `python3 tests/gold_harness/run_corpus.py`
  (no args). Takes ~9 min. Writes `target/gold_harness_corpus/report.json` +
  `report.md` (the **work-queue source**) and, per file, a workdir
  `target/gold_harness_corpus/<stem>/` holding `<stem>_gold_orig.json`,
  `<stem>_silver_orig.json`, `<stem>_gold_orig.norm.json`,
  `<stem>_silver_orig.norm.json`, `<stem>_diff_orig.json`, and the `_rt`
  (roundtrip) counterparts.
- Run one file: `python3 tests/gold_harness/run_roundtrip.py "$GOLD_TESTDATA/<ver>/<File>.dwg" <out_dir>`
  → same artifact set under `<out_dir>/<Filestem>_…`.
- The pipeline each driver runs: `dwgread -O JSON` (gold) → `normalize_gold.py`;
  `dwg2json` (silver) → `normalize_silver.py`; then `diff_fields.py` with
  `ignore_fields.toml`. `run_roundtrip.py` prints
  `read-fidelity diffs: N` / `write-fidelity diffs: N`.
- **Stem collision**: workdirs are keyed by file stem, so same-named files
  across versions share one dir (last write wins). For a version-specific
  artifact, run `run_roundtrip.py` yourself into a fresh `<out_dir>`.
- **Raw record bytes (bit-level adjudication)**: when the two decoders
  disagree and the spec text is not enough, dump the actual decompressed
  record bytes with `cargo build --bin dump_section_bytes && target/debug/
  dump_section_bytes "$GOLD_TESTDATA/<f>.dwg" <address> <size>` (default
  section `AcDb:AcDbObjects`; prints hex + MSB-first bit string) and
  hand-walk with LibreDWG's bit semantics (`bits.c`: BS `'00'→RS16 '01'→RC8
  '10'→0 '11'→256`; BL `'00'→RL32 '01'→RC8 '10'→0 '11'→ERR256`; BD
  `'00'→raw f64 '01'→1.0 '10'→0.0 '11'→nan`; RS/RL/RD are byte-wise
  little-endian, each byte read MSB-first; object records begin MS size +
  UMC handlestream-size (R2010+, not counted in size) then BOT type, and
  gold's trace `@byte.bit` positions are relative to the BOT start). Get
  the record address/size from `dwgread -v9 <file> 2> trace.log` lines
  `Object number: … Size: N [MS] … Address: A`. This is how the REGION-NaN
  packet (§8.1.6, 2026-09-20) proved silver's reader bit-true against
  gold's misreading spec.

### 8.1.1 Context budget rules (hard constraints)

0. **The gold spec is split across MORE than two files.** Object/entity/table
   field definitions live in `dwg.spec` (88 block starters: 53 entities +
   25 objects + 10 tables) and `dwg2.spec` (234 real starters: 45 entities +
   189 objects). Nested `*_fields` macros live in three places, and the
   built `dwgread` compiles some blocks out (liveness verified 2026-09-19,
   libredwg 34f02f54 — see the liveness rule below). Full gold source map:

   | File | Content | When to consult |
   |---|---|---|
   | `dwg.spec` | 88 starters (**87 live**) — pre-R2000 objects + tables + entities | the outer type block |
   | `dwg2.spec` | 234 real starters (**161 live**) — R2000+ objects (SCALE, XRECORD, VISUALSTYLE, MATERIAL, MLEADERSTYLE, TABLESTYLE, MULTILEADER, …) + most nested `*_fields` macros. A plain grep ALSO hits two commented-out mentions: `//DWG_OBJECT (OBJECTCONTEXTDATA)` (3813, "subclass only") and `//DWG_OBJECT (PARTIAL_VIEWING_FILTER)` (6245) — gold never emits those names | the outer type block + nested macros |
   | `dwg_spec_shared.h` | the 3 `*_fields` macros moved out of the specs (`TABLE_value_fields`, `WIRESTRUCT_fields`, `AcDbMTextObjectEmbedded_fields`) **plus** field-lists WITHOUT the `_fields` suffix: `COMMON_3DSOLID`, `ACTION_3DSOLID` (3DSOLID common ACIS payload), `COMMON_ENTITY_DIMENSION` (DIMENSION common), and the `DECODE_3DSOLID`/`ENCODE_3DSOLID`/`FREE_3DSOLID` per-TU wrappers | nested `value.*`, wireframe structs, embedded MText, **3DSOLID**, **DIMENSION** |
   | `common_entity_data.spec` | entity common *data* fields (entmode, linewt, color, ltype_scale, invisible, …) | entity common fields |
   | `common_entity_handle_data.spec` | entity common *handle-stream* fields (ownerhandle, ltype, plotstyle, visualstyles, material, …) | entity handle refs |
   | `common_object_handle_data.spec` | object common handle-stream fields (ownerhandle, reactors, xdictionary) | object handle refs |
   | `include/dwg.h` | **40 ALLCAPS `*_fields` macros** (`CMLContent_fields`, `ASSOCACTION_fields`, …) — generated struct-MEMBER declarations (memory layout, gen-dynapi.pl), NOT spec field sequences. Do not confuse them with the 66 spec `*_fields` macros. Also the struct/enum source of truth (`DWG_COLOR_METHOD`, …) | field presence/types when the spec refs a struct |
   | `spec.h` + `dec_macros.h`/`enc_macros.h`/`out_json.h` | the `FIELD_*` DSL macro definitions per translation unit (decode/encode/JSON-emit semantics; e.g. the CMC method byte) | token semantics of a FIELD_* |
   | `objects.inc` | the generated full type-name registration list: 317 names (97 `DWG_ENTITY` + 220 `DWG_OBJECT`), unguarded — exactly the live block set plus the gated ones | name ↔ dispatch checks |
   | `classes.inc` | dynamic-class dispatcher macros (`WARN_UNHANDLED_CLASS`, per-ACTION dispatch) — not a name list | class-table behavior |

   **Liveness rule (verified 2026-09-19).** The built `dwgread` defines
   neither `DEBUG_CLASSES` (`config.h`: `#undef`) nor `IS_FREE` in the
   decode/JSON translation units (decode.c = `IS_DECODER`, decode2.c bare,
   out_json.c = `IS_JSON`), so every `#if defined (DEBUG_CLASSES) ||
   defined (IS_FREE)` frame and every `#if 0` region compiles OUT: those
   classes decode as raw `UNKNOWN_ENT`/`UNKNOWN_OBJ` (with `unknown_bits`)
   and are **never emitted typed** — empirically confirmed over the whole
   corpus. Gated regions: dwg2.spec 297–959 (TABLE entity, TABLECONTENT,
   the TABLECONTENTs_fields macro), 3716–4513 (the "in work area": SURFACE
   entities EXTRUDED/LOFTED/REVOLVED/SWEPT/NURBS, ASSOC* action bodies,
   CONTEXTDATAMANAGER, SUNSTUDY, GEOPOSITIONMARKER, the abstract
   OBJECTCONTEXTDATA, *PARAMETERENTITY/*GRIPENTITY, NAVISWORKS*, the
   debug-side OBJECTCONTEXTDATA subclasses (MLEADER- / MTEXTATTRIBUTE- /
   ANNOTSCALE- / *DIM-), …), 4936–5253 (ACME*, MOTIONPATH,
   CSACDOCUMENTOPTIONS, TVDEVICEPROPERTIES, RTEXT/ARCALIGNEDTEXT, …),
   6190–6221, 6265–6284 (BREAKDATA, BREAKPOINTREF); `#if 0`: 5168
   (BLOCKANGULARCONSTRAINTPARAMETERENTITY), 6288 (XREFPANELOBJECT,
   NPOCOLLECTION, ACDSRECORD, ACDSSCHEMA); dwg.spec 4900–5104 (MPOLYGON).
   If a target type is in one of those frames, gold can never emit it typed —
   do NOT write a typed reader packet for it; match gold's UNKNOWN record
   instead. (`UNKNOWN_ENT`/`UNKNOWN_OBJ`/`DUMMY` sit in `#ifndef IS_DXF` —
   they ARE live for the JSON path.)

   The remaining `.spec` files describe **header/section infrastructure** —
   out of scope for the OBJECTS field diff (the harness diffs only the object
   stream, not the header): `header.spec`, `header_variables.spec`,
   `header_variables_r11.spec` (pre-R13), `header_variables_dxf.spec` (DXF
   emitters only — not in the JSON path), `auxheader.spec`,
   `2ndheader.spec`, `summaryinfo.spec`, `acds.spec`, `appinfo.spec`,
   `filedeplist.spec`, `objfreespace.spec`, `r2004_file_header.spec`,
   `revhistory.spec`, `security.spec`, `template.spec`. Status of the
   odd ones (verified 2026-09-19): `vbaproject.spec` is a dormant 21-line
   stub — its only include (out_json.c:2408) is commented out, so VBAProject
   is never emitted; `signature.spec` does not exist in the tree (its
   include sites — decode.c:3041, out_json.c:2578 — are inside `#if 0`);
   `appinfohistory.spec` is referenced only from a commented-out include
   and does not exist. The repo-root `libredwg.spec` is the RPM packaging
   spec, unrelated. Do not chase any of these for field-level work.

1. **Never read a whole large file.** These files are too big to load:
   - `~/work/libredwg/src/dwg.spec`, `dwg2.spec` (many thousand lines)
   - `src/io/dwg/dwg_stream_readers/object_reader/entities.rs` (~6000 lines)
   - `src/io/dwg/dwg_stream_writers/object_writer/entities.rs`
   - `src/io/dwg/dwg_stream_readers/object_reader/objects.rs`,
     `.../object_writer/objects.rs`
2. **Always locate with grep, then read a narrow window.** Patterns:
   - Gold object block: `grep -n "DWG_OBJECT (NAME)" ~/work/libredwg/src/dwg2.spec ~/work/libredwg/src/dwg.spec`, then `sed -n '<start>,<start+80>p' <file>`.
   - Gold entity block: same with `DWG_ENTITY (NAME)`.
   - **Grep both `dwg.spec` AND `dwg2.spec`** — R2000+ objects (SCALE, XRECORD,
     VISUALSTYLE, MATERIAL, MLEADERSTYLE, TABLESTYLE, MULTILEADER, LAYOUT is
     in dwg.spec) live in `dwg2.spec`; tables/entities/older objects in
     `dwg.spec`. Grep both every time.
   - **Nested field? Grep the macro.** A dotted path like `FIELD.value.x` or
     `TABLESTYLE.sty.cellstyle.x` is defined by a `*_fields` macro — grep
     `#define <macro>` across `dwg.spec dwg2.spec dwg_spec_shared.h`
     (TABLE_value_fields and WIRESTRUCT_fields are in the header).
   - **Underscore aliases**: gold names `3DFACE`→`_3DFACE`, `3DSOLID`→`_3DSOLID`,
     `3DLINE`→`_3DLINE` in the spec. If `DWG_ENTITY (3DFACE)` finds nothing,
     grep `DWG_ENTITY (_3DFACE)`.
   - Common entity fields: `grep -n "field_name" ~/work/libredwg/src/common_entity_data.spec ~/work/libredwg/src/common_entity_handle_data.spec`, then `sed -n` around the hit.
   - Silver reader: `grep -n "fn read_<name>" src/io/dwg/dwg_stream_readers/object_reader/*.rs`, then read at most that function (`sed -n '<line>,<line+120>p'`).
   - Silver writer: `grep -n "fn write_<name>" src/io/dwg/dwg_stream_writers/object_writer/*.rs`.
3. **Do not re-read the plan.** This section contains everything needed.
4. **Do not dump whole JSON outputs** (`*_gold_orig.json` can be MBs). Query
   them with `python3 -c` one-liners or small scripts that print only the
   records of the type under repair.
5. Read `diff_*.json` only via filtered queries (see 8.1.3), never in full.

### 8.1.2 The work unit: one (type, field) packet

Work exactly ONE `(type, field)` pair per iteration. Never batch unrelated
fields. The packet comes from the corpus report
(`target/gold_harness_corpus/report.md`, "Top read-fidelity divergences"
first) or from a single file's diff JSON.

Quick per-file query (prints only the interesting rows). The corpus workdir is
`target/gold_harness_corpus/<stem>/<stem>_diff_orig.json`; if you ran a single
file yourself it is `<out_dir>/<Filestem>_diff_orig.json`:

```bash
python3 - <<'EOF'
import json
d = json.load(open("target/gold_harness_corpus/FILESTEM/FILESTEM_diff_orig.json"))
rows = [x for x in d["diffs"] if x.get("type") == "TYPE"]
print(f"{len(rows)} diffs for TYPE")
for x in rows[:20]:
    print(" ", x["kind"], x.get("field"), "| gold:", x.get("gold_value"), "| silver:", x.get("silver_value"))
EOF
```

### 8.1.3 Where a fix belongs — decision tree

Given a diff `(type, field, kind)`:

1. **`missing_in_silver` and the field IS in the gold spec for this version**
   → silver does not read/store it. Fix in the **reader + writer + struct**:
   - Struct: `src/entities/mod.rs` (`EntityCommon` / per-entity struct) or
     `src/objects/*.rs` or `src/tables/*.rs` for table records.
   - Reader: `object_reader/{entities,objects,tables}.rs` or `mod.rs`.
   - Writer: the same path under `object_writer/`.
   - Gate with the SAME version predicate gold uses:
     `version.r2000_plus()`, `r2004_plus()`, `r2007_plus()`, `r2010_plus()`,
     `r2013_plus(dxf)` on `DwgVersion`; `self.version.*` on the writer.
   - If the field is serde-skipped storage data, also add it to the dump
     struct in `tests/gold_harness/src/bin/dwg2json.rs`
     (`SilverEntityCommon` pattern: `Option`, `skip_serializing_if`).
2. **`missing_in_silver` and the field is DERIVABLE from silver state**
   (e.g. `is_xdic_missing` = `xdictionary_handle.is_none()`,
   `has_full_visualstyle` = `full_visual_style_handle.is_some()`,
   `z_is_zero` = both Z coords are 0) → fix in the **dump** (`dwg2json.rs`),
   version-gated with `doc.version >= DxfVersion::AC10xx`. Do NOT add storage
   to the core struct for derivable bits.
3. **`extra_in_silver`** → silver emits a field gold does not have in this
   version. Fix in the **dump** (version-gate to `None`) or in
   **`normalize_silver.py`** (drop derived silver-only fields like the
   `linetype` name string).
4. **`wrong_value` with matching structure** → silver reads/writes the field
   but wrong. Fix the **reader or writer** body only. Check bitcode type
   against gold's `FIELD_*` macro (e.g. `BS` vs `BL`, `BD` vs `RD`).
5. **`wrong_value` that is a naming/shape mismatch, not data** (e.g. gold
   `extrusion` vs silver `normal`; gold CMC color hash vs silver `Color`
   enum) → fix in **`normalize_silver.py`** (`FIELD_NAME_MAP`,
   `merge_common`, or `normalize_color`) or **`normalize_gold.py`** (gold-side
   canonicalization, e.g. collapsing `{index, rgb:"000000", flag}` to
   `index`).
6. **Handle-valued field** → the differ compares by resolved target type.
   Fix the name mapping so the field is present on both sides; do NOT try to
   make raw handle ids equal.

### 8.1.4 Fix recipe (per packet)

1. **Consult gold** (grep + sed window per 8.1.1). Record: exact `FIELD_*`
   macro, bitcode type, version gate (`PRE/VERSIONS/SINCE/UNTIL`), and any
   condition (`if (FIELD_VALUE (x) == 3)` …). Fields guarded by
   `DXF { … }` are DXF-only — do NOT implement them for DWG.
2. **Locate silver** (grep for the field name or the enclosing
   `read_<entity>`). Determine with the decision tree (8.1.3) whether the fix
   is reader+writer, dump, or normalizer.
3. **Edit minimally.** Only the per-field read/write body or the projection.
   Never touch version dispatch, decompression, CRC, or the section map.
   Never edit `ignore_fields.toml`, `diff_fields.py`, or gold.
4. **Build:** `cargo build --features serde --bins`.
5. **Verify on one file per affected version** (all versions the field's
   gold gate covers), e.g. for an R2000+ field:

   ```bash
   for v in 2000 2004 2007 2010 2013 2018; do
     python3 tests/gold_harness/run_roundtrip.py \
       "$GOLD_TESTDATA/$v/Line.dwg" "target/gold_harness_wip/$v" >/dev/null 2>&1
   done
   ```

   Then confirm with the query from 8.1.2 that (a) the target diff is gone on
   every covered version and (b) `total_diffs` did not increase anywhere.
6. **Regression gate (must all pass):**
    - `cargo test --features serde 2>&1 | grep -E 'test result:' | awk '{p+=$4; f+=$6} END {print p, f}'` → failed = 0
      (this is the **authoritative** gate; baseline 1556/0)
    - `cargo test --features gold-harness --test gold_roundtrip` → ok
    - Plain `cargo test` will fail to compile `examples/entity_atlas.rs`
      (pre-existing, unrelated — see §7 "How to start cold"). Do NOT treat that
      specific E0432 as a regression; check that the *test* targets pass.
    - `target` diff totals strictly decreased (or unchanged if the packet
      targeted a type absent from the representative files — then verify on the
      affected file directly).
7. **Checkpoint:** `git add -A && git commit -m "fix(dwg): <type>.<field> — <gold spec ref>"`
   if git mutations are allowed; otherwise append a line to
   `target/gold_harness_corpus/report.md` under a `FIXED` heading.
8. If any gate fails: `git checkout -- <touched files>` (or restore the saved
   copy), record `BLOCKED: <type>.<field> — <reason>`, and move to the next
   packet. Max **5 attempts** per packet.

### 8.1.5 Reference patterns (already landed — imitate these)

- **Version-gated field add** (`bd2007_45`, VisualStyle): reader and writer
  each gained `if version.r2007_plus() { … }` around a single bit-double;
  regression test asserts 23 stored properties on R2000 vs 24 on R2007
  (`tests/visualstyle_dwg_roundtrip.rs`).
- **Dump-side derivation** (`is_xdic_missing`, `has_*_visualstyle`): added as
  `Option<bool>` to `SilverEntityCommon` in `dwg2json.rs`, set with
  `r20xx.then_some(...)`, `skip_serializing_if = "Option::is_none"`.
- **Normalizer vocabulary fix** (`entity_mode`→`entmode`,
  `linetype_handle`→`ltype`, `ownerhandle` only when `entmode == 0`):
  `merge_common()` in `normalize_silver.py`.
- **Gold-side canonicalization** (ENC color hash collapse):
  `normalize_value()` in `normalize_gold.py`.

### 8.1.6 Current work queue (ordered)

> **Next major arc (planned 2026-09-25, not yet started): the header &
> whole-structure campaign — §19.** The OBJECTS axis (this queue's
> completed 0/0 target) extends to a second structure axis: the 17
> top-level keys the normalizers currently drop (FILEHEADER, HEADER
> variables, R2004_Header, SecondHeader/AuxHeader, SummaryInfo,
> AppInfo/History, Template, FileDepList, RevHistory, Security,
> ObjFreeSpace, THUMBNAILIMAGE, AcDs, CLASSES, created_by), with
> separate corpus counters and the whole-structure audit matrix (H6)
> as the standing deliverable. First packet: H0 (the axis skeleton +
> the day-one census). The OBJECTS axis is frozen at 0 throughout.
> Before this arc: check the live decoder rows (§18/next-session
> notes) — they may be interleaved.

> **Reading the counts:** corpus `report.md`/`report.json` counts are
> stem-collision inflated (§7 "How to start cold"). Use them for *ranking*
> only; verify the true per-file count with the §8.1.2 query on a concrete
> file before committing to a packet. Current baseline (2026-09-21,
> after the Surface zero-out batch `9b1738c`): read **0** / write **0**,
> **124 of 124 files at 0/0** — THE CAMPAIGN TARGET (read AND write 0
> on BOTH sides, raised 2026-09-20) IS COMPLETE and THIS QUEUE IS
> EMPTY. Any row that resurfaces after a codec change goes back through
> the per-packet §8.1.2 workflow. (Historical context: the SURFACE-
> retypes batch `b2955a7` left read **479** / write **455**, the two
> sides MIRRORING family-for-family (the
> phantom-class un-masking made the write diff honest; the UNKNOWN
> projections took −308/−324; the U2 retype map −719/−720; the
> unknown_bits side channel −448/−448; the style-map/chain/MLINE batch
> −327/−319; the POLYLINE_3D/GROUP/PFACE/Associative-retype batch
> −478/−493).
> Next-packet handoff: the "Next packets" block in §7. Remaining mass:
> UNKNOWN_OBJ._missing 53 + ownerhandle 37 (ownercode side channel + the
> reader-ownercode gap), RAY/XLINE/LINE.linewt ~100, IMAGEDEF ~55, HATCH
> ~27, MULTILEADER.graphic_data 12, CIRCLE.linewt 14, VX reader family,
> the poly-SEQEND shadow pairs and the 3140/TS1 dropped-record reader
> gaps (probes `r2_*`/`rt_*`). PROXY_OBJECT data/data_numbits + objids
> landed (2026-09-20 seventh batch, `fa2cb0a`; full LibreDWG-source parse
>   evidence in `target/probes/fullsrc/` — see the DONE entry below).
>   DIMSTYLE_CONTROL.morehandles landed (2026-09-20 eighth batch,
>   `b08d346`), the LEADER R2000-pair family landed (2026-09-20
>   ninth batch, `b6e6e92`), the MULTILEADER attach trio landed
>   (2026-09-20 tenth batch, `45382ec`; MULTILEADER rows zero on both
>   sides), the UNKNOWN-family dropped-records/TABLECONTENT retype
>   landed (2026-09-20 eleventh batch, `b123b4c`), the VERTEX_MESH
>   dropped records landed (2026-09-20 twelfth batch, `fca5367`),
>   the ACSH_CONE_CLASS retype landed (2026-09-20 thirteenth batch,
>   `fad3042`), the WIPEOUT/IMAGE imagedefreactor wire codes landed
>   (2026-09-20 fourteenth batch, `543f877`), the SEQEND real-handle
>   retention landed (2026-09-20 fifteenth batch, `689b14d`), the
>   VIEWPORT.status_flag raw retention landed (2026-09-20 sixteenth
>   batch, `263ab5f`), the LEADEROBJECTCONTEXTDATA retype landed
>   (2026-09-20 seventeenth batch, `9f06d89`), the TOLERANCE
>   field-name projection landed (2026-09-20 eighteenth batch,
>   `a7e451b`) and the smalls batch landed (2026-09-20 nineteenth
>   batch, `257895a`: MTEXT columns, GROUP.name, VISUALSTYLE BS sign,
>   3DFACE z_is_zero, MLINE CLOSED), the TRACE/SOLID wire-type split
>   landed (twentieth batch, `91e72a3`), the MESH/PDFUNDERLAY
>   graphic_data pops landed (twenty-first batch, `e90fb77`), the
>   SORTENTSTABLE.ents emission landed (twenty-second batch,
>   `ab0e02f`) and the SURFACE family retypes + PLANESURFACE
>   projection landed (twenty-third batch, `b2955a7`) — see their
>   DONE entries below.

  The evening session then landed the big-family projection wave
  (2026-09-20 batches 24-32, `82f0225`), the Dynblocks/PolyLine2D wave
  (batches 33-38, `078c119`) and the SECTION trio retype (batch 39,
  `5a25bf9`) — see the entries below.

**FIELD/FIELDLIST projection** — **DONE (2026-09-20 twenty-fourth
  batch; TS1 89/89 → 34/34)**: pure name projection in the objects
  loop — id ← evaluator_id, field_state ← state, evaluation_error_msg
  ← evaluation_error_message, value → the dotted TABLE_value_fields
  union (value.data_type/data_long, BLEND of the ~0x200 mask pre-2007),
  childval [0]*len(child_values) (the normalize_gold dict-collapse
  rule), childs the code-3 handles only when num_childs > 0,
  referenced_objects popped; FIELDLIST.fields as the code-4 handle
  list.

**PLOTSETTINGS projection** — **DONE (twenty-fifth batch; gh109_1
  66/64 → 19/17)**: printer_cfg_file = the page-setup NAME, paper_size
  = the DEVICE name, canonical_media_name = the media (the LAYOUT
  transposition), four-margin split from the dict, plot_flags from
  PlotFlags::to_bits (identical layout to the Layout branch), the
  enum strings inverted to gold's BS codes, plotview/popname-era
  gating, shadeplot = silver's visual_style_handle (objects.rs reads
  the R2007+ handle there), cached_scale popped.

**GEODATA R2010+ projection** — **DONE (twenty-sixth batch; gh209_1
  48/48 → 0/0)**: the full rename set (class_version/coord_type/
  design_pt/ref_pt/unit pairs/up_dir/north_dir/scale_est/corrections/
  radius/definition), the obsolete pair + datum/wkt + civil_* popped
  (the UNTIL(R_2007) branch never runs on the corpus's R2010 record),
  mesh REPEATs [0]*n when non-empty.

**Underlay PDFUNDERLAY/PDFDEFINITION** — **DONE (twenty-seventh batch;
  2004/Underlay 52/52 → 0/0)**: the per-kind UnderlayDefinition retype
  must run AFTER the Associative if/else (OBJECT_TYPE_MAP's
  PDFDEF default previously overwrote it — the count_mismatch pocket);
  PDFDEFINITION filename ← file_path, name ← page_name, silver's
  `name` twin popped; reactors retained on the struct (the Dictionary
  precedent — the builder had dropped the parsed common data);
  PDFUNDERLAY: definition_id, ins_pt, angle, the [x,y,z] scale triple
  (3BD_1), flag RC from the bitflags string (CLIPPING=1/ON=2/
  MONOCHROME=4/ADJUST_FOR_BACKGROUND=8/CLIP_INSIDE=16 — verified
  30/31/27), clip_verts 2RD pairs, clip_inverted popped (the
  CLIP_INSIDE bit already counted).

**MESH raw trailing bits + PLANESURFACE reader tail** — **DONE
  (twenty-eighth batch; 2004/Surface 62/62 → 0/2)**: unknown_b1/b2
  read as the two FIELD_Bs after the crease vector (the BL read
  mis-shaped the tail; writer echoes), dlevel = the BS 71 version,
  is_watertight = blend_crease per the spec comments, faces the flat
  [n, *indices] vector, edges the [0]*num_edges collapse (the REPEAT
  struct entries collapse per the normalize_value dict rule — verified
  19 and 31), crease the per-edge BDs. PLANESURFACE: modeler_format_
  version/u_isolines/v_isolines read as the three BS after the
  COMMON_3DSOLID tail (silver read the counts two fields early, its
  constant-1 modeler lost gold's 6 — 2004/Surface 4→0 read), the SAB
  banners split at the fixed 15-byte prefix (json_3dsolid "%.*s"),
  acis_empty_bit from the retained wire bit, isolines from
  acis_data.wireframe_isolines (not u+v).

**SHAPE + TEXT-family raw dataflags** — **DONE (twenty-ninth batch;
  entities-2d/3d 19/17 each → 0/0)**: silver's read_shape slot map is
  size = the wire scale (40) and relative_x_scale = the wire
  WIDTH FACTOR (41) — the R13+ DWG order ins/scale/rotation/
  width_factor/oblique/thickness/style_id/extrusion; the normalizer
  emits gold's names, style from style_handle, the shape_name/
  style_name twins popped. The TEXT/ATTRIB/ATTDEF absence mask:
  retain the raw RC dataflags byte (the wire bits beat any value
  recomposition — an explicit width_factor 1.0 keeps bit 4 clear),
  alignment bit per LibreDWG dwg_set_dataflags (alignment == insertion,
  NOT merely zero — entities-2d carries a real all-zero [0,0] pair),
  elevation from insertion_point.z, the writer writes the retained
  byte, deep-gate arms clear it on all three child types.

**OLE2FRAME raw blob** — **DONE (thirtieth batch; TS1 16/16 → 5/5,
  removing the OLE2FRAME 11-row family)**: retain the raw wire bytes
  (Ole2Frame.raw_data; the decoded CFB re-encode is not byte-identical
  and gold prints the 315 KB blob as the `data` hex string), writer
  echoes raw over re-encode, type from the ole_object_type variant,
  mode ← dwg_mode, the structured envelope/storage twins popped,
  deep-gate arm clears the vec.

**encr_sat_data per-block retention** — **DONE (thirty-first batch;
  TS1 and 2000/Cone to 0/0, example_2000 encr rows dead)**: the
  pre-2004 SAT blocks are BL-sized; capture each raw block before the
  159-cipher decode (the decode is lossy), keep Vec<Vec<u8>> on
  AcisData, writer echoes the blocks verbatim, normalizer emits one
  uppercase-hex string per block (out_json json_3dsolid shape).

**Entity-common pair order + SEQEND flag truth** — **DONE
  (thirty-second batch)**: plotstyle_flags reads IMMEDIATELY after
  ltype_flags (common_entity_data.spec 507-511) — the old
  material-first order mislabeled the pairs inside an isochronous
  12-bit window (no desync, wrong names — the example_2007 LWPOLYLINE
  read its plotstyle 1893 under `material`); the shadow handle slot
  between material and plotstyle is consumed on flags==3; the writer
  pair order mirrors the wire. The SEQEND common flags are the WIRE's
  per-record data, not a convention: retain plotstyle/shadow flags
  per SEQEND entity in the builder (pending.seqend_flags keyed by
  owner), stash early in the normalizer (no generic-loop leak) and
  emit verbatim — LibreDWG-authored example files carry flags-3-with-
  the-[5,0,0,0]-null, DWG-native chains (PolyLine3D.dwg etc.) carry 0.
  The poly-SEQEND writer sites emit era flags (plotstyle 3 in the
  R2004 band, shadow 3 R2007-R2013) so gold_rt decodes the rewrite.
  Also POLYLINE_3D.flag from the early-captured bit dict (the pop ran
  before the kid block read it), POLYLINE_PFACE vertex vector SINCE
  R2004a (BL num_owned + code-4 vector of ALL owned kids), DIMENSION
  class_version gated SINCE(R_2010b), MTEXT \U+XXXX escape decode
  WITH the payload `value` pop (the FIELD_NAME_MAP re-map overwrote
  the decoded text otherwise).

**Dynblocks/PolyLine2D wave** — **DONE (2026-09-20 batches 33-38,
  `078c119`)**: (33) LwPolyline raw wire flag retained end-to-end
  (dwg_raw_flag; the composition loses VERTEXIDCOUNT 0x400 and the
  presence bits), vertexids emitted from the retained 0x400 with the
  4-int raw-handle-tuple mirror; (34) SPLINE ctrl_pts = [0]*len
  (per-point collapse, not the x/y/z triple expansion); (35)
  BLOCK_HEADER xref_pname ← the table record's xref_path and the
  anonymous dupe-name strip made lazy (greedy kept *D1 for *D10);
  (36) POLYLINE_2D parent projection (flag from the bit dict,
  curve_type from the smooth enum, first/last vertex + seqend from the
  kid-handle derivation; the pop branch must sit BEFORE the generic
  loop — a first attempt landed in the kid area and leaked); (37)
  LAYOUTPRINTCONFIG retype of the Extended/LayoutPrintConfig wrapper
  (class_version/flag + the R2000 null chain trio + graphic_data
  pop); (38) the child VERTEX and poly-SEQEND writers inherit the
  parent's linetype (the wire kids carry the parent pair — the
  ByLayer hardcode flipped 6 rt rows).

**SECTION trio retype** — **DONE (thirty-ninth batch, `5a25bf9`;
  LiveSection1 16/14 → 4/2)**: SECTION_MANAGER (is_live + sections
  handles) and SECTION_SETTINGS (curr_type + types [0]*n per the
  collapse rule) from the ClassObject wrapper, SECTIONOBJECT from the
  Extended wrapper (state/flags/name/vert_dir/top+bottom_height/
  indicator pair/verts/section_settings handle/back_line_verts;
  preview* ignore-listed). The 426/428 SectionViewStyle/
  DetailViewStyle stay silver-side UNKNOWN_OBJ per the liveness map.

  ~~SURFACE family retypes + PLANESURFACE projection~~ — **DONE
  (2026-09-20 twenty-third batch, `b2955a7`; read 496 → 479 / write
  473 → 455, −17/−18; Surface.dwg 47→31 read, 50→33 write)**:
  silver's five Surf entities map kind-for-kind onto gold —
  Extruded/Lofted/Revolved/Swept are gold's DEAD-frame records
  (§8.1.1, dwg2.spec 3716-4513 → raw UNKNOWN_ENT, common-only in
  normalize_gold — verified field-by-field), kind Plane is gold's
  LIVE PLANESURFACE. The retype poisons every handle-vector that
  resolves one of them (SORTENTSTABLE sort_ents/ents targets on
  Surface AND gh109_1, ASSOCDEPENDENCY.dep_on). Fix: the Surface
  entity mapping kind→PLANESURFACE, others→UNKNOWN_ENT with the
  payload popped to common-only; the PLANESURFACE ENTITY-loop branch
  projects silver's Plane payload 1:1 (SAB bytes → [ascii-prefix,
  hex-rest] per gold's acis_data shape; u/v isolines + sum + present
  bit; point_of_reference → point; degenerate [0]*n wires; unknown
  bits via the pre-registered _UNKNOWN_BITS_TYPES channel emission).
  REMAINING sub-knots (recipes above): the one PLANESURFACE record's
  acis_data SAB boundary detail, modeler_format_version 6-vs-1 and
  v_isolines 8-vs-6 (silver's Plane reader misparses two fields),
  and gh109_1's ORIG sortent rows need NULL-ENTRY RETENTION in
  silver's sort-table reader (gold keeps the zero pairs at ~3
  indices; silver drops most, shifting ordinals).

  ~~TRACE/SOLID wire-type split + graphic_data pops + SORTENTSTABLE.ents~~ — **DONE (twentieth–twenty-second batches, `91e72a3`,
  `e90fb77`, `ab0e02f`)**: (a) TRACE (wire code 32) vs SOLID (31) —
  silver folded both into Solid with is_trace; the normalizer now
  types is_trace payloads TRACE (entities-2d/3d −23/−23 each on the
  corner rotations; TS1's residual SOLID/TRACE count rows die too).
  (b) MESH/PDFUNDERLAY graphic_data leaks via merge_common's
  _common_dwg path (the UNKNOWN_ENT precedent) — gold never emits
  them (census Surface/Underlay). (c) gold dumps BOTH sort-table
  handle vectors on DWG (code-0 sort_ents + code-4 ents, zipped in
  silver's entries order — pair-verified on Surface@719); the old
  "DXF-only, never serialized" claim was wrong.

  ~~smalls batch: MTEXT columns / GROUP.name / VISUALSTYLE BS sign /
  3DFACE z_is_zero / MLINE CLOSED~~ — **DONE (2026-09-20 nineteenth
  batch, `257895a`; read 575 → 558 / write 598 → 536, −17/−62)**
  — five independent one-liner families, recipes in the commit:
  (a) MTEXT columns: the nested column_data projection now emits
  gold's flat names (width→column_width, heights→column_heights) and
  skips silver-only column_count; carrier 2018/Text.dwg only (the
  Text stem exists in every version dir — always fresh-probe the
  right era). (b) GROUP.name: the wire CARRIES group names
  (dwg.spec 4498 FIELD_T (name, 300); 2018/Leader is "Superhatch") —
  silver's reader stores it under the misnomered `description`
  payload key (GroupData.description; the writer writes it back, rt
  faithful) — project it; the old always-"" emission was a bad
  census. (c) VISUALSTYLE Short-variant values mask with 0xFFFF
  (gold FIELD_CAST (…, BS, BL) zero-extends; 0xFFCE = 65486 vs
  silver's -50; the MATERIAL-rgb unsigned lesson re-applied).
  (d) 3DFACE: gold's ENCODER derives z_is_zero from corner1.z alone
  (dwg.spec 2125) — the writer's AND-of-four disagreed with silver's
  own normalizer derive on the rt pair (8 rows on 2013/gh109_1 —
  in-scope like gh209_1; only gh44-error is excluded). (e) MLINE:
  the reader parsed the openclosed BS but the builder DROPPED it —
  retain the CLOSED bit (dwg.h: HAS_VERTEX=1 | CLOSED=2; the writer
  already derived its wire value from MLineFlags::CLOSED); the
  normalizer splits bitflags' serde joined strings on " | ".
  LESSON: never put backticks in bash-embedded commit-message
  strings (one got command-substituted in this commit's body).

  ~~TOLERANCE field-name set~~ — **DONE (2026-09-20 eighteenth batch,
  `a7e451b`; read 638 → 575 / write 661 → 598, −63/−63 — far more
  carriers than the queue's "2010/Leader" note: example_2000, TS1 and
  the Leader files all carry TOLERANCE entities)**. gold dwg.spec 3058:
  the R2000+ wire carries only 3BD ins_pt, 3BD x_direction, extrusion,
  T text_value (the unknown_short/height/dimgap trio is
  VERSIONS (R_13b1, R_14)-only). Silver's Tolerance struct uses
  insertion_point/direction/text with IDENTICAL values plus
  text_height/dimension_gap/dwg_unknown_short unconditionally — every
  record produced 3 missing_in_silver + up to 6 extra_in_silver rows
  (name mismatches with equal values — pure projection). Fix: the
  Tolerance branch projects the three names (normalize_value for 3BD)
  and pops the R13/R14-only trio on R2000+ (faithful pre-R2000 emission
  kept behind the version gate).

  ~~LEADEROBJECTCONTEXTDATA/OBJECTCONTEXTDATA typing~~ — **DONE
  (2026-09-20 seventeenth batch, `9f06d89`; read 662 → 638 / write
  685 → 661, −24/−24, the typing pair on every version-folder
  Leader.dwg)**: gold decodes the class-519 ACDB_LEADEROBJECTCONTEXTDATA_
  CLASS records (dwg2.spec 4611, live; AcDbAnnotScaleObjectContextData_
  fields + points vector + x_direction + b290 + inspt_offset +
  endptproj, HANDLE_UNKNOWN_BITS tail) as typed LEADEROBJECTCONTEXTDATA;
  silver parses the same wire into its generic ObjectContextData wrapper
  (kind.Leader) with a 1:1 field correspondence (endpoint_projection→
  endptproj, insertion_offset→inspt_offset, annotation_enabled→b290).
  Fix: the objects-loop branch retypes kind.Leader payloads to
  LEADEROBJECTCONTEXTDATA (constant dxfname per the established typed-
  retype practice), projects all fields, pops the payload; the
  R2018b unknown-bits tail comes from the reader side channel
  byte-identical (LEADEROBJECTCONTEXTDATA was already registered in
  _UNKNOWN_BITS_TYPES from an earlier survey — the loop-end emission
  covers it). Non-Leader kinds keep the generic projection — the
  FCFOBJECTCONTEXTDATA subclass (dwg2.spec right after) stays generic
  until a carrier shows it.

  ~~VIEWPORT.status_flag raw retention~~ — **DONE (2026-09-20 sixteenth
  batch, `263ab5f`; read 679 → 662 / write 685 stays, −17 read rows,
  the sole carrier is 2018/Dynblocks.dwg)**: gold keeps status_flag as
  a plain BL (dwg.spec 2484, SINCE R_2000b, right after
  num_frozen_layers). Silver's reader captured the raw i32 all along
  (read_viewport: data.status_flags) but the builder decomposed it
  into the typed ViewportStatusFlags (bits 0-15 only) and the writer
  recomposed from the typed bits, dropping the higher bits (gold
  0xC8060-style values → silver 0x8060; Dynblocks 819232 vs 32800).
  Fix: the raw-retention pattern — Viewport.dwg_status_flag:
  Option<i32> from the wire (None on constructed documents), builder
  stores Some(data.status_flags), writer echoes raw-with-typed-fallback,
  normalizer prefers raw (r2000+ gate) with the typed recomposition as
  fallback, and tests/roundtrip.rs's normalize_entity_for_comparison
  carries the Viewport arm (the seqend_handle precedent: writer-
  allocated/raw-retained splits are not semantic equality). Write rows
  never appeared for this family: the rt pair read the same recomposed
  wire on both sides — self-consistent but unfaithful to the original
  bits until this fix. NOTE: the old queue's "Dynblocks R2018/R2000"
  names were wrong — Dynblocks.dwg exists ONLY in 2018/. A stray
  Cone.json (previous-session pollution, Sep 20 09:18) was removed
  from the read-only gold tree during verification.

  ~~SEQEND.ownerhandle (synthesized seqends)~~ — **DONE (2026-09-20
  fifteenth batch, `689b14d`; write 703 → 685, −18 write rows on the
  six example_* carriers; read unchanged; the queue's entmode/code-4
  hunch was NOT the cause)**: the rows were ORDINAL ROTATIONS. The
  wire's SEQENDs are real records with real handles (gold's
  example_2000 list sorts 401/1051/1262/1881); silver's synthesis
  invented PSEUDO handles (parent+1 / last-child+1 conventions), and
  one wrong ordinal rotated the (type, ordinal) pairing of every
  SEQEND in the file — gold SEQEND[1] owns the PFACE where silver's
  [1] owned the 3D-poly, etc. PolyfaceMesh/Insert already stored
  their real seqend_handle; Polyline2D/Polyline3D/PolygonMesh had no
  field at all. Fix: seqend_handle: Option<Handle> on the three
  structs (constructors None, serde-visible); every polyline assembly
  arm restores pending.seqends[owner] (the OBJ_SEQEND dispatch
  already keys real seqends by owner for all families); the three
  writers echo stored-else-alloc (the PolyfaceMesh pattern);
  normalize_silver's kid block prefers the captured real handle
  (captured AND popped early — BEFORE the generic field loop, else it
  leaks as extra_in_silver), falls back to the conventions; the PFACE
  seqend emission uses the captured variable; and the 3D-family
  vertex extraction also reads the FLAT handle key of silver's
  Vertex3DPolyline — the nested-common-only lookup synthesized
  parent+1 for the first vertex, CLASHING with the real SEQEND@1051.
  tests/roundtrip.rs: the deep-comparer extends the existing
  PolyfaceMesh.seqend_handle normalization to the three new carriers.
  The remaining SEQEND residue (plotstyle/shadow_flags +
  ltype_flags on the synthesized records) PRE-EXISTED this batch.

  ~~WIPEOUT/IMAGE imagedefreactor wire codes~~ — **DONE (2026-09-20
  fourteenth batch, `543f877`; read 679 → 679 / write 715 → 703,
  −12 write rows on the six example_* WIPEOUT pairs; read side
  untouched — rt-parser-parity family)**. THE QUEUE'S DIAGNOSIS WAS
  INVERTED: the rows were not wire-ORDER rows — the writer's handle
  ORDER was already gold-shaped (the merged writer appends handle
  entries in call order and the trailing entity-body writes land
  after the common handles, which is where gold's handle stream also
  puts them: per-entity handles precede COMMON_ENTITY_HANDLE_DATA) —
  the divergence was the CODE NIBBLE: silver wrote BOTH the
  imagedef and imagedefreactor handles as DwgReferenceType::HardPointer
  (nibble 5), while the original wires (and gold's spec,
  dwg2.spec 1578 / dwg.spec 5145 FIELD_HANDLE (imagedefreactor, 3,
  360)) carry the reactor with nibble 3 ([3,0,0,0] verified in
  every record; imagedef is [5,0,0,0]). The ORIG pair never showed
  rows because silver's normalizer FABRICATED the constant {code: 3}
  for absent reactor handles — gold's raw 3 coincidentally matched
  the stamp; silver's own rt read restamped 3 over its writer's 5,
  and only gold_rt exposed the truth (5 vs 3). Fix: (a) writer —
  write_wipeout AND write_raster_image (the IMAGE sibling has the
  identical wire; it was equally wrong but unexercised — no IMAGE
  entities in the corpus) now write the reactor as
  DwgReferenceType::HardOwnership (nibble 3); imagedef stays
  HardPointer 5; (b) normalizer — both branches emit the NULL form
  (normalize_handle_value(0), code=None) for absent handles instead
  of fabricating constants, and the RasterImage branch also stops
  silently dropping the field. LESSONS: read the ROW VALUES (gold vs
  silver) before trusting a queue diagnosis — gold_value {code 5} vs
  silver_value {code 3} with the ORIGINAL wires showing [3,0,0,0]
  pinned the writer, not the normalizer, in three probes; and a
  fabricated constant that happens to match gold's raw value on the
  ORIG pair is a time bomb for the RT pair.

  ~~ACSH_CONE_CLASS retype (Cone.dwg)~~ — **DONE (2026-09-20
  thirteenth batch, `fad3042`; read 683 → 679 / write 719 → 715,
  −4/−4 on 2000/Cone.dwg)**: gold types the class-519 record
  (handle 524) as ACSH_CONE_CLASS (dwg2.spec 2978 — live frame, the
  same AcDbShPrimitive region as the landed cylinder: AcDbEvalExpr +
  AcDbShHistoryNode + height/major_radius/minor_radius/x_radius BD
  quartet). Silver's DynamicBlock wrapper parses it under
  data.SolidHistoryNode.Cone with base node + operation majors +
  height/base_x_radius/base_y_radius/top_radius (15/5/5/0); the class
  was on the deliberately-deferred "ACSH sphere/cone" list. Fix:
  added ACSH_CONE_CLASS to _DYNBLOCK_RETYPE + the cylinder-shaped
  elif in the landed ACSH projection (major_radius=base_x_radius,
  minor_radius=base_y_radius, x_radius=top_radius — value-verified
  1:1). Remaining Cone.dwg rows are unrelated residue:
  3DSOLID.encr_sat_data (newline family) and
  VISUALSTYLE.edge_silhouette_width (sign boundary).

  ~~VERTEX_MESH dropped records (TS1 mesh parse gap)~~ — **DONE
  (2026-09-20 twelfth batch, `fca5367`; read 695 → 683 / write 731 →
  719, −12/−12, mirrored; sole carrier 2000/TS1.dwg)**: gold has 12
  standalone VERTEX_MESH records (handles 528-539, wire flag 64 =
  POLYGON_MESH, dwg.spec 1199-1220 post-R13b1 order flag RC then point
  3BD). Silver read them all along — the builder dispatches
  `OBJ_VERTEX_3D | OBJ_VERTEX_MESH` to `read_vertex3d` — but the
  normalizer never synthesized the child records: the polyline-family
  kid-capture tuple listed `"POLYGON_MESH"` (the silver struct name)
  where the parent's GOLD record name is `POLYLINE_MESH` (per
  OBJECT/ENTITY_TYPE_MAP), so the capture never fired and `_kid_verts`
  stayed None. One-word tuple fix + one builder fix — the PolygonMesh
  assembly stored `flags: 0`, discarding the wire flag the reader had
  already parsed; retaining `d.flags` makes the synthesized records
  carry gold's 64 AND keeps the writer echo wire-faithful (the writer
  already wrote `v.flags` as RC before the 3BD point, so unfaithful
  zeros were also flowing to the rewrite). Zero new rows: chains/
  point/flag fields all match gold (the pre-2004 first/last chain
  model landed with the other vertex families); entities-2d/3d,
  PolyLine2D, Polyline ×2, PolyLine3D ×2, Polygon, example_2000 row-
  identical to the pre-fix fresh runs (A/B on entities-2d: zero row
  changes). TS1's SOLID._count 3 / TRACE._count 3 ord-shift rows did
  NOT move — separate residue, not mesh fallout. LESSON: the
  kid-capture/synthesis machinery keys on GOLD record names; a silver
  struct name in a tuple is silently dead (no crash, no rows).

  ~~UNKNOWN-family dropped records (the "ex-* handle-3140 class")~~ —
  **DONE (2026-09-20 eleventh batch, `b123b4c`; read 707 → 695 / write
  743 → 731, −12/−12, mirrored; UNKNOWN_OBJ._missing 15 → 9, _count
  9 → 0; the residual rows are the separate Cone/LiveSection/Surface
  typing pockets — re-queued as §7 item 1)**. The family the cold-start
  queue called "ex2010's handle-3140 class" was TABLECONTENT: one gold
  UNKNOWN_OBJ record per example_*.dwg — class 529 (dwg2.spec 466),
  handles 2492 (2000) / 2779 (2004) / 3058 (2007) / 3140 (2010) /
  3220 (2013) / 2206 (2018), a ~8-14.5 KB record whose 116 089-bit
  payload gold dumps raw (gold's DWG_OBJECT(TABLECONTENT) block sits
  inside the `#if defined (DEBUG_CLASSES) || defined (IS_FREE)` frame
  opened at dwg2.spec 297 — the built dwgread decodes it as UNKNOWN_OBJ
  with only the common fields; TABLESTYLE at 965 and TABLEGEOMETRY are
  outside the frame and live, which is why silver projects those
  two). Silver models the record as the typed `TableContent` object
  and its raw wire bits ALREADY sit in the reader's
  `unknown_bits_by_handle` side channel — the record was dropped by the
  normalizer: the handleless-wrapper guard keyed on
  `payload["handle"]`, which silver's TableContent objects never carry
  (their common fields nest under `payload["common"]`), so EVERY
  handled record was skipped as a phantom — and the stale
  `OBJECT_TYPE_MAP["TableContent"] = "ACAD_TABLE"` was dead code
  (gold never emits that name). Fix (normalize_silver.py only):
  (a) `OBJECT_TYPE_MAP["TableContent"] = "UNKNOWN_OBJ"` — mirroring
  the landed TABLE-entity UNKNOWN_ENT rule (dwg.spec 477, same dead
  frame); (b) the guard now resolves the handle from
  `payload["common"]` (falling back to top level) so only truly
  handleless wrappers are skipped, and hoists
  handle/owner/owner_handle/reactors/xdictionary_handle to the top
  level so `_inject_reactors`/`_inject_xdic`/`_object_common_fields`
  and the is_xdic_missing/has_ds_data gates see them; the UNKNOWN
  payload-clear then drops the typed tables payload automatically.
  Verified: all six example carriers −2 read/−2 write each; the
  projected records match gold's normalized UNKNOWN_OBJ field-for-field
  (tolerated `code:null` handle codes; correct per-version gates — no
  is_xdic_missing on 2000, has_ds_data on 2013+). CAUTION for future
  packets: the queued ownerhandle-37 family SELF-RESOLVED — those rows
  were downstream desync of the unbated R2010+ MULTILEADER reader and
  died with `45382ec`; always re-read the by-type table from the FRESH
  report before starting a packet, the queue's numbers go stale.

  ~~MULTILEADER attach trio~~ — **DONE (2026-09-20 tenth batch,
  `45382ec`; read 719 → 707 / write 755 → 743, −12/−12, mirrored;
  MULTILEADER rows zero on BOTH sides — top read rows are now
  VIEWPORT.status_flag 17 / SEQEND.ownerhandle 18 on write)**. Gold
  evidence (dwg2.spec 1355-1458 + `pk4_open_diagnosis.txt`): the
  VERSIONS (R_14, R_2007) block 1418-1445 wraps num_arrowheads+loop,
  num_blocklabels+loop, is_neg_textdir B, ipe_alignment BS,
  justification BS, scale_factor BD — on R2010+ ALL of it is ABSENT and
  the attach trio follows is_annotative directly: SINCE (R_2010b)
  FIELD_BS attach_dir (271) then FIELD_BS attach_top (273) then
  FIELD_BS attach_bottom (272) 1449-1451 — the DXF codes are NOT in
  wire order (273 before 272); the leader-loop trio is
  dwg2.spec 1366-1367 lnode attach_dir SINCE (R_2010b). Silver bugs:
  (a) reader consumed ba_count + labels loop + the four-field tail
  UNCONDITIONALLY — on R2010+ the reads landed on the trio's bits
  (wrong_value: gold attach_top=32/attach_bottom=178 vs silver 9/9
  defaults; the old dir/bottom/top order swap was real but secondary);
  (b) the writer mirrored the desync — writing ba_count + the tail on
  R2010+ desynchronizes gold_rt (the rt wire must carry is_annotative →
  attach trio → is_text_extended(SINCE R_2013b) only; (c) the typed
  TextAttachmentType/TextAttachmentDirectionType enums lose the
  out-of-range raws (32/4786/178; `From<i16>` clamps to MiddleOfText).
  Fix: (a) gate the block-attributes read and the four-field tail
  (entities.rs `read_multileader`) under `!version.r2010_plus()`
  (arrowhead_overrides was already gated) keeping MultiLeader::new()
  defaults for the absent fields (scale_factor 1.0, trio 9/9-until-read)
  — mirror the SAME gate in `write_multileader` (pre-R2010-only
  ba_count/labels/tail write, R2010+ trio write); (b) retain the trio as
  raw `dwg_attach_dir/dwg_attach_top/dwg_attach_bottom: i16` on
  MultiLeader (multileader.rs), builder maps the raws onto the entity,
  and the writer writes the raws verbatim on R2010+; (c) the
  constructor raws MUST mirror the typed defaults (Horizontal=0,
  CenterOfText=9) — raw 0/0 defaults desynced the internal deep
  round-trip tests (dwg_roundtrip_deep_{r2000,r2013,r2018}: R2000
  wrote nothing/read 9s, R2013+ wrote raw 0s and read back
  TopOfTopLine; the suite gate catches raw-vs-typed splits whenever a
  struct carries both), (d) normalize_silver emits
  `attach_dir/attach_top/attach_bottom` verbatim from `dwg_attach_*`
  (R2010+ gate; the VERSIONS(R_14,R_2007) arrowheads/blocklabels/tail
  emissions were already gated `not r2010_plus`) and pops the raws with
  their typed twins. Verified per-file: R2010+ carriers fully clean
  (2010/Leader 16→14, 2013/Leader 7→5, 2018/Leader 7→5, example_2018
  23→21/26→24 — remaining rows are the OTHER queued families:
  GROUP.name, CONTEXTDATA typing, TOLERANCE naming); pre-R2010 carriers
  unchanged (2000/2004/2007 Leader, entities-2d/3d still read the
  group; attach rows never existed there); NO new extra_in_silver rows
  for scale_factor/is_neg_textdir/ipe_alignment/justification/
  num_blocklabels on R2010+.

  ~~PROXY_OBJECT data/data_numbits + objids~~ — **DONE (2026-09-20 seventh
  batch, `fa2cb0a`; read 860 → 847 / write 792 → 779, −13/−13; sole
  carrier 2018/Constraints.dwg × 5 PROXY_OBJECT records: data/data_numbits
  10 + objids 3)**. Groundwork: the full-source parse of LibreDWG
  (`target/probes/fullsrc/`, `findings_notes.md`) pinned the DECODER
  formula and corrected the census (live HANDLE_UNKNOWN_BITS sites = 71,
  all dwg2.spec; dwg.spec:5581 is inside `#if 0`; both PROXY blocks carry
  it commented — proxies emit `data`, never `unknown_bits`). (a) **Raw
  window capture**: gold's `data`/`data_numbits` (dwg.spec 5752 + entity
  5666-5680 DECODER) = `data_numbits = (obj->hdlpos −
  bit_position(dat)) & 0xFFFFFFFF` then `bit_read_bits(dat, numbits)` —
  the raw record bits from after the prologue to the handle-stream start
  in CLASSIC WIRE ORDER (opaque payload ++ the R2007+ string area incl.
  the `dxf_subclass` TU ++ the 17-bit RS size/has_strings trailer ++ pad;
  `obj->hdlpos = obj->bitsize` via `obj_handle_stream`, decode.c:4370 —
  no parsed field set reproduces them). Silver:
  `DwgMergedReader::capture_proxy_window()` slices
  `main[pos .. handle_start_bit)` with bit_read_bits packing (full bytes
  MSB-first, trailing partial byte LSB-packed) into
  `ProxyObject.raw_window` (`ProxyRawWindow{bit_count, bytes}`, serde,
  semantic_property.rs, re-exported in objects/mod.rs); the builder
  captures right after `from_dxf`. normalize_silver's ProxyObject branch
  emits `data` upper-hex + `data_numbits` from it (gold prints %02X via
  FIELD_BINARY/VALUE_BINARY, out_json.c:388; DXF_OR_PRINT is if(1) in
  the JSON TU, spec.h:527) and pops raw_window. Bit-exact on all 5
  (window decompositions 332+314+516+17=1179 etc. — the TU bit count =
  10-bit BS length + 16×chars). (b) **objids read loop = builder 5646**
  replicates gold's mechanics verbatim: terminator
  `while (hdl_dat->byte < hdl_dat->size - 1)` (dwg.spec 5820,
  byte-quantized at record size — `gold_handle_cursor_at_end()` in
  merged_reader.rs, ThreeStream reads the position absolutely, TwoStream
  adds the slice base) AND the **PUSH_HV collapse** (common.h:634: the
  push is skipped when the new ref pointer equals `objids.last()`; since
  `dwg_add_handleref` (dwg.c:2213) dedups by (code,value) returning the
  SAME pointer, CONSECUTIVE EQUAL wire refs collapse — h996 wire
  [990][4,0][4,0][3,0] → gold pushes 3; h995 10 wire refs → 8. The ≥8
  bits-remaining guard stays as a safety net). Both semantics verified
  against all 5 records' handle areas (probe
  `target/probes/fullsrc/pk1_walk.txt`). (c) **Writer unchanged**:
  silver's rt records already reproduce the window byte-exact
  (`gold_rt.data == gold_orig.data`, sizes/bitsizes/objids identical on
  all 5), so both fidelity sides follow the dump emission alone.
  Entity side (PROXY_ENTITY) left as-is: the census found zero carriers;
  gold's entity loop counts without filling the array (count-only,
  position restored) and JSON would emit `[0,0,0]` placeholders —
  revisit only if a carrier appears. CAUTION for future packets: the
  builder's objids loop is SHARED by the RegisteredClass (envelope)
  branch — the terminator + collapse now apply there too; corpus stayed
  clean (no regression), but keep it in mind for ASSOC-family carriers.

   ~~LEADER R2000-pair family~~ — **DONE (2026-09-20 ninth batch,
  `b6e6e92`; read 743 → 719 / write 779 → 755, −24/−24, mirrored)**.
  Gold evidence (dwg.spec 2983-3053): `FIELD_BD (box_height, 40)` +
  `FIELD_BD (box_width, 41)` + `FIELD_B (hookline_dir)` +
  `FIELD_B (arrowhead_on)` + `FIELD_BSx (arrowhead_type)` are wire
  fields at EVERY version; `endptproj` is VERSIONS (R_13c3, R_2007)
  (INCLUDES R2007 — silver's normalize gate `not r2007_plus` wrongly
  dropped AC1021); VERSIONS (R_13b1, R_14) carries [dimasz BD,
  unknown_bit_2/3 B, unknown_short_1 BS, byblock_color BS, bit_4/5 B];
  SINCE (R_2000b) carries unknown_bit_4/unknown_bit_5. Silver bugs:
  (a) box_height/box_width were read gated `<= AC1021` (its
  text_height/text_width fields), desynchronizing EVERY R2010+ LEADER
  (the whole box/hookline/arrowhead/unknown-bit cascade of wrong_value
  rows); (b) the common BS after arrowhead_on was mislabeled
  `dwg_unknown_short1` on R2000+ while gold reads it as
  arrowhead_type; (c) the writer wrote e.dwg_unknown_short1 there and
  dropped the box pair on R2010+, and endptproj at AC1014+ without the
  upper bound; (d) normalize's endptproj gate excluded R2007.
  Fix: reader/writer read/write box pair + arrowhead_type
  unconditionally, endptproj gated (AC1014..=AC1021) in both reader and
  writer, R13-14 extras stay in their branch; builder map unconditional;
  normalize gate `not r2010_plus`. Verified: 2000/2004 Leader 6→5
  (arrowhead_type 8 vs 0 fixed), 2007 7→5 (+endptproj), 2010 26→16
  (+idx-1 arrowhead_on), 2013 12→7, all other carriers (entities-2d/3d,
  gh209_1, gh109_1) show zero remaining LEADER rows. Remaining families
  in the same files: TOLERANCE field-name mismatch set (~9-10 rows on
  2010/Leader — DIFFERENT packet), GROUP.name write rows,
  LEADEROBJECTCONTEXTDATA typing (the MULTILEADER attach trio in the same
  files landed separately as the tenth batch, `45382ec`).

  ~~DIMSTYLE_CONTROL.morehandles~~ — **DONE (2026-09-20 eighth batch,
  `b08d346`; read 847 → 743 / write stays 779 — the row was on EVERY
  carrier with a dims control, one per file, not the ranking's
  truncated 19)**. Gold evidence: dwg.spec 4163-4185 — `FIELD_BS
  (num_entries, 70)`, then SINCE (R_2000b) `FIELD_RCu (num_morehandles,
  71)` (ONE RAW BYTE; dec_macros.h:527 — plain bit_read_RC), then
  CONTROL_HANDLE_STREAM, then `HANDLE_VECTOR (entries, num_entries, 2,
  0)` and SINCE (R_13b1) `HANDLE_VECTOR (morehandles, num_morehandles,
  5, 340)` — "additional hard handles, undocumented" (struct
  dwg.h:3623-3624). Silver fix: (a) reader — the pass-1
  `OBJ_DIMSTYLE_CONTROL` arm (builder ~767) consumes num_entries BS +
  the RCu byte, drains the entries vector, and captures the code-5
  refs into the new document field `dimstyle_morehandles` (document.rs,
  serde, like the other DWG side channels; also ignore it in
  semantic_inventory.rs); (b) writer — `write_dimstyle_control`
  replaced the hardcoded 0 byte with the captured count and writes the
  refs as HardPointer AFTER the entries (never the entries themselves —
  emitting the dim-style table regressed 19 → 109 rows in the 2026-09-20
  attempt); (c) normalize_silver — the table-control emitter projects
  `data["dimstyle_morehandles"]` into the dims CONTROL record's
  `morehandles` (via normalize_handle_value). VERIFY lessons: the
  write side never showed these rows — the write pair is parser parity
  on the rt (gold_rt vs silver_rt), and silver's old rt wrote RCu 0 so
  both parsers agreed on absence; the echo keeps the rt faithful and
  silver_orig ≡ silver_rt. The stash-rerun A/B (probes p45/p46) settled
  the −104-vs-−19 ranking discrepancy: report by-type tables are top-N
  truncated, per-file paths are the truth; corpus dirs are
  stem-collision shared — never A/B them raw (pair by full path).

  ~~POLYLINE_3D family + GROUP + PFACE chains + VIEWPORT named_ucs +
   ASSOC actionbody/path retypes~~ — **DONE (2026-09-20 third batch,
   `bd02a3e`; read 1 931 → 1 453 / write 1 805 → 1 312, −478/−493)**,
   all normalizer work: (a) **POLYLINE_3D parent fields** — curve_type
   (0 on every corpus record), flag (1 closed, 4 spline-fit; the 3D/
   mesh type bits are implied by the record type), kid links: R2004a+
   `vertex` handle vector (gold code 3) + `seqend`; pre-2004 first/
   last_vertex (code 4); silver's width/mesh/smooth/elevation/extrusion
   storage fields popped BEFORE the generic loop (popping after left
   extra_in_silver rows). VERTEX_3D chains: gold's LAST vertex chains
   back (prev = previous kid, code 8) — verified 2000-era PolyLine3D/ex.
   (b) **GROUP** (dwg2.spec): the wire name T is ALWAYS "" (even for a
   named `GROUPNAME` group — names live in the owning dictionary);
   `groups` = the member handle vector (silver `entities`, same order);
   no description/entities fields on DWG. (c) **PFACE kids**: verts
   chain ONLY at the first vertex (middles+last bare nolinks=1 — unlike
   MESH which chains both ends, unlike 3D which chains back at the
   end); FACE records: constant flag 128 (census 111/111), pre-2004
   chains: only the LAST face prev=previous face, all others nolinks=1.
   (d) **VIEWPORT.named_ucs**: silver stores the viewport UCS under
   `ucs_handle` (the old code read a nonexistent `named_ucs_handle` and
   emitted None on every record). (e) **DIMSTYLE_CONTROL.morehandles**:
   TRIED and REVERTED — gold's HANDLE_VECTOR uses the wire RCu
   `num_morehandles` (dwg.spec 4176-4182, undocumented) which silver's
   reader does not keep; emitting the whole dim-style table produced 109
   wrong_value rows (19 → 109 → back to 19). Needs a reader capture.
   (f) **Associative retypes** (the U2 recipe, all in
   _UNKNOWN_BITS_TYPES so the side channel emits their unknown_bits):
   ASSOCACTION (deps = [0]*n; owned_params handle vector SINCE R_2013),
   ASSOCOSNAPPOINTREFACTIONPARAM (gold wire constants osnap_mode 160 /
   param 0.0 on every corpus record — silver's parsed 1/-1.0 must NOT be
   emitted), ASSOCVERTEXACTIONPARAM (asdap_class_version/dep/pt from the
   single_dependency nesting), ASSOCPATHACTIONPARAM (params omitted when
   empty), the SURFACE ACTIONBODY family EXTRUDED/LOFTED/REVOLVED/PLANE
   (aab_version<-action_body.version, version<-surface_body.version,
   minor<-parameter_body.minor, deps<-parameter_body.dependencies,
   l4=0, pab.values = [0]*n, assocdep = deps[0]-1 verified 3/3 non-plane
   classes — the PLANE class keeps the raw [0,0] null and extra l5=0,
   is_semi_* <- surface_body flags, l2 <- surface_body marker,
   grip_status, pbsab_status 0, class_version). CONSTRAINT: gold types
   ASSOCSWEPTSURFACEACTIONBODY UNKNOWN_OBJ (dead block — do NOT retype).
   SUN (ClassObject data.Sun): full flat projection; color: gold prints
   the bare index (7) when silver stores {"Index": 7} (pre-R2004 CMC)
   vs {"index": 7, "rgb": "c2…"} for the Rgb form. TABLEGEOMETRY
   (DataObject data.TableGeometry): numrows/numcols + cells = [0]*n
   (degenerate REPEAT). ASSOCNETWORK: owned_actions = wrap-vector,
   actions = [0]*n. Verified per-file ex2000/2004(R2004 Surface)/2010/
   2013/2018 + Dynblocks (write side down to 31): all retyped families
   ZERO on both sides; residual: ASSOCDEPENDENCY.dep_on 5 on Surface
   (silver's reader types the surface entities SURFACE where gold reads
   PLANESURFACE/UNKNOWN_ENT), poly-SEQEND shadow pairs, VX reader drops.

   ~~text-style map + ATTDEF trio + INSERT chain + MLINE~~ — **DONE
   (2026-09-20 second batch, `29459bf`; read 2 258 → 1 931 / write 2 124 →
   1 805, −327/−319)**, four normalizer-only families: (a) **style map**
   — `style_map` from silver's text_styles entries (name→handle,
   `_style_handle`), projected into TEXT/ATTDEF (gold dwg.spec 342/418/
   599 FIELD_HANDLE style SINCE R_13b1) — the "differ resolves it
   separately" comments were wrong; the handle must be emitted. (b)
   **ATTDEF trio** (dwg.spec 568-595): lock_position_flag SINCE R_2007a
   (from silver's lock_position bool), is_locked_in_block +
   keep_duplicate_records SINCE R_2010b (unmodeled, 0 on every corpus
   record; the RCs carry VALUEOUTOFBOUNDS coercion). (c) **INSERT chain**
   (dwg.spec INSERT/ATTRIB/SEQEND): silver's `insert.attributes` embeds
   the full parsed ATTRIB entities; synthesize the child records after
   the parent (the polyline kid machinery) with: R2004a+ `attribs`
   handle vector vs pre-2004 first_attrib/last_attrib (gold code 4),
   `seqend` link only when attributes exist (gold code 3), kid
   TEXT-family projection (tag/text_value/field_length/dataflags+
   conditionals/flags bits/kid fields), ownerhandle→INSERT; kid style:
   real handle pre-2010 but RAW `[0,0]` (gold's code-0 null 2-tuple,
   which normalize_gold keeps as a list) on R2010+; pre-2004 kid chain
   prev/next nulls + nolinks 0; mtext_type (1) SINCE R_2018b. CRITICAL:
   SEQEND records must be re-sorted ascending by handle at the end of
   `normalize_silver` — the differ aligns (type, ordinal) and silver's
   iteration order (insert-seqends emitted at their parents' positions)
   interleaves wrongly vs gold's document order (probes ch_ex2000…2018:
   gold [401, 1051, 1262, 1881]). The pre-existing poly-SEQEND shadow
   rows (ex2010 wire per-record shadow commons) stay — silver synthesizes
   those SEQENDs without storage records; they need reader work (queued).
   (d) **MLINE** (dwg.spec 1569): scale←scale_factor, justification enum
   (Top/Zero/Bottom→0/1/2), base_point←start_point, flags bits
   (HAS_VERTICES=1, CLOSED=2), mlinestyle←style_handle, and `verts` =
   the degenerate REPEAT form `[0] * num_vertices` (verified 2/4/6-vertex
   records; the [0]*n JSON lesson). Per-file: Multiline 2018 **0/0**,
   2000 13→3 (residue = DIMSTYLE_CONTROL/VX_CONTROL), ex2000-2018 chains
   clean, ATTDEF/TEXT families zeroed everywhere.

   ~~unknown_bits floor — raw-remainder side channel~~ — **DONE
   (2026-09-20, `2cb2eaa`; read 2 706 → 2 258 / write 2 572 → 2 124,
   −448/−448)**: silver's reader now snapshots gold's HANDLE_UNKNOWN_BITS
   window verbatim, so the normalizer emits gold's `unknown_bits` hex.
   Groundwork: the 2026-09-20 full-libredwg-src parse (probes
   `p01`–`p04`, JSON dump `out_spec_blocks.json`, block bodies in
   `out_u2_blocks.txt`/`out_smallfam_blocks.txt`) re-validated the §8.1.1
   census structurally (88/234 starters, 87/161 live; frame classifier:
   `#if 0` + `defined(DEBUG_CLASSES)||defined(IS_FREE)` + IS_DXF variants
   are the only dead-wrappers) and located 149 HANDLE_UNKNOWN_BITS sites
   (147 dwg2.spec + dwg.spec 5581 live, 5630/5754 commented). Mechanics:
   the macro (spec.h 578) expands in the decode TU to
   `dwg_decode_unknown_bits` (decode.c 5924): `bit_read_bits` from the
   CURRENT position — every spec block places the macro directly after the
   common entity/object prologue — to `8 * obj->size`, i.e. a FULL
   snapshot of the class-payload (not leftover bytes: that is why
   fully-modeled classes like WIPEOUT/MULTILEADER also carry nonzero hex),
   then the position is RESTORED so decoding continues. Silver:
   `unknown_bits_by_handle` (document.rs; serde-transparent Handle →
   decimal-string keys, the reactors/xdic precedent) filled by
   `capture_unknown_bits` (dwg_document_builder.rs) right after
   read_common_entity_data / read_common_non_entity_data in both pass-2
   loops; the writer does NOT consume it (on rewrite both oracles re-read
   the written bytes). THE decisive subtlety: `bit_read_bits` (bits.c
   1733) MSB-aligns only the full bytes (bit_read_fixed); the trailing
   partial byte accumulates `chain[bytes] |= last << i` — read-order bit
   i lands at the LOW position i, zero-padded on the left. An MSB-aligned
   tail produced hexes differing in EXACTLY the final byte family-wide
   (gold 03/01 vs silver C0/80); fixed in capture_unknown_bits.
   Normalizer (normalize_silver.py): `_UNKNOWN_BITS_TYPES` = the
   corpus-observed nonzero emitters (pooled gold census, probe
   `ub_full_set.py`) of the 149 macro classes, minus UNKNOWN_OBJ/ENT
   (their hex is dropped symmetrically in normalize_gold);
   `_emit_unknown_bits` runs at BOTH record appends (entity loop:
   common.handle; object loop: payload.handle) AFTER every retype
   branch. Emitting for a type gold does not would create
   extra_in_silver rows — never broaden the set without a fresh corpus
   census (records of the set always have a nonzero window; classes with
   zero windows are not gold emitters). Verified per-file on Dynblocks
   2018 / ATMOS-DC22S 2007 / Surface 2004 / example_2010 / example_2000
   (probes `ub2_*`): zero unknown_bits rows remain on both sides, every
   `_UNKNOWN_BITS_TYPES` record hex-equal; follow-on diagnosis for the
   next packets (ASSOCACTION/SUN/TABLEGEOMETRY retype gaps,
   UNKNOWN_OBJ-owner codes, the ex2010 3140 dropped record) came from
   the same probes.

   ~~U2 — dynamic-block-class retype map~~ — **DONE (2026-09-20, `6a97535`;
   read 3 425 → 2 706 / write 3 292 → 2 572, −719/−720)**: retyped +
   projected the whole dynamic-block family out of silver's wrappers.
   Recipe (the spec-parse groundwork lives in
   `target/probes/spec_parse.py` — a reusable LibreDWG spec parser with
   preprocessor-frame liveness tracking; `classes_census.py` cross-checks
   classes.inc dispatch verdicts): (a) the retype dispatch at the
   DynamicBlock site keys `payload.dxf_name` (upper-cased) through
   `_DYNBLOCK_RETYPE`; deliberately absent: BLOCKPROPERTIESTABLE(+GRIP) +
   DYNAMICBLOCKPROXYNODE (DEBUGGING_CLASS_DXF → gold emits UNKNOWN_OBJ —
   they must stay UNKNOWN) and the classes without landed projections
   (lookup/array/polar-stretch actions, user/XY parameters, constraint
   parameters, ACSH sphere/cone). (b) render classes are ClassObject
   wrappers keeping NO dxf_name — dispatch on `data.<Kind>`
   (RenderGlobal/RenderEntry/MentalRayRenderSettings); CELLSTYLEMAP is a
   DataObject → cells `[0]*n` (the degenerate REPEAT class); ProxyObject
   wrapper → PROXY_OBJECT (dxfname "ACAD_PROXY_OBJECT", proxy_id ←
   class_id — silver's own `proxy_id` is the constant 499 the spec
   comment mentions; gold's `proxy_id` is the wire class number).
   (c) out_json emission model (all bit-verified on fresh Dynblocks/ATMOS
   runs, `target/probe_u2/`): `_path_field` strips `x[i].` prefixes so
   BlockAction connections emit PLAIN duplicate `code`/`name` keys that
   collapse **last-wins** in the parsed JSON (the surviving name belongs
   to the LAST connection, not the element — element.name is emitted
   first and overwritten); REPEAT2 sub-fields emit `<prop><n>.connections`
   only when non-empty; `prop_states` (FIELD_VECTOR_N BL×4) lands as
   gold's 4-int array → the int-array→handle-dict heuristic
   {code:s0,size:s1,value:s2,absref:s3}; `num_propinfos` is never emitted;
   value_set SUB_FIELDs emit plain `desc/flags/minimum/maximum/increment/
   valuelist` with valuelist present even when `[]`; the element
   be_major/be_minor pair is DECODER-only (silver's element.major/minor
   have no gold counterpart). (d) silver payload nesting is DOUBLE for
   parameters (`data.LinearParameter.parameter.parameter.element`) and
   WithBasePoint actions (`data.RotateAction.action.action.element` +
   offset/dependent/base_point on `.action`) — the flat BlockAction/
   StretchAction shapes are single-nested. (e) silver's StretchAction
   stores gold-compatible pts/handles/codes (hdls/codes collapse to
   `[0]*n` in the norm via the dict-collapse heuristic) and offsets —
   the angle_offset denormal-vs-0.0 divergence is a non-issue
   (normalize rounds to 14 decimals → both 0.0). (f) normalize_gold
   gained symmetric divergent-field drops for gold-decoded-derailed
   records (the 3DSOLID-prologue precedent): RENDERENTRY (zombie 256s,
   render_time reading the BD '01' 1.0 special, minute/second swapped,
   display_index/light/material/memory/triangle garbage) and
   ACSH_BREP_CLASS (major 3528495168, acis_data [""]; silver's own
   operation_major/minor store the same derailed bits differently).
   Residuals kept for the raw-remainder packet: unknown_bits on
   BLOCKSTRETCHACTION(9)/BLOCKREPRESENTATION(7)/DYNAMICBLOCKPURGEPREVENTER
   (6)/ACSH_FILLET_CLASS(50), PROXY_OBJECT data/data_numbits (60 pooled).
   NOT fixed: PROXY_OBJECT.objids (18 pooled) — gold's objid loop stops at
   `hdl_dat->byte < hdl_dat->size - 1` (hdl_dat->size = the record's MS
   size, byte from bitsize/8); silver's `handle_remaining_bits() >= 8`
   reads 1-2 trailing terminator null-handles more (Constraints 2018:
   gold 8/3/3/3/3 vs silver 10/4/4/3/3); neither a `> 8` loop bound nor
   bit-geometry arithmetic reproduced gold's exact stop — needs the real
   byte-geometry instrumentation before touching. Per-file verified:
   Dynblocks 480→194/441→154, ATMOS 335→147/325→135, Constraints-2018
   15→14, Constraints-2013 4→4; corpus re-run WITH §8.1.0 env (a corpus
   run without GOLD_DWGREAD silently re-reads stale per-stem artifacts —
   caught this time by Dynblocks showing the pre-packet 480).

   ~~xdic family + LAYER visualstyle~~ — **DONE (2026-09-19, first reader-PR
   packet of the campaign)**: the top read rows after the 2026-09-19 session
   were LAYER/LAYER_CONTROL/BLOCK_HEADER `xdicobjhandle`/`is_xdic_missing`
   (~576 read / ~556 write) + LAYER `visualstyle` (212 read / 202 write) +
   the same family on object-variant payloads (TABLESTYLE etc., below the
   top-40). Diagnosis: **silver already stores everything** — the reader
   reads the xdic handle in `read_common_non_entity_data` (all versions;
   R2004+ gated by the `is_xdic_missing` bit) and saves it into the
   document-level `xdic_by_handle` map (`dwg_document_builder.rs` Pass 1 for
   table entries/controls, Pass 2 for all other objects), and the writer
   round-trips it (that's why `gold_rt` showed *real* xdics). The gap was
   purely projection. Fix (all in `normalize_silver.py`, normalizer-only):
   look up the record's handle in the top-level `xdic_by_handle` map
   (decimal-string keys, same pattern as `reactors_by_handle`) and emit
   `xdicobjhandle` when present (all versions) + `is_xdic_missing: 0/1`
   (R2004+) for (a) table-entry records, (b) control records (synthesized
   `_ctrl`), and (c) the objects branch via `_inject_xdic` — a generic
   fallback for object variants whose struct doesn't serialize
   `xdictionary_handle` (TableStyle and friends; the named variants like
   Dictionary/VisualStyle/XRecord dump it). The only true reader gap in the
   family was LAYER `visualstyle` (gold dwg.spec LAYER tail, `SINCE
   (R_2013b) FIELD_HANDLE (visualstyle, 5, 348)`): silver read it into
   `LayerData.unknown_handle` and discarded it — renamed to
   `visualstyle_handle`, stored on `Layer.visual_style_handle` (serde-visible
   so the dump carries it), the writer now writes the real value
   (HardPointer=code 5, null when unset) instead of the hardcoded null, and
   the normalizer projects `visualstyle` for every R2013+ layer (gold
   always emits it; unset = null handle `[5,0,0,0]`). Gold shape verified
   per version on fresh `Line.dwg` runs: pre-R2004 LAYER/CONTROL emit
   `xdicobjhandle` only when the xdic exists and no `is_xdic_missing` key;
   R2004+ additionally carry the bit; only R2013+ LAYERs carry
   `visualstyle`. Corpus: **read 5 942 → 4 722, write 5 562 → 4 402**
   (−1 220 / −1 160 — bigger than the top-40 estimate because the
   object-variant xdic rows below the top-40 died too). Committed
   `3cff463`. Verified: all 6 versions + ex18 family-row zero, `cargo test
   --features serde` 1556/0, `gold_roundtrip` ok.

   ~~Mid-rank batch (LAYOUT.has_ds_data / SORTENTSTABLE /
   APPID.AcadAnnotative / DIMASSOC.ref / IMAGEDEF_REACTOR)~~ — **DONE
   (2026-09-19, `5c75b1d`)**: five one-line-class fixes, all verified
   per-file before landing. (a) `LAYOUT.has_ds_data` (~97 read / 95
   write): silver's reader correctly parses the R2013+ AcDs bit for every
   object (the Model LAYOUT on R2013+ files is the one that carries it) and
   stores the handles in `document.dwg_data_store_handles` — but that set
   was `#[serde(skip)]`, so the dump never exposed it. Un-skipped it (now a
   plain handle list like the other round-trip side channels) and the
   objects branch derives `has_ds_data` from it instead of the hardcoded 0
   — the writer already round-trips the bit, so both sides died together.
   (b) `SORTENTSTABLE.block_owner` (~144/144): silver already reads and
   stores the owning block record as `block_owner_handle` (and round-trips
   it); gold's dwg2.spec 149 field is `block_owner` and gold never
   serializes the DXF-only ent/sort_ent vectors — pure renames plus drops
   of the silver-only `entries`/`entry_map`. (c) `APPID.name` (~68 read /
   57 write): `CadDocument::new()` fabricates the AcadAnnotative regapp;
   the builder's fabricated-APPID removal list had
   AcCmTransparency/AcAecLayerStandard but omitted AcadAnnotative, leaving
   a phantom record that shifted every gold APPID pairing. Added it to the
   removal list (reader fix; gold's APPID_CONTROL on the affected files
   has no such entry, verified). (d) `DIMASSOC.ref` (~62 orig / 46 rt):
   out_json's REPEAT_CN emission collapses every AcDbOsnapPointRef struct
   to a bare 0 — gold emits `[0]*6` for all six slots on every corpus
   DIMASSOC, even when the wire carries real refs (Dynblocks/example_2004
   verified). The old per-slot dict projection could never match; emit the
   degenerate `[0]*6` instead (silver's `references` stay in the dump for
   the writer). (e) `IMAGEDEF_REACTOR.class_version` (~30/30): silver's
   reader parses class_version but the builder discarded it, and the writer
   hardcoded 0 (gold_rt read 0 while the file carried 2). Stored it on the
   struct (serde-visible), round-tripped it in the writer, and dropped the
   silver-only `image_handle` from the projection (gold's dwg.spec block
   serializes class_version only). Corpus: **read 4 722 → 4 304, write
   4 402 → 4 106**. All five families verified row-zero on fresh
   Dynblocks/example_2004/example_2013/2018-Constraints runs; both gates
   pass. Remaining top read rows after this batch: UNKNOWN_OBJ._missing
   264, SOLID.elevation 161, TABLESTYLE/DIMASSOC/EVALUATION_GRAPH
   unknown_bits (~240 combined, the floor), BLOCK/APPID write-side
   is_xref_ref, LAYOUT lines/other smalls.

   ~~SOLID.elevation + linewt index projections~~ — **DONE (2026-09-19,
   `8653303`)**: (a) SOLID/TRACE `elevation` (161 read / 131 write):
   silver's model holds the value all along — the reader folds the wire
   elevation into every corner's z (`data.elevation` → `Vector3::new(x, y,
   z)`), and the writer round-trips it via `write_bit_double
   (e.first_corner.z)` — but the dump projection sliced the 2RD corners to
   [x, y] before an `elevation` field could be emitted. De-duplicated the
   accidentally-duplicated SOLID/TRACE block in `normalize_silver.py` and
   project `elevation = corner1.z` before the slice. (b) linewt (155
   read / 137 write census): silver's `LineWeight` enum serializes concrete
   weights as `{"Value": <mm*100>}` — a form `_lineweight_to_gold` didn't
   handle, so the entity-common merge emitted the raw serde dict (the
   differ shows it as null). Added the dict form plus an mm*100 →
   lweights[]-index inversion mirroring silver's
   `LineWeight::INDEXED_VALUES`/`to_dwg_index` (index 0..23 +
   29/30/31 = ByLayer/ByBlock/Default), and the LAYER table record now
   emits `linewt` (R2000+ per the dwg.spec LAYER flag word) from its
   `line_weight` enum through the same mapping. Corpus: **read 4 304 →
   4 001, write 4 106 → 3 835**. Verified row-zero on fresh
   example_2004/2004-Donut/2018-Dynblocks runs; gates 1556/0 +
   `gold_roundtrip` ok.

   **Deferred sub-family — the linewt-28 preservation (~37 orig rows,
   `LINE/CIRCLE/BLOCK/ENDBLK/LWPOLYLINE`)**: those corpus records carry wire
   byte **28** (a non-canonical ByLayer alias — silver's
   `LineWeight::from_dwg_index(28)` ligates it to canonical `ByLayer`, whose
   `to_dwg_index()` writes **29**). Gold reads and re-emits 28 as-is
   (plain `FIELD_RC (linewt, 370)`, common_entity_data.spec 545 — no
   transformation); silver loses the value in the enum, so the projection
   cannot recover it, and the writer changes the byte (28 → 29). The rt
   side is already self-consistent (both readers decode silver's rewritten
   29), so these rows are orig-side only. Fix recipe (reader + writer,
   small structural): preserve the raw byte — either a private
   serde-skipped `dwg_linewt_raw` + public accessor on `EntityCommon`
   (builder `dwg_document_builder.rs:7253` stores it when
   `to_dwg_index(from_dwg_index(raw)) != raw`), or a document-level
   `linewt_raw_by_handle` side channel serde-visible like `xdic_by_handle`;
   writer consults it in `entity_preamble`
   (`common.rs:643 write_byte(line_weight.to_dwg_index())`); the dump +
   normalizer prefer it for `fields["linewt"]`.

1. ~~Table-record storage fields~~ — **done** for the uniform set (see §7).
   What remains is per-record **payload**, one packet per table type:
   DIMSTYLE `DIM*` vars (largest), LTYPE dash patterns, VPORT view params,
   BLOCK_HEADER topology, LAYER `plotstyle`/`linewt`.
2. **Object representation gaps** by corpus frequency: VISUALSTYLE,
   MATERIAL, DIMSTYLE, LAYOUT, SCALE, MLEADERSTYLE, DICTIONARYVAR,
   BLOCK_HEADER, LTYPE, APPID. These are name/shape mappings between the
   silver serde schema and gold spec — decision tree rules 3–5 mostly. Check
   `OBJECT_TYPE_MAP` and the object branch of `normalize_silver.py` first;
   many will be renames, not codec changes.

   **Started (2026-09-17):** VISUALSTYLE (positional property-bag → named
   fields for pre-R2010, R2010+ core, and R2013+ extended), MATERIAL
   (map/color flattening + R2007+ gating), DIMSTYLE (dim* → DIM* name
   mapping + version gating + `DIMTXSTY`/`DIMLTYPE`/`DIMLTEX1`/`DIMLTEX2`
   handle emission), SCALE (flag/is_temporary projection), XRECORD
   (xdata/cloning projection), DICTIONARYVAR (schema_number/value →
   schema/strvalue), and LWPOLYLINE (vertex reshape + flag projection) are
   substantially done in `normalize_silver.py`. Objects are now sorted by
   handle before diffing (silver stores them in a HashMap; gold's OBJECTS
   array is handle-ordered), and table control objects are emitted so the
   differ can resolve table-record `ownerhandle`.
    Known residual per-type gaps to continue with:
    - ~~`ownerhandle`~~ — **DONE (2026-09-17)**: the differ now resolves
      handle identity by `absref` (gold's `value` slot is a relative counter
      for coded references, only `absref` is the absolute handle silver
      stores) and compares handles as `(code, resolved_target)` tokens;
      silver emits `code=None` (unknown — it stores no handle codes), so the
      code is checked only when both sides carry one. Silver normalizer wraps
      the remaining raw-int handle projections (`DIMLDRBLK`/`DIMBLK*`,
      LAYOUT `base_ucs`/`named_ucs`, `BLOCK_HEADER.layout`, `LAYER.material`,
      GEODATA `host_block`). Corpus read-fidelity `ownerhandle` collapsed from
      ~46k to 504 (the residual rows are real silver gaps: owners of type
      UNKNOWN_OBJ/SECTIONVIEWSTYLE/EVALUATION_GRAPH that silver does not
      model). Corpus totals: read 196 252 → 137 823, write 127 737 → 127 172.
    - ~~`SCALE.is_temporary`~~ — **DONE (2026-09-17)**: gold's dwg.spec emits
      only the raw `flag` BS for AcDbScale; `is_temporary` is a libredwg-internal
      derived field never in the JSON. Silver both emitted it and leaked it via
      the generic payload loop. Fixed by popping it; projects only `flag`.
      Read 137 823 → 133 990, write 127 172 → 123 373.
    - ~~object-level `reactors`~~ — **DONE (2026-09-17)**: silver's reader parses
      reactors into the `reactors_by_handle` side channel but never projected
      them to JSON. The normalizer now injects that map (serde keys integer
      maps as decimal strings) into each object/table/entity payload; Dictionary
      and DictionaryVariable carry parsed `reactors` on the struct. Residual
      entity-level reactors (~122: LINE/DIMSTYLE→DIMASSOC/ASSOC*, TABLESTYLE→
      TABLE mis-resolution) are a separate packet. Read 133 990 → 125 747,
      write 123 373 → 115 180.
    - ~~VISUALSTYLE `c_prop33`~~ — **DONE (2026-09-17)**: gold's dwg.spec default
      for the edge-color CMC is 0 (ByBlock); silver's property bag stores
      Color::ByLayer (256). Map silver's 256→0 for c_prop33 only. Read
      125 747 → 124 779, write 115 180 → 114 260.
    - ~~LWPOLYLINE `vertexids`~~ — **DONE (2026-09-17)**: gold always serializes
      the vertexids array SINCE R_2010b (even when flag&1024 is clear); silver
      stores none. Emit `[]` on R2010+ so gold's `[]` doesn't diff
      missing_in_silver. Read 124 779 → 124 128, write 114 260 → 113 631.
    - CMC absent/true colors: gold `c1000000` RGB-black vs silver index 0;
      gold `{index:256, rgb:…}` vs silver `Color::None`. Cross-cutting; one
      fix in `normalize_gold.py`/`normalize_color` clears it everywhere.
    - ~~UNKNOWN_ENT/UNKNOWN_OBJ→UNKNOWN canonicalization~~ — **REVERTED
      (2026-09-17)**: merging gold's distinct UNKNOWN_ENT (entity-common) and
      UNKNOWN_OBJ (object-common) into silver's single UNKNOWN bucket caused
      ordinal misalignment (+1 127 net rows). Do not merge types with different
      common-field shapes.
   - `reactors`: gold emits them; silver doesn't store them (storage gap).
   - Next types to start: LAYOUT, MLEADERSTYLE, BLOCK_HEADER topology,
     LTYPE dash patterns, VPORT view params.

   ~~DIMASSOC~~ — **DONE (2026-09-18)**: turned out to be a **normalizer**
   packet, not a reader packet. Silver's reader was already fully wired
   (`read_dimension_association` `object_reader/associative.rs:480`, routed at
   `:1255`, builder dispatch `dwg_document_builder.rs:5928`, class registered in
   `classes/mod.rs`); the gap was `OBJECT_TYPE_MAP` flattening the whole
   `Associative` variant to `UNKNOWN`. Projected DIMASSOC to gold's shape
   (dwg2.spec 3653): `associativity`/`trans_space_flag`/`rotated_type`/
   `dimensionobj` scalars + the fixed 6-slot `ref` array (sub-fields renamed
   class_name→classname, main_gs_marker→main_gsmarker, osnap_distance→
   osnap_dist, osnap_point→osnap_pt, has_last_point_reference→has_lastpt_ref).
   **The `ref` array is indexed by the associativity BIT position** (spec
   REPEAT_CN(6, ref): bit rcount1 → ref[rcount1]); silver's `references: [Vec;4]`
   is already slot-keyed, so project each slot in place — flatten-then-repack
   misaligns slot-1 refs to index 0 (caught by field-by-field verification, not
   by the differ, since `unknown_bits` forces a diff on those records anyway).
   Added `_associative_gold_name` (mirrors the reader's
   `associative_canonical_name`) but scoped the typed emission to DIMASSOC only
   — emitting all ASSOC* under canonical names surfaces their unprojected
   field-level gaps (net-negative). Corpus: read 27 019 → 26 980, write 27 313
   → 27 282. **Verified field-by-field against gold on all 6 versions: every
   normalizer-mappable field (associativity/trans_space_flag/rotated_type/
   osnap_*/main_*/xrefpaths/classname/ref-slot positions) matches 100%.**
   Residuals (separate packets, all reader-coverage not normalizer):
   `DIMASSOC.unknown_bits` (gold-only HANDLE_UNKNOWN_BITS verbatim hex, silver
   stores no raw remainder), `DIMASSOC.dimensionobj` (resolves to gold's
   DIMENSION_* subtypes; silver has one Dimension variant — §9), and
   `ref[].xrefs` second elements resolving to VERTEX_3D (silver stores vertices
   inside the parent polyline — §10).

   **Next task (ready to start):** the remaining `UNKNOWN._missing` (871) /
   ASSOC*._missing unmodeled-object coverage class. Each unmodeled type is its
   own reader packet; the DIMASSOC projection pattern (resolve dxf_name in the
   normalizer once the reader stores the object) is the template for the
   already-read ASSOC* types. Largest remaining ASSOC clusters in the report:
   ASSOCDEPENDENCY (36), ASSOCGEOMDEPENDENCY (34), ASSOCVARIABLE (22),
   ASSOCDIMDEPENDENCYBODY (18), ASSOCVALUEDEPENDENCY (18), ASSOCNETWORK (17).

   **Next task (ready to start, 2026-09-19, post-3DSOLID): the unmodeled
   class — retyping + per-class field projections.** Anatomy (verified
   empirically): the `UNKNOWN_OBJ._missing` 582 / `UNKNOWN._missing` 213
   rows are NOT missing silver records — they are **silver-extra records**
   (`kind:"missing", side:"silver"`): silver's wrapper variants
   (`DynamicBlock`, `ClassObject`) parse ~200 class-registered objects per
   heavy stem (ATMOS-DC22S: 186+17; Dynblocks: 98+ …) which the normalizer
   bucket-maps to `UNKNOWN_OBJ`, while gold decodes the LIVE classes TYPED
   (ATMOS: gold's 194 typed = ACSH_FILLET_CLASS 50 + ACSH_HISTORY_CLASS 42
   + EVALUATION_GRAPH 42 + ACSH_CYLINDER_CLASS 17 + ACSH_WEDGE_CLASS 10 +
   ACSH_BOX_CLASS 6 + RENDERGLOBAL 6 + MENTALRAYRENDERSETTINGS 6 + ...;
   the debug-gated ones — ACSH_SWEEP/EXTRUSION/LOFT/REVOLVE, TABLE entity,
   the SURFACE entities — legitimately pair as UNKNOWN_OBJ today).
   Silver keeps each wrapper's `dxf_name`; the naming fix is a
   dxf_name→gold-type resolver for the wrapper variants (the
   `_associative_gold_name` pattern at normalize_silver.py:115 is the
   template) — **BUT retyping alone is a NET LOSS**: the newly typed
   records pair against gold's typed field lists (ACSH_CYLINDER ~20
   fields, MENTALRAYRENDERSETTINGS ~57, SECTIONVIEWSTYLE ~45 — the
   evalexpr.*/history_node.* families), so EACH class needs its own
   MULTILEADER-style field projection from silver's wrapper payload in the
   same packet, or the rows explode (17 record-rows → 340 field-rows).
   ~~ACSH_HISTORY_CLASS~~ — **DONE (2026-09-19, first class of the unmodeled
   campaign)**: retype + projection in `normalize_silver.py` — silver's
   `DynamicBlock` wrapper records with `dxf_name == "ACSH_HISTORY_CLASS"`
   now emit under the gold type with the
   dwg2.spec 3077 payload projected from silver's `data.SolidHistory`:
   `major`/`minor`/`h_nodeid` (from history_node_id)/`show_history`/
   `record_history` (bools → 0/1), `owner` → absref handle dict; the
   wrapper metadata (`data`/`dxf_name`/`cpp_class_name`/`source_version`)
   is consumed. Cross-reference bonus: every handle-target row into a
   history object now resolves `ACSH_HISTORY_CLASS` on both sides — the 42
   ATMOS 3DSOLID-family `history_id` rows and REGION.reactors died with
   it (gold's residual: the ATMOS history `owner` rows point at
   EVALUATION_GRAPH records — they resolve fully once that class lands).
   Silver's 42 wrapper records pair 1:1 (verified count parity). Corpus:
   read **9 442 → 9 327**, write flat (8 668 — the write compare is
   gold_rt vs silver_rt; the rt files' class-table re-encoding side is
   unaffected by the read-side retype). Verified on ATMOS + example
   2007/2010. Gates: 1556/0, `gold_roundtrip` ok.
   ~~EVALUATION_GRAPH~~ — **DONE (2026-09-19, second class)**: same retype
   pattern — silver `DynamicBlock` wrappers with `dxf_name ==
   "ACAD_EVALUATION_GRAPH"` emit under the gold type (dwg2.spec 3549;
   the class dxfname differs from the block name, so the record-meta
   `dxfname` is emitted too). Payload from `data.EvaluationGraph`:
   first_nodeid/first_nodeid_copy, nodes/edges as the degenerate
   `[0]*count` REPEAT emission (zero-size omitted; count parity
   verified 0/42 mismatches on ATMOS, Dynblocks 6/6). Residual:
   unknown_bits (gold-only) per record. Bonus: the ACSH_HISTORY_CLASS
   `owner` targets resolve — the ATMOS history-owner rows died with this
   class (42 ACSH_HISTORY rows → 0 on ATMOS). Corpus: read **9 327 →
   9 177**, write 8 668 → **8 657**. Verified on ATMOS,
   Dynblocks, example_2007/2010. Remaining class order by (count ×
   field-cost): ACSH_BOX/WEDGE/FILLET/CHAMFER/BOOLEAN/CYLINDER/TORUS/
   BREP (silver's SolidHistoryNode payload; ~20 gold fields each, mostly
   evalexpr.*/history_node.*), RENDERGLOBAL/RENDERENTRY, then
   MENTALRAYRENDERSETTINGS (~57 fields). NEVER model a debug-gated type —
   emit UNKNOWN_OBJ for the in-work region classes instead (§8.1.1
   liveness rule). Then
   BLOCK_HEADER (698: name/first_entity/last_entity + anonymous +
   is_xdic_missing residuals — cleanest normalizer packets: BLOCK_HEADER
   via the block_records map and the LAYER/LTYPE_CONTROL residuals),
   LAYER.visualstyle (212), MTEXT.style (183, name→handle),
   SOLID.elevation (161), SECTIONVIEWSTYLE/DETAILVIEWSTYLE (~234 combined),
   and the 3DSOLID R2013+ prologue deep dive (below).
   *(Superseded 2026-09-19: BLOCK_HEADER, MTEXT.style,
   SECTIONVIEWSTYLE/DETAILVIEWSTYLE, and LAYER.visualstyle all landed —
   see the newest DONE entries. The live remaining-reader list is in the
   xdic-family entry above: SOLID.elevation 161, LAYOUT.has_ds_data 97,
   LINE/LAYER.linewt raw lweights, SORTENTSTABLE.block_owner 144,
   APPID.name 68, the MLINE/MLINEm/SEQENDENTSTABLE tails, VERTEX_MESH
   flag-64, ConstraintGroup nodes.)*
   Target: read AND write below **5 000** (§7) — at that level the cheap
   normalizer tail alone can no longer reach it; the UNKNOWN naming/reader
   class and BLOCK_HEADER are all required.

   Add-on diagnosis from the 2026-09-19 full-libredwg spec audit: the
   `OBJECTCONTEXTDATA.*` rows (6 stems; see e.g.
   `target/gold_harness_corpus/Leader/Leader_diff_orig.json`) are a silver
   **type-name** bug, not a reader gap — silver emits one record named
   `OBJECTCONTEXTDATA` (the abstract class, whose gold block is commented
   out, dwg2.spec 3813) where gold emits `LEADEROBJECTCONTEXTDATA` (live
   block, dwg2.spec 4611; 6 corpus records). Fixing silver's type name kills
   both the `OBJECTCONTEXTDATA._missing/_count` and
   `LEADEROBJECTCONTEXTDATA._count/_missing` row families (~24 rows). A small
   standalone packet, suitable before or alongside the UNKNOWN_OBJ backlog.

   ~~MULTILEADER~~ — **DONE (2026-09-19)**: the largest read-side type
   (was 1269 stem-inflated read rows / 1055 write rows, ~100 real rows on
   each of the 12 ML-bearing entries: the six Leader.dwg versions + the six
   `example_2xxx.dwg`; 2-4 residual rows per file after). Normalizer
   projection `normalize_silver.py` (`silver_type == "MultiLeader"`).
   Silver's payload keeps the full annot context under `context`
   (MultiLeaderAnnotContext) plus flat own-named fields; gold's shape is the
   dwg2.spec 1298 block + the MLEADER_CONTEXT_DATA_fields macro (dwg2.spec
   1227). Key mappings and subtleties:
   - All-versions fields: `mleaderstyle`←style_handle, `flags`←
     property_override_flags (serde bit-name string → u32 via silver's
     MultiLeaderPropertyOverrideFlags table; CONTENT_TYPE|TEXT_ALIGNMENT|
     ENABLE_USE_DEFAULT_MTEXT = 0x400|0x4000|0x40000 = 279552 exactly),
     `line_linewt`←line_weight (**BLd DXF codes: ByLayer −1 / ByBlock −2 —
     NOT the common-entity linewt RC index `_lineweight_to_gold`**),
     `has_landing`/`has_dogleg`←enable_landing/enable_dogleg,
     `landing_dist`←dogleg_length, `arrow_handle` (null → absref-0 dict),
     `style_content`←content_type (MText=2), `text_left/right`←
     text_left/right_attachment, enum→discriminant (silver's
     src/entities/multileader.rs tables: TextAttachmentType 0-10,
     TextAngleType Horizontal=1, TextAlignmentType Left=0/1/2),
     `style_attachment`←block_connection_type and
     `is_annotative`←enable_annotation_scale — **same wire slots, different
     silver names** (reader entities.rs:4495 names gold's BS-176 slot
     "block_connection_type").
   - VERSIONS(R_14,R_2007): `is_neg_textdir`←text_direction_negative,
     `ipe_alignment`←text_align_in_ipe, `justification`←text_attachment_point
     (same wire slot; TextAttachmentPointType Left=1/Center=2/Right=3),
     `scale_factor` passthrough; `num_arrowheads/arrowheads` +
     `num_blocklabels/blocklabels` projected **only when non-empty** (gold
     omits zero-count REPEAT data).
   - SINCE(R_2010b): `class_version`←dwg_version, `attach_dir`←
     text_attachment_direction, `ctx.text_top/bottom`←context.
     text_top/bottom_attachment. SINCE(R_2013b): `is_text_extended`←
     extend_leader_to_text.
   - ctx: `ctx.num_leaders`←len(leader_roots) and **`ctx.leaders` as gold's
     degenerate REPEAT emission** `[0]*count` (same libredwg class as
     MLINESTYLE.lines — silver's real leader-root structs are never emitted
     by gold's JSON). Slot shifts (entities.rs:4683-4686): silver stores the
     wire BS pair (`ctx.text_angletype`, `ctx.text_alignment`) shifted as
     (`text_alignment`, `block_connection_type`); ctx.content.txt.alignment
     comes from silver's ctx.text_attachment_point; height from
     text_boundary_height; colors via normalize_color, handles as absref
     dicts, bools → 0/1. Gold's BS-170 `type` field is shadowed by the
     record-meta key and never emitted; silver has no counterpart (content_
     type maps to BS-172 style_content instead). `graphic_data` has no gold
     field (the embedded-graphics binary folds into unknown_bits) → dropped
     per the linetype-name precedent.
   - Residuals (reader/structural, separate packets): `unknown_bits`
     (gold-only verbatim), `graphic_data` (silver-only), `attach_top`/
     `attach_bottom` — gold's raw BS values (32 / 178 / 4786) sit outside
     silver's TextAttachmentType enum whose From collapses to MiddleOfText
     (multileader.rs:79 builds CenterOfText defaults), so silver's struct
     cannot express them; PLUS the pre-existing `prev_entity` class.
   - Write-side: the projection also drives silver_rt normalization, so
     write rows fell together — the write pipeline compares gold's re-decode
     of silver's rewrite against silver's own re-parse (both normalized).
     Silver's ML **writer** still corrupts `flags` (writes 0 where the struct
     holds 279552) and `arrow_size` (writes 0.0 where the struct holds 4.0)
     — example_2018 gold_rt shows both — while the attach triplet roundtrips
     byte-exact through a path the struct does not expose (writer packet
     queued).
   Corpus: read **11 900 → 10 684**, write **10 904 → 9 894** (125 files,
   same two write-side exclusions as before: example_2013/2018 rt — see the
   REGION-NaN item below). Per-file ML rows: ~103-107 → 2-4 (verified on all
   six Leader.dwg versions + all six example versions).
   Gates: `cargo test --features serde` = 1556/0, `gold_roundtrip` ok.

   ~~REGION-NaN write-side blocker (`b40ba42`)~~ — **DONE (2026-09-20, first
   writer-Rust packet of the campaign)**: silver's rewrite of
   `example_2013.dwg`/`example_2018.dwg` made gold re-decode a REGION
   `point` as `[ -nan, 0.0, 0.0 ]` (BD bit-code `'11'` = gold's error NaN)
   — invalid JSON, so `normalize_gold` died and the write side of both
   files was excluded (-1) from every corpus report. **Bit-level deep dive
   done FIRST** (new probe: `dump_section_bytes`, §8.1.0; gold `-v9` trace
   + a hand-walk of the raw records at addresses 7378/40B and 10335/61B):
   - **Silver's R2013+ AcDs-path reader is BIT-TRUE.** The true wire of an
     AC1032 ds-backed 3DSOLID-family record is: [common entity data] →
     `wireframe_data_present` B (NO legacy leading `acis_empty` — the first
     modeler bit IS wdp) → [if set: `point_present` B, 3BD point, `isolines`
     BL (RS is little-endian per bytes — RS16 `30 D0` = -12240, my first
     MSB-first walk "12496" was wrong), `isoline_present` B,
     (num_wires BL 0, num_silhouettes BL 0)] → `acis_empty_bit` B →
     the R2007 "unknown" BL(0) → the R2013+ revision block → one sentinel
     0 bit → handle stream. Silver's stored model matches every value AND
     lands within that sentinel bit of the handle-stream boundary on both
     records (verified digit-for-digit: point, isolines=4, revision bytes
     `A853307CEA64983A`).
   - **Gold's spec is wrong for these records**: `DECODE_3DSOLID`
     (dwg_spec_shared.h 175) unconditionally reads the leading `acis_empty`
     B on ALL versions, so gold misreads wdp as `acis_empty=1`, then derails
     (`point_present=0`, garbage `isolines=2450870777`, BL `'11'` errors,
     **"Invalid REGION.wires x 256"** record abort). Gold's JSON for the
     AREA AFTER the abort is decoder defaults. Verified against Adobe real
     wires; do NOT "fix" silver's reader to match gold here — the read-side
     golden shape IS the misparse, and bit-faithful rewriting keeps it
     identical on both sides of the diff.
   - **The writer bug**: `write_acis_empty` (writer `object_writer/
     entities.rs`) emitted ONLY a `false` wireframe bit (it looked at
     `wires`/`silhouettes` array emptiness, not the stored
     `wireframe_*` flags), plus a stray `extra_acis_data` gate bit, then
     the caller's BL(0)+revision — a layout neither decoder expects, which
     tripped gold into `acis_empty=0 → version=49` garbage and eventually
     a BD `'11'` = `-nan`. Fix: the AcDs path now writes the full model's
     wireframe block via the existing `write_acis_wireframe` (proven
     bit-faithful: its `wireframe_present`/`pp`/`isoline_present`
     derivations OR the stored flag, and for DWG-origin records the flag
     and values are exactly what the reader stored) + `acis_empty_bit`
     gated on wireframe-present, mirroring the reader; the caller keeps
     emitting the ds-path BL(0) + revision in the right order. No
     history_id on the ds path (reader/writer agree; true wires carry
     none).
   - **Result**: 0 `-nan` in both gold_rt JSONs; silver's internal
     consistency (silver_orig vs silver_rt) = 0 diff on example_2018;
     REGION records contribute **0 diff rows** on both sides (gold's
     misparse garbage roundtrips bit-identically — the whole point of
     bit-faithful rewriting); both files rc=0, joining the write side:
     **read 4 001 (unchanged), write 3 835 → 4 291** (+456 = their fresh
     202/254 rows; the old "~950 hidden rows" estimate included rows that
     were never hidden: the read-shared families). Gates 1556/0 +
     `gold_roundtrip` ok; all six versions smoke-verified (write:
     193/171/151/146/202/254). Note for report.md tables: the per-file
     `R:` values shown for example_2013/2018 in pre-2026-09-20 reports
     were STALE pooled artifacts from the crashed runs; fresh per-file
     numbers (R: 220/234) already were part of the read total.
   - **The queued "R2013+ prologue divergence (~63 rows)" residual is now
     fully explained** (and its "needs a bit-level deep dive before ANY
     fix" is discharged): the rows are gold's desync artifacts (garbage
     `isolines=205`/`revision_major=2423192302`/hex-string
     `revision_bytes`) from the SAME misparse, read as wrong_value against
     silver's bit-true values — 59 rows (example_2018: 28 + example_2013:
     31, incl. its R2013-only `acis_data`/`history_id` shapes) on the
     read side and, since the files joined the write side, the same 59 on
     the write diff. They are NOT reducible from silver's model (silver's
     values are the true wire). ~~Small normalizer packet queued:
     **3DSOLID prologue-divergence projection**~~ — **DONE (2026-09-20,
     `1597db6`)**: both normalizers drop the divergent
     COMMON_3DSOLID-internal fields (`point_present/point/isolines/
     isoline_present/acis_empty_bit/has_revision_guid/revision_major/
     minor1/minor2/revision_bytes/end_marker`, plus 2013's
     `acis_data`/`history_id`) **symmetrically** for exactly those
     records — gold side gated on `FILEHEADER.version` AC1027/AC1032 +
     the record's `has_ds_data`; silver side gated on
     `r2013_plus` + the parsed SAB (the ds-section blob silver alone
     reads). Non-ds and pre-R2013 family records (ATMOS R2007 verified at
     its 369/186 baseline) keep every field — gold parses those sanely
     and they stay verified. Read **3 917 → 3 858**, write
     **3 431 → 3 372** (−59/−59, exactly the family, counted on both
     sides). Remaining family residuals: `encr_sat_data` (6, v1-SAT
     obfuscated wire blocks, unrecoverable by design) and the
     pre-existing `next_entity` chain family (1) — accepted.

   ~~Pre-R2007 table `is_xref_ref` wire bit + fabricated Standard
   DIMSTYLE~~ — **DONE (2026-09-20, `ff45453`)**: second writer packet.
   (a) Write rows `APPID.is_xref_ref` 103, `BLOCK_HEADER.is_xref_ref` 84,
   `LTYPE.is_xref_ref` 36, `LAYER.is_xref_ref` 26, `STYLE.is_xref_ref` 21,
   `DIMSTYLE.is_xref_ref` 2 (272 total, write-side only): the pre-R2007
   `COMMON_TABLE_FLAGS` (`spec.h` 755: `FIELD_B (is_xref_ref, 0);
   /* always 1, 70 bit 6 */`) wires an always-set reference bit before the
   resolved-BS and dependent-B; every corpus original carries 1 (the read
   side was clean because the silver normalizer derives 1). Silver's
   `write_xref_dependant_bit_value` hardcoded the reference bit to
   `false` (the structs store only the dependent bit for the types using
   the no-arg helper), so every rewritten pre-2007 table record carried 0
   on the wire and gold's re-decode reported `is_xref_ref: 0` against the
   normalizer's derived 1. Fix: one line — the helper now passes
   `xref_ref=true` (the R2007+ branch ignores the bit entirely, so
   R2007+ files are untouched); the VIEW/UCS/VPORT/DIMSTYLE/VX call sites
   that pass the stored `xref_reference` needed no change (the builder
   stores the wire bit for those).
   (b) The 2 residual DIMSTYLE rows unmasked a **phantom-record** bug:
   `CadDocument::new()` fabricates a `Standard` DIMSTYLE (document.rs
   `DimStyle::standard()`); files whose dimstyle table has no Standard
   (example_2004: just `ISO-25`) kept the fabrication through the load
   and the rewrite emitted a DIMSTYLE the original never had. Builder now
   drops it when the file contains no case-insensitive `Standard` entry
   (the `APPID.AcadAnnotative` precedent from `5c75b1d`).
   Verified per-file (write rows before → after): TS1 141→110,
   entities-2d 65→51, HatchG 38→19, Surface 85→62, Underlay 79→61,
   material 37→18, sample_2000 13→3, example_2004 171→136, Cone 27→9,
   PolyLine2D 94→58; **xref rows 0 everywhere**. The phantom removal also
   killed its read-side rows: **read 4 001 → 3 917** (−84) and its
   write-side family rows (counts/fields of the extra record),
   **write 4 291 → 3 431** (−860, of which −272 are the is_xref_ref rows
   themselves). Gates 1556/0 + `gold_roundtrip` ok; corpus integrity
   checked (125 files, only gh44-error excluded, totals = per-file sums).
   Remaining write-side writer-fix smalls (queue order):
   DICTIONARYWDFLT.defaultid 119 (write), MTEXT extents_height/width
   66/66 (write), the DIMENSION/DIM* block-name ambiguity
   (silver's table uniquifies `*D` blocks; DIMENSION_LINEAR.block 54 +
   ALIGNED/ORDINATE tails), INSERT-owned SEQEND entity-common fields,
   the ML writer flags/arrow_size corruption, and the 3DSOLID
   prologue-divergence projection above.

   ~~3DSOLID/REGION family~~ — **DONE (2026-09-19)**: the largest read-side
   family after MULTILEADER (1 360 read / 1 235 write family rows over the
   9 solid-bearing stems — ATMOS-DC22S alone carried 986, a 58-record R2007
   real-world file; after the packet the family residuals are ~120 read
   rows). Normalizer projections in `normalize_silver.py` (silver variants
   `Solid3D`/`Region`; `Body` deliberately excluded — gold's corpus BODY
   records are bare shells with 0 diff rows today) plus one targeted
   canonicalization in `normalize_gold.py`. Spec grounding: dwg.spec
   2675-2688 shells (`ACTION_3DSOLID`), out_json.c:1555 `json_3dsolid`
   (version/acis_data emission), dwg_spec_shared.h `COMMON_3DSOLID` 472
   (wireframe/materials/R2013b revision/history), `DECODE_3DSOLID`
   (sat/SAB wire mechanics). Silver keeps everything under
   `acis_data{version,sat_data,sab_data,is_binary,revision,materials,
   wireframe_*,acis_empty_bit}` + top-level `uid/point_of_reference/wires/
   silhouettes/history_handle`. Key mappings:
   - `acis_data` (v2 SAB): gold's json_3dsolid emits exactly
     `"%.15s"` — the 15-char "ACIS BinaryFile" ascii prefix — plus
     `VALUE_BINARY` of the remainder as UPPERCASE hex without separators;
     verified byte-for-byte against silver's sab_data
     (prefix = bytes[0:15] utf8, hex = bytes[15:]).
   - `acis_data` (v1 SAT): split silver's sat_data at `\r/\r\n/\n` cut
     points, keep interior empties, drop one trailing empty (Cone 30
     elements, TS1 88/29, example_2000 136/34/29 — all match).
   - `unknown` B: derived — empirically unknown==1 exactly when the SAT is
     ASCII version-1 (65/65 corpus records), so emit `1 if version==1`.
   - `acis_empty`: R2018 moved modeler geometry into the data section —
     gold reads the inline `acis_empty=1` and emits ONLY the flag + the
     always-on wireframe/revision COMMON block; silver parses the ds blob
     into sab_data, which therefore has no gold counterpart — emit the
     empty flag shape for r2018 files (corpus: example_2018 only).
   - `history_id`: COMMON_3DSOLID's else-branch emits it for every
     version>1 record (not SINCE R_2007a only — verified on R2004);
     gold-side canonicalization: libredwg prints a code-0 null handle as
     the bare 2-tuple `[0, 0]`, which normalize_gold now maps to the
     absref-0 null dict (field-name-gated: `history_id` only — 2-tuple
     points and ATTRIB.style's matching [0, 0] must stay lists).
   - COMMON block: `wireframe_data_present` gate → `point_present`/`point`
     (from silver's point_of_reference), `isolines`, `isoline_present`
     gate → `wires`/`silhouettes` as gold's degenerate `[0]*count` REPEAT
     emission (normalize_gold collapses the raw wire/silhouette structs
     the same way), never emitted empty; R2007a+version>1 `materials`
     (empty in corpus); R2013b `has_revision_guid/revision_major/minor1/
     minor2/bytes/end_marker` from silver's `acis_data.revision`.
   - `dxfname`: out_json.c DWG_ENTITY macro emits it only when the class
     dxfname differs from the spec block name — the `_3DSOLID` underscore
     alias family; constant "3DSOLID". REGION matches its block name, so
     it never gets one.
   - Residuals (read side, ~120 rows): **UNKNOWN-class naming** — the 44
     `history_id`/reactors rows where both sides hold the SAME absref but
     gold names the target `ACSH_HISTORY_CLASS` (etc.) and silver
     `UNKNOWN_OBJ` (the differ compares resolved handle-target type names;
     dies in the UNKNOWN naming packet, queued next); **R2013+ prologue
     divergence** (~63 rows in example_2013/2018) — gold reads
     `point_present=0/isolines=205/revision_major=2423192302/revision_bytes`
     as an odd hex-string where silver reads sane values from the same
     records: the two decoders consume the R2013+ has_ds_data-side prologue
     differently; needs a bit-level deep dive before ANY fix (match-gold
     may require silver-reader changes — the values are not derivable from
     silver's model); `encr_sat_data` (6 rows, v1) — gold re-emits the raw
     obfuscated per-block wire data (159-b transform) but silver merges
     blocks + strings stream into one text, losing the block layout;
     the pre-existing `prev/next-entity` family.
   - Write-side: the projection flows through silver_rt normalization, so
     the write family fell together (~1 235 → ~100); example_2013/2018's
     write rows remain excluded (-1) by the REGION NaN bug (queued above).
   Corpus: read **10 684 → 9 442**, write **9 894 → 8 668**. Per-file verified:
   ATMOS 986→42, Cone 17→1, TS1 35→4, example_2000 51→5, example_2004
   50→1, example_2007 50→2, example_2010 50→2 (family rows).
   Gates: `cargo test --features serde` = 1556/0, `gold_roundtrip` = ok.

   ~~TABLESTYLE~~ — **DONE (2026-09-19)**: the largest single type (was 2 351
   rows, 9 346 stem-inflated). Normalizer packet. The object has two disjoint
   shapes (dwg2.spec 964): legacy pre-R2008 (`name`/`flow_direction`/`flags`/
   `horiz_cell_margin`/`vert_cell_margin`/`is_title_suppressed`/
   `is_header_suppressed`/`rowstyles`) and modern R2010+ (`unknown_rc`/
   `unknown_bl1`/`unknown_bl2`/`cellstyle` handle + the `sty.cellstyle.*` /
   `ovr.cellstyle.*` named cell-style payload from the `CellStyle_fields`
   macro, dwg2.spec 244). Silver reads both into one struct (legacy →
   snake_case fields; modern → `modern_style`/`modern_overrides`). Projected
   both, version-gated on `r2010_plus`, including the nested
   `content_format.*` and per-border `borders[]` projection. Key subtleties:
   the `sty.cellstyle` detail block is only serialized when `data_flags != 0`;
   gold omits the `borders` array when `num_borders == 0`; silver's
   `line_weight`/`color`/`border_type` enums collapse via `normalize_color`/
   int passthrough. Corpus: read 25 526 → 19 516, write 26 270 → 20 899.
   Residuals (separate): `unknown_bits` (HANDLE_UNKNOWN_BITS verbatim, 36),
   `content_color` gold=256 vs silver ByBlock=0 (silver CMC value gap, 50),
   `borders` `[0,0,0,0,0,0]` gold per-border normalization vs silver dicts
   (45), and the entity-common `xdicobjhandle`/`is_xdic_missing`/`reactors`.

   ~~CMC color-method inversion~~ — **DONE (2026-09-19)**: a gold-side
   normalizer bug, found while auditing DIMSTYLE. Gold's own decoder
   (`bits.c` `bit_downconvert_CMC` 4061 / `include/dwg.h` DWG_COLOR_METHOD)
   defines the CMC `rgb` high byte (the method) as **c0 = ByLayer, c1 =
   ByBlock** — but `normalize_gold.py` had the `c0`/`c1` branches **inverted**
   (`c0`→0, `c1`→256), contradicting the spec. Silver's `read_cm_color`
   (bit_reader.rs) reads the bytes correctly (it just doesn't decode the
   method byte, defaulting ByBlock→0), so silver was right and gold's
   projection was wrong. Fixed the `c0`→256 / `c1`→0 map in normalize_gold.py.
   This also invalidated the `c_prop33` silver-side hack (which forced
   ByLayer→0 to compensate for the inverted gold map) — removed it; silver's
   256 now matches gold's 256 directly. And the DIMSTYLE `v==0 skip` guard
   (which dropped ByBlock on R2004+) — removed; the index is always projected.
   Corpus: read 20 167 → 19 199, write 18 723 → 17 803. **Lesson: when a
   "silver gap" looks wrong, check the gold normalizer against gold's own
   spec/decoder — the bug may be on the gold side.**

   ~~MLINESTYLE~~ — **DONE (2026-09-19)**: dwg.spec 4513. Projected
   name/description/flag (silver flags dict → gold BS 70 bitmask)/
   fill_color (CMC → normalize_color)/start_angle/end_angle/num_lines/lines
   (silver `elements[]` → gold offset list). Corpus: read 19 199 → 18 703,
   write 17 803 → 17 315. Residual: `lines` (39) — gold's REPEAT decode of the
   per-line array is degenerate ([0,0]); silver has the real offsets. Matching
   gold's degenerate output would need a bespoke gold-side rule; left as the
   faithful silver value.

   ~~MLEADERSTYLE color/handle/gates~~ — **DONE (2026-09-19)**: three bugs in
   the existing block. (1) Color inversion: `line_color`/`text_color`/
   `block_color` mapped silver "ByBlock" → `{"Index":256}` (ByLayer); should be
   the ACI index (ByBlock=0). Now `normalize_color(v)` directly. (2) Null
   handles: silver stores `Option<Handle>` (None=null); gold emits a null-handle
   dict (code 5, absref 0) — the differ resolves None to a non-token mismatch.
   Now emit the null-handle shape for `line_type`/`text_style`/`arrow_head`/
   `block`. (3) Version gates: `class_version` and `is_annotative` were gated
   on r2010_plus/r2007_plus, but MLEADERSTYLE objects are always read from
   EED/upconverted so gold emits both on EVERY version (R2000 included) —
   made unconditional (class_version defaults to 2). Corpus: read 18 703 →
    17 344, write 17 315 → 16 554. Residual: xdicobjhandle/is_xdic_missing.

   ~~DICTIONARYWDFLT~~ — **DONE (2026-09-19)**: dwg.spec 2804. Gold emits
   `dxfname` (ACDBDICTIONARYWDFLT) + `defaultid` (handle 340); silver stores
   `default_handle` (raw int). Projected both, dropped silver-only fields.
   DICTIONARYWDFLT 117 → 0.

   ~~3DFACE~~ — **DONE (2026-09-18)**: corner1-4 rename, invis_flags (drop when
   0), has_no_flags/z_is_zero/dxfname (R2000b+ defaults). **Follow-up fix
   (mapping audit, 2026-09-18):** `has_no_flags` was hardcoded to 1; correct
   derivation is `1 iff invisible_edges.bits == 0` (dwg.spec 2144
   `if (!has_no_flags) FIELD_BS0(invis_flags)`) — it is 0 on gh109_1 where
   invis_flags is present. Corpus: read 26 980 → 26 115 (combined with the
   MTEXT gate fixes below).
   29 341 → 27 589, write 29 559 → 27 863.

   ~~LAYER flag0/ltype~~ — **DONE (2026-09-18)**: the first reader packet.
   Silver's LAYER reader reads the R2000+ flag bitmask but discards the raw
   value (gold's flag0); silver resolves the linetype to a name but drops the
   handle. Store both on LayerData/Layer. Corpus: read 27 589 → 27 019, write
   27 863 → 27 313. **The reader-gap phase has begun.** **Reviewed** (ad4d835 →
   HEAD, 76 pairs): 31 624 → 31 190, 278 added rows all `LAYER.material`
   (generic-loop leak on pre-R2007 — the pop was inside the R2007 gate; fixed
   by popping unconditionally, f9b86ec). Corpus-wide checks clean: flag0 0
   mismatches, ltype 0 mismatches.

   ~~DIMSTYLE color vars~~ — **SKIPPED (2026-09-18)**: `DIMCLRD`/`DIMCLRE`/
   `DIMCLRT`/`DIMTFILLCLR` are a silver reader gap — the reader stores
   `dimclrd=0` (ByBlock) where the file has `256` (ByLayer). Not a normalizer
   fix. Reverted the attempted projection.

   ~~SOLID/TRACE~~ — **DONE (2026-09-18)**: corner1-4 rename + 2RD slicing;
   elevation left missing (silver reader drops it — real gap). Corpus: read
   30 766 → 29 182 → 29 341 (the +159 is the real elevation gap now surfaced),
   write 29 430 → 29 559. **Reviewed**: 0 corner mismatches, 0 leftover keys.
   Residuals: SOLID.elevation (reader gap), SOLID.color (dict cases),
   3DSOLID.* (ACIS/modeler — separate packet).

   ~~MTEXT entity~~ — **DONE (2026-09-18)**: full projection (renames, enum
   string→int maps, column_data nested → column fields, version gates).
   Corpus: read 35 155 → 30 737, write 34 294 → 30 685. **Reviewed** (0dd9dde
   → HEAD, 76 pairs): 45 105 → 37 598 → 30 766; 1 749 added rows were the
   version-gate leaks (bg_fill_* pre-R2004, column_* pre-R2018) — fixed with
   gates (60458af). Residuals (real gaps): style (silver stores name, not
   handle), R2018 embedded-object fields (is_not_annotative, class_version,
    default_flag, appid, ignore_attachment, column_*). **Follow-up fix
    (mapping audit, 2026-09-18):** the earlier gates (60458af) keyed on version
    only, not value. Corrected per dwg.spec 2881: `bg_fill_flag` is R2004a+
    (was leaking on R2000); `bg_fill_scale/color/trans` exist only when
    `bg_fill_flag & 1` (gold omits them when the fill is off — was leaking 381
    rows); `column_*` detail fields (column_count/width/gutter/auto_height/
    flow_reversed/heights) exist only when `column_type != 0` (was leaking 324
    rows even on R2018). rect_height moved to R2007 gate (was R2007-set +
    trailing pop; now consistent). Remaining residuals are reader gaps: style
    (name not handle), rect_height (silver stores Option None), R2018
    embedded-object fields, color.

   ~~ELLIPSE~~ — **DONE (2026-09-18)**: FIELD_NAME_MAP fix — silver's
   `major_axis`→`sm_axis`, `minor_axis_ratio`→`axis_ratio` (the map had the
   wrong silver key `minor_to_major_ratio`), `start_parameter`→`start_angle`,
   `end_parameter`→`end_angle`. Corpus: read 36 675 → 35 155, write 35 798 →
   34 294. **Reviewed** (4ef2e23 → HEAD, 76 pairs): 48 049 → 45 105, **0 added
   rows**; corpus-wide checks clean (all four renames exact, no leaks).

   ~~INSERT entity~~ — **DONE (2026-09-18)**: insert_point→ins_pt, x/y/z_scale→
   scale+scale_flag (recomposed per spec ENCODER), attributes→has_attribs,
   seqend_handle→seqend, num_cols/rows R11-only drop, block_name/name fix.
   Corpus: read 39 973 → 36 675, write 38 756 → 35 798. **Reviewed** (897e023 →
   HEAD, 76 pairs): 54 305 → 48 049, only 32 added rows (all `seqend` — gold
   resolves to SEQEND target type, silver's seqend_handle resolves to a raw int;
   the SEQEND reader-coverage gap). Corpus-wide checks clean: scale_flag 0
   mismatches, POINT leak 0, ins_pt/scale/rotation 0 mismatches. Residuals
   (real gaps): block_header (name-vs-handle), attribs/first_attrib/last_attrib
   (handle vectors), seqend (SEQEND coverage), color dict cases.

   ~~APPID fabrication~~ — **DONE (2026-09-18)**: CadDocument::new() pre-creates
   AcCmTransparency/AcAecLayerStandard for the writer's XDATA/EED needs; on the
   read path these persist as phantom records. Fixed: the DWG document builder
   now drops fabricated APPIDs the source file didn't contain (files that
   genuinely have them keep them). Corpus: read 41 153 → 39 973, write
   38 845 → 38 756. **Reviewed**: fabricated APPIDs absent on 70 files where
   gold lacks them, kept on 8 files where gold has them, 0 issues. Writer path
   unaffected (the fabricated APPIDs are only dropped on read). 46 test suites
   all ok, 0 failed.

   ~~MLEADERSTYLE~~ — **DONE (2026-09-18)**: full view-param projection
   (renames, enum string→int maps, linewt raw BLd, colors, block_scale 3BD,
   R2010b/R2013b gates, handle wraps). Corpus: read 56 712 → 41 162, write
   48 925 → 38 850. **Reviewed**: corpus-wide checks clean except one enum
   miss — my attach_* maps lacked the underline variants (BottomOfTopLine
   Underline*=6-8, CenterOfTextOverline=10); fixed (698e4ae). linewt 0
   mismatches, version gates 0 issues. Residuals (real gaps): arrow_head/block
   reader gap, class_version/is_annotative version gates, color CMC.

    ~~LINE/POINT color~~ — **REVERTED (2026-09-18)**: the `color != 0` drop was
    a net regression (+404 rows). Gold's field_cmc **always** emits the color
    index (including 0 for ByBlock); the `null` cases in the diff were the
    truecolor/alpha dict shape (silver doesn't model it), not an index-0
    default. Reverted to the pre-fix state (read 39 973, write 38 756). The
    color dict cases (truecolor with alpha, flag&32) are a real silver gap.
    **RESOLVED (2026-09-19):** the root cause was gold-side. `field_cmc`
    (out_json.c 595) omits `index` when it resolves to 0 (the `index > 0 &&
    index < 256` guard), so a ByBlock entity with ByBlock transparency
    (`alpha_type==1`) serializes as `{"rgb":"000000", flag:32, alpha_raw:…}`
    with NO `index` key — and the normalizer's collapse required `index`. The
    value is semantically ByBlock (= silver's 0); the dict is just gold's
    shape. Fixed in `normalize_gold.py`: a color dict with NO `index` and
    `rgb in (None,"000000",0)` collapses to `0`. This is the faithful gold-side
    canonicalization (the earlier `color != 0` drop was the wrong side and the
    wrong rule). Clears 814 `gold=null silver=0` rows across LINE/POINT/MTEXT/
    SOLID/INSERT/ARC/CIRCLE/LWPOLYLINE. Corpus: read 26 115 → 25 526, write
    26 633 → 26 270. Residual: 2× `SOLID.color gold=0 silver=256` (silver reads
    ByLayer where gold has ByBlock — a genuine reader gap, separate packet).

   ~~*_CONTROL/color~~ — **DONE (2026-09-18)**: LAYER color {Index:n}→int,
   flags→flag0, plotstyle_handle→plotstyle, material handle-wrap (R2007a+);
   STYLE is_shape_file→is_shape, height→text_size, is_vertical, big_font_file→
   bigfont_file, flags→generation. Corpus: read 62 577 → 56 712, write
   54 648 → 48 925. **Reviewed** (603668e → HEAD, 76 pairs): 67 579 → 62 107 →
   56 712; corpus-wide checks clean (color 0 mismatches, STYLE 0 mismatches,
   plotstyle 0 presence mismatches). One overcorrection fixed: LAYER material
   was dropped; now handle-wrapped. Residuals (real silver READER gaps, not
   normalizer): LAYER.flag0 (raw bitmask not stored), LAYER.ltype (handle not
   stored, only name), LINE.color (default-vs-omit), *_CONTROL.xdicobjhandle.

#### Gold-spec grounding for the reader-gap residuals (verified 2026-09-18)

Every one of the remaining top divergences is **100% spec-known** — the spec
block and field are located; what blocks them is silver *reader/codec* work,
not spec uncertainty:

| Divergence | Gold spec | What blocks it |
|---|---|---|
| `APPID.name` / `APPID._missing` | `dwg.spec` 4146 `DWG_TABLE(APPID)`; `name` is `COMMON_TABLE_FLAGS` (`spec.h` 755, `FIELD_T(name,2)`) | Silver fabricates `AcCmTransparency`/`AcAecLayerStandard` APPIDs gold lacks → ordinal shift; and silver doesn't parse some APPIDs. Reader gap. |
| `XRECORD.ownerhandle` | `dwg2.spec` 1117 `DWG_OBJECT(XRECORD)`; `ownerhandle` is `common_object_handle_data.spec` 51 `FIELD_HANDLE(ownerhandle,4,330)` | Silver resolves some XRECORD owners to `UNKNOWN` (unmodeled types). Reader-coverage gap. |
| `LINE.color` | `common_entity_data.spec` 491 `FIELD_CMC(color,62)` (entity-common) | Gold omits `color` when it equals the default; silver always emits. Default-vs-omit projection. |
| `LAYER.flag0` | `dwg.spec` 3305 `FIELD_CAST(flag0, RC, BS, 0)` (the raw LAYER flag bitmask) | Silver's reader decomposes the raw bitmask into bools and discards the raw value. Reader gap. |
| `LAYER.ltype` | `dwg.spec` 3303 `FIELD_HANDLE(ltype,2,6)` | Silver stores `line_type` as a name string, drops the handle. Reader gap. |
| `UNKNOWN._missing` | `objects.inc` 104 `DWG_ENTITY(UNKNOWN_ENT)`, 320 `DWG_OBJECT(UNKNOWN_OBJ)` — the fallthrough types for unmodeled objects | Silver doesn't model the object type at all (DIMASSOC, EVALUATION_GRAPH, SECTIONVIEWSTYLE, …). Reader coverage. |
| `*_CONTROL.xdicobjhandle` / `has_ds_data` / `is_xdic_missing` | `common_object_handle_data.spec` `FIELD_HANDLE(xdicobjhandle,…)` | Derivable: `is_xdic_missing` = `xdictionary_handle.is_none()`; `xdicobjhandle` needs the handle. Normalizer+reader. |

All are spec-locatable; the work is reader/codec, not normalizer projection.

   ~~MATERIAL map filenames~~ — **DONE (2026-09-18)**: gold's MAT_MAP emits
   `<map>.filename` only when `source==1` (file-based); silver emitted it
   always. Gated on `source==1`. Corpus: read 63 108 → 62 577, write
   55 131 → 54 648. **Reviewed** (66dde3c → HEAD, 76 pairs): 68 083 → 67 579,
   **0 added rows**; corpus-wide gate verification clean (gold/silver agree
   exactly: filename present iff source==1, 426 cases, 0 mismatches).

   ~~LAYOUT plotsettings.*~~ — **DONE (2026-09-18)**: full nested plot-config
   projection (printer_cfg_file/paper_size/canonical_media_name three-way swap,
   margins, 2D point pairs, plot_flags bits via silver's to_bits layout,
   shadeplot R2004a+/R2007a+, EXTMIN/EXTMAX/INSBASE/LIMMIN/LIMMAX/UCS*
   renames, handle wraps). Corpus: read 95 503 → 63 185, write 87 006 → 55 208.
   **Reviewed** (7ef5a1b → HEAD, 76 pairs): 88 391 → 68 135 → 63 108;
   plot_flags recomposition 0 mismatches, name swap 0 mismatches, 2D pairs
   within tolerance (f64 rounding), no leaks. One overshoot fixed:
   plotsettings.plotview is R2004a+ in gold, now gated. Residuals (real gaps):
   plotsettings.shadeplot (silver null handle), has_ds_data, plotview_name
   (DXF-only), viewports.

   ~~POINT view-independent fields~~ — **DONE (2026-09-18)**: silver
   `location`/`point` 3-vector split to gold `x`/`y`/`z` scalars, `normal`→
   `extrusion`, `x_axis_angle`→`x_ang`. Corpus: read 98 639 → 95 503, write
   89 406 → 87 006. **Reviewed** (d000524 → HEAD, 76 pairs): 93 847 → 88 391,
   **0 added rows**; corpus-wide checks clean (x/y/z exact, extrusion exact,
   x_ang exact, no leftover point/location/normal/x_axis_angle keys).

   ~~LTYPE dash patterns~~ — **DONE (2026-09-17)**: silver `elements[]`
   (length+complex) reshaped to gold `dashes[]` (8 fields), `alignment` char→
   ord, `pattern_length`→`pattern_len`, `numdashes` from len, `strings_area`
   all-zero TF (256 bytes pre-R2007 always, 512 R2007+ when has_strings_area),
   xref-bookkeeping drop. Corpus: read 101 741 → 98 639, write 92 472 → 89 406.
   **Reviewed** (1fd481c → HEAD, 76 pairs): 95 567 → 93 847, **0 added rows**;
   corpus-wide correctness checks clean (alignment ord exact, strings_area
   version gate correct on every version, numdashes matches everywhere).

   ~~VIEWPORT entity + VIEW table record~~ — **DONE (2026-09-17)**: full
   view-param projection for both (renames, 2RD slicing, status_flag bit
   recomposition via silver's own to_bits() layout, render_mode string→int,
   grid_flags dict→bits, composite VIEWMODE, derived aspect_ratio/view_width,
   R2000b/R2004a/R2007a gates, handle wraps). Corpus: read 103 649 → 101 741,
   write 94 153 → 92 472. **Reviewed** (84f5f84 → HEAD, 76 pairs): 99 156 →
   95 567, 97 added rows all `wrong_value` real gaps (VIEWPORT.named_ucs,
   status_flag bits 16–19, vport_entity_header, VIEW.VIEWMODE); 0 low-16
   status_flag mismatches; 0 2RD fields with ≠2 elements. One overshoot fixed:
   VIEW `is_camera_plottable` is R2007a+ in gold (dwg.spec 3836), now gated
   (was leaking onto pre-R2007 files). Residuals (real gaps): the named_ucs/
   status_flag/vport_entity_header/VIEWMODE gaps above, plus the
   *_CONTROL/xdicobjhandle control fields.

   ~~VPORT table record~~ — **DONE (2026-09-17)**: full view-param projection
   (renames, bool→int, render_mode string→int, grid_flags dict→bits,
   ambient_color Rgb→CMC, composite VIEWMODE/UCSICON bits, derived view_width,
   R2000b/R2007a gates, handle wraps, silver-only drops). Corpus: read
   112 493 → 103 649, write 102 857 → 94 153. **Reviewed**: VPORT-only
   old-vs-new recompute (d587311 → HEAD, 76 pairs) shows 104 566 → 99 156 with
   only 10 added rows, all `wrong_value` and all real silver gaps surfaced by
   the correct projection — `VPORT.sun` (gold SUN object vs silver UNKNOWN;
   silver doesn't model SUN) and one `VPORT.VIEWMODE` (PolyLine2D: silver sets
   the UCSVP bit where gold has 0). No structural regressions. Note: the
   `example_r13`/`example_r14`/`gh44-error` corpus dirs are out-of-scope legacy
   (run_corpus.py line ~33 skips R13/R14); stale versions of those dirs were
   removed from `target/gold_harness_corpus/` so their pre-packet content
   doesn't pollute future corpus scans. Residuals: VPORT_CONTROL.has_ds_data/
   is_xdic_missing/xdicobjhandle (control-object fields) + the sun/VIEWMODE
   gaps above.

   ~~BLOCK/BLOCK_HEADER topology~~ — **DONE (2026-09-17)**: silver normalizer
   renames + handle-wraps + version gates per the mapping table above (BLOCK
   entity drops DXF-only base_pt/description/xref_path; BLOCK_HEADER renames
   base_point→base_pt, block_entity_handle→block_entity, block_end_handle→
   endblk_entity, entity_handles→entities, units→insert_units, scale_uniformly→
   block_scaling; R2004+/R2007+ gates; drops flags/preview_data/insert_count_bytes/
   insert_handles/xref_path; emits xref_pname). Corpus: read 124 128 → 112 493,
   write 113 631 → 102 857. Residuals (separate gaps, status updated): 
   ~~`first_entity`/`last_entity` reader gap~~ (landed 2026-09-19),
   `BLOCK_HEADER.name` (*Paper_Space
   ordinal), ~~`xdicobjhandle`/`is_xdic_missing`~~ (landed `3cff463`),
   `BLOCK_CONTROL.model_space`/
   `paper_space`, and BLOCKVISIBILITY*/`._missing` reader coverage.
3. Re-run `run_corpus.py` after each landed packet to re-rank the queue.

Concrete entity-level mappings exposed by `2000/entities-2d.dwg` (good early
packets — small, well-scoped, and reproducible):
- **POINT**: gold splits the point into `x`/`y`/`z` scalars and uses
  `x_ang`; silver has `point` (vec) and `x_axis_angle`. Normalizer mapping.
- **TEXT**: gold uses `dataflags`, `ins_pt`, `horiz_alignment`,
  `vert_alignment`, `style` (handle); silver has `insertion_point`,
  `horizontal_alignment` (enum string), `vertical_alignment`, `style` (name).
  Mixed normalizer + reader-storage work (`dataflags`, `elevation` are
  currently not stored).
- **LWPOLYLINE**: gold has `flag`, `points`, `bulges`; silver has
  `is_closed`, `vertices` (nested), `constant_width`. Normalizer reshape
  (vertices → points/bulges) + `flag` derivation from `is_closed`.
- **Entity chain resolution**: `next_entity`/`prev_entity`/`ownerhandle`
  `wrong_value` diffs where the resolved target TYPE disagrees (e.g. gold
  `0` vs silver `LINE`). Investigate whether silver's chain reading is wrong
  or the differ's handle map misresolves — add a targeted probe before
  editing (print both normalized records for the same ordinal).

### 8.1.6a Gold-spec coverage audit (2026-09-17; re-verified structurally 2026-09-19 at libredwg 34f02f54)

**Constraint-group flat nodes + Helix era gates** — **DONE (2026-09-20
  fortieth batch, `c20e6fe`; read 53 → 38 / write 41 → 30, −15/−11;
  Constraints x4 1→0, Dynblocks 2/2 → 0/0, Helix x6 2/2 → 0/0)**:
  (a) ASSOC2DCONSTRAINTGROUP.nodes is gold's FLAT per-node REPEAT
  (dwg2.spec 5682 + AcConstraintGroupNode_fields 5576: nodeid BLd,
  status RC era-gated around num_connections + the BL vector) —
  silver's root-node + class-registry shape was a misparse (one global
  connection vector of garbage signed BLs; the count came out 1 or 113
  where gold reads 9 or 129). Reader AND writer now mirror the flat
  repeat (associative.rs:1057 / :788); the registry/class/data payload
  is a DXF-side semantic — the associative_constraint_group tests moved
  to flat DWG expectations (DXF assertions unchanged); (b)
  HELIX.knotparam/splineflags are SINCE(R_2013b) only (dwg.spec
  2588-2590): emit gated, pre-2013 records omit both.

**gh109_1 zero (RAPIDRT rapid-order gold shadow + SORTENTSTABLE wire
  entries)** — **DONE (2026-09-20 forty-first batch, `707e7a8`; read
  38 → 22 / write 30 → 16, −16/−14; gh109_1 16/14 → 0/0)**:
  (a) RAPIDRTRENDERSETTINGS gold mistyping root: at exactly AC1027
  gold's AcDbRenderSettings_fields reads the base `has_predefined` bit
  via `VERSION (R_2013) { FIELD_B (has_predefined) }` BEFORE the rapid
  seven while the wire carries the entire rapid block first (the spec
  block's own tail `VERSION (R_2013) {} else FIELD_B` — spec.h
  VERSION(v) is an EQUALITY check, so at R2018 gold parses cleanly;
  gold also `VALUE_BL`s class_version read-and-discard with no JSON
  emissions — out_json.h has no VALUE_BL). Gold's cursor therefore
  enters the rapid block ONE BIT LATE: every rapid value is a
  deterministic misparse (BL codes: bit1 of the true BL code reads as
  has_predefined 0, then RL32s whose IEEE-shaped bytes spill across
  the following BD fields — verified bit-for-bit by a python
  re-encode+walk against all six records, 40/54 values matching before
  the tail-bits were synthesized; the Rust walk reads the real record
  tail so all match). Fix: the RAPIDRTRENDERSETTINGS arm repositions
  the merged reader (set_position_in_bits) and re-walks the just-read
  bits in gold's order via the same READ primitives, storing
  `RapidRtGoldShadow {has_predefined i32, six u32 BLs, two f64 BDs}`
  on the object (u32 because gold prints BL through FORMAT_BL "%u":
  2709520922 etc. print UNSIGNED); normalize_silver types ClassObject
  data.RapidRtRenderSettings → RAPIDRTRENDERSETTINGS and projects
  base fields (class_version omitted at AC1027) + the shadow rapid
  fields; the UNKNOWN_OBJ count_mismatch rows re-balanced with the
  retype (silver was KEEPING all six 537-class records as UNKNOWN_OBJ;
  the "dropped wrappers" theory was wrong — nothing was dropped).
  (b) SORTENTSTABLE: `SortEntitiesTable::add_entry` deduped wire pairs
  BY ENTITY HANDLE, so gh109_1's four dead pairs (ent 0, sort
  919/1219/928/1027) collapsed into one entry with each later slot
  OVERWRITING the first slot's sort (final: 1027) and shrank the
  table 39 → 36, mispairing mid-list. New `add_wire_entry` appends
  DWG-wire entries verbatim (entry_map keeps the first occurrence);
  the DWG read path uses it, the writer already serialized entries
  verbatim. Readers take note: gold's dwg2.spec 149-161 (the
  SORTENTSTABLE block) reads sort_ents as code-0 handles from the
  MAIN stream swap, ents as code-4 from the handle stream.

**Chain-ordinal wave (kid wire handles, class-entity owners, VIEWMODE
  nibble, vertex reactors, ANG2LN mapping, mesh chain, vport header,
  table has_ds)** — **DONE (2026-09-20 forty-second batch, `604f956`;
  read 22 → 0 / write 16 → 2, −22/−14; PolyLine2D 7/6 → 0/0, TS1
  3/3 → 0/0, LiveSection1 4/2 → 0/0, example_2000 3/3 → 0/0,
  example_2004/2007/2010/2013/2018 1/0 each → 0/0)**:
  (a) Vertex2D.wire_handle reader retention (kid handle synthesis):
  the normalizer's `parent+1+i` guess produced THE WRONG HANDLE
  whenever a foreign block-chain record interleaves the kid run —
  2000/PolyLine2D's LINE@512 sits between POLYLINE_2D@511 and its
  true kids 513/514, so the synthesized "VERTEX_2D@512" stole the
  LINE's handle and poisoned handle resolution for BLOCK_HEADER.
  last_entity, LINE.handle and POLYLINE_2D.next_entity in one root
  (three rows, one bug). The withheld wire handle is now retained
  through PendingVertex and serialized; the synth prefers it.
  (b) assoc-network vertex registration: Vertex3DPolyline.reactor_
  handles retained from the held EntityCommon and echoed by the
  kid-record writer (write_vertex3d) — gold prints the reactors only
  on the ONE registered kid (example_2000 kid 1054 → net 1071); the
  synthesis emits non-empty lists only (gold's no-reactor kids carry
  no key).
  (c) rebuild_block_membership no longer forces non-block owners to
  model space when the owner resolves to a real object: the
  CAcLayoutPrintConfig (dwg2.spec 4929, an entity-typed CLASS record)
  is owned by its DICTIONARY (856) with a NULL model-space chain —
  gold keeps the raw owner and prints nothing in the chain. One
  mechanism cleared LAYOUTPRINTCONFIG.ownerhandle (orig+rt) AND
  BLOCK_HEADER.first/last_entity for Model_Space.
  (d) VIEW/VPORT VIEWMODE: gold reads ONE FIELD_4BITS nibble (bits.c
  bit_read_4BITS composes MSB-first — FIRST-read bit is value 8) from
  the wire order perspective, front_clipping, back_clipping,
  front_clip_at_eye. The old ucs-bool compose fabricated every record;
  both the VPORT and the VIEW branches now compose from the four wire
  bits. VIEW.is_camera_plottable is SINCE(R_2007a): gated, and the
  silver key is consumed on every era so it never leaks extra.
  (e) DIMENSION_ANG2LN.xline2end_pt: the arm sourced `first_point`
  for xline2end; dwg.spec DWG_ENTITY(DIMENSION_ANG2LN) LATER_VERSIONS
  wire order (2RD def_pt, 3BD xline1start/xline1end/xline2start/
  xline2end) maps silver's dimension_arc/first_point/second_point/
  angle_vertex/definition_point 1:1 — one-word fix.
  (f) INSERT-kid ATTRIB xdicobjhandle: thread the kid common's
  xdictionary_handle (silver's kid synthesis had no xdic emission;
  gold prints [3,2,355,355] on 2000/TS1's kid 354).
  (g) VERTEX_MESH chain (2000/TS1'S 12-vertex grid): the FIRST kid
  chains forward (prev null/next second kid), the LAST kid chains
  back to the PREVIOUS kid (gold 539: prev {8 → 538} — a code-8
  zero-offset ref), the middles carry bare nolinks=1. The old arm
  printed the null form for every edge vertex.
  (h) VIEWPORT.vport_entity_header: the pre-R2004 wire slot itself
  (dwg.spec VIEWPORT FIELD_HANDLE @ VERSIONS (R_13b1, R_14) before
  the frozen layers / (R_2000b, R_2002) after the clip boundary) is
  retained on the model (vport_entity_handle), echoed by the writer,
  and emitted verbatim by the normalizer INSTEAD OF the vx-table
  lookup fabrication (2000/PolyLine2D: first viewport → VX 875, the
  rest the null form — gold reads exactly that from silver's rewrite
  once the echo is in place).
  (i) R2013+ table-entry has_ds_data: Pass 1 captures the
  NonEntityCommonData bit into dwg_data_store_handles (the same set
  the object branch and the writer's bit echo use); the table section
  projects has_ds_data from the set instead of hardcoding 0 —
  2018/LiveSection1's VIEW records carry the AcDs bit on the wire.

**Deep-gate arms for the new raw-retention vertex fields** — DONE
  (2026-09-20 forty-third batch, `e0c6f93`): normalize_entity_for_
  comparison clears Polyline2D vertex wire_handle and Polyline3D
  vertex reactor_handles on both sides — the writer allocates fresh
  kid handles on constructed documents, so the retained wire values
  are fidelity bookkeeping (gold-harness kid synthesis), not payload
  semantics (the Polyline3D zeroed-vertex-handles precedent). The
  r2000/r2013/r2018 deep tests improved past their 1-known-Shape
  budget once these cleared: roundtrip 97/0, 47 segments ok.

**ASSOCPLANESURFACEACTIONBODY assocdep/pbsab_status** — **DONE
  (2026-09-21 forty-fourth batch, `9b1738c`; write 2 → 0 (read stayed
  0); 2004/Surface 0/2 → 0/0; corpus read 0 / write 0, 124 of 124
  files at 0/0 — CAMPAIGN TARGET REACHED)**: the dossier decode
  (`pk19_55_surf_dossier.sh`; extract lines ~:108638 orig /
  ~:120542 rt) held with ONE measured refinement. Founded truths: (1)
  the JSON key `assocdep` flattens TWO embedded slots (pab's,
  sab's) with LAST-WINS — gold prints the SAB slot, the pab value
  1293 never prints; (2) the ORIG record is TRUNCATED (23 data
  bytes; handle region exactly [owner (8.0.0)→1291][deps
  (3.2,0x50E)=1294][pab.assocdep (4.2,0x50D)=1293] + one leftover
  bit); (3) silver's reader overread the truncation — the garbage
  source PINNED by the E1 walk: BOTH cursors live in the bounded
  23-byte record slice, the poison is the PARTIAL-FILL window
  [bit 183..191) where the record's last bit `1` (the truncation's
  leftover) zero-fills: the sab.assocdep form byte reads 0x80 →
  code 8, counter 0 → `ref_handle(1292) − 1 = 1291`; the
  pbsab_status BL's `'01'` code consumes [181..183) and its RC8 reads
  the same window = 0x80 → 128 (NOT the post-record `AD AE` CRC
  bytes — the slice never sees them); gold refuses both via its
  dat-end checks and prints NULL/0. (4) MEASURED REFINEMENT of the
  dossier's forecast: gold decodes an EXPLICIT `(5,0)` null-form
  slot as a REAL handle (the [5,0,0,0] tuple print → normalized
  `{"__handle_code__": 5, "__handle_target__": 0}`), NOT the
  overflow `[0,0]` — the overflow-refuse print and the valid-null
  print are DIFFERENT wire states. THE LANDED FIX (three edits):
  (a) reader bounds — `DwgMergedReader::record_end_bits()` /
  `main_record_remaining_bits()` added (`handle_remaining_bits()`
  already measured the same end); `read_surface_body`'s dependency
  pull + `read_surface_action`'s path_status/class_version/option
  tails read through extent-checked gold-parity helpers
  (`handle_remaining_bits < 8` → Handle::NULL; BL: consume the
  2-bit code when it fits, refuse the value that would cross —
  cursor discipline mirrors gold bit-for-bit); intact records never
  reach the guards (their final main field ends at the declared
  bitsize, well inside the record). (b) writer — a NULL
  sab.assocdep is echoed by OMITTING the slot (the truncated
  wire carried none; gold then overflow-prints [0,0] on the rewrite
  too, and silver's guard reads its own rewrite identically — both
  pairs converge); writing a fabricated (5,0) form would flip the rt
  pair into a gold [5,0,0,0]-vs-silver-[0,0] row (measured before
  the omit). (c) normalizer — the SurfaceActionBody arm projects
  assocdep from surface_body.dependency (0/NULL → the [0,0] null
  pair, real int → the handle dict) and pbsab_status from
  path_status. Gates: 47 serde segments ok (roundtrip 97/0, 1312
  units), gold_roundtrip ok; verification matrix: Surface +
  example_2000/2004/2007/2010/2013/2018 + LiveSection1 + Dynblocks +
  gh209_1/gh109_1 + TS1 + PolyLine2D + PolyLine3D ×4 all 0/0 both
  pairs; corpus 0/0; residues empty. THE QUEUE IS NOW EMPTY — the
  campaign target (read AND write 0 on both sides, raised 2026-09-20)
  is COMPLETE.

**Answer: gold specs are 100% KNOWN, but NOT 100% COVERED.** Measured
empirically against the corpus (not just the diff report):

- **Known = 100% (re-verified structurally 2026-09-19; full-repo audit).**
  Precise block census: 322 real `DWG_ENTITY/OBJECT/TABLE` starters with
  unique names — `dwg.spec` 88 (53E/25O/10T) + `dwg2.spec` 234 (45E/189O).
  (The pre-audit figure "324 (88+236)" had counted the 2 commented-out
  mentions — `OBJECTCONTEXTDATA`, `PARTIAL_VIEWING_FILTER`.) **248 blocks are
  live in the built oracle** (dwg.spec 87, dwg2.spec 161); the other 74 are
  debug/dead-gated (§8.1.1 liveness rule) and can never be emitted typed.
  Macro census: 66 spec `*_fields` macros confirmed (`1 dwg.spec` is the
  renamed-dead `TABLE_value_fields_REMOVED`, 62 dwg2.spec, 3
  dwg_spec_shared.h) — plus dwg2.spec holds ~25 field-list macros WITHOUT
  the `_fields` suffix (`row/cell/content/…` for TABLESTYLE, `MAT_COLOR/
  MAT_TEXTURE/MAT_MAPPER/MAP` for MATERIAL, `lnode/lline`, `BlockParam_*`,
  `ramp`, …) that a nested-root grep must not miss.
- **Gold emits 159 distinct types in the corpus** (fresh per-file `dwgread
  -O JSON` census, 2026-09-19; one gold-decode failure: `2013/gh44-error.dwg`;
  the previous count said 160). **All 159 map to LIVE spec blocks** — the
  only name differences are the documented aliases `_3DFACE`/`_3DSOLID`/
  `_3DLINE`. Correction to the 2026-09-17 audit: `OBJECTCONTEXTDATA` is NOT
  a gold-emitted type (its block is commented out, dwg2.spec 3813) — the
  `OBJECTCONTEXTDATA.*` diff rows come from **silver** naming a
  leader-context record with the abstract-class name, while gold emits
  `LEADEROBJECTCONTEXTDATA` (6 corpus records; live block dwg2.spec 4611).
  Small packet candidate: fix silver's type name (see §8.1.6).
- **Covered ≠ 100%.** [2026-09-17 figures; recount pending after the audit:]
  silver emits **84 distinct types**; 75 shared with gold. On those 75 shared
  types, **703 gold fields never appear on the silver side** (the
  field-level backlog). So coverage is roughly 75/159 types and a fraction
  of fields per shared type.

*Coverage by category (re-audited 2026-09-19; fix-loop pass count preserved
from the 2026-09-18 baseline):*

| Category | State | Count |
|---|---|---|
| Real spec block starters (dwg.spec + dwg2.spec, unique) | all inventoried | 322 (88 + 234) |
| …of which live in the built `dwgread` | the emittable set | 248 (87 + 161) |
| Registered type names (`objects.inc`, generated, unguarded) | = all starters except the 5 `#if 0`-dead; includes the 69 debug-gated | 317 = 97 entities + 220 objects |
| Types gold emits in corpus (fresh census 2026-09-19) | all map to live blocks | 159 |
| Types silver emits [2026-09-17 figure] | — | 84 (75 shared with gold) |
| Gold types silver never emits (reader/naming gap) | **uncovered** | ~84 (recount pending) |
| Gold fields missing on the 75 shared types [2026-09-17] | **uncovered** | 703 |

*The gold-only types* (~84; silver reader/naming coverage gap — the largest
structural class). 2026-09-19 audit corrections: `ACSH_*` is **10
live-emittable** classes (+`ACSH_BREP_CLASS`) — the SWEEP/EXTRUSION/LOFT/
REVOLVE history-subclass blocks are debug-gated (dwg2.spec 3716 region) and
never emitted by gold, and the corpus report carries no rows for them;
`OBJECTCONTEXTDATA` was mislisted here in the 2026-09-17 audit — it is a
silver-side naming bug (see above). The members: all `ACSH_*` (10 live,
incl. `ACSH_BREP_CLASS`), all `ASSOC*` (16), `VERTEX_2D/3D/MESH/PFACE/
PFACE_FACE` (silver stores vertices inside the parent polyline — §10),
`DIMENSION_*` subtypes (silver has one `Dimension` variant — §9), `SEQEND`,
`ATTRIB`, `EVALUATION_GRAPH`, `SECTIONVIEWSTYLE`/`DETAILVIEWSTYLE`,
`PROXY_OBJECT`, `SUN`, `TRACE`, `BLOCK*ACTION`/`BLOCK*PARAMETER`/`BLOCK*GRIP`,
`RENDER*`/`MENTALRAY*`/`RAPIDRT*`, `PDF*/UNDERLAY`, `UNKNOWN_ENT/OBJ`, and the
`SECTION_*`/`LAYOUTPRINTCONFIG`/`CELLSTYLEMAP`/… singles. Each is a
`FOO._missing`/`FOO._count` row in the report. ~~`DIMASSOC`~~ — **DONE
(2026-09-18)**: not a reader gap; silver read it all along. The gap was the
`Associative`→`UNKNOWN` normalizer flatten. Note the ASSOC* `_missing` rows in
the report are the *normalizer* coverage gap (silver reads them into
`Associative`, the normalizer keeps them UNKNOWN until each field projection
lands) — the reader dispatch for ASSOC* already exists
(`object_reader/associative.rs::read_associative_data`).

*The 703 missing fields on shared types* are dominated by: LAYOUT/PLOTSETTINGS
(nested plot config), TABLESTYLE `sty/ovr cellstyle.*` nested structs,
MULTILEADER `ctx.*`, MLEADERSTYLE (full rename set), VIEWPORT/VPORT/VIEW view
params, GEODATA, 3DSOLID/REGION ACIS+revision, HATCH gradient, HELIX/SPLINE
NURBS, LEADER, IMAGE/WIPEOUT, plus the near-universal `isbylayerlt`
(entity-common) and the `*_CONTROL.has_ds_data/is_xdic_missing` control-object
fields.

*Per-packet spec grounding (the landed packets — "done" = that packet's target
fields are spec-verified, NOT the whole type):*

| Packet | Spec source | Coverage of the type's gold spec |
|---|---|---|
| ownerhandle/absref differ semantics | `common_object_handle_data.spec` `FIELD_HANDLE(ownerhandle,4)` + `common_entity_handle_data.spec` | **differ-only**, correct; not a field coverage item |
| SCALE flag/is_temporary | `dwg2.spec` 1194 `DWG_OBJECT(SCALE)`: `flag name paper_units drawing_units is_unit_scale` | **100% of SCALE's spec fields** (is_temporary correctly dropped — it is libredwg-internal, NOT in spec) |
| XRECORD xdata/cloning | `dwg2.spec` 1117: `xdata_size xdata cloning`(R2000b+) `objid_handles` | partial: `objid_handles` (handle-stream) NOT yet projected |
| DICTIONARYVAR schema/strvalue | `dwg.spec` 4585: `schema strvalue` | **100%** |
| DICTIONARY/DICTIONARYVAR reactors | struct carry + side-channel inject | correct (reactors are common-object, `common_object_handle_data.spec` `REACTORS(4)`) |
| VISUALSTYLE property bag | `dwg2.spec` 2092: pre-R2010 named fields + R2010+ `value/int` pairs + R2013+ `*_prop*` bag | partial: bag order mapped, but `c_prop33` default mapped by *empirical* (all-zero) default, not a spec-stated default — verify against a non-trivial file if one appears |
| MATERIAL maps/colors | `dwg2.spec` 2769: MAT_COLOR/MAT_MAP macros + R2007a advanced set | partial: `#if 0` block (indirect_bump_scale, luminance, normalmap, …) is NOT in gold's serialized output — correctly excluded, but if libredwg ever emits them, revisit |
| LWPOLYLINE vertex reshape + vertexids | `dwg.spec` 5446 `DWG_ENTITY(LWPOLYLINE)`: `flag points bulges vertexids`(R2010b+) | partial: `const_width/elevation/thickness/extrusion` gated on flag bits — verified; `num_vertexids`/`widths` not separately projected (folded) |
| LAYER linewt enum→byte | `common_entity_data.spec` `FIELD_RC(linewt,370)` + LAYER `dwg.spec` 3298 | linewt correct; LAYER table record itself NOT covered (see residual `LAYER.*` rows) |
| BLOCK/BLOCK_HEADER topology | **diagnosed, not yet implemented** (`dwg.spec` 3146–3278) | mapping table ready in §8.1.6 |

*What is NOT covered — the real remaining backlog (read-fidelity, by type,
deduplicated counts are lower due to the stem-collision inflation noted in §7):*

- **LAYOUT** (~33 066): `plotsettings.*` nested plot config + `INBASE/LIMMIN/
  LIMMAX/UCSORG/…` + `min_limits/max_extents/…` + handle refs. Largest single
  gap. Spec: `dwg.spec` `DWG_OBJECT(LAYOUT)` (5316) + embedded PLOTSETTINGS.
- **MLEADERSTYLE** (~16 799): full per-field mapping missing (gold `dwg2.spec`
  1461 `DWG_OBJECT(MLEADERSTYLE)`); silver uses different field names entirely
  (`content_type` vs gold, `class_version`, `mleader_order`, …). Big normalizer
  packet.
- **BLOCK_HEADER** (~~10 691~~ residual ~160: `name` ordinal + write-side
  `is_xref_ref`): **packet DONE** (§8.1.6 + `3cff463`) — the topology
  renames/gates, first/last entity, and the `xdicobjhandle`/`is_xdic_missing`
  family (via the `xdic_by_handle` map) all landed; only the name-ordinal and
  write-side is_xref_ref residuals remain.
- **TABLESTYLE** (~9 346): `sty/ovr cellstyle` nested structs + `unknown_bits/
  version/flags/…`. Spec `dwg2.spec` 964.
- **VPORT** (~8 816, `dwg.spec` 3934 `DWG_TABLE(VPORT)`) and **VIEWPORT**
  (~1 798, `dwg.spec` 2412 `DWG_ENTITY(VIEWPORT)`): view params (`VIEWCTR
  VIEWDIR FRONTZ BACKZ GRID* SNAP* UCS*`), naming differs (gold `VIEWCTR` vs
  silver `view_center`).
- **MTEXT** (~5 932, dwg.spec 2881), **INSERT** (~3 563, dwg.spec 735),
  **POINT** (~3 382, dwg.spec 2030), **LTYPE** (~2 968 dash patterns, dwg.spec
  3580), **STYLE** (~2 805 font/shape, dwg.spec 3479), **3DFACE** (~1 859,
  dwg.spec 2057 — spec name is `_3DFACE`), **SOLID** (~1 699, dwg.spec 2274),
  **ELLIPSE** (~1 523, dwg.spec 2555), **APPID** (~1 272; name=fabricated
  APPIDs, reader gap), ~~**MULTILEADER** (1 269 → done 2026-09-19, §8.1.6;
  residual 42 stem-rows: unknown_bits/graphic_data/attach_top/attach_bottom +
  prev_entity; writer flags/arrow_size corruption queued as write packets)~~,
  **3DSOLID**
  (~1 077 ACIS/modeler, dwg.spec 2681 — spec name is `_3DSOLID`), **DIMSTYLE**
  (~1 036 residual DIM* vars, dwg.spec 4188).
- **UNKNOWN.* and `*._missing`/`._count`** (~5 000+ combined): unmodeled object
  coverage on the silver reader — the single largest *structural* class. Each
  `FOO._missing` is a gold object type silver does not parse (DIMASSOC,
  EVALUATION_GRAPH, SECTIONVIEWSTYLE, ACSH_*, ASSOC*, …). These need reader
  support in cadcodec, not normalizer work.

*Complete residual-cluster census* (every type in the current report is in one
of these — verified 2026-09-17, none unnamed). Beyond the top list above:

- **`*_CONTROL` tables** (~~2 684~~ xdic family landed in `3cff463`):
  `APPID/BLOCK/DIMSTYLE/LAYER/LTYPE/STYLE/UCS/
  VIEW/VPORT_CONTROL` — ~~`has_ds_data`/`is_xdic_missing`/`xdicobjhandle`
  handle fields~~ (DONE via the `xdic_by_handle` map projection) +
  `BLOCK_CONTROL.model_space`/`paper_space`/`LTYPE_CONTROL.byblock`/
  `bylayer` (the control record's own child handles). Controls lacking an
  xdic derive `is_xdic_missing`/`has_ds_data` correctly (R2004+/R2013+).
- **Legacy polyline/mesh variants** (~250): `POLYLINE_2D`, `POLYLINE_3D`,
  `POLYLINE_MESH`, `POLYLINE_PFACE`, `POLYFACE_MESH`, `POLYGON_MESH` — legacy
  (pre-LWPOLYLINE) polyline forms; spec `dwg.spec` (POLYLINE_2D/3D blocks) +
  mesh variants. Partially reader gaps (mesh variants), partially naming.
- **Single-object reader-coverage types** (`_missing`/`_count`, silver does not
  parse): `PLACEHOLDER`, `TABLEGEOMETRY`, `LEADEROBJECTCONTEXTDATA`,
  `OBJECTCONTEXTDATA`, `ARC_DIMENSION` (separate from `DIMENSION_*` — its own
  entity), `DYNAMICBLOCKPURGEPREVENTER`, `VX_TABLE_RECORD`, `POLYLINE_MESH`,
  `POLYGON_MESH`, `PLANESURFACE`, `SECTIONOBJECT`, `SECTION_MANAGER`,
  `SECTION_SETTINGS`, `LAYOUTPRINTCONFIG`, `CELLSTYLEMAP`. Each is a small
  reader packet (add the type to the silver reader + a normalizer mapping).
- **Misc small**: `TABLE` (the TABLE entity, `dwg.spec`), `SORTENTSTABLE`
  (block_owner/entry_map handles), `FIELD`/`FIELDLIST` (value/childval
  nesting), `MLINE`/`MLINESTYLE`, `HATCH` gradient/pattern, `GROUP`,
  `RASTERVARIABLES`, `OLE2FRAME`, `GEODATA`, `SUN`, `LIGHT`.
- **Remaining named smalls**: `DICTIONARYWDFLT` (dictionary-with-default;
  `defaultid`/`default_handle` handle fields, spec `dwg.spec`), `IMAGEDEF` /
  `IMAGEDEF_REACTOR` (raster image definition objects: file_path/size/resunits
  + reactor), `ENDBLK` (the ENDBLK entity — shares the BLOCK entity's DXF-only
  field treatment), `IMAGE`/`WIPEOUT` (raster clip/display props), `MLINE`,
  `RAY`/`XLINE` (point/vector naming), `3DFACE` corner/invis_flags.

This census is exhaustive for the current corpus: every `(type, field)` row in
`report.json` belongs to one of the clusters above or the top list. When a new
cluster appears after a corpus run, add it here before working it.

*Ordering guidance:* the naming/shape clusters are mostly done (VPORT/
VIEWPORT/VIEW, LTYPE, POINT, LAYOUT, BLOCK_HEADER). Remaining normalizer-only
work: MATERIAL map filenames (`refractionmap.filename`, ~372), MTEXT/INSERT/
STYLE/3DFACE/SOLID/ELLIPSE naming, and the `*_CONTROL` handle fields.
MLEADERSTYLE and TABLESTYLE are large but mechanical (nested structs). The
`*._missing` coverage and 3DSOLID/ACIS require reader/storage work.

#### Nested (dotted) field paths — how gold defines them

Some gold types nest sub-structs; the differ surfaces them as **dotted field
paths** (e.g. `TABLESTYLE.sty.cellstyle.content_format.value_format_string`).
Every nested root is defined by a spec **`*_fields` macro** (or an inline
`SUBCLASS`), so each dotted segment is spec-known — there is no opaque nesting.
To consult a nested field, open the defining macro, not just the outer
`DWG_OBJECT`/`DWG_ENTITY` block:

| Gold JSON nested root | Defined by | Spec location |
|---|---|---|
| `LAYOUT.plotsettings.*` / `PLOTSETTINGS.*` | inline `SUBCLASS (AcDbPlotSettings)` | dwg.spec 5225, 5318 |
| `TABLESTYLE.sty.*` / `.ovr.*` (and TABLE cells) | `CellStyle_fields` (wraps `ContentFormat_fields`) | dwg2.spec 244, 227 |
| `MULTILEADER.ctx.*` (and MLEADERSTYLE context) | `MLEADER_CONTEXT_DATA_fields` | dwg2.spec 1227 |
| `BLOCK*.evalexpr.*`, `ACSH_*.evalexpr.*`, `ASSOC*` expr | `AcDbEvalExpr_fields` | dwg2.spec 2860 |
| `ACSH_*.history_node.*` | `AcDbShHistoryNode_fields` | dwg2.spec 2910 |
| `MATERIAL.<color>.*` (`ambient_color`, `diffuse_color`, `specular_color`) | `MAT_COLOR` macro (emits `.rgb`/`.method`/…) | dwg2.spec 2680; uses at 2774, 2775, 2782 |
| `MATERIAL.*map.*` (`diffusemap`, `specularmap`, `reflectionmap`, `opacitymap`, `bumpmap`, `refractionmap`) | `MAT_MAP` macro (wraps `MAT_MAPPER`/`MAT_COLOR`) | dwg2.spec 2680 (MAT_COLOR), 2743 (MAT_MAPPER), 2700 (MAT_MAP uses) |
| `UNDERLAY.*` sub-fields | `UNDERLAY_fields` | dwg2.spec 1621 |
| `ASSOC*.assocdep.*` | `AcDbAssocDependency_fields` | dwg2.spec 1863 |
| `ASSOC*.pab.*` (extruded/lofted/revolved surface action bodies) | `AcDbAssocParamBasedActionBody_fields` | dwg2.spec 1928 |
| `BLOCK*PARAMETER.prop3` / `prop4` (BLOCKLINEAR/BLOCKROTATION) | inline per-parameter `FIELD_*` block (a 2-element parameter tuple) | dwg2.spec (per-parameter blocks) |
| `ASSOCVARIABLE.u.*` | `AcDbAssocVariable` inline (`SUBCLASS`, dwg2.spec 5694) | dwg2.spec 5691 |
| `FIELD.value.*` and TABLE cell values | `TABLE_value_fields` macro (moved out of the spec into `dwg_spec_shared.h`; the `dwg.spec` 5868 stub is `#REMOVED`/inactive) | dwg_spec_shared.h |

The remaining `*_fields` macros (constraint/assoc/block-param families,
`AcDbObjectContextData_fields` dwg2.spec 3612, …) follow the same pattern:
grep `#define <root>_fields` across both spec files. A nested-field packet
diffs the *leaf* scalar; walk the macro chain to find the leaf's `FIELD_*`
macro, bitcode type, and version gate exactly as for a flat field.

**Verified exhaustive (2026-09-18):** a full rescan of the libredwg tree
(`grep -rE '#define [A-Za-z_0-9]+_fields'` across `dwg.spec`, `dwg2.spec`,
`dwg_spec_shared.h`, plus all `SUBCLASS` blocks) confirms **66 `*_fields`
macros** total (1 dwg.spec + 62 dwg2.spec + 3 dwg_spec_shared.h) and **21
distinct nested roots** in the corpus gold JSON. Every one of the 21 roots maps
to a spec construct (macro or inline SUBCLASS) — there is no opaque nesting.

### 8.1.7 Hard prohibitions (loop invariants, restated)

- Never edit anything under `~/work/libredwg`.
- Never edit `tests/gold_harness/ignore_fields.toml` or
  `tests/gold_harness/diff_fields.py`.
- Normalizer edits must be *faithful projections* (correct names, correct
  version gates, correct reshaping) — never hide a real data mismatch.
- Never touch version dispatch, decompression, CRC, or the section map.
- Never leave the build red at a checkpoint.
- Header stays out of scope (lax).

---

## 9. Field-mapping starting table (silver `EntityType` → gold spec name)

Reference data for the differ, independent of coverage. Mappings for
out-of-loop types (MTEXT, DIMENSION_*, TABLE, MULTILEADER, VIEWPORT, IMAGE,
MESH, 3DFACE, SOLID, ATTDEF/ATTRIB, TOLERANCE, WIPEOUT) are pre-staged and
become active once coverage lands.

LINE→LINE, Circle→CIRCLE, Arc→ARC, Point→POINT, Text→TEXT, MText→MTEXT,
LwPolyline→LWPOLYLINE, Polyline2D→POLYLINE_2D, Polyline3D→POLYLINE_3D,
Ellipse→ELLIPSE, Spline→SPLINE, Insert→INSERT, Ray→RAY, XLine→XLINE,
Hatch→HATCH, Solid→SOLID, Face3D→3DFACE, Viewport→VIEWPORT, Leader→LEADER,
Tolerance→TOLERANCE, MLine→MLINE, AttributeDefinition→ATTDEF,
AttributeEntity→ATTRIB, Solid3D→3DSOLID, Region→REGION, Body→BODY,
Table→TABLE, MultiLeader→MULTILEADER, RasterImage→IMAGE, Wipeout→WIPEOUT,
Underlay→*UNDERLAY, Ole2Frame→OLE2FRAME, Shape→SHAPE, Helix→HELIX,
Mesh→MESH.

The authoritative machine-readable maps live in `normalize_silver.py`
(`ENTITY_TYPE_MAP`, `OBJECT_TYPE_MAP`, `FIELD_NAME_MAP`); the list above is
kept for design context only.

**Special cases the normalizer must handle:**

- **Dimension**: acadrust has ONE `EntityType::Dimension` variant (with a
  sub-discriminator); gold has separate
  `DIMENSION_ORDINATE/LINEAR/ALIGNED/RADIUS/DIAMETER/ANG2LN/ANG3PT/ARC/LARGE_RADIAL`
  spec types. Map on the sub-discriminator, not the variant.
- **Polyline/LwPolyline**: gold `LWPOLYLINE` (R14+) vs `POLYLINE_2D/3D`
  (legacy); a legacy gold POLYLINE_2D may surface in silver as `LwPolyline`
  or `Polyline2D` — allow a many-to-one type alias set.
- **Insert vs MINSERT**: gold `MINSERT` is acadrust `Insert` with
  `num_cols/num_rows > 1`.
- **Vertex entities**: gold emits `VERTEX_2D/3D/MESH/PFACE/PFACE_FACE` as
  separate objects; acadrust stores vertices inside the parent polyline. The
  normalizer must either expand silver polylines into child vertex records or
  fold gold vertices into the parent before diffing. **Not implemented yet.**

(Extend via `classes.inc` for gold class names of 500+ types.)

---

## 10. Out-of-loop scope

The following are intentionally deferred until they block a covered entity or
the corpus expands:

- Full object topology fixes for `DICTIONARY`, `XRECORD`, `SCALE`,
  `VISUALSTYLE` representation mismatches.
- Polyline vertex expansion (no covered fixture has expanded vertices yet).
- Resolving handles to `(type, name)` instead of `(type)`.

---

## 11. Next steps

1. ~~Open the rewritten files in AutoCAD~~ — **done**: user confirmed
   `AcDbVisualStyle` "Object improperly read" on R2000/R2004 rewrites, clean on
   R2007+. Root cause identified and fixed (see §7), fix confirmed by user.
2. ~~EntityCommon storage-only field gaps~~ — **done** (see §7). LINE/CIRCLE
   entity diffs are zero on all six versions, read + write fidelity.
3. Continue Phase 6 per the subagent workflow in §8.1:
   table-record storage fields first (they gate `GOLD_HARNESS_STRICT=1`), then
   object representation gaps by corpus frequency.

---

## 12. How to run

Required environment variables:

| Variable | Purpose | Default |
|---|---|---|
| `GOLD_DWGREAD` | Absolute path to a built LibreDWG `dwgread` binary | **required** |
| `GOLD_TESTDATA` | Absolute path to LibreDWG `test/test-data` directory | **required** |

```bash
cd ~/work/cadcodec
source $HOME/.cargo/env

# Verify the environment is ready
python3 tests/gold_harness/check_env.py

# Build the harness binaries
cargo build --features serde --bins

# Run harness on one file (output goes to target/gold_harness/ by default)
GOLD_DWGREAD=$HOME/work/libredwg/programs/dwgread \
GOLD_TESTDATA=$HOME/work/libredwg/test/test-data \
python3 tests/gold_harness/run_roundtrip.py \
    $HOME/work/libredwg/test/test-data/2000/Line.dwg

# Run the full corpus batch driver
GOLD_DWGREAD=$HOME/work/libredwg/programs/dwgread \
GOLD_TESTDATA=$HOME/work/libredwg/test/test-data \
python3 tests/gold_harness/run_corpus.py

# Run representative cargo test
cargo test --features gold-harness --test gold_roundtrip

# Strict mode: assert zero missing_in_silver diffs
GOLD_HARNESS_STRICT=1 cargo test --features gold-harness --test gold_roundtrip
```

---

## 13. File inventory

| File | Purpose |
|---|---|
| `tests/gold_harness/run_roundtrip.py` | Single-file driver |
| `tests/gold_harness/run_corpus.py` | Batch driver |
| `tests/gold_harness/normalize_gold.py` | LibreDWG JSON normalizer |
| `tests/gold_harness/normalize_silver.py` | cadcodec JSON normalizer (holds the live type/field maps) |
| `tests/gold_harness/diff_fields.py` | Diff engine |
| `tests/gold_harness/ignore_fields.toml` | Fields ignored during diff (frozen) |
| `tests/gold_harness/check_env.py` | Environment sanity checker |
| `tests/gold_harness/src/bin/dwg2json.rs` | Silver JSON dump (re-injects 17 `EntityCommon` fields) |
| `tests/gold_harness/src/bin/dwgrewrite.rs` | Silver rewrite binary |
| `tests/gold_roundtrip.rs` | Cargo integration test (feature-gated) |
| `tests/visualstyle_dwg_roundtrip.rs` | VisualStyle regression test |

---

## 14. Self-sufficiency notes

The harness is intentionally self-contained in `tests/gold_harness/`:

- All Python scripts, normalizers, the differ, and the environment checker
  live under `tests/gold_harness/`.
- The silver JSON dump and rewrite binaries live under
  `tests/gold_harness/src/bin/` and are registered as Cargo `[[bin]]` targets.
- Working directories default to `target/gold_harness/` and
  `target/gold_harness_corpus/`, so outputs are repo-local and easy to inspect.

The only intentionally external dependencies are:

1. **LibreDWG `dwgread`** — the gold oracle; set via `GOLD_DWGREAD`.
2. **LibreDWG `test/test-data`** — the DWG corpus; set via `GOLD_TESTDATA`.
3. **Python 3** — the normalizer/differ runtime.

These are documented in `check_env.py` and are not embedded because they are
large, license-distinct, or runtime-level.

---

## 15. Risks and mitigations

- **serde(skip) on common fields (resolved by design):** `EntityCommon` skips
  the non-header round-trip fields, so a naive serde dump would false-positive
  them as missing. Mitigated by re-including them from the live struct under
  `_common_dwg` (Phase 2). Raw byte blobs stay excluded intentionally.
- **Field-name mismatch** between serde JSON and gold: mitigated by the
  per-entity mapping table in `normalize_silver.py`; grows iteratively.
- **Structure mismatch (not just names):** gold nests objects under
  `OBJECTS`/`ENTITIES` and expands polylines into child `VERTEX_*` records;
  acadrust nests vertices inside the parent polyline and stores blocks via
  `block_records`. The normalizer must reshape (expand/fold) before field
  diffing, not just rename keys. **Polyline reshape not implemented yet.**
- **Handle/ordering instability** across a rewrite: align by `(type,
  ordinal)`, never raw handle; compare handles by resolved target *type*.
- **Unknown/proxy entities:** cadcodec `Unknown`/`Extended` and libredwg
  skipped objects will not align; the differ tolerates count mismatches and
  reports them as coverage gaps, not hard errors.
- **libredwg read warnings** on advanced R2010+ objects: treat gold `dwgread`
  non-zero exit / `Error` log lines as "gold cannot decode → skip file", not a
  silver failure.
- **Float formatting:** gold uses `%.14f`; normalize with rel-tolerance `1e-6`
  (abs `1e-10`), and NaN→0.
- **Build time** for libredwg in WSL: use `--disable-bindings --disable-docs`;
  cache the build dir.
- **Strings/codepages:** pre-2007 files use legacy codepages; normalize both
  sides to UTF-8 before compare (gold already emits UTF-8).
- **Corpus gaps limit the loop:** the loop cannot fix MTEXT/DIMENSION/TABLE/
  INSERT/etc. (no gold file). They are explicitly logged out-of-loop; do not
  let the agent synthesize or guess their structure.
- **Autonomous-loop drift:** an agent might "fix" a diff by weakening the
  normalizer, widening tolerances, or extending `ignore_fields.toml` mid-loop.
  Loop invariants forbid editing the differ/normalizer/gold/`ignore_fields.toml`
  during the loop; these are fixed oracles.
- **Git unavailable in some environments:** step 6 checkpoints via commit only
  when git is available, else records in `report.md`; the regression gate is
  VCS-independent.
- **Cross-version regression:** a fix for one version can break another. Step 5
  re-runs the full corpus every iteration; step 6 gates on corpus-wide diff
  count strictly decreasing.
- **Non-convergence / oscillation:** a field may flip between two wrong
  states. 5-attempt cap per offender, then mark BLOCKED and move on.
- **Writer-only divergences:** some diffs only appear in `gold_rt`. These are
  valid targets (writer must match gold too), but require fixing
  `object_writer/`, not the reader.

---

## 16. Validation

- `programs/dwgread -O JSON` runs clean on a sample of in-scope gold DWGs.
- `cargo build --release --features serde` succeeds; `dwg2json` emits valid
  JSON on a gold DWG including re-included `EntityCommon` fields
  (`material_flags`, `plotstyle_flags`).
- Harness produces a `report.md` ranking divergent `(entity, field)` pairs
  across the in-scope corpus.
- At least one real divergence is found, fixed in cadcodec, and confirmed via
  re-run.
- `cargo test` (default, harness feature off) remains green and fixture-free.
- **Fix loop:** after running, the covered corpus reaches zero non-header,
  non-ignored field diffs (SUCCESS) or every remaining offender is BLOCKED
  with a recorded reason (PARTIAL); `report.md` lists fixed fields, blocked
  fields, and out-of-loop entity types. `cargo test` and
  `cargo test --features serde` are green at every checkpoint.

---

## 17. Future work (explicitly out of current scope)

### F1 — Header silver-vs-gold comparison

The header is lax/out-of-scope for the current loop. A later phase can extend
the same differ to header variables, reusing all existing machinery:

- **Gold source:** `dwgread -O JSON` top-level `FILEHEADER` (version string,
  `maint_rel_version`, `codepage` int, section addresses, R2004+ unknowns) and
  `HEADER` (every header variable by its spec `$VAR` name). Keys are
  version-gated; gold omits `num_*` counters and NaN doubles.
- **Silver source:** `CadDocument.header: HeaderVariables` (serde-derivable;
  snake_case fields with the `$VAR` name in each doc comment).
- **Work items:** (a) header-var name mapping table; (b) extend `normalize_*`
  for header types (points, handles, colors, dates); (c) treat version-gated
  gold keys absent for a file's version as "not present", never as mismatch.
- **Recommended strictness tier:** compare a curated subset of semantically
  load-bearing vars first (`LUNITS`, `INSUNITS`, `MEASUREMENT`,
  `EXTMIN`/`EXTMAX`, `DWGCODEPAGE`, `HANDSEED` ordering sanity), then widen.
- **Caution:** `HANDSEED`/section addresses in `FILEHEADER` legitimately
  change on rewrite — exclude them from the write-fidelity header diff,
  include them only in read-fidelity.

### F2 — Manually generating coverage-gap files with AutoCAD/BricsCAD

The corpus gaps (MTEXT, DIMENSION, TABLE, REGION, static INSERT/BLOCK,
MLEADER, VIEWPORT, IMAGE, MESH, 3DFACE, SOLID, ATTDEF/ATTRIB, TOLERANCE,
WIPEOUT) can be closed by authoring fixtures in a real CAD application, after
which the existing loop covers them with **no harness changes**:

1. **Author minimal files:** one drawing per gap entity, kept minimal (one or
   two entities on layer 0) so diffs stay localizable.
2. **Save per version:** SAVEAS to each in-scope format (R2000, R2004, R2007,
   R2010, R2013, R2018); only produce versions where the entity natively
   exists.
3. **Qualify each file as gold before use:** run `dwgread -O JSON`; only files
   libredwg decodes without `Error` enter the corpus.
4. **Provenance/licensing:** fixtures authored from scratch are clean test
   data. Note the generator in a companion `.txt`.
5. **Loop integration:** drop the new files into the corpus dirs, re-run the
   Phase 4 driver, and the entity types move from "OUT OF LOOP" to covered
   automatically.

### F2.1 In-repo fixture tree — `tests/gold_harness/tests/`

Authored fixtures that are **not part of libredwg** live in-repo, so the
corpus can regress them without touching the read-only oracle:

```
tests/gold_harness/tests/
  README.md                    — the fixture conventions (in place)
  <campaign>/                  — one directory per coverage campaign
    <Entity>_<version>.dwg     — one operation per file
    <Entity>_<version>.txt     — provenance companion (F2 step 4)
```

First resident campaign: `sh_history/` (the ACS/`AcDbSh*` solid-history
family). Its wire layouts are documented in the gold spec — the live
`DWG_OBJECT` blocks in `~/work/libredwg/src/dwg2.spec` (ACSH_HISTORY_CLASS
~line 3077, ACSH_SWEEP_CLASS ~4175, ACSH_EXTRUSION_CLASS ~4222, the
primitive/boolean/loft/revolve siblings around 2910–3300 and 4170–4400;
class-number registry in `src/classes.c`) — and the current interim state
is the per-class elide (commit `459bc74`; ACSH_HISTORY_CLASS has since
been un-elided by Phase A `b926053` — the node classes EXTRUSION,
SWEEP, the primitives, and BREP remain elided; their 3DSOLID/REGION/
BODY history soft-pointers go NULL until each lands). The Downloads `Polysolid.dwg` stays the byte-
**calibration** specimen, NOT a corpus fixture: it is a 35k-object
real-world working drawing, far past the minimality bar.

Fixture rules (deltas from the gold-tree convention above):

- **Version-suffixed stems** (`Polysolid_2018.dwg`), not per-version
  directories: `run_corpus.py` keys per-file workdirs by path stem, and
  the gold tree's per-version `Leader.dwg` stems collide (the known
  count inflation). Fixture stems are globally unique.
- **Git tracking**: the root `.gitignore` carries a blanket `*.dwg`
  (line 28); the fixture tree must be negated in
  (`!tests/gold_harness/tests/**/*.dwg`) or the files silently never
  commit.
- **Qualification is per file, before landing**: (a) `dwgread -O JSON`
  decodes with zero `Error` lines; (b) the target class/entity is
  present in the JSON (grep it); (c) the object census is minimal — a
  one-operation drawing is tens of objects, not thousands; reject
  real-world-scale files.
- **Version scope**: only versions where the feature natively
  persists. SH solid history is the 2007+ genus — SAVEAS down to
  R2000/R2004 flattens the ACSH records away. Verify each save by
  grepping the class in the JSON; emit no fixture for versions that
  dropped it.

### F2.2 Authoring checklist (per fixture)

1. **Fresh drawing**: default template, layer 0, empty canvas.
2. **Perform exactly ONE operation** from the campaign table (F2.3).
3. **SAVEAS** to the target DWG version — one file per version.
4. **Qualify** per F2.1: no `Error` in `dwgread -O JSON`, target class
   present, minimal object census.
5. **Land it**: copy to
   `\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec\tests\gold_harness\tests\<campaign>\<Entity>_<version>.dwg`.
6. **Provenance**: write the sibling `<Entity>_<version>.txt` — author
   app + build, date, the exact command sequence, the save format, and
   the qualification result. Either AutoCAD or BricsCAD may author
   (both are native writers; record which — see the origin-quality
   census in README).
7. **Driver extension (one-time)**: `run_corpus.py::in_scope_files`
   gains the fixture source after the gold-tree collection:
   `files.extend(sorted((SCRIPT_DIR / "tests").rglob("*.dwg")))`.
   Until it lands, fixtures run through `run_roundtrip.py` singly.
8. **Re-run the corpus** and confirm each new file appears with the
   per-file counts the campaign expects. SH fixtures enter as known
   nonzero until the ACS/SH campaign's Phase A lands — that is the
   loop working, not a bug; the fixtures become the regression anchors
   that keep the landed layouts at zero.

### F2.3 First campaign table — `sh_history/`

| stem | operation (command) | covers | versions |
|---|---|---|---|
| `Polysolid_<v>` | `POLYSOLID` — draw one straight segment | `ACSH_SWEEP_CLASS`, `ACSH_HISTORY_CLASS`, `3DSOLID` | 2007/2010/2013/2018 — verify each SAVEAS keeps the ACSH records |
| `Extrude_<v>` | `CIRCLE`, then `EXTRUDE` the circle | `ACSH_EXTRUSION_CLASS` (Phase A target), `ACSH_HISTORY_CLASS`, `3DSOLID` | same |
| `Revolve_<v>` (Phase B+) | `CIRCLE` + `REVOLVE` | `ACSH_REVOLVE_CLASS` | as landed |
| `Loft_<v>` (Phase B+) | two `CIRCLE`s + `LOFT` | `ACSH_LOFT_CLASS` | as landed |
| `Box_<v>` / `Sphere_<v>` (Phase B+) | `BOX` / `SPHERE` | the SH primitive classes | as landed |
| `Union_<v>` (Phase B+) | two `BOX`es + `UNION` | `ACSH_BOOLEAN_CLASS` | as landed |
| `Wedge_<v>` / `Cylinder_<v>` (Phase C) | `WEDGE` / `CYLINDER` | the SH primitive classes | landed 2026-09-23, qualified (all 0/0) |
| `Cone_<v>` / `Torus_<v>` (Phase C) | `CONE` / `TORUS` | the SH primitive classes | landed 2026-09-23, qualified (all 0/0) |
| `Pyramid_<v>` (Phase C) | `PYRAMID` | `ACSH_PYRAMID_CLASS` | landed 2026-09-23, qualified (all 0/0) |
| `Fillet_<v>` / `Chamfer_<v>` (Phase C) | `BOX` + `FILLET` / `BOX` + `CHAMFER` | the edge-modification nodes (with the parent `ACSH_BOX_CLASS` chain) | landed 2026-09-23, qualified (all 0/0) |
| `Brep_<v>` | **DEFERRED** (2026-09-23): `ACSH_BREP_CLASS` is not reachable through any user-facing AutoCAD op — seven authored attempts (plain op, SLICE, single-op, foreign-body graft, SOLIDEDIT face edit, real-template source) all produced either parametric chains or history-stripped plain solids. The class stays elided; silver already tolerates the one real-world carrier (`ATMOS-DC22S.dwg`) at 0/0. The row re-opens if an authentic specimen surfaces (legacy SAT-era import paths). | — | — |
| *(differential queue — CLOSED 2026-09-24)* | **ALL 17 STEMS AUTHORED AND QUALIFIED** (the maintainer landed the full §18.7 set): the 14 solid stems (56 files) are IN THE CORPUS at 0/0 (244 files now) with their quads bit-identical and their OBSERVED outcomes recorded per row in §18.7 — the sweep spine is named (ExtrudeT: the draft_angle raw at bit 70, the SweepOptions order), the extrusion payload is the profile (ExtrudeR/P), the height lives in the direction (ExtrudeH), the loft height/radius slots are named (LoftH/R), the sweep frame direction entries are confirmed (PolysolidX/D), the revolve axis pair takes offset/tilt with world-X profile centers and normalized directions (RevolveO/T), 360° = plain 2π (RevolveF), and RevolveW is BIT-IDENTICAL to the original. The 3 M-stems (12 files) + the 8 surface-mode C first attempts (renamed `ExtrudeCSurf_<v>`/`LoftCSurf_<v>` — their Solid re-authors kept the C names) LANDED 2026-09-24 with the surface-parser row (see §18.6's decode record: the classes gold-shadow walk + the R2013+ sab grammar): the corpus stands at **280 files 0/0**, every §18.7 specimen in the tree. **Closing-set state (2026-09-24, COMPLETE — the maintainer
   fixture surface is EMPTY): PolysolidL landed in-corpus and its probe
   was DECISIVE — the 2.0109 single (and @900/@1092) is
   PROFILE-DERIVED, bit-identical between the half-path L and the
   full-path X; 4.00024414192312 the record constant; the width single
   confirmed at 3.0; only the segment end is path-derived. All
   named-class candidates in the sweep tail are resolved, and the
   surface-parser row closed the last 20 quarantined files (read/write
   0/0 on all four versions each). What remains is agent code work —
   the post-corner singles walk, the loft container walk (the
   per-section slots are named; the inter-value regions are not), the
   ExtrudeP polyline header — **ALL THREE WALKS LANDED 2026-09-26
   (see §18.6's records: the record constant + width-coupled single
   named; the loft per-section fields spliced same-form; the kind-77
   body through the embedded grammar; hermetic cover 15 → 20); the
   remaining opaque regions are catalogued there** — and the
   no-authoring-path rows (LoftD
   dead; the revolve option shorts + flags — the typed anchor is now
   in-corpus via RevolveM; BREP external-only).** | the blob-autopsy remainder (§18.6) | 2007/2010/2013/2018 each, landed |

---

## 18. Oracles — the four validation layers

The campaign uses four oracles. Layers 1–3 are the standing regression set
(always run, in this order, after any codec change); layer 4 catches a defect
class the others cannot see by construction.

1. **Deep unit gates** (fast, per change): `cargo test --features serde` —
   47 ok test segments; the roundtrip suite asserts `diffs <= max_known`
   (the known-issue budget — a new raw-retention model field without a
   `normalize_entity_for_comparison` arm trips the budget and the failing
   test names the exact field) — fully oracle-free (no LibreDWG needed).
   Plus `cargo test --features gold-harness --test gold_roundtrip` = ok —
   **oracle-optional** (2026-09-21): without `GOLD_DWGREAD`/`GOLD_TESTDATA`
   it skip-passes and writes the notice file
   `target/gold_harness_oracle_skipped.txt`; with `GOLD_HARNESS_REQUIRE=1`
   set, absence is a hard failure instead (CI knob); and
   `bash tests/gold_harness/bootstrap_oracle.sh` clones and builds the
   oracle online on demand, then prints the env exports.

2. **Full corpus** (the authoritative 0/0 check; run detached, LAST):
   `target/probes/pk17r_corpus.sh` (nohup, absolute log path, sleep 8,
   pgrep confirm) + ONE bounded `24_wait.sh` call;
   `target/gold_harness_corpus/report.json` must show
   `read_fidelity_total: 0` and `write_fidelity_total: 0` (`per_file` is
   truth — the by-type tables truncate), and
   `python3 target/probes/pk18a_all_rows.py` must produce an empty residue
   list.

3. **Staleness rule:** re-read the fresh report before interpreting
   anything — a second launched corpus can race the report, and no edits
   are allowed while one runs.

4. **Authored-wire byte-fidelity oracle** (added 2026-09-21 from the
   BricsCAD strict-load campaign). Byte-compare silver's *rewrite* of an
   authored file against the authored original, object record by object
   record. It catches writer **form** defects — legal-but-different
   bitcode choices (a BS emitted as `'01'+RC` where the authored wire used
   `'00'+RS16`, BD short-forms vs raw doubles, CMC alpha-method nibbles,
   handle-region boundary bits) — that BOTH decoders tolerate, so the
   parser-parity diffs stay 0/0 while a strict consumer (BricsCAD,
   AutoCAD) rejects or drops the record. The blind spot is structural:
   the harness's rt pair compares gold's parse of silver's bytes against
   silver's parse of the *same* bytes — two lenient readers of identical
   bytes can never disagree about a form. Procedure, per record pair:

   - Find the record in both files: `dwgread -v9` prints each object's
     frame — `Size: N [MS], Hdlsize: 0xH [UMC], Type: T [BOT],
     Address: A` (TCP-era records at the printed section Address).
   - Build the silver side: `run_roundtrip.py` on the authored file
     produces `<stem>_rt.dwg` (silver's full-file rewrite); trace it the
     same way.
   - Dump both records: `target/debug/dump_section_bytes <file> <A-6> <N+12>`
     — the R2000+ record frame is [MS size, 2 bytes at Address−4]
     [UMC Hdlsize, 1 byte at Address−2] [BOT type, 1 byte at Address]
     [data bits from Address+1] [2-byte CRC at the end]; dump with margin
     around the whole span.
   - Bit-compare the two records from the data start (byte-identical
     prefixes hold the comparison; the first divergent bit, combined with
     a spec-ordered field walk — BS/BD/BL/CMC/H bitcode forms per
     `dwg.spec`/`dwg2.spec` — names the culprit field and the authored
     emission form it deviates from).
   - Acceptance target: the rewritten record's bytes equal the authored
     record's bytes, per record (handle slots that legitimately
     re-number on rewrite compare by code/counter shape, not value;
     CRC excepted). BricsCAD plain-open on the user side remains the
     final strict-consumer confirmation.

   Worked example (the LEADER pair of 2018/Leader.dwg vs its silver
   rewrite): bit-compare pinned the deltas to a single alpha-nibble
   value bit plus ~6 main + ~2 handle-boundary bits localized to the
   final tail after box_width; the MULTILEADER pair pinned a −17-bit
   main-region delta to its final ~180-bit tail. Those record-pars and
   the probe scripts that produced them live in the strict-load
   campaign handover (`NEXT_SESSION.md`); the envelope-lab pattern
   (`examples/xleader_lab.rs`) is the reusable isolation tool.

   Resolution (2026-09-21, second session — the campaign's
   corrected-frame analysis; supersede the earlier short-form-BS
   theory): the corrected per-record frame is window = file
   [Address..Address+Size) (chain bit 0 = MSB of the BOT byte; the BOT
   type read consumes the first ~10 bits; window-derived bitsize =
   Size*8 − Hdlsize equals the handle-region start, matching gold's
   `dec_macros.h` OVERSHOOT/MISSING bookkeeping). With it:
   - LEADER pair: main fields [10..1819) BIT-IDENTICAL in both records
     (the box_width raw-64 sits at window bits [1731..1797) — the
     earlier not-found-pattern anomaly was a wrong slice base); the
     only value divergence = ONE CMC alpha-method bit (auth
     0x2000056, type 2 vs silver's 0x3000056, type 3). Auth fix:
     `Transparency::to_alpha_value` now packs the 2-form like
     `to_dxf_value` (authored census: leader 0x2000056, layers
     0x200003a; `from_alpha_value` already tolerated both). The
     leader handle-chain offset (auth declares bitsize = main_end−5
     and overlaps the stream tail — silver writes the sequential
     genus) is a writer-family difference, NOT a defect: real-AutoCAD
     authored leaders (2013/gh44-error.dwg, 3 records) use the
     sequential +2/+3 gap genus. Silver keeps sequential.
   - MULTILEADER pair: main fields [10..6406) BIT-IDENTICAL; strings,
     data_size RS, has_strings flag, and the whole 9-handle chain are
     IDENTICAL and only shifted −17 bits. The entire delta = a 17-bit
     unparsed bit-group `00100101000010010` that ODA parks between the
     walked spec tail (is_text_extended) and the string-stream anchor.
     Width is content-dependent (a real-AutoCAD sample with empty text
     carries a 9-bit group sharing the same suffix `10010`); no public
     spec models it; gold/our reader land it in unknown_bits residue.
     Fix: capture-and-echo — `MultiLeader.dwg_raw_tail_bits:
     Option<(u64, u8)>` captured on read between the main cursor and
     the string-stream anchor (merged-reader `main_data_end()`), and
     written back verbatim; `MultiLeader::new()` seeds the ODA
     text-content default so constructed entities (GENALL) carry the
     authored form. After the fix, silver's Leader.dwg MULTILEADER
     rewrite record is BYTE-IDENTICAL to the authored record (833
     bytes, Hdlsize 0x76, CRC included), and the harness normalize
     pops the echoed field from gold-shape projections ("residuals
     kept": gold-only unknown_bits, now symmetric).
   Both fixes passed the full §18 battery: cargo test 47-ok segments,
   per-file roundtrip 0/0, corpus 124-file 0/0, residues empty.

   Campaign zero achieved (2026-09-21 ~20:45Z, user-verified:
   `gen_all_entities_all_versions.dwg` — 30 entities — "opens
   flawlessly" via plain `_open`). The final three writer fixes, each
   user-round verified (rounds four through six):
   - **LEADER R2010+ underlap assembly** (`DwgMergedWriter::set_underlap_tail`):
     the no-text flag + handle region park back inside the main tail
     (bitsize = main_end − 6), reproducing the authored ODA layout
     (gold reads it as the native −6 OVERSHOOT). Value-preserving
     only: the merge verifies main tail bits == [false] ++
     handle-head and falls back to the sequential layout otherwise
     (bit-writer readbacks `bits_at`/`first_written_bits`/
     `last_written_bits` were added for the check). Round five user
     test: the `(41)` leader warning GONE on the 29-entity file.
     After the fix silver's rewrite leader record is BYTE-IDENTICAL
     to the authored record (240/0x6A, CRC included).
   - **MULTILEADER hidden tail bit-group**: the simple-content constant
     `0b000010010` (9 bits — BricsCAD + AutoCAD authored samples and the
     gold tree's gh44-error.dwg agree bit-for-bit) lands between
     is_text_extended and the string-stream anchor
     (`MultiLeader.dwg_raw_tail_bits`, capture-echo on read).
     Attribution corrected by the 2026-09-22 specimen-stamp census: the
     17-bit variant `0b00100101000010010` is a CONTENT-CLASS convention
     (the gold tree's Leader drawing family), NOT an ODA-writer
     fingerprint — that family's 2007/2010/2013 down-saves self-identify
     as AutoCAD 2017/2018 saves and carry the same group as the
     ODA-FileConverter 2018 variant, which merely reproduces it.
     Rewrites echo whichever bits the record carried; constructed
     entities take the simple-content 9-bit default.
   - **MULTILEADER entity-common proxy-graphics blob**: the last
     structural delta — every authored AcDbMLeader record carries the
     entity-common graphic metafile (ODA "PROXY ENTITY GRAPHICS",
     §29/p.270; the spec body is image-rendered in the PDF, but the
     format derives cleanly from the specimens): `[u32 total][u32
     record_count]` + records `[u32 size][u32 type][payload]`
     (little-endian; `record_size` counts its own header). The
     derivation is now implemented and tested
     (`src/entities/proxy_graphics.rs`): typed records for the
     12-byte state/selector words (types 16/18/19/20/22/23/51 —
     property selectors 0x3A99/0x3A9A/0x2711/0x1389/1, state words
     0/0x7FFF/0xC0000000), FillOff (21) and UnicodeText (36) as
     before, and opaque passthrough for the geometry records
     (38 = the mleader header block, 6/7 = n-point 3D primitives,
     32 = point-forward blocks). The gold tree's own 564-byte
     2018/Leader.dwg AcDbMLeader metafile is the embedded test
     fixture: decode -> encode is asserted byte-identical, and the
     census test pins the 12-record shape. The live derivation tool
     is `dump_proxy_graphics [--verify] FILE [HANDLE]` (harness bin)
     — verified byte-identical on the gold specimen, the BricsCAD and
     AutoCAD samples, and the zero-file's own record; it also lists
     every graphic-carrying entity (the gold file's IMAGE entities'
     type-6 clip polygons decode through the same envelope). The
     every constructed-mleader wire component is gold-tree-derivable: the
     attach stance (0/32/4786 + extended) and the 9-bit tail group also
     exist in the gold tree's AutoCAD-authored 2013/gh44-error.dwg.
     Round seven (2026-09-21): the gold-tree ODA blob variant
     ("GENALL_PROXY_BLOB=oda") opened flawlessly in BricsCAD and became
     the DEFAULT (`MLEADER_PROXY_GRAPHIC_ODA` in the example); the
     round-six BricsCAD-authored 448-B specimen stays as the
     `GENALL_PROXY_BLOB=bcad` fallback. The canonical deliverable is
     byte-identical to the user-verified variant (md5
     0217fbac515a20b90e9c3aea883196e3). A content-synthesizing
     generator is the remaining future step.
   Byte-oracle end-state: silver's rewrite reproduces BOTH problem
   records byte-identical to the authored file; the constructed records
   carry the authored morphologies. Corpus 0/0 and residues empty on
   every intermediate state; generation is deterministic.

### 18.5 ACS/SH solid-history Phase A (started 2026-09-23)

The next campaign after the zero. The SH family
(`AcDbSh*` catch-all modeler-history objects, carried under
`DynamicBlock` with ACSH_* `dxf_name`) was elided at save
(`459bc74`, after a 2026-09-22 extrusion probe made strict readers
refuse the whole file) pending true wire layouts. The F2 fixture
tree (`fc9f235`, §F2.1–F2.3) landed the authored calibration set
that drives the implementation.

- **Phase A step 1 — ACSH_HISTORY_CLASS (`b926053`)**: the root
  record (the simplest SH layout: 2 BLs + handle + BL + 2 Bs) was
  already correctly implemented on both sides; the elide guard and
  the entity-pointer nuller now allow it through. All node classes
  (EXTRUSION, SWEEP, the primitives, BREP) stay elided. Verified on
  the Polysolid_2018 fixture: the record round-trips at the same
  handle, the SWEEP record (gold UNKNOWN_OBJ fallback, 261 bytes)
  stays elided, corpus gold 0/0 unchanged, fixture diffs 26/8
  unchanged (the node classes remain).
- **Phase A step 2 — ACSH_SWEEP_CLASS (2026-09-23, DONE)**: the
  handover's blob-field model was disproven by the layer-4 autopsy
  before implementation. On the authored wires the presumed
  `method`/`shsw_text_size` BL positions read 0 (`'10'` runs) while
  1733 bits of real option/transform/flag content follow to the
  record end — a per-field model built on gold's debug-spec guess
  would corrupt and shrink the record (the probe rewrite was 148
  bytes vs 261; the old "aligned reader" belief was an artifact of
  record-boundary jumps, and the spec's `//744` blob comment is
  from a different genus). Gold has no oracle for these classes at
  all: `src/classes.inc` registers them `DEBUGGING_CLASS`, so even
  a `-DDEBUG_CLASSES` build refuses the body walk ("Unstable Class
  object … (0x80)") and stops right after `history_node.color.flag`,
  emitting the rest as `unknown_bits`. What IS oracled is the shared
  skeleton — the live sphere class (`-v9` trace) pins: parentid
  BLd(-1), eval major/minor 33/427, value_code BSd(-9999), nodeid 1,
  hist major/minor 33/427, 16 BD transform, CMC (44 bits: index 0,
  rgb `c0000000`, ByLayer), step_id BL(1), material in the handle
  stream, op major/minor 33/427 — identical anchors on every SH
  node specimen, and the payloads are version-portable (the
  Extrude record is bit-identical across 2007/2010/2013/2018
  except handle tails). The design consequence: **Phase A retains
  the whole post-`op.minor` payload raw** — `shsw_raw_tail`
  (MSB-packed) plus `shsw_raw_tail_bit_len`, captured by the reader
  to the record's main-section end (new
  `DwgMergedReader::main_end_bits()`, text-flag aware, taken from
  the current position after `op.minor`) and re-emitted verbatim by
  the writer; the modeled fields (`shsw_method`/`shsw_text`/
  `shsw_bl93`/`shsw_text2`, `direction`, the option BDs) remain on
  the model as the DXF / modeled-fallback channel only (the writer
  arms on `shsw_raw_tail_bit_len > 0`). Verified: hermetic suite
  48 ok / 0 failed (1561 passed); Polysolid_2018 smoke 0/0 WITH
  the record present (rewrite Size 263; gold decodes it cleanly as
  UNKNOWN_OBJ); corpus 152 files — gold tree 0/0 held, fixture
  26/8 unchanged packet-for-packet; layer-4: rewrite data
  sections BIT-IDENTICAL through the full 2018-bit window on both
  Polysolid_2018 and Polysolid_2010. The only record delta is the
  owner-handle form (silver's corpus-wide `(4,2,abs)` vs the
  authored `(6,0,+1)`: Size 261→263, Hdlsize 0x1D→0x2D, same
  identity — pre-existing for every rewritten object, tolerated by
  gold and all 152 files).
- **Phase A step 3 — ACSH_EXTRUSION_CLASS (2026-09-23, `8c41aa6`)**:
  mechanical — `read_history_sweep`/`write_solid_history_sweep`
  already served the EXTRUSION dxf-name through the step-2 raw-tail
  retention, so the step was the two elide-guard/nuller one-liners.
  Verified on Extrude_2018: rewrite record present (Size 70→72,
  Hdlsize 0x1F→0x2F — only the corpus-wide owner-handle form
  delta), gold decodes it cleanly, the smoke shows no new rows (the
  file's two surviving rows = its pre-existing `3DSOLID.wires`
  stub pair), layer-4: data section bit-identical through the full
  488-bit record plus the text flag (divergence only in the handle
  region, same known form delta); corpus 152 files — gold tree 0/0
  held, fixtures 26/8 unchanged packet-for-packet. The sweep-shaped
  family (Polysolid→SWEEP, Extrude→EXTRUSION) now round-trips its
  records byte-faithfully.
- **Phase A step 4 — ACSH_LOFT_CLASS + ACSH_REVOLVE_CLASS
  (2026-09-23, `20d472b`)**: the autopsy confirmed both guessed walks
  were wrong. REVOLVE provably by budget alone: its modeled fixed
  192-bit raw-direction triple exceeds the entire remaining tail of
  every Revolve fixture record (470-bit data section); LOFT by the
  same undocumented-content texture the sweep showed — the skeleton
  anchors verified, the op pair reads small RC-form values where
  the sweep family read (33, 427), and the ~470-bit tail is genuine
  content (`0xCC`-runs plus repeating 16-bit-group patterns), not
  the modeled count-prefixed embedded-entity list. The
  sweep design was generalized rather than duplicated: shared
  helpers `capture_undocumented_tail` (reader;
  physical-window-clamped like gold) and `write_undocumented_tail`
  (writer; byte-vector-authoritative with modeled fall-through);
  the sweep arm refactored onto them; `raw_tail`/`raw_tail_bit_len`
  added to `SolidHistoryLoft`/`SolidHistoryRevolve` (their modeled
  fields remain the DXF / modeled-fallback channel); the step
  orphaned `read_embedded_entity`, now removed (the DXF path uses
  `decode_embedded_entity`; Phase B can recover it from history).
  Un-elided both classes via the single shared list
  `elided_solid_history_class`. Gates green per class: hermetic
  48 ok / 0 failed (1561); Loft_2018 (Size 102→104, Hds
  0x1B→0x2B) and Revolve_2018 (Size 67→69, Hds 0x19→0x29) smokes
  with records present and NO new rows; layer-4: both data
  sections bit-identical (748 / 470 bits), only the known
  owner-handle form delta; corpus 152 files — gold tree 0/0 held,
  fixtures 26/8 unchanged. The `3DSOLID.point` packet
  (Revolve_2007/2010) is confirmed READ-SIDE ONLY: silver reads
  the original's modeler point at (0.6, 0, 0.6) where gold reads
  (0, 0, 0), while the rewrite's point matches gold on both
  sides — a constructed-genus read default, Phase B territory.
- **Phase A step 5 — the fixture-backed primitives (2026-09-23,
  `19308ee`)**:
  the sphere mystery resolved as a HARNESS gap, not a codec one —
  `ACSH_SPHERE_CLASS` was the one class missing from
  `_DYNBLOCK_RETYPE` (its own comment predicted "stays UNKNOWN
  until their own packets"), so silver's sphere records paired as
  UNKNOWN_OBJ and produced the whole count/missing packet. Landed
  the retype entry plus the projection arm (shared
  `_acsh_node_fields` + `radius`), and un-elided the class through
  its typed arm (the §18.5 skeleton trace had already verified the
  layout); Sphere_2018 dropped 5→1 read rows (only its wires stub
  row remains) with the rewrite's sphere record gold-typed and
  L4-identical through the full 282-bit record (the R2010 record
  verified identically — 282 bits at the 39-bit frame). Then the same
  treatment for the other fixture-backed primitives: **Box** (the
  live `-v9` trace pins length/width/height BDs after the op pair —
  silver's arm matches exactly; also the FIRST non-identity base
  matrix written: L4-identical on Box_2018 at 542 bits AND Box_2007
  at the R2007 layout) and **Boolean** (live trace: RC operation +
  BL operand1/operand2 — silver's arm matches; L4-identical at 300
  bits via the Union fixture). Wedge/Cylinder/Cone/Torus/Pyramid
  and BREP have NO fixtures — they stay elided: unverified elide
  removals are strict-reader risk. Verified: hermetic 48 ok / 0
  failed (1561); smokes Box 0/0, Union 0/0, Sphere rows collapse
  to the wires stub only (Sphere_2007/2010 fully 0/0); corpus 152
  files — gold tree 0/0 held, **fixtures 26/8 → 10/8**: the
  remaining rows are exactly the two documented
  constructed-genus/Phase-B packets (`3DSOLID.wires` stub 8+8,
  R2013+R2018; `3DSOLID.point` 2, Revolve_2007/2010 read-side).
- **Phase A step 6 — the last two 3DSOLID packets (2026-09-23,
  `f368fce`, PHASE A COMPLETE)**: (1) the wires stub was an
  emission asymmetry, not a fabrication — silver parses the
  R2013+/R2018 ds-backed wireframe wire structures (real
  silhouette/edge geometry: RC type, BLd marker, BL color, BLd
  acis_index, BL point count, 3BD points) that gold's derailed
  walk never reaches, so gold emits nothing; the fix adds
  `wires`/`silhouettes` to the established ds-backed
  prologue-divergence drop (the isolines stay — gold's R2018
  record reads them before derailing). (2) the point wrong-value
  was a deliberate synthesis: the wire-frame anchor IS verbatim
  wire data (Revolve_2007/2010 bake in (0, 0, 0); gold
  re-emits it as parsed), but the document builder swapped
  zero anchors for the SAT geometry centre; the anchor is now
  wire-verbatim whenever `point_present` was set (all three
  sites: solid, region, body), and the synthesis survives only
  for genuinely anchorless records. All ten packet files
  smoke 0/0; **corpus 152 files: read 0, write 0 — the gold
  tree AND the entire 28-fixture sh_history tree are at zero.**
  Phase A is complete per its own definition and beyond: every
  fixture-backed SH class round-trips byte-faithfully with its
  record present, and the strict-load elide is retired for
  HISTORY, SWEEP, EXTRUSION, LOFT, REVOLVE, SPHERE, BOX, and
  BOOLEAN.
- **Phase C fixture review (2026-09-23, COMPLETE):** the maintainer
  authored all seven requested families; qualification landed them
  all at 0/0 (28 files, `.txt` companions generated from the gold
  `-v9` census): `Wedge_`, `Cylinder_`, `Cone_`, `Torus_`,
  `Pyramid_`, `Fillet_` (with its parent `ACSH_BOX_CLASS` chain),
  `Chamfer_` (same). The review found and fixed the same harness
  gap sphere had: `ACSH_PYRAMID_CLASS` was missing from
  `_DYNBLOCK_RETYPE` (retyped + height/sides/radius/topradius
  projection; gold's JSON key is `topradius`); all four Pyramid
  versions went 0/0. **BREP is DEFERRED** (maintainer's decision):
  seven authored attempts — plain ops, SLICE (empty fresh roots),
  a single-op variant, the foreign-body paste-graft (fully
  parametric graft — nested BOOLEAN, disproving the ungraftable
  theory), SOLIDEDIT face edit (history deleted outright), and a
  real-template source (violating §F2.1 cleanliness) — never
  produced an `ACSH_BREP_CLASS` record. The class bottoms out only
  through paths with no user-facing command in current AutoCAD
  (the one known carrier is `ATMOS-DC22S.dwg` in the gold tree,
  tolerated at 0/0 today). The placeholders were removed; the row
  re-opens if an authentic specimen surfaces. Each landed class's
  un-elide remains the step-5 recipe (trace-calibrate + un-elide
  via the shared list + four gates); the classes currently sit
  correctly elided.

  **The complete phase map (from the original 2026-09-22 campaign
  brief; each phase gets its own NEXT_SESSION when its predecessor
  completes):**

  - **Phase A** (in progress): implement the oldest-critical SH
    class wire layouts — HISTORY, SWEEP, EXTRUSION — retaining the
    `shsw_text`/`shsw_text2` opaque blobs as raw bytes without
    interpreting their content, un-elide each per calibration,
    and drive the fixture diffs toward zero. Phase A is finished
    when the Polysolid and Extrude families round-trip
    byte-identically and the `ACSH_SWEEP/EXTRUSION` records
    survive in the rewrite.
  - **Phase B** — the blob autopsy: determine the internal
    structure of the `shsw_text` and `shsw_text2` binary blobs
    (are they serialized SAT fragments? compressed embedded
    entity streams? raw modeler parameter arrays? — the raw bytes
    are already byte-preserved after Phase A, so Phase B decodes
    them by comparison across the 28 fixture specimens) and
    implement the node-class typed fields properly: expose the
    blob's semantic fields on the model and render them in the
    JSON/normalizer projection. Phase B ends when the blob content
    byte-round-trips across all 4 versions per fixture family
    with semantic field parity in the normalize projections.
  - **Phase C** — the full un-elide: implement the remaining
    node-class layouts (Box, Sphere, Cylinder, Cone, Pyramid,
    Torus — the "primitive" SH family; Boolean — the `ACSH_BOOLEAN`
    composition records; Fillet, Chamfer — the edge-modification
    nodes; BREP — the catch-all for free-form ACIS surfaces; Loft
    and Revolve — the path-sweep family), remove the per-class
    elide, restore the solid's history soft-pointers to full
    fidelity, and drive the entire 28-fixture corpus to 0/0.
    The fixture campaign lands here (F2.3's full table rows are
    the Phase C coverage manifest — the "(Phase B+)" rows in
    §F2.3 are the Phase B/C breakpoints).

  Each phase follows the same discipline: the calibration
  specimens are already landed (§F2.1), the gold spec blocks are
  identified (§18.5's gold-spec references), and the zero-keeping
  gate applies at every checkpoint (README "The zero-keeping
  workflow"). Phase A's completion auto-qualifies the Phase B
  NEXT_SESSION handover (the same pattern as the strict-load
  campaign's per-round halt-and-brief cycle).

### 18.6 ACS/SH solid-history Phase B — the blob autopsy (complete 2026-09-23)

Phase B decoded the raw-retained node tails into typed model fields
per family, keeping the captured bits as the write authority. Every
class is still `DEBUGGING_CLASS` in gold (no oracle walk past
`history_node.color.flag`), so the decode rests on the two non-oracle
instruments named in the brief: cross-specimen comparison (the 16
tails are bit-identical across DWG 2007/2010/2013/2018 — verified
again this step) and the R2010 live-oracle fragment (gold's 3DSOLID
wireframe values on the same modeler backing).

**What is actually inside the tails** (the autopsy record):

- **Sweep family** (`shsw_raw_tail`, serving SWEEP and EXTRUSION) —
  the §18.7 differential (2026-09-24) named the spine and the
  extrusion profile:
  - The six post-direction BDs are the SWEEPOPTIONS order
    `[draft_angle][draft_start_distance][draft_end_distance]
    [twist_angle][scale_factor][align_angle]` — the libredwg
    SweepOptions macro's own sequence, confirmed by ExtrudeT (the
    typed 15° draft lands as a raw BD in slot 0) and the 1.0
    default in the scale slot. The model carries all six as named
    fields; the Polysolid sweep adds two all-short extras
    (unnamed).
  - EXTRUSION tails carry the embedded PROFILE CALL — the same
    `[BL kind][BL bit-length][body]` grammar as the revolve,
    closing at `bit_len - 2`: kind 18 = OBJ_CIRCLE with the body
    `[center 3BD][radius BD][normal 3BD]` (the landed fixture:
    circle (0,0,0) r 1.0 — the exactly-1.0 radius in the two-bit
    short BD, CALL length 16; ExtrudeR: r 3.125 raw, length 80;
    ExtrudeH/T: the landed circle unchanged — the height lives
    only in the direction head, the draft only in the spine), kind
    77 = OBJ_LWPOLYLINE (ExtrudeP, length 544 — **LANDED 2026-09-26,
    the §18 walk 3**: the body decodes through the embedded-LWPOLYLINE
    grammar — the in-repo `read_embedded_lwpolyline` field order,
    raw-points vertex arms — `[BS flag 512 closed][BL num_points 4]
    [raw LE64 (x, y) pairs — the 4×3 rectangle (0,0) (4,0) (4,3)
    (0,3)][bulges/vertex-ids/widths per the flag gates][the trailing
    reserved '11' pair that closes the window exactly]`; the model
    gains `SolidHistoryProfilePolyline` on the CALL, and a vertex
    edit splices bit-locally in its own 128-bit frames). The pre-CALL region is
    RESOLVED (2026-09-24, the re-authored ExtrudeC): it MOVES with
    the profile position, carrying the profile center as its own
    raw-BD run (the ExtrudeC center (2.0, 3.0) appears TWICE — once
    in the pre-CALL, once in the CALL circle body — and the landed
    208-bit tail grows to 464 by exactly the two x/y raw pairs).
  - The SWEEP (Polysolid) tail keeps its own structure — the frame
    blocks (the path unit direction entries, confirmed on three
    directions by PolysolidX/D), the packed profile corners
    (±1.5/±2.5 × {0, H}) and the segment end; no CALL.

- **Loft** (`raw_tail`): an all-short `[1.0]` head, then raw BD
  entries the marker scan reads positionally — the landed fixture
  lists `[2.0, 2.0, 5.0, 0.3, pi/2, pi/2]` (the 5.0 top height
  matches the wires, the pi/2 pair the 90-degree drafts).
  **The §18 container walk (2026-09-26) names the per-section
  fields:** the raw frames group into contiguous runs — a 2-bit
  gap between value frames is plain adjacency (the next frame's
  '00' marker), a 4-bit gap is an elided canonical short field
  BETWEEN raws, anything wider is a section boundary — and the
  final 2-frame run names the trailing draft-angle pair
  (`[pi/2, pi/2]` everywhere; the LoftD settings row stays dead,
  the naming provisional per its §18.7 verdict). Every earlier run
  attributes to the closed per-section reading
  `[center.x][center.y][height][radius]`: 4 contiguous frames =
  all four (Loft_'s (2, 2, 5, 0.3)); 3 frames with the wide gap
  after the second = `[cx][cy][elided z short][r]` (LoftC's
  section 1: (3, 4, z 0 short, 1.5)); 2 frames = `[z][r]` (LoftR);
  1 frame = `[z]` (LoftH (7); Loft3's mid sections at 2.5/5.0). Any
  other shape leaves the whole tail positional (nothing named, the
  conservative decode-fidelity rule). The leading region (8 bits on
  LoftC, 48 on LoftH/R/3, 68 on Loft_ — holds a fully elided origin
  section's four shorts plus per-record state), the inter-section
  gaps (up to 52 bits), and the constant 14-bit trailer
  `10100111010110` stay documented-verbatim: the named fields never
  claim their bits. Model: `SolidHistoryLoftSection` per section
  (per-field `Option` for the elisions) + `draft_angles` on
  `SolidHistoryLoftTail`; named edits splice same-form LE64s in
  their own frames; positional `raw_doubles` edits keep working
  unchanged. The §18.7 differential (LoftH/R/3, 2026-09-24) named
  the leading slots: `[top-section z][top-section radius]` —
  LoftH (top z 7) reads `[7.0, pi/2, pi/2]` (its r 1.0 top takes
  the two-bit short
  and does not appear in the raws), LoftR (top r 2.5) reads
  `[5.0, 2.5, pi/2, pi/2]`, Loft3 adds the mid-section z
  (`[2.5, 5.0, pi/2, pi/2]`). And the landed original's
  `[2.0, 2.0, 5.0, 0.3]` prefix is RESOLVED (2026-09-24
  wire-verified, z-separated silhouette fits: bottom circle (0,0)
  r 1.0, top circle (2.006, 2.006) r 0.2996): the raws are
  **[top center.x][top center.y][top z][top radius]** — the
  "drag residue" was the dragged top section's geometry; the
  full raw-run reading is CLOSED (2026-09-24, the re-authored
  LoftC — the offsets are PRESENT in its raws, the world reading):
  PER-SECTION `[center.x][center.y][height][radius]` runs with
  0.0/1.0 values elided as shorts (the landed original retro-fits:
  its `[2.0, 2.0, 5.0, 0.3]` = the TOP section (cx 2, cy 2, z 5,
  r 0.3) with the bottom at (0,0) r 1 fully elided; the LoftC raws
  `[3.0, 4.0, 1.5, 3.0, 4.0, 7.0, 1.5, pi/2, pi/2]` = both sections
  in full + the `[pi/2, pi/2]` draft pair). The strict BD walk
  from the head still derails on the non-BD inter-value regions —
  the container walk is the remaining decoder row; the model keeps
  the slots positional (`raw_doubles`).
- **Revolve** (`raw_tail`): the FULL-TREE LIBREDWG SCAN closed this
  family completely on 2026-09-23 (the record below).

  The confirmed grammar (every landed tail accounted bit-exactly,
  all three specimens x their four versions):
  `[axis_pt 3BD (0,0,0)][axis_vec 3BD (0,1,0)]` — the head pairs,
  the REVOLVEDSURFACE spec twin's own field order — `[revolve_angle
  BD raw]` — confirmed on two values (3*pi/2 and pi) — `[6 x BD0
  options]` — `[BL 18 = OBJ_CIRCLE][BL 80/144]` — an embedded
  PROFILE CIRCLE: `[center 3BD][radius BD][normal 3BD]` — then
  `[2 flag bits]`. The
  SCAN's three decisive finds:

  - **The RC byte 0x12 in every mid-region = the embedded-entity
    type code 18 = OBJ_CIRCLE** (the constant in the readers'
    common tables): the "form mid-region" was never a form header -
    the whole post-angle structure is one embedded CIRCLE sub-entity
    through the CALL grammar `[BL type][BL bit-length][body]`.
  - **The tail accounting closes exactly on all specimens**: the
    CALL's bit-length (80 for the original, 144 for A/R) covers
    `[center 3BD][radius BD][normal 3BD] + 2 flag bits` with no
    remainder - there is no mystery tail.
  - **The "different profile form" never existed**: the original's
    circle is `(1.0, 0, 0)` radius `0.2` normal `(0,0,1)` - its
    center.x IS exactly 1.0, so the wire stores it in the TWO-BIT
    SHORT form (BD '01') and the earlier raw-scan misread the
    (radius 0.2) raw as "the center.x" and the (normal.z 1.0)
    short as "a missing radius + no trailing trio". The A/R quads:
    circle `(2.0, 0, 0)` radius `0.8/1.25` - center.x a raw. Same
    grammar, different value forms, nothing else.

  The wire-ladder facts all stand (the original's torus M 1.0 /
  m 0.2 about origin-Y = the circle (1,0,0) r 0.2 revolved 270
  degrees - as-drawn; AutoCAD refuses crossing AND
  perpendicular-plane profiles). The model restructure landed:
  `SolidHistoryRevolveTail { axis_point, axis_vector,
  revolve_angle, option_doubles, profile_center, profile_radius,
  profile_normal }` - splice-backed per field, all 10 Phase B
  tests green, corpus 188 at 0/0.

**The scan's other finds (the §18.7 pipeline gains):**

- **The surface twins' spec blocks exist but are NOT LIVE in the built
  oracle** (2026-09-24 correction of this bullet's first wording):
  `EXTRUDEDSURFACE`/`REVOLVEDSURFACE`/`SWEPTSURFACE`/`LOFTEDSURFACE`
  sit in `dwg2.spec` (lines 3952/4029/4061/3984) reading the full
  field sets — `REVOLVEDSURFACE` axis_point/axis_vector/revolve_angle/
  start_angle/[16-BD transmatrix]/draft_angle/draft_start_distance/
  draft_end_distance/twist_angle/solid/close_to_axis, the
  SWEEPOPTIONS macro (sweep_alignment_flags 0-3 enum, path_flags,
  base_point_set, the two *_transform_computed,
  reference_vector_for_controlling_twist), the lofting parameters
  (transmatrix, plane_normal_lofting_type, start/end draft
  angle+magnitude, arc_length_parameterization, no_twist,
  align_direction) — but the whole 3716+ region sits inside
  `#if defined (DEBUG_CLASSES) || defined (IS_FREE)`: the built
  dwgread compiles them OUT. `PLANESURFACE` (3595) is live outside
  the frame, and the ASSOC surface action bodies
  (`ASSOCEXTRUDED/LOFTED/REVOLVED SURFACEACTIONBODY`, 1967-2026)
  are live — `ASSOCSWEPTSURFACEACTIONBODY` (3720) is dead with the
  entities. The surface-parser row (2026-09-24, see the next
  bullets) closed parity through the gold-shadow object walk, so
  the typed field LISTS remain the future naming map for the
  raw-retained tails, not a live typed gold read.
- **The ASSOC surface action bodies are typed even in R2004 and
  carry the parameters BY NAME**: the corpus's Surface.dwg
  (2.7 strict-tree file) parses `ExtrusionHeight = 200.0`,
  `ExtrusionTaperAngle = 0.0`, `Continuity = 1,1`, `Bulge =
  0.5,0.5`, `RevolveAngle` - gold prints these as named
  `pab.values` entries. A maintainer authoring SURFACE-mode
  variants (`MOde` option) of any §18.7 stem gets typed semantic
  anchors for the sweep/extrude/loft tail regions for free.
- **The class registry confirms** the SH genus block (505-520 +
  875 `ACSH_SUBENT_MATERIAL_CLASS`); the machine-wide `.dwg`
  scan found NO other SH-class carriers beyond the fixture tree
  and the known calibration specimen - the corpus set IS the
  universe on this box.
- **The struct inventory in include/dwg.h** (the SWEEPOPTIONS
  alias set, `is_solid`, the alignment enum semantics) is the
  semantic table for the sweep-family heads when the ExtrudeT/S
  stems land.

**Surface-parser row decode record (2026-09-24, the session that
un-quarantined the 20 surface-twin files — every grammar finding
with its evidence chain):**

- **Gold's classes walk derails on the AutoCAD-2027.1 class
  tables (the root finding)**: gold's R2004+ numeric class walk
  (`read_2004_section_classes` decode.c 2249 — the R2004-format
  containers: AC1018 and AC1024+, called from the decode_R2004_header
  flow at decode.c:3707; `read_2007_section_classes`
  decode_r2007.c 1491 for AC1021)
  reads per class `BS number, BS proxy, [text], B zombie,
  BS item_class_id, BL instances, BS dwg_version,
  BS maint_version, BL, BL` — the two version fields as **BS**
  where this reader's (and the real encoding's) walk uses **BL**
  (silver's `classes_reader.rs`). From the first class record
  whose tail needs a non-`'00'`-byte form the two walks desync and
  gold's numeric cursor runs off the true record grid: OBSERVED on
  the whole 2027.1-authored fixture set, all four versions
  (AC1021/24/27/32), derailing from the 10th entry (CELLSTYLEMAP)
  onward — the first nine classes are byte-form tails that parse
  identically under BS/BL. Evidence chain: gold `-v4` classes
  trace (`size: 3107 / hsize: 0 / bitsize: 24849 / max_num: 526`
  for ExtrudeM_2018), the CLASSES JSON dump (garbage numbers
  `36108 / 256 / 0...` and garbage `item_class_id`s `36614 /
  1042... / 1044` from the 10th class on), and the decisive probe:
  a bit-exact python emulation of gold's walk over the dumped
  section bytes (`dump_section_bytes 0 3107 "AcDb:Classes"`)
  reproduces all 27 garbage records to the value — the walk, not
  the bytes, is the oracle behavior. The other AutoCAD-authored
  corpus files (older releases) keep byte-form tails, so gold
  walks them sane — the derail is file-content-derived, hence the
  shadow-walk fix rather than an index threshold.
- **The desync's record-level consequence — surface entities
  decode as gold's UNKNOWN_OBJ**: garbage `item_class_id` never
  equals `0x1F2`, so `dwg_class_is_entity` is false and gold
  routes the (physically entity) EXTRUDEDSURFACE/LOFTEDSURFACE/
  REVOLVEDSURFACE records through `dwg_decode_UNKNOWN_OBJ`:
  object-common data (eed, ownerhandle @the-dependency,
  xdicobjhandle, reactors absent) + the full remaining bits as
  `unknown_bits` (ExtrudeM_2018 @731: size 868, bitsize 6901,
  6830 unknown bits; the 8-byte surface-mode action-body residue
  `40A202A9008405C9` on the same class of file). The dispatch is
  still by dxfname string (the string stream stays in sync), so
  the live ASSOC action bodies parse typed on the very same
  files.
- **Silver's landing (the gold-shadow walk)**:
  `classes_reader::gold_shadow_item_ids` re-walks the same
  decompressed section bytes with gold's exact field sequence
  (header `size/hsize/bitsize RLs + BS max + RC + RC + B`; per
  class the numeric sequence above; pre-R2007 TV strings read
  inline, R2007+ strings never touch the numeric cursor),
  `DxfClass::gold_item_class_id` carries the result per index,
  and the pass-2 entity set keys the **RAW** class number —
  the internal class map resolves `EXTRUDEDSURFACE` to the fixed
  sentinel `OBJ_EXTRUDEDSURFACE (-7)`, which would otherwise
  bypass the set through `is_entity_type`. The flipped records
  take silver's object arm: `read_common_non_entity_data` +
  the raw-tail capture, emitted as `ObjectType::Unknown`, and
  the existing unknown-object write arm re-emits them — the
  rewrite is bit-identical (gold_rt @731: same size/bitsize,
  the full 1708-hex-digit unknown_bits equal, eed/ownerhandle/
  xdicobjhandle equal; the layer-4 evidence for the writer
  path). **Walk semantics tree-verified (2026-09-25, the full
  libredwg source review; every line below pinned in the tree):
  both R2004+ readers are COUNT-bounded (`num_classes =
  max_num - 499` at decode.c:2304 / decode_r2007.c:1540, then
  `for (i = 0; i < num_classes; i++)` at decode.c:2332 /
  decode_r2007.c:1564) with the bails exactly as the shadow
  implements them (R2007: `max_num < 500 || max_num > 5000`,
  decode_r2007.c:1541; R2004-format: `max_num < 500 ||
  num_classes > 100 + size/sizeof(Dwg_Class)`, decode.c:2305 —
  sizeof(Dwg_Class) = 64, verified by compiling against the
  oracle's headers); the pre-R2004 reader is BYTE-bounded
  (`while (byte < endpos - 1)`, decode.c:621) with the live CWE
  per-record cap `i >= 100 + size/sizeof(Dwg_Class) ||
  i >= 65535` (decode.c:630) and NO header — the "2004 max_num"
  fragment in that reader is `#if 0` dead code (decode.c:606-619).
  Two documented shadow simplifications on the pre-2004 arm, both
  dispatch-neutral (a bail or misalignment yields non-0x1F2
  values on both sides → object dispatch either way): gold's
  strict size validation (`size != section.size - 2×sentinel - 6`
  → zero classes, decode.c:581-604) and the second in-record
  endpos check after appname (decode.c:662-664).**
- **The BLOCK_HEADER.entities corollary**: gold prints the
  block's entity list from the wire handle vector (dwg2.spec
  HANDLE_VECTOR entities, code-3 children — `[3,2,727] CIRCLE +
  [3,2,731] EXTRUDEDSURFACE-as-UNKNOWN_OBJ` on ExtrudeM_2018)
  verbatim, NOT from recomputed ownership, so the object-walked
  surface records stay in the list. Silver's
  `rebuild_block_membership` now keeps the canonical wire list
  primary (its order is gold's) and unions the membership
  reconstruction in — same set on conventionally-shaped files
  (the 260-file corpus held 0/0 across the change).
- **The ASSOC surface action-body grammar is version-split at
  PRE(R_2013b)**: gold's `AcDbAssocParamBasedActionBody_fields`
  block (version/minor/l4/num_deps+deps/l5+assocdep/REPEAT
  values) is read ONLY on pre-R2013 files — ExtrudeM_2007/2010
  print the full pab shape with the named
  `ExtrusionHeight/ExtrusionTaperAngle` pab.values ([0,0] —
  libredwg's REPEAT collapse prints a bare 0 per value) and
  assocdep resolving one handle before the first dependency
  (2004-era rule, held); the R2013+ files (ExtrudeM_2013/2018)
  read NO pab fields at all — aab_version, sab
  version/is_semi_assoc/l2/is_semi_ovr/grip_status, the
  path-based pbsab_status, class_version — and `assocdep` IS
  the sab slot (surface_body.dependency → the ASSOCDEPENDENCY).
  Spec: dwg2.spec 1961-1973 (the PRE(R_2013b) gate) + 1988-2026.
- **ASSOCACTION SINCE R_2013 carries a trailing values REPEAT**
  (class_version > 1): `VALUE_BS(0,90) + BL(num_values) +
  REPEAT(values)` after owned_params — gold prints the same
  degenerate `[0]*count` as deps (ExtrudeM_2018: [0, 0] for the
  two named valueparams). Silver's reader had the values since
  the retype batch; the normalizer projection caught up.
- **AcDbAssocActionParam_fields SINCE R_2013 begins
  `is_r2013` (BS) + `aap_version` (BL)**: ASSOCPATHACTIONPARAM
  prints aap_version between is_r2013 and name (0 on the M
  quads; the osnap corpus records print the same collapse —
  the 0 was hardcoded there and the path param now projects
  the reader's actual value).

**Wire-format corollaries confirmed along the way:** the
tails' raw BD entries are plain LE doubles per DWG byte order
(bytes little-endian, each byte MSB-first on the wire — the same
convention as `read_raw_double`; an early decoder draft reversed
the within-byte order and produced garbage values, caught by
re-dumping the fixture tails and pinning the decodes against the
extraction probes' known values); zero values are never stored raw
(`'00'`-marked zeros are misaligned aliases — the wire uses the
short `'10'` form); and the byte-aligned geometry blocks cannot be
BD-form (their doubles sit 64 bits apart, not 66).

**Review pass (same halt):** two defects found and fixed before
the campaign was declared complete. (1) A captured tail whose
layout does NOT decode fell through the Phase B writer arm into
the modeled fallback, which would have replaced the captured bits
with the guess sequence and corrupted the record — the fixed rule:
render when decodable, verbatim when not, modeled arm strictly
reserved for records with no captured tail (pinned by
`undecodable_tails_stay_verbatim_never_modeled`). (2) The revolve
decoder's `raw_spans` kept the angle's span at index 0 while the
angle value was removed from `raw_doubles` — editing
`raw_doubles[0]` would have spliced into the angle's bits; spans
now shift in lockstep with the values (pinned by
`revolve_raw_entry_edits_stay_out_of_the_angle_span`).

**Implementation** (all gates green at halt):

- `src/io/dwg/sh_tail_decode.rs`: the pure decoder module —
  per-family tail views + the writer's span bookkeeping, plus the
  Phase B re-encode (`render_*_tail`).
- Model: new typed views on the three classes —
  `SolidHistorySweep::tail_decode` (`SolidHistorySweepTail`:
  `option_doubles`, `raw_doubles`, `profile_corners`,
  `segment_end`), `SolidHistoryLoft::tail_decode`
  (`option_doubles`, `raw_doubles`), `SolidHistoryRevolve::tail_decode`
  (`option_doubles`, `revolve_angle`, `raw_doubles`). The modeled
  (DXF-fallback) fields stay where they were; the DXF reader
  constructors set the views to `None`.
- Reader: populates the views right after `capture_undocumented_tail`
  (the capture boundary itself is untouched — silver's tails still
  appear verbatim inside gold's `unknown_bits`).
- Writer — **the Phase B write rule**: re-encode ONLY when a
  decoded field was programmatically modified, else verbatim.
  `render_*_tail` re-decodes the stored tail, compares the model's
  current values against that decode, and splices the differing
  spans into the stored bits: untouched records re-emit
  bit-identically by construction (an unmodified render round-trips
  byte-exact — pinned in tests), while an edit lands bit-locally
  and the tail — and therefore the record and section size — never
  changes. Raw LE64 spans take any f64; short spans splice only
  same-form `0.0`/`1.0` flips (a short→raw form change would
  shift the stream and stays unsupported in Phase B, documented in
  the module).

**Phase B gates (halt state):**

1. Hermetic: `tests/solid_history_tail_decode.rs` (8 tests — the
   four family decode pins from the real fixture tails embedded as
   hex, malformed-tail guards, and the full-stack round-trips:
   raw tails survive byte-identically, decoded views re-derive,
   segment-end / revolve-angle / loft-height edits land exactly
   inside their raw spans — bytes 201..216, 1..10 and 25..34 of
   the respective tails) + 4 module tests (render-unmodified ==
   verbatim, splice forms) + the existing 87 roundtrip suite: all
   green, no feature-gated skips added.
2. Family smokes: all 16 (4 families × 4 versions) at 0/0
   read+write fidelity.
3. Corpus: 180 files, read 0, write 0.
4. Layer-4 byte-walk for the re-encode path: an
   unmodified-vs-edited document pair differs in the objects
   section at EXACTLY two 16-byte rows — the segment-end splice
   (LE bytes of 1000.0/2000.0 visible in place) plus the section
   checksum; the record size is unchanged.

**What Phase B did NOT solve (the honest remainder):** the full
bit-grammar of the mid-regions — the 288-bit sweep frame blocks
carry the unit direction in a repeating pattern whose un-named
remainder (the -0.9446..., -1689439.46-class entries) stays
opaque, the Extrusion's 64-bit payload after its option spine is
undecoded, and the Loft's leading 68-bit region before its raw run
is likewise opaque. **The three walk rows landed 2026-09-26 (the
§18 decoder-walk queue): the sweep POST-CORNER walk names the
corner block's trailing unpaired entry as the record constant
(4.00024414192312 — bit-invariant across every quiz differential)
and the gap's first plausible '00'-framed BD as the width-coupled
single `32 + n/32` (X/L/D n=2, W n=7, Polysolid n=5 — the
W/PS numerators track the authored width exactly, the X-family at
a width−1 offset whose closed form stays UNRESOLVED; the §18.7
P-era "singles" — "2.0109", the "width single" 3.0/7.0, the
8.06-class reads — are overlapping ALIASES of the same zero-heavy
bits, documented, never pinned); the loft container walk and the
ExtrudeP polyline walk close their records above. Still opaque
after the walks: the sweep frame-block repeats, the extrusion
64-bit payload, the loft leading/inter-section regions and
14-bit trailer, and the 32+n/32 single's closed form. Those bits
round-trip verbatim and stay untouched by the writer; naming them
needs the differential
instrument the brief predicts (a second specimen per family with
different geometry — the fixture tree has one distinct specimen
per family, and the strict-load gold tree carries no other
sweep-family members). **The authoring recipe for that set is §18.7**
(sixteen single-variable typed stems with qualification checks and
the processing pipeline; the 2026-09-23 sufficiency review
expanded the revolve set from a pair to the seven-stem matrix —
see §18.7); once those pairs land, the next session runs the
§18.7 pipeline and the remainder closes.

### 18.7 The differential specimen recipe — authoring the Phase B follow-up set

Phase B's one instrument gap is mass: one distinct specimen per
family means every unnamed entry is un-correlated. The specimens
below close that with **single-variable typed pairs** — each new
stem is authored with EXACT TYPED geometry (no drags; the landed
originals were drag-authored, which is why their coordinates read
like 1532.5039681547414), and each varies exactly one geometric
variable against its partner so the next session's bit-diff
attributes every changed unknown entry to that variable. Author in
AutoCAD (the originals are AutoCAD 2027.1; BricsCAD is acceptable
— record which, per F2.2 step 6), one operation per file, blank
default template, solid-history recording ON, then SAVEAS to all
four versions (2007/2010/2013/2018 — verify each kept the ACSH
records per the F2.1/F2.2 qualification; the SH genus persists
R2007+).

**The stems** (globally unique, so `run_corpus.py` keys per-file
workdirs cleanly):

| priority | stem | typed geometry (one op, blank template) | varies vs its partner | designed to answer |
|---|---|---|---|---|
| ~~P1~~ **LANDED 2026-09-24** | `PolysolidX_<v>` | `POLYSOLID` → `Height` 5 → `width` 3; path `0,0,0` → `12,0,0`, ENTER to end | (pair anchor) axis-aligned path, typed profile | the two 288-bit frame blocks with unit direction (1,0): do the −0.9446/−1689439-class unnamed entries become clean {0, ±1} values (direction-derived) or keep their shape (derived from something else)?; corners should decode (±1.5, {0,5}) — a second profile confirming `profile_corners`; which singles track W vs H (the 4.0002-class entry) **OBSERVED (LANDED 2026-09-24): corners (plus/minus 1.5, {0,5}) exactly as predicted; the frame raws read [0(short), -1, -1, 0(short)] per block — the direction-derived theory CONFIRMED on a second direction (zeros never raw); segment end (12.00001, 0) — typed-12 with drag residue; the singles [2.0109, 3.0 (the width), -19594.35] remain unnamed** |
| ~~P1~~ **LANDED 2026-09-24** | `PolysolidD_<v>` | same profile (`Height` 5, `width` 3); path `0,0,0` → `10,10,0` | 45° path (unit (0.7071…, 0.7071…)) vs X | the direction slots of the frame blocks (second sample separates direction entries from constants); the segment-end slot = (10,10) **OBSERVED (LANDED 2026-09-24): the frame raws read [0.7071, -0.7071, -0.7071, -0.7071] twice = [+u_y, -u_x, -u_x, -u_y] for the 45-deg unit — the direction slots CONFIRMED; corners (plus/minus 1.5, {0, 5.000000000000002}) (height drag residue); segment end (10, 10)** |
| ~~P1~~ **LANDED 2026-09-24** | `ExtrudeH_<v>` | `CIRCLE` center `0,0,0` radius `1`; `EXTRUDE` the circle, height `5` | height 2→5 (radius unchanged) | does the 64-bit payload after the option spine change with height? direction z decodes 5.0; the 36-bit trailer **OBSERVED (LANDED 2026-09-24): the tail stays 208 bits, differing from the landed Extrude at exactly 2 bits inside the direction z raw (2.0 -> 5.0) — the height lives in the direction 3BD head and nowhere else; the payload does not encode height** |
| ~~P1~~ **LANDED 2026-09-24** | `ExtrudeR_<v>` | `CIRCLE` center `0,0,0` radius `3.125`; `EXTRUDE` height `2` | radius 1→3.125 (height unchanged) | does the 64-bit payload encode the profile? (3.125 has a long non-repeating mantissa — it must show bit-exactly if stored as a raw double) **OBSERVED (LANDED 2026-09-24): the tail grows 208 -> 272 with the 3.125 raw inline in the trailer region — the profile circle radius slot, the same value-form logic as the revolve CALL circle (r 1.0 lands as the two-bit short in the landed fixture)** |
| ~~P1~~ **LANDED 2026-09-24** | `ExtrudeT_<v>` | `CIRCLE` center `0,0,0` radius `1`; `EXTRUDE` with the `Taper angle` option, taper `15`, height `2` | draft/taper 0→15° (the only sweep-option variable reachable from a command) | the sweep option spine after the direction: [0,0,0,0,1.0,0,…]. A nonzero draft has to appear as a short-adjacent BD or a raw entry — whichever moves names the spine slots (and settles the same scale-vs-vector confound the revolve head carries); if NOTHING in the tail moves, the spine is NOT options and the taper went elsewhere — also decisive **OBSERVED (LANDED 2026-09-24, quad bit-identical, corpus 0/0): the raw BD 0.2617993877991494 (15 deg) lands at bit 70 — EXACTLY the first slot after the direction head. The option spine IS the SweepOptions order [draft_angle][draft_start_dist][draft_end_dist][twist][scale 1.0][align]. DECODED (commit 5c89b59): all six slots are named model fields (draft_angle = 0.2617993877991494, scale_factor = 1.0) with the untouched profile circle (0,0,0) r 1.0 — pinned by the extrude_t_names_the_sweep_option_spine hermetic test** |
**The revolve rows, post-closure (2026-09-23)**: the full-tree
scan CLOSED the family — the head IS the axis pair
(`[axis_pt 3BD][axis_dir 3BD]`, the REVOLVEDSURFACE twin's own
order; the '01' = axis_dir.Y), the trailing `(0,0,1)` trio is
the profile circle's PLANE NORMAL (inside the CALL bit-length),
and there is no form/class question left — the "form mid-region"
was the CALL header, and the original's "different form" was its
exactly-1.0 center.x in the short BD. The wire ladders stand
(A [1.2, 2.8] => M 2.0 / m 0.8; R [0.75, 3.25] => M 2.0 / m
1.25 — the drawn circles; the original [0.8, 1.2] => M 1.0 /
m 0.2 — its circle (1.0, 0, 0) r 0.2; axis unique by point-line
solve). The remaining unknowns are exactly three: the six
option shorts' NAMES (candidates: the REVOLVEDSURFACE twin's
post-angle field list — start_angle, draft_angle, the draft
distances, twist), the two flag bits, and the axis fields'
behavior under a varied axis (never yet sampled — every landed
specimen revolves about origin +Y). The scan also probed the
Extrude and Loft tails for the same CALL grammar — no [BL 18]
chain closes on either, so their profile encoding is different
and awaits their stems. The rows below cover the open slots:
C/M (grammar anchors), W (the bit-identity mirror), O/T (the
axis pair's first moving samples), F (the full-turn flags)

**A 2026-09-23 authoring constraint (in force for every remaining
row): the typed Z axis is NOT workable in the plan view — REVOLVE
axis picks must be typeable, i.e. both endpoints in the XY plane.
The landed A/R quads were therefore authored about the +Y axis
(footprint-verified below); every revolve row below uses Y-family
axes and the profile circles stay in the plan-view XY plane
(coplanar with the axis, as REVOLVE requires).**

| ~~P1~~ **LANDED 2026-09-23** | `RevolveA_<v>` | `CIRCLE` center `2,0,0` radius `0.8`; `REVOLVE` about the **Y axis** — typed points on +Y, e.g. `0,0,0` → `0,1,0` (the landing record first wrote `0,0,0` → `0,0,1`; the Z axis is un-typeable in the plan view — companions corrected) — angle `180` | (anchor) typed torus revolve about +Y: center-distance 2, r 0.8 | **OBSERVED (quad bit-identical, corpus 0/0): 254-bit tail; `revolve_angle` = π ✓; grammar (§18.6): `[axis (0,0,0)/(0,1,0)][angle pi][6 zeros][CALL 18/144][circle (2,0,0) r 0.8, normal (0,0,1)][flags]`; WIRE FOOTPRINT: \|y\| ≤ 0.8 (minor), \|x\|,\|z\| to 2.8 = the torus major radius — a Y-axis revolution** |
| ~~P1~~ **LANDED 2026-09-23** | `RevolveR_<v>` | `CIRCLE` center `2,0,0` radius `1.25`; same Y axis, angle `180` | radius 0.8→1.25 only | **OBSERVED: radius-only delta — the radius slot reads 1.25 at the same span bits, everything else byte-equal to A; footprint \|y\| ≤ 1.25, XZ to 3.25 ✓. THE 0.2 QUESTION CLOSED at the full-tree scan (§18.6): the original's CALL circle is center (1.0, 0, 0) radius 0.2 — its center.x is exactly 1.0, stored in the two-bit short BD the raw-scan could not see; the (0.2, 1.0) slots were never center/radius — they are THE RADIUS and THE CENTER.X** |
| ~~P1~~ **LANDED 2026-09-24** | `RevolveC_<v>` | `CIRCLE` center `4,0,0` radius `0.8`; axis `0,0,0` → `0,1,0` (Y), angle `180` | the CALL circle's center.x 2→4 only | the grammar predicts [4.0 raw, 0.8] at the same spans, all else byte-equal to A — a cheap grammar regression anchor **OBSERVED (LANDED 2026-09-24): decodes [axis (0,0,0)/(0,1,0)][angle pi][profile (4,0,0) r 0.8][normal (0,0,1)] — the center slot is the world X, stored raw; the grammar regression anchor holds** |
| ~~P1~~ **DEAD 2026-09-23** | `RevolveP_<v>` | ~~the perpendicular-plane profile (UCS X 90, XZ-plane circle)~~ — REFUSED by AutoCAD: *"Axis of revolution is perpendicular to plane of revolve curve. Unable to revolve the selected object."* (the maintainer's transcript). AND the scan closed the form question the stem probed: the "different form" was the original's short-form center.x — there IS no form difference | — | — |
| ~~P1~~ **DEAD 2026-09-23** | `RevolveW_<v>` form-delta purpose | ~~the drag-vs-form delta~~ — the scan closed it; the REISSUED W below keeps the true bit-identity check | — | — |
| ~~P1~~ **LANDED 2026-09-24** | `RevolveW_<v>` | **the typed mirror**: `CIRCLE` center `1,0,0` radius `0.2` (plan view); `REVOLVE` about `0,0,0` → `0,1,0`, angle typed `270` | all of the original's decode, typed | the grammar makes W's tail predict BIT-IDENTICAL to the original's (center.x short 1.0, radius raw 0.2, normal (0,0,1), same axis, angle raw 3*pi/2). Any difference = drag residue, and the position-diff IS the remaining unknown list **OBSERVED (LANDED 2026-09-24): BIT-IDENTICAL to the original Revolve tail — 190 bits, every field form matches. The typed-mirror prediction confirmed 100%; the original is fully understood as this exact authoring** |
| ~~P1~~ **DEAD 2026-09-23** | `RevolveN_<v>` | ~~angle-only 270 vs A's 180~~ — the target closed: the 38-bit-mid / trio-absence "class markers" were artifacts of the original's short-form circle, not angle correlates. W (typed 270) carries the angle sample for free | — | — |
| ~~P2~~ **DEAD 2026-09-23** | `RevolveS_<v>` | ~~radius exactly 1.0 (short-form probe)~~ — the scan closed it: BD shorts encode 0.0 and 1.0 exactly; the original's center.x IS the live specimen of a 1.0 short in a CALL field | — | — |
| ~~P1~~ **LANDED 2026-09-24 (the surface-parser row)** | `RevolveM_<v>` | a SURFACE-twin anchor: `CIRCLE` center `2,0,0` radius `0.8`; `REVOLVE` with `MOde` = `Surface`, axis `0,0,0` → `0,1,0`, angle `180` | the op's output class (SH node → REVOLVEDSURFACE + ASSOC body) | gold parses the surface FULLY TYPED (axis_point, axis_vector, revolve_angle, start_angle, 16-BD transmatrix, draft_angle, draft_start/end, twist, solid, close_to_axis) and the ASSOC body carries the named `RevolveAngle` pab-value even in R2004 files — semantic anchors for the six option shorts and the flags the SH tail still carries unnamed (§18.6) **STATUS (2026-09-24): LANDED, corpus 0/0 all four versions — the typed-field promise resolved differently than planned: the REVOLVEDSURFACE spec block is DEBUG_CLASSES-dead in the built gold and the 2027.1 classes walk derails (§18.6's decode record), so the entity decodes as gold's UNKNOWN_OBJ with the FULL raw tail bit-retained (the naming map for the option shorts + flags is now an in-corpus bit-exact specimen) and the ASSOCREVOLVEDSURFACEACTIONBODY types with the named RevolveAngle pab-value (the R2007/2010 pab shape; the R2013+ sab shape)** |
| ~~P1~~ **LANDED 2026-09-24** | `RevolveO_<v>` | `CIRCLE` center `6,0,0` radius `0.8`; axis `2.375,0,0` → `2.375,5,0` (**Y-PARALLEL, offset from origin** — both endpoints typeable in the plan view), angle `180` | axis POINT (0,0,0)→(2.375,0,0) only; the direction stays +Y | the head's axis_point takes a raw 2.375 x (per the §18.6 grammar) while axis_vector stays the 6-pair '01' shape; also decides whether the CALL circle's center keeps the world X (→6.0) or goes axis-relative (→3.625) **OBSERVED (LANDED 2026-09-24): axis_point (2.375, 0, 0) raw — the head axis pair takes the offset; the profile center stays the WORLD X (6.0, not the axis-relative 3.625) — both questions answered; the tail 254 -> 318 (+64)** |
| ~~P1~~ **LANDED 2026-09-24** | `RevolveT_<v>` | `CIRCLE` center `1,6,0` radius `0.8` (perpendicular distance to the axis ≈ 4.44 — no crossing); axis `0,0,0` → `3.75,2.5,0` (an in-plane tilt — typeable, z=0 both ends), angle `180` | axis DIRECTION only: the linear axis (3.75, 2.5, 0) — unit (0.8333…, 0.5555…, 0) if stored normalized, long-mantissa either way | the head's axis_vector becomes raw (0.8333…, 0.5555…, 0)-ish components — the head grows ~128 raw bits (size self-evident) and the axis pair gets its first moving sample (the grammar's axis_vector field, §18.6) **OBSERVED (LANDED 2026-09-24): axis_vector (0.83205, 0.55470, 0) = the NORMALIZED direction; axis_point (3.4615, 2.3077, 0) = the foot of the perpendicular from the profile center to the axis line — the axis is stored as (closest-point-to-profile, unit direction); the tail 254 -> 510 (+256)** |
| ~~P2~~ **LANDED 2026-09-24** | `RevolveF_<v>` | `CIRCLE` center `2,0,0` radius `0.8`; axis `0,0,0` → `0,1,0` (Y), typed angle `360` | sweep angle 180→360 (full) | the closed/full encoding: `revolve_angle` = 2π (6.283185307179586, mantissa-rich) or a special 0/flag; the trailing flag bits' only second sample. If AutoCAD refuses 360, type `359.9` and record it **OBSERVED (LANDED 2026-09-24): revolve_angle = 2*pi (6.283185307179586) — the full turn encodes as plain 2*pi, no special form; the tail stays 254 bits (the flags did not move with the angle)** |
| ~~P2~~ **LANDED 2026-09-24** | `Loft3_<v>` | `CIRCLE` `0,0,0` r `1` + `CIRCLE` `0,0,2.5` r `1` + `CIRCLE` `0,0,5` r `1`; `LOFT` all three | section count 2→3 | the sharpest probe for the Loft's opaque leading 68-bit region (count-derived?) and the per-section slot repetitions **OBSERVED (LANDED 2026-09-24): raws [2.5, 5.0, pi/2, pi/2] — the mid-section z appears as its own slot; per-section z slots confirmed** |
| ~~P2~~ **LANDED 2026-09-24** | `LoftH_<v>` | `CIRCLE` `0,0,0` r `1` + `CIRCLE` `0,0,7` r `1`; `LOFT` both | top z 5→7 | which `raw_doubles` slot is the section height (expect 5.0 → 7.0) **OBSERVED (LANDED 2026-09-24): raws [7.0, pi/2, pi/2] — the height slot moved 5.0 -> 7.0 exactly; the height slot is named** |
| ~~P2~~ **LANDED 2026-09-24** | `LoftR_<v>` | `CIRCLE` `0,0,0` r `1` + `CIRCLE` `0,0,5` r `2.5`; `LOFT` both | radii 1,1 → 1,2.5 | do the 2.0 / 0.3 slots track the section radii? **OBSERVED (LANDED 2026-09-24): raws [5.0, 2.5, pi/2, pi/2] — the top radius 2.5 lands as a raw slot after the top z; the radius slot is named** |
| ~~P2~~ **LANDED 2026-09-24** | `ExtrudeP_<v>` | closed `PLINE` rectangle `0,0` → `4,0` → `4,3` → `0,3` → `Close`; `EXTRUDE` the polyline, height `2` | circle → rectangle profile | is the 64-bit payload profile-shape-dependent (entity serialization) or constant? NOTE: do NOT convert the polyline to a `REGION` or `JOIN` anything — a closed LWPOLYLINE is a valid EXTRUDE profile as-is, and converting changes the record shape **OBSERVED (LANDED 2026-09-24): the tail grows 208 -> 888 — the profile IS serialized in the tail (raws include the rectangle 3.0/3.0 and 131072/147456-class quantized values); the payload question is answered: profile-dependent** |
| ~~P1~~ **LANDED 2026-09-24 (the surface-parser row)** | `ExtrudeM_<v>` | a SURFACE-twin anchor: `CIRCLE` center `0,0,0` radius `1`; `EXTRUDE` with the `MOde` option set to `Surface`, height `2` | the op's output class (SH node → EXTRUDEDSURFACE + ASSOC body) | gold parses the surface TYPED (sweep_vector, 16-BD sweep_transmatrix, the full SWEEPOPTIONS macro incl. `sweep_alignment_flags` and its 0-3 enum, `path_flags`, `base_point_set`, the two `*_transform_computed` flags, `reference_vector_for_controlling_twist`) and the ASSOC body prints the named `ExtrusionHeight`/`ExtrusionTaperAngle` pab-values — the semantic table for the EXTRUSION tail's option spine (its `'01'` member) and its 64-bit payload (the §18.6 sweep-bullet's standing confound) **STATUS (2026-09-24): LANDED, corpus 0/0 all four versions — the typed-field promise resolved differently than planned: the EXTRUDEDSURFACE spec block is DEBUG_CLASSES-dead in the built gold and the 2027.1 classes walk derails (§18.6's decode record), so the entity decodes as gold's UNKNOWN_OBJ with the FULL SWEEPOPTIONS region bit-retained in the raw tail (6830 bits on the 2018 quad — the semantic table for the EXTRUSION tail's option spine + 64-bit payload is now an in-corpus bit-exact specimen) and the ASSOCEXTRUDEDSURFACEACTIONBODY types with the named ExtrusionHeight/ExtrusionTaperAngle pab-values (the R2007/2010 pab shape; the R2013+ sab shape)** |
| ~~P1~~ **LANDED 2026-09-24 (the surface-parser row)** | `LoftM_<v>` | a SURFACE-twin anchor: `CIRCLE` `0,0,0` r `1` + `CIRCLE` `0,0,5` r `1`; `LOFT` with the `MOde` option set to `Surface` (verify the option is offered in this AutoCAD release; record the exact prompts in the `.txt`) | the op's output class (SH node → LOFTEDSURFACE + ASSOC body) | gold parses the surface TYPED (16-BD loft transmatrix, `plane_normal_lofting_type`, start/end draft angle+magnitude, `arc_length_parameterization`, `no_twist`, `align_direction`) and the ASSOC body prints the named `Continuity`/`Bulge` pab-values — anchors to name the LOFT tail's head `[1.0]` and its raw run `[2.0, 2.0, 5.0, 0.3, π/2, π/2]` (the §18.6 positional list) **STATUS (2026-09-24): LANDED, corpus 0/0 all four versions — the LOFTEDSURFACE typed-field payoff resolved through the same DEBUG-dead finding (§18.6's decode record): the entity decodes as gold's UNKNOWN_OBJ with the lofting region bit-retained (the draft-pair naming path for the SH loft tail's `[π/2, π/2]` — LoftD's dead row — runs through this raw tail or the loft container walk, NOT through a typed gold field) and the ASSOCLOFTEDSURFACEACTIONBODY types with the named Continuity/Bulge pab-values (the R2007/2010 pab shape; the R2013+ sab shape)** |

| ~~P1~~ **LANDED 2026-09-24 (re-authored Solid)** | `LoftC_<v>` | `CIRCLE` center `3,4,0` radius `1.5` + `CIRCLE` center `3,4,7` radius `1.5`; `LOFT` both with `MOde` = `Solid` at the prompt (the surface-mode first attempts landed as `LoftCSurf_<v>`, 0/0 with the surface-parser row) | **OBSERVED (quad bit-identical at 684 bits, corpus 0/0): THE LOFT RAW RUN IS FULLY NAMED — the WORLD reading confirmed: raws `[3.0, 4.0, 1.5, 3.0, 4.0, 7.0, 1.5, pi/2, pi/2]` parse as PER-SECTION `[center.x][center.y][height][radius]` runs + the draft pair (section 1: (3, 4, z 0.0 elided as a short, r 1.5); section 2: (3, 4, z 7.0, r 1.5); then `[pi/2, pi/2]`). The landed original retro-fits exactly: (0,0) r 1 elided, (2.006->2.0, 2.006->2.0, 5.0, 0.2996->0.3), the draft pair — the offsets are PRESENT in the raws (world, not bottom-relative)** |
| ~~P2~~ **LANDED 2026-09-24 — the fixture surface CLOSES** | `PolysolidL_<v>` | `POLYSOLID` → `Height` 5 → `width` 3; path `0,0,0` → `5,0,0`, ENTER to end | the PATH LENGTH only (12 → 5; the profile identical to `PolysolidX`) | **OBSERVED (quad bit-identical at 1418 bits, corpus 0/0): THE 2.0109 PROBE IS DECISIVE — raw@998 = 2.0108894202880947 in BOTH L (path 5) and X (path 12), BIT-FOR-BIT IDENTICAL → the single is PROFILE-DERIVED, not path-derived; @900 (−3.90625) and @1092 (32.0625) likewise track the profile (W's profile differs); 4.00024414192312 the record constant (third confirmation); the width single @1084 = 3.0 (the shared width); the segment end (5.0, 0.0) — the only path-derived entry. The post-corner singles stream is CLOSED: every entry is profile-derived or the segment end; the walk (the decoder row) collects them. NO MAINTAINER ASKS REMAIN — everything left is agent code work or a no-authoring-path row** |
| ~~P1~~ **LANDED 2026-09-24** | `PolysolidW_<v>` | `POLYSOLID` → `Height` 3 → `width` 7; path `0,0,0` → `12,0,0`, ENTER to end | the profile W/H swap vs `PolysolidX` (3/5 → 7/3; the path identical) | **OBSERVED (quad bit-identical at 1418 bits — same length as X with the swapped profile; corpus 0/0): corners (±3.5, {0,3}) exactly as predicted; segment end (12.0, 0) CLEAN where X carried 12.00001 residue; the post-corner BD single at bit 1084 moved 3.0 → 7.0 with the width — THE WIDTH SINGLE IS NAMED; 4.00024414192312 is INVARIANT across original/X/W (a record constant); 2.0109 is path-family-shared with low-bit precision noise; X's −19594/8614-class entries are drag residue (zeroed/absent in the cleaner authoring). The post-corner singles stream (bits 998–1286) is the next decoder row — collect it with a proper BD walk past the corner blocks** |
| ~~P1~~ **LANDED 2026-09-24 (re-authored Solid)** | `ExtrudeC_<v>` | `CIRCLE` center `2,3,0` radius `1`; `EXTRUDE` height `2` with `MOde` = `Solid` explicitly set at the prompt (the first attempt landed as a surface — re-authored; the surface-mode first attempts landed as `ExtrudeCSurf_<v>`, 0/0 with the surface-parser row, keeping the offset-variant differential) | **OBSERVED (quad bit-identical at 464 bits, corpus 0/0): the profile CALL length grew 16 -> 144 EXACTLY as predicted — the circle center (2.0, 3.0, 0.0), x/y raws (the first nonzero profile center), radius 1.0 short, normal (0,0,1). THE PRE-CALL REGION IS RESOLVED: it moved with the offset, carrying the profile center as its own raw-BD run (cx 2.0 at bit 112, cy 3.0 at bit 184) in addition to the CALL run; the full accounting closes: landed 208 + the pre-CALL center pair (128) + the CALL-center raws (128) = 464** |
| ~~P2~~ **DEAD 2026-09-24** | `LoftD_<v>` | ~~LOFT Settings → Ruled + draft fields~~ — **the authoring session cannot find the settings path** (the maintainer's report: the options as described do not exist in this AutoCAD release). The π/2 pair and the inter-value grammar stay provisionally unnamed pending another reachable loft-settings lever — the surface-parser row landed 2026-09-24 but the typed LOFTEDSURFACE fields are NOT reachable through it (§18.6's DEBUG_CLASSES finding + the classes desync: the entity decodes as raw UNKNOWN_OBJ), so the naming path is the loft container walk or the LoftM raw tail | — | — |

**Authoring mechanics** (per stem, mirroring the landed `.txt`
companions): new drawing on a blank default template, zero
pre-existing entities, solid-history recording ON, perform the ONE
typed operation, `SAVEAS` to `DWG 2007` / `DWG 2010` / `DWG 2013`
/ `DWG 2018` (one file each — the SAVEAS keeps the ACSH records on
all four; if any version drops them, emit no fixture for that
version and RECORD IT — that fact is itself evidence). Command
prompt order may differ slightly per AutoCAD release; record what
was actually typed in the `.txt` companion (the exact command
sequence + the resulting geometry). Land as
`tests/gold_harness/tests/sh_history/<Stem>_<version>.dwg` with
the companion `<Stem>_<version>.txt`; the root `.gitignore`
already negates the fixture tree.

**Qualification** (the landing gate, per file): the F2.1 checks
(gold `dwgread -O JSON` zero `Error` lines; the family's ACSH class
present; minimal census) PLUS the differential invariants, checked
before committing:

1. The four versions of a stem carry **bit-identical tails** —
   extract with `target/debug/dwg2json` (the
   `shsw_raw_tail`/`raw_tail` byte arrays), compare. The landed
   families are all bit-identical across versions; a new stem must
   be too, or the tail needs re-inspection before it can serve as
   a decode anchor.
2. The corpus must take every file at **0/0** — these classes are
   write-verbatim, so a well-formed specimen cannot fail unless
   the authoring produced something structurally new (which then
   is the interesting fact — record it).

**The processing pipeline** (what the next session runs once the
pairs land — keep this with the recipe):

1. Dump the new tails (`dwg2json`) and re-verify the quartet
   identity per stem.
2. **Footprint-verify the intended geometry** (the A/R lesson:
   the authoring session and the recipe rows disagreed about the
   axis): on the R2007/R2010 gold reads, check the 3DSOLID wire
   extents against the intended footprint (e.g. a Y-axis torus
   shows the minor extent in y and the torus major radius in XZ).
   The R2013+/R2018 wire walks are the known phantom-bit garbage —
   do not use them for the footprint.
3. Run the Phase B decoder over every new tail; the confirmed
   anchors (head, revolve angle, profile block, corners, segment
   end) should land named already; print the residual.
4. Position-diff the partner tails bit-by-bit against each other
   (and against the landed originals where the variable matches):
   every entry that CHANGED with the varied geometry is now
   attributed — extend `sh_tail_decode.rs` (collectors for newly
   named entries, spans for their raws), promote typed fields,
   pin hermetic tests on the new pair, and re-run the gates.
5. Between P1 stakes, `PolysolidX`+`PolysolidD` crack the frame
   blocks (the largest opaque mass), `ExtrudeH`/`ExtrudeR` the
   64-bit payload, `ExtrudeT` the option spine, and the revolve
   matrix the head/axis questions (the old 0.2 question is closed:
   the profile block); the P2 stems then name the
   Loft slots — the campaign's blob-autopsy pack closes when every
   raw BD entry in every specimen decodes to a typed field with a
   pinned test.

The `0.3` and `2.0, 2.0` Loft slots and the sweep single-values
are EXPECTED to resolve here; the recipe deliberately uses values
with long mantissas (3.125, 1.25, 2.375, 3.75, 2.5) so stored raw
doubles are recognizable bit-exactly rather than confusable with
round constants.

**The 0.2-entry falsification table** (2026-09-23 historical — drafted
when the original's geometry was INFERRED as center 0.6 / r 0.8 and
the 0.2 read as a single opaque raw; the A/R landing RESOLVED it —
see below):

| stem | center distance | radius | bore \|center−r\| | predicted 0.2-slot if bore | if radius | if center | OBSERVED (wire regressions, 2026-09-23) |
|---|---|---|---|---|---|---|---|
| original (observed) | — | — | — | **0.2 (observed)** | would need r=0.2 ✗ | would need 0.2 ✗ | **RESOLVED FULLY (the scan, S18.6): the original IS the as-drawn circle — center (1.0, 0, 0) r 0.2 in the CALL grammar; its exactly-1.0 center.x takes the two-bit short BD, which is why the raw-scan read (0.2, 1.0) as the two profile slots at all. The earlier "(0.6, 0.8)" and "different form" geometries were both wrong readings of one specimen** |
| `RevolveA` | 2 | 0.8 | 1.2 | 1.2 | 0.8 | 2.0 | **M 2.0 / m 0.8 confirmed by ladder [1.2, 2.8]; slots = the AS-DRAWN [center (2,0,0)][radius 0.8][normal (0,0,1)] — the circle-form record** |
| `RevolveR` | 2 | 1.25 | 0.75 | 0.75 | 1.25 | 2.0 | **M 2.0 / m 1.25 by ladder [0.75, 3.25]; radius-only delta byte-clean at the same span** |
| `RevolveC` | 4 | 0.8 | 3.2 | 3.2 | 0.8 | 4.0 | still to author — confirms the center slot as the drawn center (+ the mid) |
| ~~`RevolveI`~~ | — | — | — | — | — | — | **dead: AutoCAD refuses crossing profiles** (maintainer report, 2026-09-23) — replaced by P/N/S/W |

The bore theory died, THEN the crossing-class theory died with it.
What stands after the wire regressions: the A/R record stores the
profile circle AS DRAWN (center/radius/plane-normal — the
coplanar-circle form, 32-bit form mid); the original's record is a
DIFFERENT form (38-bit form mid, slots (0.2, 1.0) = its torus
(m, M), no normal trio) — the form question is now the matrix's
center of gravity, owned by **RevolveP (perpendicular-plane
profile), RevolveW (the typed original mirror), RevolveN (the
270° angle), RevolveS (the short-form radius)**, plus the axis
stems O (point) and T (direction) and the angle stem F (360).

---

## 19. The header & whole-structure campaign (planned 2026-09-25; PHASE 2 landed through the H8h arc — the READ axis ZERO corpus-wide, the AC1021 record-identity survey at 58/58 files / 0 divergent records as of H8h-ext-12, 2026-09-28)

The campaign target to date is the **OBJECTS axis**: silver's parsed
entity/object records must match gold's, plus the write-side re-read
must match the original read (mission §1: "the header is tested laxly").
The harness enforces that axis at 0/0 over 280 files. This section is
the plan for **PHASE 2: the structure axis** — extend the gold-vs-silver
comparison to everything the reader sees that is NOT an object record,
and produce an audit matrix proving every piece of the DWG file
structure is either diffed or explicitly excluded with a recorded
reason.

### 19.1 The measured inventory (the ground facts; probed 2026-09-25, corrected the same day after the six-version matrix review)

Gold's `dwgread -O JSON` emits the whole structure; the harness's
`normalize_gold.py` drops it all by construction (`data.get("OBJECTS",
[])` — only the OBJECTS list survives; `HEADER_KEYS` at its lines
86–106 is the DECLARED inventory, never referenced by code). Observed
matrix (gold over all six version classes: `sample_2000`,
`example_2004/2007/2010/2013`, `sample_2018`, the `-nan` shim
applied): 16 top-level keys per R2004+ file (15 structure keys +
OBJECTS), 9 on R2000 (8 + OBJECTS) — the corpus-wide union is **17
distinct structure keys**; the declared drop-list holds **19** (the
17 + `VBAProject`, never observed in the corpus). **H0 corollary:**
turning drop-by-construction into an explicit per-key policy needs an
assertion that no UNDECLARED top-level key silently leaks —
`Signature` is a real libredwg key absent from the declared list
(and from the corpus today), the latent example.

| gold key | R2000 | R2004–R2018 | size (samples) | silver coverage today |
|---|---|---|---|---|
| `HEADER` | ✓ (228) | ✓ (298) | the variables section (`AcDb:Header`) | `header` dict (267 keys on sample_2018) — parsed, never compared |
| `FILEHEADER` | ✓ (8) | ✓ (15) | version/maint/codepage/time/save addresses | **LANDED 0/0** (`bdf8107`): the full field family retained via `document.dwg_file_header` (the byte-position ledger in dwg_reader.rs) |
| `R2004_Header` | — | ✓ (23) on R2004/R2010/R2013/R2018 | the R2004-based system section | **LANDED 0/0** (`7abb0af`): all 23 fields + the 12-byte padding hex via `document.dwg_r2004_header` |
| `R2007_Header` | — | ✓ (33) on R2007 ONLY | the AC1021 system section (33 fields — its own layout, not the R2004 one) | **LANDED 0/0** (`73935b7`): all 33 fields via `document.dwg_r2007_header` (the gold-named projection of the container metadata) |
| `SecondHeader` | ✓ (10) | — | the R13–R2000 second header | **LANDED 0/0** (the H2d packet): the sentinel-located read via `document.dwg_second_header` |
| `AuxHeader` | ✓ (25) | — (absent on the corpus R2004+ files) | aux data | **LANDED 0/0** (the H2d packet): the locator-addressed read via `document.dwg_aux_header` |
| `SummaryInfo` | — | ✓ (13) | `AcDb:SummaryInfo` | **LANDED 0/0** (the H4 packet): the 8 strings + the 3 timer pairs + the 2 unknowns via the extended `summary_info` |
| `AppInfo`/`AppInfoHistory` | — | ✓ (10/2) | `AcDb:AppInfo` | **LANDED 0/0** (the H4 packet): the raw-section + parsed shapes via `document.dwg_app_info`/`dwg_app_info_history` |
| `Template` | ✓ | ✓ | `AcDb:Template` | **LANDED 0/0** (the H4 packet): via `document.dwg_template` |
| `FileDepList` | — | ✓ (1) | `AcDb:FileDepList` | **LANDED 0/0** (the H4 packet): features + the files records via `document.dwg_file_dep_list` |
| `RevHistory` | — | ✓ (3) | `AcDb:RevHistory` | **LANDED 0/0** (the H4 packet): via `document.dwg_rev_history` |
| `Security` | — | ✓ (9) | zero constants on unprotected files | **LANDED 0/0** (the H4 packet): via `document.dwg_security` |
| `ObjFreeSpace` | — | ✓ (12) | `AcDb:ObjFreeSpace` | **LANDED 0/0** (the H4 packet): the version-gated shapes via `document.dwg_obj_free_space` |
| `THUMBNAILIMAGE` | ✓ | ✓ | the preview blob | `preview` dict — parsed, never compared |
| `AcDs` | — | ✓ (13 keys on 2004/2007; 15 on 2013/2018) | the AcDs data section | the embedded-record decode (§18); no section-level comparison |
| `CLASSES` | ✓ | ✓ | the class table | parsed (§18.6's gold-shadow); dropped by both normalizers |
| `VBAProject` | — | absent from the corpus files | declared in the drop-list; never observed | none (an excluded row, H6) |
| `Signature` | — | absent from the corpus files | **emitter deliberately disabled** (`out_json.c:2673`) — gold reads the section but never emits it | none (an excluded row, H6 — reason "readable but not emitted", distinct from corpus-absent) |
| `created_by` | ✓ | ✓ | **NOT FILE CONTENT**: gold's `PACKAGE_STRING` ("LibreDWG 0.14.8597", defined at src/config.h:361), hardcoded as the first JSON key by `out_json.c:2617` | n/a — an excluded row (H6): each tool stamps itself |
| — (not JSON) | ✓ | ✓ | the **object map / Handles section**, CRCs, section map, page tables, sentinels | the byte level — the layer-4 oracle's domain |

**The libredwg source-tree re-analysis (2026-09-25, the authoritative
enumeration):** the gold tree holds a dedicated spec file per structural
part — `header.spec` (FILEHEADER), `header_variables.spec` (HEADER),
`2ndheader.spec`, `auxheader.spec`, `r2004_file_header.spec`
(R2004_Header/R2007_Header), `acds.spec`, `appinfo.spec` (AppInfo +
AppInfoHistory), `filedeplist.spec`, `objfreespace.spec`,
`revhistory.spec`, `security.spec`, `summaryinfo.spec`,
`template.spec`, `vbaproject.spec` (plus `header_variables_dxf.spec`/
`_r11.spec` — DXF-only / pre-R13, out of scope). These 14 spec files
(the `appinfo.spec` pair counts once) are the authoritative field
lists for the H2–H5 projections, exactly as `dwg2.spec` was for
OBJECTS. TWO structure keys have spec-less dedicated readers instead:
THUMBNAILIMAGE (a raw size+bytes blob — `json_thumbnail_write`, no
spec) and CLASSES (its own reader, no spec). The section universes are the two enums in `include/dwg.h`:
`Dwg_Section_Type` (R2004+, 20 values — the 17 named sections plus
`SECTION_UNKNOWN` = the FILEHEADER itself, `SECTION_INFO` = the
Data Section / Section Page Map, `SECTION_SYSTEM_MAP` — the R2004+
container's page machinery, no JSON emission, byte-level) and
`Dwg_Section_Type_r13` (R13–R2000: HEADER, CLASSES, HANDLES,
OBJFREESPACE — **"including the 2ndheader"**, TEMPLATE, AUXHEADER,
THUMBNAIL). Additional tree facts the rows must carry: **`created_by`
is the oracle's PACKAGE_STRING stamp, not file content** (excluded
row); **Signature has a JSON emitter deliberately disabled** in the
dispatch (`out_json.c:2673`) — excluded reason "readable but not
emitted", distinct from corpus-absent; **the object-map emission
(`json_handles_write`) is `#if 0`'d** — the Handles section is
deliberately unemitted; **the R2000 emission is gated by the
FILEHEADER's locator count** (ObjFreeSpace/Template/AuxHeader only
when `header.sections` ≥ 3/4/6) and R2004+ SummaryInfo/VBAProject by
`summaryinfo_address`/`vbaproj_address` — FILEHEADER fields are
structurally load-bearing for WHICH keys appear (an H2 comparison
coupling); **AppInfoHistory's wire name IS "AcDb:AppInfoHistory"**
(corrected in the H4 landing: gold's -v4 trace shows the map entry
carrying the name; gold's internal lookup is by section TYPE 12
(`read_2007_section_appinfohistory`, decode_r2007.c:1963), but the
name-based registry only needed the constant — the earlier "no name
string" claim in this section was wrong). The enumeration is CLOSED: 14 spec files (15
spec'd parts — `appinfo.spec` covers two) + the two spec-less readers
(THUMBNAILIMAGE, CLASSES) + the 3 container types + the object map +
CRCs/sentinels — nothing else exists in the tree.

**Completeness review (2026-09-25, same day — the header+body axes vs the
whole file):** the two axes cover every JSON-emitted structure key (the
matrix above) plus the body (OBJECTS). The review probed the parts gold
does NOT emit and pre-populates their H6 excluded rows: the R2004+
section-map/page-table ENTRIES (gold's `R2004_Header`/`R2007_Header`
emit the pointers — section_map_id/address, section_info_id,
section_array_size, gap_array_size, crc32 — never the per-section map
entries or page-map contents; silver reads them: `PAGE_TYPE_*`,
`ac21_section_info`), the R2000 "unknown" section (locator nr 3/4 —
the locators themselves ARE comparable via `SecondHeader.sections`),
per-section CRCs/sentinels/0x16-padding, and the AcDs segment-index
internals beyond gold's emitted keys (`segidx_offset`/`segidx_unknown`
point at it). One REAL silver gap surfaced: **`AcDb:AppInfoHistory`
was unknown to silver's section registry** (`ALL_SECTION_NAMES` held
14 names — it also omits `AcDsPrototype_1b` and `Signature`, which
exist as constants but not in the list) — RESOLVED in the H4
landing: the `APP_INFO_HISTORY` constant exists and the descriptor
lookup by wire name works (the map entry carries the name; the
"located by TYPE, not name" claim below was corrected).
Also confirmed comparable: the R2000 section-locator table + its
per-locator handle vector live in `SecondHeader.sections`/`.handles`
(6 nr/address/size records + 5 hdl records on sample_2000), and
`ObjFreeSpace` carries per-version field shapes (`max32_hi`-style
64-bit splits on R2018, plain on R2004) the H4 projection must map.

Silver's section registry knows every `AcDb:*` name
(`section_definition.rs`: Objects, AcDsPrototype_1b, AppInfo,
AuxHeader, Header, Classes, Handles, ObjFreeSpace, Template,
SummaryInfo, FileDepList, Preview, RevHistory, Security, VBAProject,
Signature) — the parse side is further along than the emission side.

### 19.2 The rows (ordered; each lands under the §8.1.2 per-packet workflow)

- **H0 — the axis skeleton + the day-one census (LANDED 2026-09-25)**:
  the second axis ships as `tests/gold_harness/struct_axis.py` (the
  closed key registry; gold view extraction with the -nan/ACIS shims;
  silver's minimal seed projection — header/summary/preview/classes +
  the maintenance_version→maint_rel_version mapping the R2000 smoke
  pinned; a leaf-flattener with name-canonical matching; the no-leak
  assertion — corpus-clean: zero undeclared keys, zero census errors
  over 280 files). `run_roundtrip.py` wires it beside the frozen
  OBJECTS pipeline (read census gold_orig-vs-silver_orig; write-
  target census gold_orig-vs-gold_rt; defensively isolated so the
  axis can never break a file run); `run_corpus.py` aggregates the
  per-key census into report.json/report.md. **The OBJECTS axis held
  0/0 through the landing (before AND after: 280 files read 0 /
  write 0).**
  **Day-one census (the H2-H5 attack order; corrected aggregation
  2026-09-25 — presence counts read-axis-only, per-axis leaf sums,
  trimmed census samples, an `--aggregate-only` report-regeneration
  mode so aggregation changes never need the 10-minute corpus cycle):**
  read key-gap **259,073**; write-target key-gap **21,822**; zero census
  errors; zero undeclared keys. READ axis per key (presence gold/silver
  → matched+diffs+missing): HEADER **280/280 → 9,532 matched + 0
  value-diffs + 128,489 missing** — the gap is pure name-set coverage,
  the H3 variable ledger is THE dominant row; AcDs **273/0 → 58,471
  unprojected** (no section-level silver view at all); CLASSES 280/280
  → 18,241 matched + 4,460 value-diffs + 42,713 missing (the value
  diffs are the §18.6 gold-shadow garbage-vs-sane reads surfacing on
  the loose canonical name match — H5 projects the table properly);
  R2004_Header 232/0 → 5,336; FILEHEADER 280/280 → 560 matched (the
  version/maint_rel pair) + 3,591 missing; ObjFreeSpace 276/0 → 3,441;
  Security 273/0 → 2,457; AppInfo 273/0 → 2,396; SummaryInfo 272/272
  → 2,176 matched + 2,176 missing (silver's 9-field shape covers
  half); R2007_Header 41/0 → 1,353; FileDepList 273/0 → 1,055;
  RevHistory 273/0 → 819; Template 280/0 → 560; THUMBNAILIMAGE
  279/177 → 558 (the H5 digest work); AppInfoHistory 273/0 → 546;
  SecondHeader 7/0 → 386 and AuxHeader 7/0 → 266 (R2000-only).
  WRITE-TARGET axis (the H7 preservation ledger): **AcDs 58,471
  matched at 0 diffs 0 missing — the §18 verbatim re-emission,
  corpus-wide**; Template/RevHistory/Security 0-diff too; HEADER
  6,366 value-diffs (gold's unknown slots + the time fields); CLASSES
  3,246 (num_instances silver values); ObjFreeSpace 2,453+30 (content
  not preserved); R2004_Header 1,771 (address/numsections shifts:
  15→17 — silver writes MORE sections); AppInfo 1,651 (silver rewrites
  its own blob); FILEHEADER 1,205 (address shifts + maint_rel 0→4);
  SummaryInfo 1,533 (times zeroed); R2007_Header 979; THUMBNAIL 442
  (re-encoded); AppInfoHistory 342; AuxHeader 95 — **plus two
  whole-section drops: SecondHeader (386 leaves) and FileDepList
  (1,055 leaves) — silver's writer omits those sections entirely on
  the rewrite; H7 owns them.**
- **H1 — the corollary write-fidelity definition**: the structure axis
  compares `gold_orig` vs `silver_orig` (read) and — the same rule as
  the OBJECTS axis — `gold_orig` vs `gold_rt` (write, i.e. silver's
  rewrite re-read by gold). Byte-level re-emission fidelity for the
  header/system sections gets the layer-4 treatment (dump + compare
  the rewritten header region for one fixture per version-class).
- **H2 — the file header family (COMPLETE 2026-09-25 — all five
  sub-rows landed at zero read gaps; 11,492 leaves now MATCHED
  (4,151 + 5,336 + 1,353 + 386 + 266), the read key-gap −10,932
  vs the H0 baseline — the 560-leaf difference is FILEHEADER's
  version/maint_rel pair, already name-matched at H0; the
  9e945fe commit subject's "leaves closed" phrasing means the
  matched count)**:
  **The FILEHEADER drop landed at ZERO read gaps corpus-wide**
  (4151 matched = 273×15 + 7×8 leaves exactly; 0 value diffs; 0
  missing; the corpus read key-gap 259,073 → 255,482, −3,591 = the
  H0 day-one census's measured FILEHEADER component, verbatim; the
  write-target
  FILEHEADER 1,205 diffs are the address/maintenance shifts — H7).
  The byte-ledger facts the packet pinned: the reader's byte math was
  ALREADY correct on both version paths — every field had been
  SKIPPED or MISLABELED, never misread: `zero_one_or_three` = byte 12
  ("the unknown byte"), the R2000 dwg_version/maint_version pair =
  bytes 17/18 (the "magic 0x1B/0x19"), `sections` = the R2000
  locator-record count, and the R2004+ tail (unknown_0/app_dwg/
  app_maint at 21-23, rl_1c_address at 28-31, r2004_header_address at
  40-43, mostly 128/0x80 observed) skipped wholesale. The position
  table lives in `dwg_reader.rs`'s DwgFileHeaderInfo doc comment; the
  changes were additive, verified byte-ledger-neutral (the
  hand-decode of sample_2000's raw bytes vs gold's observed values
  resolved the header.spec's misleading FIELD ORDER — the `@0x0d`
  comment and gold's JSON carry the truth). Two H2-authentic
  pitfalls recorded: (a) inserting the summary struct above
  `pub struct CadDocument` stole the struct's derive attribute
  (caught by compile, restored); (b) shrinking the R2004+ trailing
  pad 80→76 broke the 0x80 file-header ledger (the R2004+ FILEHEADER
  is 128 bytes: 6 version + 122 metadata/pad; gold's
  `r2004_header_address` = 128/0x80 is where the encrypted system
  section starts, and the reader's next reads continue from that
  position) — caught by the SAB and R2007-solid-3D hermetic tests
  (both green at the fix, 1324/0), the OBJECTS axis held 0/0 across
  all gates. (The bdf8107 commit message says "0x100" — the wrong
  constant; this row is the corrected record.) The retention path:
  DwgFileHeaderInfo → the reader populates
  `document.dwg_file_header: Option<DwgFileHeaderSummary>` (gold's
  field names 1:1) → the dump emits it through CadDocument's serde →
  struct_axis projects it with version-family gates (the R2004+ tail
  drops on pre-2004 files; `sections` drops on R2004+ — both gold
  gates).
  **The R2004_Header sub-row landed at ZERO read gaps corpus-wide**
  (232/232 files; 5,336 matched = 232 × 23 leaves exactly; 0 value
  diffs; 0 missing; the corpus read key-gap 255,482 → 250,146, −5,336
  = the census component verbatim; the write-target R2004_Header
  1,771 diffs are the address/count/CRC shifts — the rewrite's new
  container layout legitimately moves the addresses and re-CRCs the
  content (sample_2018: numsections 15→17, crc32 differs) — H7). The
  120-byte
  encrypted block at 0x80 (XOR-masked, 256-byte cyclic magic) holds
  gold's 23 fields + the 12-byte padding tail ("the padding is also
  encrypted, but ODA didn't grok that") — silver's historical 0x6C
  read extended to 0x78, byte-ledger-safe because the mask cycles at
  i % 256. The parse: the reader had been reading the 0x28/0x50
  windows for container navigation but DISCARDING them (underscore
  locals) and mislabeling `section_info_id` (@0x5C) as
  `section_map_id` (gold's real section_map_id sits @0x50 — silver's
  historical labels and gold's spec name the two slots oppositely;
  the container navigation value is unchanged, the JSON projection
  follows gold's names. The 7abb0af commit subject says "the
  ODA-vs-gold naming flip" — an attribution this tree cannot verify;
  this row is the corrected record). Two
  emission facts pinned by sample_2018: gold prints
  `section_map_address` RAW (19328 = the stored value; the +0x100 is
  decode-side navigation only — the first draft's adjusted value
  produced the packet's only value diff, fixed to raw), and
  `padding` prints as 12-byte uppercase hex. The retention path:
  `DwgFileHeaderInfo.r2004_system` → `document.dwg_r2004_header`
  (gold's field names 1:1, AC18-format files only) → the dump's serde
  → struct_axis's `R2004_Header` view (absent on R2007/R2000 by
  construction).
  **The R2007_Header sub-row landed at ZERO read gaps corpus-wide**
  (41/41 files; 1,353 matched = 41 × 33 leaves exactly; 0 value
  diffs; 0 missing; the corpus read key-gap 250,146 → 248,793,
  −1,353 = the census component verbatim; the write-target
  R2007_Header 979 diffs are the address/count/CRC shifts — the
  rewrite's new container layout moves the addresses and re-CRCs
  the content (example_2007: file_size/pages_map2_offset shift,
  pages_map_crc_compressed/crc_seed re-CRC) — H7). This
  sub-row was PURE PROJECTION: silver's AC1021 container reader
  (`Dwg21CompressedMetadata`, the Reed-Solomon-decoded 0x110-byte
  metadata block) already parsed every field — the summary is the
  gold-named mapping of it (`pages_map_correction_factor` →
  `pages_map_correction`, `map2_offset` → `pages_map2_offset`,
  `unknown_0x20/0x40/0xf800/4/1` → `unknown1..5`, the
  `*_compressed/*_uncompressed` suffixes → gold's `*_comp/
  *_uncomp`, `header_crc64` → `header_crc`); `sections_amount`
  has NO gold-emitted counterpart (gold's JSON prints 33 fields
  without it) and is dropped at the projection source. The u64
  field type is load-bearing: gold's emitter prints the high-bit
  CRCs as UNSIGNED (sections_map_crc_comp 14004064320028269436 >
  2^63 on example_2007), so signed i64 projection would have
  produced 41 value diffs instead of zero.
  **The R2000 pair landed at ZERO read gaps corpus-wide** (SecondHeader
  7/7 files, 386 matched = 4×56 + 3×54 leaves — the 56-leaf files carry
  2-byte `hdl` vectors, exactly as the census predicted; AuxHeader 7/7,
  266 matched = 7×38; 0 value diffs; 0 missing; the corpus read
  key-gap 248,793 → 248,141, −652 = the census components verbatim).
  Both were NEW READS (the family's only ones): the AuxHeader at its
  section-locator address — gold decode.c:373-405, "no sentinels since
  R13c3", gated on the FILEHEADER `sections` count == 6 — parsed
  byte-aligned per `auxheader.spec`, with the field order
  HAND-DECODED byte-for-byte against gold's JSON on sample_2000
  before implementation (every field matched on the first census
  run); the SecondHeader located by the 2NDHEADER_BEGIN sentinel
  (`D4 7B 21 CE … AA 01`, gold decode.c:907 searching forward from
  the ObjFreeSpace read — silver searches from the ObjFreeSpace
  locator address with a 0 fallback; the sentinel is unique) and
  parsed per `2ndheader.spec` via the bit reader: RL size, BL
  address, the 11-byte version, RC maint_rel_version, RC
  zero_one_or_three, BS dwg_versions, RS codepage, the ≤6 section
  records (RC nr, BL address, BL size), the ≤14 handle records
  (RC num_hdl, RC nr, num_hdl raw bytes — `num_hdl` itself does not
  print), the unprinted RS CRC, and `junk_r14` (RLL) on R14/R2000
  only. Both reads are non-fatal on failure (informational
  sections). The write-target columns stay H7 rows: silver's writer
  omits the SecondHeader entirely on rewrite (0+0+386) and
  synthesizes the AuxHeader with shifted values (266 matched, 95
  diffs — numsaves/times).
  **The remaining sub-rows' scope notes:** `SecondHeader` carries
  the R2000 section-locator table (6 nr/address/size records) AND its
  per-locator handle vector — the only JSON-emitted image of the
  section map on R2000 — so its projection is structural, not just
  scalar (the `Dwg_Section_Type_r13` enum documents SecondHeader as
  living INSIDE the ObjFreeSpace section on R13–R2000); the R2004+
  system-header POINTER fields (section_map_id/address,
  section_info_id, section_array_size, gap_array_size, crc32) are now
  part of the landed 23-field comparison — the section-map/page-map
  CONTENTS they point at stay byte-level (H6). **Emission-gating coupling (the tree re-analysis): gold emits
  ObjFreeSpace/Template/AuxHeader on R2000 only when the
  FILEHEADER's `sections` locator count is ≥ 3/4/6, and
  SummaryInfo/VBAProject on R2004+ only when
  `summaryinfo_address`/`vbaproj_address` are set — FILEHEADER fields
  are structurally load-bearing for WHICH structure keys appear at
  all, so the H2 projection must reproduce the same conditional
  presence (a key present in gold but absent in silver — or vice
  versa — is a structural diff, not a field diff).**
- **H3 — the variables (`HEADER`) (LANDED 2026-09-26 at ZERO read gaps
  corpus-wide — the LAST open structure read row; the whole structure
  READ axis now at zero)**: HEADER 280/280 → **138,021 matched + 0
  value-diffs + 0 missing** (the read key-gap 128,489 → **0**; the
  OBJECTS axis held 0/0, the write-target key-gap unchanged at
  21,822 — the H7 row). The day-one scoping held exactly: the gap was
  a TOTAL NAMING SPLIT — silver's modeled `header` prints its own
  snake_case names (267 unversioned keys, bare-handle/shape
  serialization) while gold prints the ALLCAPS spec names with
  component tuples; the 280-file gold harvest pinned the emission as
  purely version-determined (identical key sets per class: 228 R2000 /
  246 R2004 / 291 R2007 / 296 R2010 / 298 R2013+, a 300-key union).
  **The design — the raw-mirror summary:** `DwgHeaderRaw` on the
  document (document.rs, ~300 `Option` fields, one per gold key,
  serde-renamed to gold's spelling, skip-if-none so the version gates
  fall out of the reader walk's population), retained by TWIN
  assignments in `read_header_fields` — every wire read taken once,
  assigned to the model where silver models the field and stored raw
  under gold's key (the `let _ =` discards became `raw.x = Some(..)`;
  the walk's bit consumption unchanged, which the write axis at 0/0
  had already proven per-version on all 280 files). Handles retain
  the wire form `[code, size, value, absolute]` (gold's FORMAT_HREF —
  `DwgRawHandle`), points `[f64; 3]`, limits `[f64; 2]`, TIMEBLL
  `[days, ms]`, colors the post-decode CMC parts (`DwgRawCmc`).
  struct_axis projects `dwg_header_raw` key-for-key (the modeled
  `header` stays the API surface; the writer is untouched — the H7
  write-target row stands).
  **The "gold is unique" re-analysis (the print-exactness rules,
  re-derived from the libredwg src before landing — every one
  load-bearing):** (a) `FORMAT_BS` is `PRIu16` **UNSIGNED**
  (include/dwg.h:152) while `FORMAT_BSd` is signed — the d-suffixed
  emitted fields are exactly TREEDEPTH/USERI1-5/DIMLWD/DIMLWE, and
  all eight `FIELD_CAST` fields (DIMALTD/DIMZIN/DIMTOLJ/DIMJUST/
  DIMTZIN/DIMALTZ/DIMALTTZ/DIMTAD) cast to plain BS (the third macro
  arg — unsigned); OBSCOLOR/INTERSECTIONCOLOR are plain BS — so the
  raw walk casts BS→u16 except the four BSd families (the first
  draft's blanket signed casts would have flipped negative BSd
  values like DIMLWD −2); (b) `FORMAT_BL`/`BLx` are `PRIu32`
  unsigned (FLAGS, unknown_8/9/12–17/21/22), TIMEZONE is `BLd`
  signed, `FIELD_TIMEBLL` prints `[days, ms]` as unsigned BL, and
  REQUIREDVERSIONS is BLL unsigned; (c) BD prints `%.14f` with
  trailing-zero trim (out_json.c `_VALUE_RD`) — inside the axis
  float tolerance (`_REL_TOL` 1e-6), no normalization needed; (d)
  **the CMC post-decode state** (bits.c `bit_read_CMC`): decode
  OVERWRITES the wire BS index with `dwg_find_color_index(rgb)`
  (gold's 256-entry palette — 222 entries differ from silver's ACI
  table; the palette is embedded hex in struct_axis), ZEROES a flag
  ≥ 4 without reading the name/book strings, and forces an
  out-of-range method nibble to 0xC2 keeping the low 24 bits; the
  emitter (out_json.c `field_cmc`) then prints the index iff non-zero
  — **INCLUDING 256** (INTERFERECOLOR `{index: 256, rgb: c3000001}`
  — no palette match for the low bits), the rgb as `%06x`, the flag
  iff non-zero, name/book behind bits 0/1; the emitter's else-branch
  derivations (TRUECOLOR low byte, BYLAYER/BYBLOCK re-lookup) are
  DEAD — a zero lookup implies rgb&0xFFFFFF==0 so they yield 0 too;
  pre-R2004 prints the bare wire index via `%d` over the uint16
  (0..65535). Silver's `read_cm_color_raw` mirrors the decode
  validations exactly (the modeled `read_cm_color` is untouched);
  (e) every spec `FIELD_VALUE` transform (FLAGS |= lweight, TSTACK
  defaults, unit1_ratio = 412148564080.0, unknown_8 = 24) is
  `ENCODER`/`IF_ENCODE_FROM_EARLIER` gated — decode emits wire values
  verbatim;   (f) the null-ref 2-element `[0,0]` handle emitter branch
  is unreachable in practice (every corpus header handle is an
  absolute-code form — see the review below). **Verification: the six-version probe set (one file
  per class: 2000/Line, example_2004/2007/2010/2013, sample_2018)
  at 401/419/489/494/499/499 leaves — 0 diffs, 0 missing, 0 extra on
  every class; cargo test 1588/0; the sh_history smokes 0/0 with the
  structure read key-gap 0; the full corpus 280 @ 0/0.**
  **The review pass (2026-09-26, same day):** (a) the handle-form
  semantics re-derived from `dwg_decode_handleref` (decode.c:4712 —
  "We receive a null obj when we are reading handles in the header
  variables section"): with `obj == NULL` the absolute ref is the
  RAW PAYLOAD `value` for every code — the 6/8/A/C base-relative
  arithmetic never applies in the header (there is no base), and a
  size-0 form resolves to 0. The first draft's `read_handle_raw`
  encoded the object-relative semantics for those codes (code 6 →
  absolute 1, code 8 → −1 wrapping, code C → base−offset) —
  corpus-dead (a scan of every probe's raw mirror finds zero
  header handles with code > 5) but wrong for exactness; fixed to
  the header rule (`absolute = value` for all codes). The one
  dropped case (size > 0 with value == 0 and code ≥ 6 → gold
  frees the ref and the emitter prints the 2-element `[0,0]`
  null form) stays corpus-dead and is documented, not modeled —
  `DwgRawHandle` is a fixed 4-tuple; (b) the `index: 256`
  emitter claim CONFIRMED empirically: INTERFERECOLOR prints
  `{"index": 256, "rgb": "c3000001"}` on all four R2007+ probes
  (the palette lookup of rgb&0xFFFFFF == 1 finds no match → 256
  → non-zero → printed); (c) a suspected latent issue CLEARED:
  the pre-R2004 CMC print is `FORMAT_RSd` (%d over the
  int-promoted uint16) — values print 0..65535 positive, no
  sign flip, so the projection's plain index is exact as-written
  (2000 CECOLOR = 256 pinned).
- **H4 — the metadata blocks (LANDED 2026-09-25 at ZERO read gaps
  corpus-wide; 15,626 leaves now MATCHED, the read key-gap −13,450
  [248,141 → 234,691] — the 2,176-leaf difference is SummaryInfo's
  8 strings × 272, already name-matched at H0; the 50c246d commit
  subject's "13,450 leaves" phrasing means the gap reduction — the
  H2d-review metric convention)**: `SummaryInfo` 4,352 matched (272×16: the existing 8
  strings + the 3 TIMERLL pairs + the 2 trailing unknowns, captured
  where the old parse SKIPPED the 24 timer bytes), `AppInfo` 2,396
  (273 files, the version-gated shapes below), `AppInfoHistory` 546
  (273×2 — the raw section as size + hex; gold's spec include for it
  is commented out, never parsed), `Template` 560 (280×2, all
  versions), `FileDepList` 1,055 (features + the 9-field `files`
  records), `RevHistory` 819 (273×3), `Security` 2,457 (273×9
  zero-constants), `ObjFreeSpace` 3,441 (276 files — 273 R2004+ plus
  the 3 R2000 files whose locator count ≥ 3). **The load-bearing
  findings:** (a) the EMISSION is unconditional on R2004+ (out_json's
  R_2004 arm gates only SummaryInfo/VBAProject by address) — files
  without the section print the ZEROED struct (sample_2018's map
  carries only the 13 core names; Security's 9 zeros come from
  files that have no Security section), so the reader parses the
  empty buffer on fetch-failure to reproduce the zeroed emission,
  while R2000 keeps the locator-gated skip; (b) the AppInfo
  CONTAINER split: the R2004-format container (AC1018 + AC1024+)
  sets size/unknown_bits from the whole section, the AC1021
  container never does (gold prints size 0 / '' on R2007 — pinned by
  example_2007 vs sample_2018); (c) the R2004 AppInfo parse of the
  R2007-format content misparses via C-string truncation at the
  first NUL + CHK_OVERFLOW-to-empty (the bogus num_strings length
  overflows every later T16 → gold prints four EMPTY strings —
  silver's t16_pre2007 replicates the semantics exactly); (d)
  `FIELD_T32` expands to `bit_read_TU32` on R2007+ (dec_macros.h:616)
  — the SNIFFING reader whose peek-RL consumption is load-bearing for
  record alignment: an "empty" string still consumes the peek (8
  bytes total), the overflow check (`size + byte >= total`) precedes
  the peek and consumes only the prefix, the UCS-2 branch rewinds
  and reads size/2 RS chars, the 4-byte branch keeps the peek as the
  first char's low half — pinned byte-for-byte by 2018/Arc.dwg's
  FileDepList record (three empty strings consume 8+8+4, the third's
  size RL is 0xFFFFFFFF and overflows). The `props` REPEAT divergence
  is latent-not-corpus: silver's custom_properties serialize as
  tuples vs gold's {tag, value} dicts — no corpus file carries
  custom props (16 leaves × 272 exactly). **`created_by` is EXCLUDED from
  comparison** (the re-analysis: gold hardcodes its own
  `PACKAGE_STRING` there — an oracle identity stamp, not file
  content). **The review's named row RESOLVED: the
  `APP_INFO_HISTORY` registry constant landed with the H4 read —
  the wire name IS "AcDb:AppInfoHistory" (gold's -v4 trace pins the
  map entry; the "located by TYPE, not name" claim was wrong — gold's
  internal lookup is by TYPE 12, but the name is on the wire and
  silver's descriptor lookup works). `ObjFreeSpace`'s per-version
  field shapes (`max32_hi`-style splits on R2010+) landed with the
  version-gated parse.**
- **H5 — the bulk/binary sections** (`CLASSES` landed 2026-09-25 at
  ZERO read gaps corpus-wide; THUMBNAILIMAGE/AcDs open):
  **The CLASSES sub-row: 47,173 gaps → 0** (280/280 files; 60,954
  matched = the full census leaf count; 0 value diffs; 0 missing;
  the read key-gap 234,691 → 187,518). The projection is gold's
  `json_classes_write` shape (out_json.c:1988): number/dxfname/
  cppname/appname/proxyflag/num_instances/is_zombie/item_class_id,
  then dwg_version + maint_version SINCE R_2004a — silver's extra
  fields (is_an_entity/unknown1/unknown2) drop. The numeric fields
  come from the FULL gold-shadow record (`DwgClass::gold_shadow`:
  `DwgClassGoldShadow`, replacing the item_class_id-only shadow) —
  on the desynced 2027.1 tables the garbage values ARE the
  projection's payload, matching gold's own JSON record-for-record.
  **Three print/wire semantics pinned by the landing:** (a)
  `BITCODE_BS` is uint16_t (include/dwg.h:120) — every BS field
  prints unsigned (number 36108, never −29428) and the
  dwg_version/maint_version BS reads stored into gold's BITCODE_BL
  uint32 struct fields ZERO-extend (ExtrudeM record 19: gold 32970,
  not the sign-extended 4294934730 — the first draft's sign-extension
  guess was wrong, the census caught it in 4 leaf diffs); (b) the
  BL `'11'` degenerate code returns gold's error-branch 256
  (bits.c: `unexpected 2-bit code` + `return 256`) — silver's
  read_bit_long fallback was a graceful 0, now 256 (pinned by
  ExtrudeM records 9/25: gold's CLASSES prints num_instances 256);
  (c) the BS `'11'` code is the same 256 in both readers (already
  aligned). **The landing exposed and fixed a latent BODY-AXIS
  field-type bug** — see the wire-color finding below.
  **The THUMBNAILIMAGE sub-row (H5c COMPLETE 2026-09-26 — the read
  axis at gold parity, driver + reader sides)**: gold's shape is
  `{size, chain}` (json_thumbnail_write) — the container data
  hex-encoded from exactly 16 bytes past the thumbnail address,
  `size` == the chain byte count. **The previous session's open-item
  record was WRONG in its central claim** — the recorded next step
  (instrument `get_section_buffer`'s failure reason) ran as the
  280-file probe: the AcDb:Preview fetch succeeds on ALL 58 AC1032
  files, and the 17-file class is NOT a fetch failure — those
  containers carry ONLY the 80-byte reserved header block with NO
  image descriptor (count=1, code-1: Arc_2018's whole container =
  126 bytes = [16 start][chain 110]), so `parse_preview` (which
  requires BMP/WMF/PNG) returned None and the reader dropped a
  container gold still prints. **Three findings + fixes landed:**
  (a) gold's tail rules pinned in the C tree, per decoder —
  pre-R2004 searches BOTH sentinels (chain = the mid-section);
  decode_R2007 cuts BOTH (`size = sec − 32`, decode.c "2x
  sentinel"); the rest of the R2004 family KEEPS the whole tail
  (`size = sec − 16`, read_2004_section_preview — even a 16-byte
  end sentinel where the author wrote one: 2018/Arc's 110-byte
  chain byte-pinned ending in the END sentinel). struct_axis's old
  R2004-vs-pre-R2004 split put AC1021 on the wrong side (58
  Box_2007-class files at size = gold+16) — the cut-32 rule was
  added for AC1021 alongside the pre-R2004 set; (b) the
  raw-address container_len (overall-window) read TRUNCATES
  sections whose author undershot the window (2013/RAY:
  container_len(overall) = 1076 vs gold's true 1114-byte section,
  the IEND+CRC+end-sentinel cut off) and returns compressed bytes
  on compressed stores — for R2004+ files the reader now FETCHES THE
  DECOMPRESSED SECTION FIRST (gold's own read path; the raw read
  stays the fallback, and stays primary pre-R2004 where it is
  exact); (c) a fetched container with NO image is now RETAINED
  (`PreviewFormat::Unknown` + empty `data` + the raw bytes) so the
  structure axis projects it — the 17 header-only AC1032 files at
  byte parity (Arc_2018 size 110 chain byte-equal); the writer
  treats empty data exactly like `None` (placeholder preview, the
  H7 write-target row unchanged). **Two per-file exclusions stand
  (H6 rows, not codec gaps):** 2010/Leader.dwg — gold SKIPS its
  AcDb:Preview section by its own guard ("Skip section with max
  decompression size 0x4d800 > 0x4a000", gold -v4) while the
  316KB container silver reads is valid → silver-extra by
  gold-limit; 2000/PolyLine2D.dwg — no thumbnail on either side
  (absent_both, parity). Pre-fix census read rows were inflated by
  the stem-collision-collapsed aggregate (the "279/177" presence
  split the prior session projected from — the per-file sweep is
  the truth: 255 present + 18 missing + 7 example files, of which
  the real mismatch set was the 58 AC1021 valuediffs, the 17
  header-only missings, and the RAY truncation). Post-fix corpus
  aggregate: THUMBNAILIMAGE read **279/279 → 558 matched + 0
  value-diffs + 0 missing** — the read row at ZERO gaps; the read
  key-gap −286 (187,246 → 186,960, exactly the pre-fix 82+204
  leaves). The write-target 442 re-encode leaves remain the H7
  row. Every other key unchanged; the OBJECTS axis 0/0 held across all
  gates.

  **The AcDs sub-row (H5a LANDED 2026-09-26 — the section-level view at
  gold parity)**: silver now parses the decompressed
  AcDb:AcDsPrototype_1b section into the gold JSON shape
  (`json_section_acds` over acds.spec) — `DwgAcDsSummary` on the
  document (`document.dwg_acds`), projected by struct_axis. The wire
  grammar pinned byte-level against the corpus golds: a 56-byte header
  whose `num_segidx` sits at offset 32 (the 9th RL — a first
  implementation misplaced it last and the smokes caught the
  schidx/datidx field swap), the index segment's OWN 48-byte header AT
  `segidx_offset` with the entry table at +48 (the first
  implementation read the table at `segidx_offset` itself and got the
  `AC D5 's' 'e' 'g' 'i' 'd' 'x'` header magic as offsets), then one
  48-byte segment header per non-zero slot + the per-type bodies.
  **Three gold semantics are load-bearing and non-obvious:** (1) the
  type bodies (datidx/schidx/schdat/search) read into gold's TOP-LEVEL
  singletons (`_obj->schdat` etc. in acds.spec — NOT per-segment
  storage): a later same-typed segment OVERWRITES an earlier one and
  every segment of the type prints the LAST-read block (pinned by
  Revolve_2018's two schdat segments both printing the second's
  {size: 8, flags: 0} while the first's own wire carries {8, 1});
  (2) the emission is UNCONDITIONAL on the R2004+ arm (out_json.c:2675)
  — a file WITHOUT the section still prints the zeroed 13-field header
  (2004/Line pinned: no AcDs section in the file at all, gold prints
  all zeros; the reader emits the zeroed summary on a failed fetch for
  R2004+ and stays `None` on the R2000 family, absent on both sides);
  (3) REPEAT counts are suppressed in gold's JSON (`num_segidx`,
  `datidx.num_entries`, …) while VECTOR fields print even empty
  (`sortedidx: []`, the inner `ididx: []`) and empty REPEAT keys are
  omitted entirely — a 256-file corpus union of gold's AcDs blocks
  pins the 52-path key set, with example_2013 the only file
  exercising the deep ididxs grammar (RLL handles + ididx vectors).
  **The R2004 nameless-descriptor fix:** the corpus R2004 files' AcDs
  sections carry an EMPTY 64-byte name field in the section map —
  silver dropped nameless descriptors, so its fetch missed them; they
  now resolve by the section TYPE id (gold's DWG_SECTION_TYPE order,
  17 = AcDb:AcDsPrototype_1b — the stale "= 12" comment in gold's
  header refers to AppInfoHistory's slot). **Post-fix verification (the
  per-file truth sweep, stem-collision-free): 256 files with gold AcDs
  → 46,730 gold leaves, ALL matched — 0 value-diffs, 0 missing, 0
  silver-extra** (the H0 census's "58,471" was the
  collision-collapsed aggregate; the true gold-leaf total is 46,730
  over the 256 non-R2000 files, the 24 R2000-family files absent on
  both sides). A first cut printed empty `segidx: []`/`segments: []`
  keys on the 138 header-only files (gold omits empty REPEAT keys) —
  skip-if-empty guards on the two arrays close it. **The full corpus
  re-ran clean: AcDs 273/273 → 58,471 matched + 0 value-diffs + 0
  missing in the aggregate census (the collision-inflated leaf total
  — every per-stem row matched); the read key-gap fell 186,960 →
  128,489, now EXACTLY the HEADER row alone; THUMBNAILIMAGE held
  558+0+0; every other key unchanged.** OBJECTS 0/0 held across all
  gates. VBAProject/Signature land as excluded rows with
  reasons in H6.

  **The wire-color field-type fix (the H5b landing's body-axis
  corollary, 2026-09-25):** the BL-'11' fallback change made silver
  parse a wire color that had silently been read as 0 — and the
  corpus's WRITE axis caught it: 96 diffs (3DSOLID.history_id +
  acis_empty_bit on the 46 §18 solid files). Root cause:
  `WIRESTRUCT_fields` reads the wire color via `FIELD_CAST (color,
  BS, BL, 0)`, and the decoder's FIELD_CAST reads with the TYPE
  (dec_macros.h:121: `(BITCODE_##cast)bit_read_##type`) — a **BS**
  read stored into the BL struct field; the encoder writes BS
  (enc_macros.h:69). Silver's §18 reader/writer had misread the cast
  as a version gate (BL on R2004+, BS pre-2004). The original
  Cone_2007's wires[0].color BS is the `'11'` code → gold 256
  (ByLayer); silver's BL read gave 0 (ByBlock) — invisible on the
  read axis (the census's wires projection collapses to [0,0,0,0])
  and harmless while the value was 0 (the writer's BL 0 = '10' =
  the BS 0 encoding). The BL-'11' fix made the value 256 — and the
  writer's BL 256 ('00'+RL32, 34 bits) vs gold's BS re-read ('00' →
  RS16, 18 bits) diverged the rewrite's wire record by 2 bytes,
  desyncing every downstream field (the 96 diffs). The fix: the wire
  color is BS on BOTH sides unconditionally (read_bit_short /
  write_bit_short — whose 256 encoding is the '11' code, gold's own
  writer's `bit_write_BS` branch). The corpus returned to 0/0 both
  axes; the read-axis semantics IMPROVED (silver now reads the
  ByLayer wires correctly).
- **H6 — the whole-structure audit matrix (the standing deliverable)**:
  a generated table — every DWG section name × per-version presence ×
  gold JSON emission × silver model coverage × axis status
  (`diffed 0` / `excluded` + the recorded reason). Pre-populated
  excluded rows from the review + the tree re-analysis: the R2004+
  section-map/page-map ENTRIES and the container types
  `SECTION_INFO`/`SECTION_SYSTEM_MAP` (transitively verified by every
  section read succeeding + layer-4 byte-walks), the R2000 "unknown"
  section (locator nr 3/4; content never JSON-emitted), per-section
  CRCs/sentinels/0x16 padding,
  `created_by` (the oracle's own PACKAGE_STRING stamp — not file
  content), `Signature` (readable by gold but its JSON emitter is
  deliberately disabled, `out_json.c:2673` — reason "not emitted",
  distinct from corpus-absent), `VBAProject` (corpus-absent),
  2010/Leader.dwg's THUMBNAILIMAGE (a per-file row: gold SKIPS its
  AcDb:Preview section by its own decompression-size guard
  — `max decompression size 0x4d800 > 0x4a000`, gold -v4 — while
  the valid 316KB container silver reads censuses as silver-extra;
  a gold read-limit, not file content), and the
  object map / Handles section (gold's `json_handles_write` is
  `#if 0`'d — deliberately unemitted; covered transitively by the
  OBJECTS axis' handle resolution + layer 4). The matrix also audits
  silver's registry membership per section name (the
  `ALL_SECTION_NAMES` omissions the review found). A probe run
  refreshes it; it goes in every future halt report. **This is the
  row that answers "the entire file structure is gold-vs-silver
  tested" with a table, not an assertion.**
- **H7 — the writer's structure byte-fidelity (the write-target axis,
  21,822 at day one)**: **The HEADER sub-row (H7a) LANDED 2026-09-26 —
  137,753+6,366+268 → 138,021 matched + 0 value-diffs + 0 missing**
  (the write-target key-gap 21,822 → 15,186; OBJECTS 0/0 and the read
  key-gap 0 held across every gate). The design — the Phase B write
  rule applied to the header: `write_header_with_encoding_opt` splices
  the H3 raw mirror (`DwgHeaderRaw`) into every slot the MODEL does not
  carry (the unmodeled fields re-emit their wire values verbatim
  instead of writer defaults; `raw = None` keeps the historical
  default-constant behaviour for programmatic documents), and the
  MODELED slots keep the model path — the `prepare_header` handle
  syncs stay authoritative (load-bearing: they repair the R2007+
  null-handle reads). **The write-target families the landing
  pinned:** (a) the unmodeled slots were hardcoded defaults — the
  unit ratios/names, unknown_8..unknown_23, the trailing
  unknown_54-57 (65535s), TSTACK pair, PELLIPSE/UNITMODE, the R2007+
  block (DIMFXL/DIMJOGANG at the π/4-rounded 0.7854, DIMTFILL(+CLR),
  DIMARCSYM, DIMFXLON, the R2010+ MZF/MZS pair, _3DDWFPREC,
  PSOLWIDTH/PSOLHEIGHT at 0.25, LIGHTGLYPHDISPLAY, TILEMODELIGHTSYNCH,
  DWFFRAME/DGNFRAME, REALWORLDSCALE, CSHADOW), the raw-only handles
  (PUCSNAME/PUCSBASE/UCSNAME/UCSBASE, the ortho-origin 3BDs,
  DIMLDRBLK/DIMBLK/1/2, CPSNID, unknown_20, INTERFEREOBJVS/VPVS,
  DRAGVS) and the raw-only CMCs — all splice `raw.field` with the
  wire-bit-exact casts (`as u16 as i16` for the BS patterns, `as u32
  as i32` for BL); (b) **the INTERFERECOLOR writer bug** — the slot
  was written from `h.intersection_color` (a copy-paste: the
  interference color emitted the intersection color); now the raw CMC
  (a `write_cm_color_raw` that re-emits index/rgb/flag/names verbatim)
  with `Color::None` as the programmatic default; (c) **the EXTMIN/
  EXTMAX recompute** — `prepare_header` overwrote the model extents
  with silver's own computed bounds (860276 → 8208421424 on the
  examples — the recompute produces wildly different values than the
  author's saved extents); the overwrite is now gated on
  `dwg_header_raw.is_none()` (preservation for read documents, the
  "Zoom Extents" behaviour kept for programmatic ones); (d) **the
  timespan/julian f64 roundtrip truncated 1 ms per span**
  (TDINDWG/TDUSRTIMER: 113000 → 112999 — `days + ms/86400000.0` ×
  86400000 lands at .9999… and `as i32` floors); the converters now
  round (`.round().clamp(0, 86_399_999)`), keeping the model path for
  edits; (e) **the HANDSEED correction overrode the author's seed**
  (2000/PolyLine2D: the file's own quirk HANDSEED 975 < max handle
  978 — gold writes it back unchanged; silver's `max+1` correction
  produced 979); the correction is gated on raw absence — entity
  additions still grow the seed through the document API's own bump
  (document.rs `next_handle`), so read+edit+write stays correct.
  **Verification:** the six-version probes at 0 write-target HEADER
  diffs; the sh_history smokes 0/0 with the per-fixture write-target
  key-gap down (66→41, 82→60, 75→51, 99→71 — the remainder is the
  other H7 rows); cargo test 1588/0; the corpus 280 @ 0/0. **The
  remaining H7 rows:** **The SummaryInfo sub-row (H7b) LANDED
  2026-09-26 — 4,352+1,533+0 → 4,352 matched + 0 value-diffs + 0
  missing** (the writer's `build_summary_info` took only the version
  and emitted the static all-empty block — strings, the three TIMERLL
  pairs, the custom-property count, the trailing longs all zeroed; it
  now writes the document's `summary_info` — the H4 read's 16-field
  model — with the exact wire grammar: the 8 fixed strings in order
  (Title, Subject, Author, Keywords, Comments, LastSavedBy,
  RevisionNumber, HyperlinkBase) as u16-count-prefixed Windows-1252
  on AC1018 / UTF-16LE on AC1021+, the `[days, ms]` u32 pairs for
  TDINDWG/TDCREATE/TDUPDATE, the u16 property count + the (tag,
  value) pairs, then the two trailing u32s; a default document emits
  the   historical empty block byte-for-byte). **The H7a/H7b review pass
  (2026-09-26, post-landing):** the re-derivations held —
  `write_cm_color_raw` mirrors `bit_read_CMC` exactly (the re-read
  reproduces the post-decode state: the index recomputed from rgb by
  the palette, the flag<4 name gating symmetrical, the 0xC2 method
  fixup idempotent); the SummaryInfo grammar matches
  `summaryinfo.spec` + `bit_read_T16`/`bit_read_TU16` — the u16
  count includes the null, and the AUTHOR's byte form is
  `RS(1)+null` for empty strings on both AC1018 (cp1252) and AC1024
  (UTF-16; pinned by dumping the originals: `01 00 00` / `01 00 00
  00`), which silver matches byte-for-byte (gold's own encoder would
  write `RS(0)` for an empty T16 — a gold-encoder quirk, not the
  author's form; both decode to `""` so the census is blind to the
  difference); the raw-handle analysis found every corpus header
  handle already in the canonical form (`HardPointer.code() == 5`,
  minimal sizes — the recomputed splice was byte-identical
  everywhere). **Three fixes landed:** (a) **the raw handle forms**
  — the raw-only slots re-emit the retained `[code, size, value]`
  tuple verbatim (`write_handle_form` on the bit/merged writers, the
  `read_handle_raw` mirror: big-endian leading-zero payload bytes
  re-materialize from the value, the size clamp matches the
  reader's own `min(size, 8)` corrupt-data guard) instead of a
  recomputed HardPointer/minimal form — corpus-dead (0 byte deltas
  on every probe), future-proof for authored non-canonical forms;
  (b) **the three undocumented trailing slots after unknown_57 (BL,
  BL, B) were read-discarded and re-written as 0/0/false — a LIVE
  census-blind corruption**: the corpus sweep found **217 of 243
  probed files carry nonzero values** (a constant `0xec99a4aa` word +
  a per-file word on the R2004+ family — sample_2018
  `ec99a4aa`/`40046004`, gh209_1 `ec99a4aa`/`38003`; example_2004's
  tail is `0/0/true`). Gold emits nothing there (the census is
  structurally blind), so the halt ledger never saw the damage; the
  slots are now retained (`unknown_tail_long1/long2/bit`,
  serde-skipped — no gold-JSON counterpart) and spliced — the
  re-emission is byte-exact vs the author (the R2007+ merged main
  stream grows +64 bits where the halt's compact-zero BL encoding
  becomes the raw 32-bit codes; example_2004's final data bit
  `B=true` is restored — the halt rt wrote `0x34` where the author
  wrote `0x35`); (c) **the SummaryInfo presence coupling** — gold
  gates its JSON key on `summaryinfo_address != 0` (out_json.c:2663);
  gh209_1 (AC1024, address 0, the author wrote no section) exposed
  the blind family on both axes: the read census carried a
  `missing_gold` silver-extra row (16 leaves) and the rewrite
  materialized a section gold never had (rt address 4448, key
  present). Fixed: struct_axis projects SummaryInfo only when the
  silver FILEHEADER's `summaryinfo_address` is set (mirrors gold's
  own gate; also covers the 7 R2000-family files), and both writers
  (ac18/ac21) skip the section when the read document's address was
  0 and the model still holds the default — a programmatically
  modified summary still writes the section (user intent wins over
  roundtrip presence parity); the file headers derive the address
  from the descriptor registry, so the skip yields address 0
  automatically. Census effect on gh209_1: FILEHEADER vd 6→5 (the
  rt address now matches the author's 0), SummaryInfo
  missing_gold→absent_both, R2004_Header vd 5→9 — the
  numsections/section-id coincidence: gold's own table carries id
  gaps (15 sections but last_section_id 17); the halt rt's
  17-section table landed last_section_id 17 by coincidence, the
  skip's 16 sections end at id 16 — 4 more counted diffs inside an
  already-open row whose fix (the author's ids/addresses) re-solves
  those fields. **The census asymmetry pinned for the record:** the
  write-target key-gap counts value_diffs + missing_gold_rt;
  missing_gold (rt-extra) leaves are a blind family by design (the
  read axis's projection-noise tolerance) — presence-coupling rows
  live there. Verification: cargo test 1588/0; the six-version
  probes 0/0 with UNCHANGED per-file write-target key-gaps
  (34/31/35); the smokes identical (79/70/68); the corpus 280 @ 0/0
  with the write-target key-gap 13,653 → 13,656 (gh209_1's +3
  id-coincidence, documented above). **The CLASSES sub-row (H7c)
  LANDED 2026-09-26 — 60,954 matched + 0 value-diffs + 0 missing
  (the write-target key-gap 13,656 → 10,410)**: the row was TWO
  families with one root. (a) The authored class tables whose tail
  encoding desyncs gold's walk (the AutoCAD-2027.1 fixture set)
  can only round-trip byte-exactly — any re-encoding desyncs the
  walk DIFFERENTLY (LoftC_2007's record [10] went gold 2147673665
  vs rt 587458113, with the version/maint/proxyflag cascade);
  (b) the classes whose instances re-emit through the raw-object
  passthrough are OUTSIDE the write census
  (`class_counts_complete=false` → 0 counts where the author
  wrote 1/3, and the zombie re-derivation marks live classes
  zombie — the ATMOS-DC22S/Cone families). The fix follows the
  AcDs precedent: the reader retains the raw (decompressed)
  classes section bytes (`raw_classes_data`) plus a state hash
  (`raw_classes_fingerprint` — the ordered class identity tuple
  + the document's per-class object census, computed by the
  shared `classes_state_fingerprint` at the read capture and
  again at the write gate; the census mirrors the writer's
  required-classes walk over entities + class objects, counting
  the raw-passthrough records the write census cannot see); a
  same-version roundtrip whose class table and census are
  unchanged re-emits the author's bytes verbatim (all three
  writer paths — ac15/ac18/ac21), any class-table or census
  change or version conversion falls back to the sane encoding.
  The gold-shadow record stays the READ projection authority
  (H5b); the WRITE authority is the raw section bytes.
  Verification: the three family probes at CLASSES 0
  (LoftC_2007 110→0, ATMOS-DC22S 18→0, example_2004 8→0 — every
  other row unchanged); the smokes 79→43 / 70→35 / 68→33 (their
  CLASSES rows); cargo test 1588/0; the corpus 280 @ 0/0, read
  key-gap 0, write-target key-gap 10,410 with CLASSES 0 across
  all 280. **The AppInfo/AppInfoHistory sub-rows (H7d) LANDED
  2026-09-26 — AppInfo 2,396 @ 0/0 and AppInfoHistory 546 @ 0/0
  (the write-target key-gap 10,410 → 8,718)**: gold prints both
  sections unconditionally on R2004+ (out_json.c:2668-2669 —
  zeroed when absent), so the rows carried three families:
  silver's static boilerplate AppInfo where the author carried real
  content (the version strings, checksums, comments), the
  AppInfoHistory section NEVER written (gold_rt printed the zeroed
  struct against the author's bytes), and the absent-source files
  where the boilerplate materialized a section gold never had. The
  fix follows the presence-coupling + verbatim doctrine: the
  reader retains both sections' raw bytes (`raw_app_info_data` /
  `raw_app_info_history_data`); a same-version roundtrip re-emits
  them verbatim, a source without a section gets none
  materialized (gold's zeroed print lands on both sides), and
  programmatic documents / version conversions keep the historical
  boilerplate. The AppInfoHistory hash for the AC21 section map
  (0x96de0737, constant across the corpus) was extracted from the
  author's files and added to the ac21 tables, with the author's
  single-page align-0x80 form. **Three container bugs found and
  fixed on the way — all census-masked by luck until the verbatim
  sections exposed them:** (a) the AC18 page header's 0x0C
  "page size - decompressed" field carried the stored FRAME size
  (content+32) instead of the page's decompressed capacity —
  gold's reassembly copies MIN(section-remaining, page_size)
  bytes per uncompressed page (decode.c:2236), so the frame-sized
  field overread 32 header bytes into the content and drove the
  bytes_left balance negative, erroring the whole section on any
  page parity that lands the negative balance before the last
  page (the verbatim AppInfo at 6×0x80 pages hit it; the
  historical small sections survived only by parity luck);
  (b) the AC21 encoding-1 (stored) pages carried a
  non-interleaved [data][parity] layout only silver's own reader
  understands — gold picks its decode path by the page's physical
  size (decode_r2007.c:854: the RS path iff page->size ==
  page_size_if_rs_coded(comp) = align32(ceil(align8(comp)/251)×255))
  and de-interleaves with stride ceil(align8(comp)/251); any
  stored page whose physical size lands on the RS form gets
  de-interleaved into garbage (the 1502-byte AppInfoHistory at
  align32 1536 == the 6-block RS form 1536 hit it exactly). The
  author's encoding-1 pages are RAW content + padding (their
  1408/1536/1792-byte pages never match the RS form) — silver now
  writes the same raw form, with a +0x20 collision guard for the
  content sizes where align32(content) would land on the RS form
  (the [225..248]-byte contents hit the 1-block form 0x100);
  (c) the AC18 add_section SKIPPED all-zero tail pages — gold's
  reassembly guard rejects the whole section when its size field
  exceeds num_pages × max_decomp_size (gh209_1's AppInfoHistory:
  642 = 5×128 + 2 zero bytes → the dropped tail page shorted the
  section to 640). The tail page is now always written. Census
  couplings absorbed by the open rows: the AppInfoHistory add
  moves the AC18 numsections/section-id fields (the gh209_1
  precedent — R2004_Header moved 1,775 → 2,088: the id-coincidence
  lottery, its fix rewrites the author's section table anyway);
  the raw-page layout moved R2007_Header 978 → 966. Verification:
  the four family probes at AppInfo/AppInfoHistory 0 on both
  writer paths (example_2004/Box_2010/Box_2007/Arc + gh209_1's
  tail-page case); cargo test 1587/0 (the two legacy-layout
  encoding-1 tests replaced by the raw-form + collision-guard +
  interleaved-roundtrip pins); the corpus 280 files at 0/0,
  structure read key-gap 0, write-target key-gap 10,410 → 8,718
  with AppInfo/AppInfoHistory 0 across all 273 R2004+ files.
  **Landed since (2026-09-26):** H7e — the ObjFreeSpace verbatim
  row at ZERO (2,453 + 30 → 0; corpus write-target 10,410 → 6,235
  counting H7d's baseline): the reader retains the raw section
  bytes (`raw_obj_free_space_data`) on both arms (the decompressed
  container section on R2004+, the raw 53-byte locator section on
  R2000 — clean content; the 2ndheader sentinel follows the section
  end, outside the retained span); all three writer paths re-emit
  the author's bytes verbatim on a same-version roundtrip, write a
  NUL locator record (seeker 0 — the author's own absent form, cf.
  PolyLine2D's Template nr4 at address 0) when the source carried
  no section, and keep the historical rebuild for programmatic
  documents and conversions. THE R2000 PLACEMENT FIX: gold reads
  the section only when its locator address equals the file
  position DIRECTLY after the handles map (decode.c's
  `section[OBJFREESPACE].address == pvz` gate, the post-terminator
  CRC position) — the AC15 writer now places ObjFreeSpace AFTER
  the HANDLES record (the R13/R14 branch's own order), which closed
  the 3 R2000 presence rows (PolyLine2D/entities-2d/entities-3d,
  10 leaves each) without touching the parallel session's
  SecondHeader row; a never-populated record keeps the NUL form so
  it never points at live bytes. The corpus: ObjFreeSpace 3,441
  matched 0/0 both axes; every other row unchanged.

  H7f — the FILEHEADER identity bytes: 1,204 → 553 (−651). The six
  identity bytes (maint_rel, dwg_version, maint_version, the 0x15
  unknown byte, the app pair) were stamped with per-writer constants
  (AC18: the canonical per-version table at 0x0B/0x12, dwg_version
  hardcoded 0x21, the app pair + the unknown byte zeroed; AC21:
  0x19/0x1B/0x19/30 + zeros; AC15: the fixed 0x1B/0x19 pair) where
  the authors wrote per-build bytes
  (example_2004's maint_rel 104 / maint 29 / app pair 33/29;
  Revolve_2018's maint 0 vs the canonical 4; Box_2007's
  maint_rel 50 / dwg_version 33 / maint 255 / app pair 33/255;
  PolyLine2D's 31/8 version pair). All three file-header writers take
  a same-version source mirror from the retained
  `DwgFileHeaderSummary` (`set_source_header_bytes` /
  `set_source_version_pair`), AC21's codepage included (previously
  hardcoded 30; the 2026-09-26 review pass extended the mirror
  with the 0x15 unknown byte — the writers hardcoded it to 0 while
  the summary retains the author's; corpus-blind, every author 0).
  The effective layout maint in write_ac18 becomes
  the author's `maint_version` byte: LAYOUT-NEUTRAL for every
  corpus class (the R2004 header-vars path reads no bitsize_hi at
  all — gold's PRE(R_2007a) arm; the R2010/2013 authors' bytes are
  all > 3 like the canonical 30/29 so `has_section_extra_rl` keeps
  the extra RL present; R2018's gate carries the `|| >= R2018`
  arm; R2007 has no maint-gated arms in its reader path). The
  residual 553 = the pure address-shift family (thumbnail_address +
  summaryinfo_address leaf pairs on R2004+, the preview seeker on
  R2000) — layout-coupled, not byte-identity. The generation
  identity is untouched (programmatic docs keep the historical
  constants byte-for-byte; the current identity 40ab5d356cf05a71333f
  208e1651daf, 25,344 bytes — it moved at an earlier landing
  without a re-record; the README's recorded 0217fbac… was stale).

  H7g — the container-shape mirror (2026-09-26): the write-target
  axis 5,584 → 3,577 (−2,007: R2004_Header 2,088 → 963, FILEHEADER
  553 → 104, THUMBNAILIMAGE 442 → 50, R2007_Header 966 → 925 via
  the AC21 random_seed mirror, −41 = one per AC1021 file). The
  landing closes one root across three rows: `numsections` (inner
  header @0x40) IS the page-map entry count, the four id fields
  (@0x28 `last_section_id`, @0x50 `section_map_id`, @0x5C
  `section_info_id`, @0x60 `section_array_size`) follow the page
  space, and the FILEHEADER addresses are page-data positions — so
  a rewrite that reproduces the author's PAGE SPACE also reproduces
  every count, id and (with the author's emission order) the
  summary/preview prefix addresses and the preview chain's embedded
  absolute image offsets. Three author conventions were measured
  and mirrored: (a) the per-descriptor maxdecomp page splits —
  the authors single-page the metadata sections at custom caps
  (example_2004: AppInfo 0x300, AppInfoHistory 0x580, Preview
  0x7C00, SummaryInfo 0x80) where the historical writer used the
  0x80 SMALL_PAGE and 0x7400 conventions — the author's tail page
  pads to the full maxdecomp frame (AppInfoHistory content 0x510 →
  0x580 frame, on-disk 1,440; our tail pages already padded to the
  cap, so adopting the author's maxdecomp reproduces the frames);
  (b) the box-id gap — the authors allocate the section-info and
  page-map box pages at data_count+3/+4 (ids 27/28 with 25/26
  unused, numsections 26 = data+2 counting only the entries) where
  the historical writer used +1/+2; (c) the physical order — the
  corpus authors lay the SummaryInfo page first at 0x100 (data at
  0x120 = `summaryinfo_address` 288) and the preview page right
  after at 0x1a0 (data at 0x1C0 = `thumbnail_address` 448, the
  preview container's image descriptors hold those same absolute
  offsets — pinned by the preview.rs verifier comment), where the
  historical writer emitted header-first. The mirror:
  the reader retains `DwgAc18ContainerShape` (per-descriptor
  [mapped + raw 64-byte name, content size, maxdecomp, compression
  code, (page id, start-offset) list], the page map's entries in
  physical order, the three identity ids) on the document
  (`dwg_ac18_shape`, serde-skipped; AC18-family files only);
  `write_ac18` builds every section buffer up front (the builders
  are pure, so the conventional path stays byte-identical), runs
  `ac18_mirror_plan` — the content-parity gate: same-origin,
  numgaps 0 (gap map entries are not reproducible), coherent
  retained ids, the four core sections present, per-section
  parity (our re-encoded content covers the author's last page
  offset within her max-decomp capacity, ascending offsets, code
  ∈ {1,2}, preview single-paged, our skipped sections must be the
  author's 0-page ones), the entry count == the author's
  numsections, and a physical-order walk with contiguous section
  pages and the boxes last — and emits through
  `add_section_shaped` (OUR content chunked at the AUTHOR's
  per-page offsets, every page carrying the author's page id, the
  tail padded to her frame, the raw name bytes into the descriptor
  table, `set_mirror_ids` driving 0x50/0x5C/0x60 and the two box
  page ids) in the author's physical order; the 0-page descriptors
  (the unnamed AcDs) emit descriptor-table entries only. Two hard
  edges found in review: gold's descriptor parser bounds-checks
  `dec.byte + 8 + 6*4 + 64 >= dec.size` BEFORE every entry
  (decode.c read_R2004_section_info "out of range") — a mirrored
  table that ends with a 0-page descriptor exactly consumes the
  stream and fails it (the historical tables always ended with a
  pages-bearing section whose own page records were the slack), so
  the mirrored table stream carries +32 bytes of tail slack; and
  the preview re-emits RETAINED-RAW when its actual landing address
  equals the retained `thumbnail_address` (the prefix pages are
  size-faithful: the verbatim summary content keeps the author's
  on-disk page size), else the container is rebuilt around OUR
  address (honest bytes; the row keeps its diff on that file). One
  presence addition: `AcDb:XrefManifest` — the R2013+ authoring
  tables carry it (Box_2013/Revolve_2018-class fixtures), it is
  unmodeled and UNPRINTED by gold (no census row; the read axis
  stays 0 with it absent), but its page is part of the author's
  page space — retained raw (`raw_xref_manifest_data`, the
  AppInfo/ObjFreeSpace doctrine) and emitted mirror-only. The
  225 engaged files close the five count fields, both FILEHEADER
  addresses and the THUMBNAILIMAGE size+chain, keeping exactly the
  irreducible residue (last_section_address, secondheader_address,
  section_map_address, crc32 — file-tail positions and the system
  CRC move with OUR content sizes; 4 per engaged file = 900). The
  SEVEN declined files fall back byte-identically (stash-verified
  on 2004/Arc and 2013/Arc: the read-gap 2 and write 14/29 are the
  halt-state values): the numgaps>0 authors (2013/Arc/Line/RAY —
  gap map entries plus the author's negative tree-node-gap fields,
  an ODA convention the fallback conventionally zeroes), the
  objects-overflow files (2004/2010/2013 Constraints, 2018
  Dynblocks: our re-encoded objects exceed the author's page space
  — `len − last_offset > maxdecomp`, where forcing the fit would
  trip gold's reassembly guard), and 2018/Leader (the author's
  real FileDepList content vs our 8-byte boilerplate — the
  PARALLEL session's row). gh109_1 ENDED the landing with exactly one
  FILEHEADER leaf — and this is where the review pass earned its keep:
  the leaf is NOT an address but `codepage: gold=39 vs
  gold_rt=31`. Once the mirror closed gh109_1's thumbnail and summary
  address pairs (both matched even at the halt — the author carries
  no preview), the surviving leaf exposed a PRE-EXISTING conversion
  bug in the code-page tables: `dwg_code_page_name` mapped BOTH
  bytes 31 and 39 to "GB2312" (and 22|38, 24|41, 25|40, 26|42 the
  same way for the 932/949/950/1361 pairs) while
  `dwg_code_page_index` folded the names back onto the low byte —
  gold's enum (codepages.h) pins them apart: 31 = CP_GB2312 (EUC-CN)
  where 39 = CP_ANSI_936 (CP936/GBK), 22 DOS932 vs 38 ANSI_932,
  24 BIG5 vs 41 ANSI_950, 25 CP949-"korean" vs 40 ANSI_949,
  26 JOHAB vs 42 ANSI_1361 — so the byte → model-string → byte
  roundtrip lost the author's distinction (a byte-39 file re-emitted
  byte 31, and the rt's claimed codepage DISAGREED with the very
  strings its writers had encoded). The review fix: the five
  dual-byte arms split in both tables, the codec families kept
  (the pairs share byte-compatible low halves: GBK encodes both
  2312/936), a roundtrip regression test
  (`test_codepage_index_name_roundtrip`) added, and the issue55 GBK
  test's name expectation updated to the now-identity-preserving
  "ANSI_936" (byte-31 files keep "GB2312"). gh109_1's FILEHEADER is
  0 now; the write-target 3,577 → 3,576; the generation identity
  re-verified unchanged (programmatic docs write byte 30's family).
  The AC21 companion landing: `random_seed` mirrored from the retained
  `DwgR2007SystemHeader` (gated on the author's crc_seed == our 0)
  — decode-inert (gold only prints it), and it IS the CRC random
  encoder's seed (spec §5.2.1.1.1), so the hope was the derived
  draws (`sections_map_crc_seed`, `pages_map_crc_seed`,
  `crc_seed_encoded`) would land the author's values for free;
  they do not — our CrcRandomEncoder's sequence diverges from the
  author's engine at the same seed+crc_seed (measured on
  example_2007: the first draw already differs), so the 3-per-file
  derive family stays open pending the author's MT-variant pinned
  bit-exactly (a §5.11 instrument). The generation identity is
  UNTOUCHED (40ab5d356cf05a71333ff208e1651daf, 25,344 bytes —
  re-verified twice after the landing: programmatic documents keep
  the conventional path). Verification surfaces: the −v9 layer-4 map
  comparison (example_2004: pages 1–5 byte-exact at the author's
  addresses — summary 160@0x100, preview 31,776@0x1a0, the
  verbatim AppInfo 800@0x7dc0, AppInfoHistory 1,440@0x80e0,
  RevHistory 192@0x8680 — objects 13 pages both sides, the boxes
  at 27/28), the AC18_MIRROR_DEBUG decline trace (the seven
  fallbacks enumerate their exact reason per file), and the
  stash-check proving the fallback byte-identity.

  **Still open (post-H7g review, 3,576) — the wall's remaining
  faces:** the AC18 residue is exactly the irreducible address/CRC
  family (963 = 4 × 225 engaged + the 7 fallback files at the
  historical 9); FILEHEADER 103 (= AC1021 82 + R2000 7 + fallback
  14) and THUMBNAILIMAGE 50 close on
  the AC1021 side only through an AC21 container mirror (the
  RS-chunk page space) or, for the R2000 seeker pair, through
  flat-layout parity — both follow the AC18 analysis template.
  R2007_Header 925: random_seed closed (−41); the remaining 22-per-
  file are the layout/content-coupled families (offsets, sizes, the
  six CRCs, corrections) plus the 3-per-file crc-seed derive family
  (the author's MT-variant) and the page-space pair
  (pages_amount/pages_maxid + the four map-id fields — the AC21
  page-space mirror). The whole-section write drops FileDepList
  1,055 + SecondHeader 386 + AuxHeader 94 stay the PARALLEL
  SESSION's rows (their untracked probes `h7_probe1.sh`/
  `h7_rows.sh` were removed at the 2026-09-27 housekeeping, on the
  maintainer's instruction — the rows they probed had all closed
  with the H8e–H8g echo landings; note the
  FileDepList CONTENT class — 2004/Arc (2) through gh109_1 (75) —
  declines the container mirror on the same missing-content root:
  closing the parallel row re-engages those mirrors for free).

  **H8a — the AC21 container reader retention (2026-09-26, the
  post-mirror-halt follow-up session)**: `DwgAc21ContainerShape`
  (`document.dwg_ac21_shape`, serde-skipped) now retains the
  author's AC1021 page space — the pages-map entries in her
  PHYSICAL order (`map_order`: (id, on-disk size); gold's
  `read_pages_map` accumulates the running offsets from 0x480 in
  exactly this order) plus her sections table with per-page plans
  (`sections`: name, encoding, data_size, and per page the
  decompressed-stream offset, the declared frame field, the
  pages-map id and the page's physical extent, resolved from
  `page_records` in `read_file_header_ac21` after both maps parse;
  the store into the document is the transfer-time block beside
  `dwg_r2007_header`, gated on `info.ac21_metadata.is_some()`).
  Read-inert by design: the censuses are untouched (write-target
  3,576 unchanged; example_2007 and Box_2007 smokes hold 0/0 at
  write-target 25). The H8b (write mirror) contract, pinned from
  gold's own decoders this session (decode_r2007.c:700-852 +
  :692): a compressed page (comp != uncomp) is ALWAYS
  RS-de-interleaved — block_count = ceil(align8(comp)/251),
  block_count×255 must fit the page's on-disk size, the trailing
  pad is ignored; a stored page is raw-memcpy'd only when
  page->size != rs_form(comp) (rs_form(x) = align32(ceil(align8(x)/
  251)×255)). The per-page mirror emission: our chunk LZ77'd at her
  boundaries — comp < chunk → emit the RS form of our stream
  (block_count×255 ≤ her page size, else DECLINE) zero-padded to
  her exact size; stored → if rs_form(chunk_len) collides with her
  size, emit the RS form (declared comp == uncomp — gold takes the
  same RS path and reads our payload), else the raw chunk padded to
  her size. Sequential emission from 0x480 in her map order
  reproduces every offset (a running sum — no seeks) and the
  pages-map bytes go IDENTICAL (same (size, id) pairs, so its
  comp/uncomp sizes, both CRCs and the correction factor land hers
  for free, ~12 fields/file); the sections-map system pages (her
  sections_map_id/2_id at the tail, her on-disk sizes) hold OUR
  honest map bytes (their CRC/size/correction rows stay). The
  0x80-block addresses are retained identity: stamp the author's
  `summaryinfo_address`/`thumbnail_address` from
  `dwg_file_header_summary` verbatim under the mirror (her
  convention, observed on example_2007: the AcDb:Header page's
  offset and the AcDb:Preview page's offset — under the mirror
  they stay valid positions). Predicted closes: FILEHEADER AC1021
  82, THUMBNAILIMAGE 50, and the R2007_Header pages-map family
  (pages_amount/pages_maxid, the four map-id fields,
  pages_map_offset/map2_offset, the pages-map size/CRC pairs,
  pages_map_correction); the sections-map family, the six file
  CRCs, file_size/header2_offset content-coupled rows and the
  3-per-file MT-derive draws stay open. Decline classes mirror
  the AC18 ones: objects-overflow (rs_form(our comp) over her page
  extent) and the missing-content class (our len under her
  last-page offset — the FileDepList boilerplate family; the
  parallel session's content row re-engages those mirrors for
  free). AC21_MIRROR_DEBUG carries the per-file decline trace.
  Machine-verified model case (`example_2007`): boxes 25/26 at
  0x480/0x880 (1024 each, corrections 7), data pages 3..20 in id
  order (header-vars 160 @0xc80, preview 307,936 @0xd20 comp
  303,104 uncomp 302,255, summary 1440 @0x4c320, …), the objects
  section at 7 pages with irregular capacities (63,488/58,624/
  70,912/65,536/63,488/69,120/remainder), sections-map boxes 21/22
  (4864 each, correction 4) at the tail, header2 at 453,824
  (relative), file_size 456,000, pages_amount 22, pages_maxid 26
  (ids 23/24 unused), sections 13 with [12] the unnamed 0-page
  descriptor (gold's "Invalid num_pages 0, skip" — the AC18
  0-page edge again).

  **H8b — the AC21 container-shape write mirror (2026-09-26, the
  post-H8b-halt packet; MACHINERY LANDED, ENGAGEMENT WAITS ON THE
  OBJECTS-STREAM PARITY PACKET — see the H8c row)**:
  the H7g doctrine transferred to the AC1021 container, in full.
  Sites: `DwgFileHeaderWriterAC21::mirror_plan` (the content-parity
  gate), `write_mirrored_pages` (the her-map-order emission),
  `write_file_mirrored` + the shared `write_two_pass_header` tail
  (extracted verbatim from the historical `write_file` — pure code
  motion, stash-verified), `Ac21MirrorBuffers` (the up-front section
  builders in the restructured `write_ac21_impl`, the one lazy
  preview built at her landing address — the retained raw re-emits
  verbatim), plus `serialize_section_map` extracted from
  `build_section_map` for the pre-emission ours-honest table encode.
  The gate: system-id coherence (her pages_map/pages_map2/
  sections_map/sections_map2 ids, positive + distinct), her walk
  sanity (positive sizes/ids, unique ids, TERMINATOR PAIRS caught by
  the 16×n==her-uncomp cross-check), her table sanity (unique names,
  ascending offsets, extent↔map agreement, owned-count parity),
  per-section presence (our writer skips X but the author has N
  pages → decline; our real content with no page space → decline;
  the FileDepList STUB alone is droppable — no document state, gold
  prints zeroed on both sides), and the per-page fit rules exactly
  as contracted: comp<chunk → RS form of our stream zero-padded to
  her slot (bc×255 ≤ slot else decline — `decode_rs` ignores the
  tail and RS correction is commented out in gold, but the contract
  keeps the full form inside the slot), stored → rs_form(chunk) ==
  her size ? the RS form (gold de-interleaves it — the collision
  rule) : raw+pad. The emission records our honest section table
  (her offset/frame/id + OUR uncomp/comp/checksum/crc), her ids
  everywhere, pages_amount = the full recorded walk (her 22 incl.
  both pages-map boxes), maxid via next_page_id = her_max+1 —
  everything in the metadata lands by derivation (the 0x80-block
  addresses included: `find_section_page_offset` over the mirrored
  page records reproduces the author's `summaryinfo_address`/
  `thumbnail_address` — DEBUG-asserted, nothing forced). The
  XrefManifest raw (§19 H7g, ungated R2004+ read retention) is
  carried in the mirror buffers with HER table encoding for the
  unregistered-name declaration (our registry omits it; the bytes
  re-emit verbatim). **The engagement census at the halt: all 58
  corpus AC1021 files gate, ZERO engage — 57 decline on an
  `AcDb:AcDbObjects` window (our AC21 LZ77 encoder's ratio runs
  1.16–1.87× the author's on those windows [ratio attribution
  CORRECTED by the H8c row below — the measurements stand, the
  encoder is NOT the wall]; her slots are the
  EXACT rs_form of her comp — zero slack — measured: Box_2007
  window 2 ours 22,836 vs her ~12,2xx; example_2007 window 2
  16,725 vs ~14.4xx; circle.dwg 17,993 vs ~9.4xx; every
  non-objects section fits on every file) and 1 gap entry
  (ATMOS-DC22S: her map carries a real negative-id gap (−29,
  768) — a new legit decline class). The residue therefore holds
  3,576 — but the whole pages-map/FILEHEADER/THUMBNAIL column is
  now gated LIVE: any file that fits re-emits her container
  byte-exactly. ZERO-KEEPING: the conventional path is
  stash-verified byte-identical (196/196 rewrites unchanged vs the
  pre-H8b baseline), the generation identity re-verified twice
  (40ab5d356cf05a71333f208e1651daf, 25,344 bytes), and 7 new
  hermetic tests pin the author rs_form table (note
  `rs_form(302255) = 307,296`), the stored/collision/overflow
  rules and the map-byte/prefix walks — 42/42 module green.
  **H8c — the AC21 encoder-parity measurement: THE WALL REFUTED AS
  CONTENT, not encoder ratio; the one true grammar gap (the
  chain-long 0xF_ form) FOUND AND VERIFIED, LANDING DEFERRED PER
  ZERO-KEEPING (2026-09-27)**. The instrument:
  `ac21_token_diff` (a new harness bin) — extracts the author's
  on-disk comp per page (RS de-interleave, factor 1, RS(255,251)),
  walks it with a token walker replicating the decoder state
  machine EXACTLY (the ctor 0x2X pseudo-op skip, the trailing state
  machine, the chain-continuation `0xF_ → 0x0_` remap), and
  replay-validates every page's walk byte-for-byte against
  `decompress_ac21`; the `--our-rt` mirror-sim reconstructs our
  rewrite's objects stream, slices it at HER window boundaries and
  scores the H8b fit per window (rs_form vs slot — the exact gate
  rule); the anatomy mode maps both streams (per-8KB zero-density
  and aligned-equality). **REFUTATION, measured**: on the author's
  own decompressed content our compressor BEATS her encoder —
  objects sections: example_2007 118,803 vs her 121,989 (−2.6%),
  Box_2007 25,589 vs 26,744 (−4.3%), circle 25,126 vs 26,356
  (−4.7%) — the prior halt's "1.16–1.87×" figures were
  our-content-in-her-windows vs her-content: LAYOUT measurements,
  not encoder ratios. The real wall: our objects stream carries
  the same constituent masses in a different ORDER (circle: ours
  [dense head][zero mass @16–40K][uniform ~35% mass to the tail]
  vs hers [sparse head][uniform ~35% @8–129K][dense pair
  129–150K][zero mass @172–199K]; totals 199,227 vs 199,139) and
  record-level VALUE divergences from the first bytes (first
  divergence at byte 4–22 — head-record content incl. raw-double
  mantissa deltas, not bit-form deltas), while rs_form's staircase
  leaves 9–50K comp windows a one-block (~255-byte, 0.5–1.8%)
  headroom: circle window 0 measures 17,993 vs slot 9,696
  (−8,672 on one window) — unclosable by encoder work. **The
  census reprised through the instrument**: some windows sit
  exactly one block over (Constraints window 0 rs_form 9,952 vs
  slot 9,696), the uniform minis transpose the same two masses
  (Arc/ConstructionLine/Donut/Constraints/Ellipse: window 0 over
  10–12K, one-two middle windows fitting with slack, tail windows
  one block over), and at least one file fits every reachable
  window on content alone (ATMOS-DC22S: all 10 reachable windows
  FIT with margins +256 to +17,600 — its decline is the gap entry
  class plus our shorter stream, NOT encoding). CONCLUSION: the
  mirror's objects windows close ONLY through the layer-4
  objects-stream parity the prior halt itself named as the
  "alternative unlock" — that is the real next packet: her record
  layout/order and her head-record byte values. **FOUND AND
  VERIFIED, LANDING DEFERRED (the census's one true encoder-grammar
  gap)**: chain-continuation long matches should ride the author's
  remap form — opcode `0xF0|len_field` (3 bytes; the decoder masks
  0xF_→0x0_ at exactly that continuation position — CONFIRMED IN
  GOLD C at `decompress_r2007`'s chain loop, decode_r2007.c:537-538)
  instead of the 4-byte extended fallback; her census long(F): 519
  of 3,676 matches on circle, 1,417 of 11,849 on example_2007; our
  extended usage on identical content collapses accordingly
  (889→~520 on circle). The emission change was implemented,
  hermetic-tested (the continuation 0xF_ form pinned; the
  after-gap class-0 form of the prior halt's 0xF-misuse fix
  preserved) and measured on the FULL corpus: it moved
  R2007_Header 925→929 (net +4: 9 files gained a layout-coupled
  row — Box/Chamfer/ExtrudeP/Fillet/PolysolidD/L/W/X/example_2007
  — and 5 lost one — Cone/Cylinder/RevolveF/Sphere/Torus_2007 —
  the pages_map_correction/header2_offset/offset family
  reshuffling COINCIDENCES as our comp sizes crossed 251-block
  boundaries, not parity moves). §19.3's rule (struct moves only
  toward 0) → the emission REVERTED, the compressor stays at HEAD,
  3,576 re-verified by a stash-run corpus with the change removed.
  The change RELANDS with the objects-parity packet, where it is
  REQUIRED (her streams use the 0xF_ form; byte-identity demands
  it) and every affected row closes anyway. The gold re-scan this
  session re-verified the frozen tree (34f02f54, the config.h.in
  requote the only tracked drift), the corrected oracle
  fingerprints (wrapper 8dad57211b78f42e7594ba6c211cc0e5, ELF
  d852da1db0894b866e86042d4b26b91d, both mtime 2026-09-15
  20:56:22), every pinned decode anchor at its line (rs_form @:692,
  decode_rs @:553, read_system_page @:590, read_data_page @:700,
  the size-only RS/stored dispatch @:856, read_pages_map @:1099,
  the LZ77 state machine @:142/320/361/431 incl. the 31-pattern
  literal reorder table byte-for-byte vs our Rust port; decode.c
  1432/1683; dwg2.spec:3984; dwg.spec:5446;
  r2004_file_header.spec:41-53) — and found NO `encode_r2007.c`:
  gold has no R2007 writer, so her on-disk streams remain the only
  author-encoder ground truth (this instrument). **THE H8d OPENING
  MEASUREMENT (same session, refined through the cycle; circle.dwg
  — `dwgread -v9` traces her file vs our HEAD rewrite, fresh raw
  dumps via the instrument)**: 211 objects both sides, **handle
  maps IDENTICAL 211/211 (same order, same handles) — the earlier
  "our rewrite renumbers handles" attribution is REFUTED**. The
  class census (current writer): 7 records byte-identical; **82
  differ ONLY in the final span byte** (her byte = ours with high
  bits set: 0x3f/0x7f/0xff/0x6f/0x21 vs our 0x00/0x40/0xc0/0x20 —
  the CRC/pad-skeleton tail, the record bodies byte-identical
  prefixes); 58 diverge within the last 8 bytes (the handle-stream
  region); 58 size deltas (mostly ±1–2: her 87 vs our 88, the
  24-object 5598-vs-5600 repeated class; total stream delta only
  +88); 6 deep. **Gold's own per-object internal reads prove the
  data prefixes identical everywhere the sizes hold** (obj 0:
  bitsize 74 = 74, hdl extent @9.2–@17.0 both, the same reported
  pad bytes) — the divergence concentrates in the HANDLES-STREAM
  TAIL: the `ownerhandle` H-reference FORM (her `(8.0.0)`-class
  encodings vs our `(4.1.X)` absolute forms — the same resolved
  targets, ~+1 byte per record in ours; obj 10: Hdlsize 0x1C vs
  our 0x24 = the whole size delta; obj 68: 0x4E vs 0x56). The
  skeleton per the ODA spec §20.1/20.2 (the PDF at
  `~/work/OpenDesign_Specification_for_.dwg_files.pdf`, 270pp, the
  record-format chapter pp. 99–103; pypdf is installed --user in
  WSL python): `[MS size][OT/BOT type][data][string stream R2007+]
  [handles stream][B* pad to byte — THE AUTHOR PADS THE FINAL
  HANDLE-SHIFT BITS WITH 1s]` — plus ONE CRC 2-byte field AFTER
  the span (`crc16(0xC0C1, [MS..span])` — identical rule both
  sides, already correct in our writer; the spec's "RS CRC
  inside the size" is a phantom: gold's "(address+size−2)" read
  lands in the handle tail and the check never validates on the
  R2007 path — no verdict lines at -v9, only section-level
  checksums; byte-identity is the sole authority). PLACEMENT
  (verified): the same records at different addresses (ours
  compact 6..64,008, hers scattered 6..146,328 — a byte-identical
  record pair at our 64,008 vs her 146,328), the interstices
  carry the same `00 29 00 2a…` masses at different positions.
  **The skeleton — SOLVED by the crc16 micro-experiment (the
  DWG CRC-16 table ported from gold `bit_calc_CRC`, candidate
  spans brute-forced against both files' trailing bytes)**: the
  second field between her span end and the next MS =
  `crc16(0xC0C1, [MS..span])` — IDENTICAL RULE BOTH SIDES, our
  writer already emits it correctly (it differs only because its
  input differs). The 82-record last-byte class = THE FINAL PAD
  of the handle stream: her final span byte = the handle data's
  last bits OR'd with 1s to the byte boundary (obj 0: `0.3f` =
  data `00` + pad `111111`), ours 0-padded (`0.00`) — the fix is
  a 1s-final-pad variant of the merge's last spear-shift. Gold's
  "(address+size−2)" CRC read is a phantom (no per-object CRC
  verdicts exist on the R2007 path at -v9; byte-identity is the
  sole authority). The ownerhandle offset math VERIFIED: her
  relative forms vs our absolute code-4 (obj 11 `(12.1.2)` =
  own.E − 2 = owner.C; obj 68 `(8.0.0)` = own.D8 − 1 = D7; ours
  `(4.1.C)`) — the ownerhandle (first ref in the handle stream)
  is encoded RELATIVE TO THE OBJECT'S OWN HANDLE; the exact code
  tables: libredwg `bits.c` `bit_read_H` + our `bit_writer.rs`
  `write_handle`. **THE H8d α SLICE (same session — implemented,
  measured, VERIFIED, then reverted per zero-keeping)**: the
  ownerhandle rule solved and verified 196/196 (relative iff
  rel_len < abs_len, or at equal length iff |offset| <
  owner_value; her census = 78 absolutes + 118 relative + 10
  nulls — NOT "always relative") + the 1s final pad (the
  record's final partial byte is MAIN's closing shift — the
  handle bits pack into main's tail mid-byte, the handle
  sub-writer always byte-aligned at merge, verified by
  instrumentation; her pads all-ones at AC15/AC18/AC21 alike).
  MEASURED: form census == hers exactly (4:88, 8:38, 12:75,
  10:5); **206/211 records byte-identical (from 6)**; stream
  totals identical (199,139); smokes 0/0; the 5 remaining rows
  named (obj 69 STYLE + 74 DIMSTYLE diverge at +7 in the head;
  obj 26 + 29 + 33 LAYOUTs at +225/+93/+93 in the handle-tail —
  all internals identical). The corpus: R2007_Header 925→927
  (+2, the coincidence-row mechanism — the 0xF_ lesson), read
  0/0 held, every other key unchanged → per §19.3 the writer
  edits reverted; the tree at the d935181 3,576-verified
  state; **the re-apply package (4 edits) + the remaining
  layers (the 5 autopsies + THE PLACEMENT LAYER — with records
  byte-identical the section still diverges from byte 164; the
  mirror still OVER: window 0 18,177 vs slot 9,696; her
  per-record addresses + the interstice filler need
  retention-or-rule) are recorded in NEXT_SESSION.md's H8d
  section. The packet lands as a UNIT** (every writer change
  here reshuffles the layout-coupled coincidence rows — the
  0xF_ lesson), with acceptance = the mirror engage census
  closing the R2007_Header/FILEHEADER-AC1021/THUMBNAILIMAGE
  rows wholesale.

  **H8d — the objects-stream parity packet: LANDED AS THE ONE
  UNIT (2026-09-27; the measure→verify→revert loop BROKEN —
  the first row movement since H8b)**: the α slice re-applied
  per the recorded package (write_spear_shift_ones on the
  merge's final pad in both merge_two_stream and
  merge_three_stream — intermediate pads stay zero;
  write_first_ref_handle = the ownerhandle rule, wired into
  the entity entmode==0 path and the non-entity internal
  path, both r2007_plus-gated, nulls and explicit
  relative_owner paths untouched) + the 0xF_ chain-long
  reland (emit_chained_match's nibble-0 branch `|= 0xF0` —
  the author's remap form; hermetic test
  `test_chained_long_match_rides_0xf_form`) + THE PLACEMENT
  LAYER as the raw-echo re-scope: the READER retains her
  reconstructed objects-section stream + her handle map +
  the identity fingerprint (`objects_handle_set_fingerprint`
  — the classes-verbatim doctrine: sorted handle set of the
  objects walk; adds/deletes/renumberings decline the echo)
  on AC1021-format files (`raw_acdb_objects_data` /
  `raw_acdb_objects_handles` / `raw_acdb_objects_fingerprint`
  on the document, serde-skipped); the WRITER's mirror arm
  passes her raw as the `Ac21MirrorBuffers.objects` buffer
  and re-emits her handle pairs through `write_handles`
  (her offsets address her echoed stream) when the
  document's object identity is unchanged — the conventional
  arm keeps our own emission untouched, and an
  identity-changed document falls back to it by construction.
  VERIFICATION: serde green (1,337 lib tests), gold_roundtrip
  green, smokes 0/0 (circle 21, example_2007 11, Box_2007 12
  — all ENGAGED; example_2004 AC18 at 4 and example_2000
  AC15 at 80, both unchanged); the generation identity
  RE-RECORDED: `39e51dfe3fe281b2373994e2ebd82453`, 25,375
  bytes (was `40ab5d356cf05a71333f208e1651daf`, 25,344 — the
  1s-pad + ownerhandle forms touch the generated file; 30 OK
  / 0 FAIL). THE ACCEPTANCE CENSUS (`AC21_MIRROR_DEBUG=1`
  per AC1021 corpus file): **50/58 ENGAGED**; 8 DECLINED —
  the 7 solid-model files (Chamfer/Cone/Cylinder/Fillet/
  Pyramid/Torus/Wedge_2007) + ATMOS-DC22S decline at the
  OBJECT-IDENTITY gate (our emission's handle set ≠ her
  handles map — the skipped-records mechanism on the ACIS
  solids; ATMOS also carries the gap/terminator map entry),
  then the H8b gate declines them on the objects-overflow
  class. THE CORPUS: 280 files, read 0/0, write 0/0,
  **write-target 3,576 → 3,121 (−455)**. Per-key:
  R2007_Header 925→569, FILEHEADER 103→37, THUMBNAILIMAGE
  50→17; FileDepList 1,055 + SecondHeader 386 + AuxHeader 94
  (the parallel session's rows) and R2004_Header 963
  (accepted residue) ALL UNCHANGED — zero-keeping satisfied,
  no key worsened. THE FORECAST RECONCILIATION (the halt
  projected ~1,057 rows → ~2,519; the honest result is −455):
  (a) only the pages-map family rows closed on the engaged
  files — the content-coupled crc/size family
  (sections-map CRCs/sizes/correction, pages-map
  crc_compressed/crc_seed/size_comp, the MT-derive draws)
  stays open because the OTHER sections' bytes are still
  ours (the handles section re-encoded from her pairs —
  same content, our chunk layout; template/classes/aux/
  header our emissions): the map bytes and their CRCs
  derive from those; (b) the 8 decliners keep their full old
  families (25–26 rows each; ATMOS 48). THE H8e QUEUE (the
  next levers, in order): (1) the object-set parity on the
  8 decliners — autopsy why our emission's handle set
  differs from her handles map (the skipped/undecoded
  records census per file; the ACIS-solids pattern suggests
  the failsafe skip); (2) the other-sections byte-identity —
  first VERIFY whether `write_handles(her_pairs)` is already
   byte-identical to her handles-section bytes (the instrument
   compares them directly), then the map-bytes derivation for
   the crc/size family; (3) the 5 record autopsies (the
   single-bit handle-form delta on the three LAYOUTs + the
   STYLE/DIMSTYLE field walk) — polish that gates only the
   conventional arm and the decliners' re-encode path; the
   echo bypasses our emission on the engaged files.

  **H8e-1 — the objects-echo identity gate re-scoped to the
  document universe (2026-09-27; the 8 decliners engaged)**: the
  autopsy trace (the decline branch's debug diff) showed all 8
  decliners as PURE MISSES — our emission a strict subset of her
  handles map, zero extras: the 7 ACIS-solids files each carry ONE
  orphan record (0x2E1/0x2E7/0x2EB — a handle in her map the
  builder never materialized; gold prints it neither, so read 0/0
  held throughout), ATMOS carries 84. The emission-vs-source-map
  comparison falsely declined them. THE FIX: the gate hashes the
  DOCUMENT'S object universe (`document_object_handles`: entities +
  the objects map + the ten tables' control/record handles) on both
  sides — post-build capture at read time, the same enumeration at
  the write gate — so reader-skipped orphans are outside the
  universe on both sides and an unedited document engages; adds,
  deletes and renumberings still decline. MEASURED: the 7 solids
  ENGAGE (11–12 rows each, from 25–26); ATMOS passes the identity
  gate and keeps its legit H8b gap/terminator decline (48). Corpus:
  read 0/0, write 0/0, write-target 3,121 → **3,020 (−101)**
  (R2007_Header 569→489, FILEHEADER 37→23, THUMBNAILIMAGE 17→10;
  every other key unchanged). Generation identity unchanged.

  **H8e-2 — the compressed-page echo: THE AC1021 SURFACE AT ZERO
  (2026-09-27; the crc/size family's unlock — the echo doctrine's
  endpoint)**: the map crc/size family (pages-map
  size_comp/crc_compressed, the sections-map sizes/correction/CRCs)
  is UNREACHABLE BY DERIVATION — the H8c refutation's own finding:
  our compressor beats her encoder on identical content, so our
  compressed bytes never reproduce hers; the system pages' bytes
  (her map pairs compressed by US: 107 vs her 112 on circle) and
  the sections-map descriptors (our comp sizes) can only match by
  echoing her on-disk bytes. THE IMPLEMENTATION: the reader
  retains her WHOLE on-disk file (`raw_ac21_tail`, serde-skipped,
  AC1021-gated — the 0x80 metadata block with its unknown-region
  bytes, the 0x400 file-header page with her check-data and
  MT-derive draws, every page of her walk with her RS coding, the
  header2 copy); the writer's full-echo arm (`write_full_echo`)
  re-emits it verbatim when BOTH identity gates hold (the document
  universe AND the classes fingerprint) on a same-origin write.
  The rewrite of an unedited same-version roundtrip is
  BYTE-IDENTICAL to the source (verified `cmp`-clean on circle,
  example_2007, ATMOS-DC22S, Cone_2007); the writer's own
  derivations stay live as the AC21_MIRROR_DEBUG oracle (the
  derived 0x80 addresses still assert against the retained
  values); edited documents and conversions fall back to the
  mirrored or conventional paths by construction. THE HONEST
  FRAMING (recorded plainly): for an unedited same-version
  roundtrip the AC21 writer is now a byte-copy — the echo doctrine
  ("unmodelable authored state echoes") applied to her encoder's
  exact output; the read axis (0/0 corpus-wide) independently
  verifies the model the echo bypasses. MEASURED: 58/58 AC1021
  files ENGAGE (ATMOS's gap entry and the solids' orphans echo
  naturally — the gap is her map's own bytes now). Corpus: read
  0/0, write 0/0, write-target 3,020 → **2,507 (−513)**:
  **R2007_Header 489 → 0 corpus-wide**, FILEHEADER 23→21 (the
  R2000 7 + fallback 14 remain), THUMBNAILIMAGE 10→8 (the
  non-AC21 share), FileDepList 1,055→1,035 (the AC21 share closed
  as a side effect — her file-dep bytes ride in the tail; the
  parallel session's non-AC21 rows untouched). Generation identity
  unchanged (39e51dfe…, 30 OK). serde green; gold_roundtrip
  green. THE 5 RECORD AUTOPSIES + the MT-variant pinning + the
  0xF_-family polish are now CONVENTIONAL-ARM-ONLY concerns (the
  echo path never runs our emission for unedited roundtrips);
  their queue value is edited-document correctness, not rows.

  **H8f — the R2000 whole-file echo: the flat container joins the
  doctrine (`dfd0ff8`, 2026-09-27)**: the R2000 pair's autopsy
  (the per-stem struct JSONs): the thumbnail/summaryinfo addresses
  (her preview sits EARLY in her flat layout at 220/448; our
  conventional emission lands it late) + the preview bytes'
  embedded offsets — her editor's section placement,
  unmodelable by rule; echo. The READER retains her whole
  on-disk file for the flat formats (AC1012/AC1014/AC1015 —
  extended to every container family in H8g);
  `write_ac15` gained the echo arm under the same gates.
  MEASURED: byte-identical on example_2000, sample_2000,
  circle (R2000), PolyLine2D; corpus 2,507 → **2,014 (−493)**:
  FILEHEADER 21→14, THUMBNAILIMAGE 8→2, and TWO SIDE EFFECTS —
  **SecondHeader 386 → 0 and AuxHeader 94 → 0** (their 7 files
  are R2000-family; the parallel session's rows closed
  wholesale). The remaining 2,014 sat ENTIRELY on the
  AC18-family files.

  **H8g — the unified whole-file echo: THE WRITE-TARGET AT ZERO
  (`c5b7adf`, 2026-09-27; the maintainer's AC18 decision, asked
  and answered in-session)**: the AC18-family echo extended the
  doctrine to the paged containers, then UNIFIED with the R2000/
  AC21 echoes into ONE arm at the write entry
  (`write_to_writer`): same-version source + retained whole file
  + the state hash holds → her bytes verbatim, every family.
  THE GATE — the document-state fingerprint
  (`document_state_fingerprint`): the H8e-1 handle-set gate had
  a hole the issue-80 regression caught (`repro_issue80_in_place_
  layer_rename_survives_dwg_roundtrip` FAILED — an in-place layer
  rename is invisible to handle-set fingerprints and the echo
  silently reverted it). THE FIX: the sorted per-part hash of the
  semantic inventory's visit (the header variables, the ten
  tables' records, the classes, the entities with their
  ownership/extension-dictionary/reactor relationships, the
  objects, the summary, the preview, the EED channel) plus the
  ten table control handles and the retained metadata models
  (aux/template/file-dep/rev-history) — captured at the END of
  the read, compared BEFORE the prepare pipeline at the write
  entry (the writer's own fixups — table-key resync,
  database-reference repair, surface-class preparation — are
  not user edits; the first attempt compared post-prepare and
  falsely declined example_2000/example_2004). issue80 green.
  THE PREPARE-PIPELINE LESSON: any identity gate over the
  document must hash the document AS HANDED TO THE WRITER, not
  the post-fixup state. MEASURED: byte-identity verified per
  family (circle/example_2000/example_2004/example_2010/
  Dynblocks — cmp clean); corpus 280 files: read 0/0,
  write 0/0, read key-gap 0, **write-target 0** (from 2,014;
  the arc 3,576 → 0 through H8d/H8e-1/H8e-2/H8f/H8g).
  R2004_Header 963 (the ACCEPTED residue — the acceptance
  OVERRIDDEN by the maintainer's explicit decision), FileDepList
  1,035, FILEHEADER 14, THUMBNAILIMAGE 2 all closed. Generation
  identity unchanged (39e51dfe…, 30 OK). serde green;
  gold_roundtrip green. THE HONEST FRAMING, FINAL FORM: for an
  unedited same-version roundtrip the DWG writer is a byte-copy
  gated on a full-content hash; edited documents and conversions
  keep the real writers (the conventional arms, the mirrors, the
  prepare pipeline); the read axis (0/0 corpus-wide)
  independently verifies the model the echo bypasses; the
  writer's derivations stay live as the AC21_MIRROR_DEBUG
  oracle.

  **H8h — the conventional-arm record identity on circle: 211/211
  (`32a2e80`, 2026-09-27; the maintainer's "optional
  conventional-arm quality" directive — the five record
  autopsies resolved into TWO fixes)**: the study tooling —
  `DWG_NO_ECHO` (force every echo arm to decline → the
  conventional compressed emission), `DWG_RECORD_TRACE` (the
  reader's her-map + the writer's our-map, handle → offset),
  `DWG_MERGE_TRACE` (per-record main/text/handle-start/handle
  bytes), `DWG_EED_TRACE` (the parsed and the emitted EED
  blocks), the `dwgrewrite --no-lz77` diagnostics flag, and the
  record-wise byte comparison (the maps pair the offsets; her
  record bounded by ours — sizes verified equal). The
  reproduction: 206/211 identical, 5 diverging, no size
  mismatches — the α-era state exactly. THE AUTOPSIES: (1) the
  three LAYOUTs share ONE ref — the PlotSettings plot-view
  handle: the author writes it as a HARD pointer (code 5)
  AGAINST THE SPEC'S DECLARED CODE 4 (gold's own
  dwg_resolve_handleref comment: 4 soft pointer, 5 hard pointer;
  her ref byte 0x50 vs our 0x40 — the straddled record bytes
  0x94/0x14 vs 0x90/0x10 of the α record were this ref viewed
  through the merge bit-alignment); fixed: the LAYOUT-embedded
  `plot_view_handle` writes `HardPointer` (r2004_plus). (2) the
  STYLE + DIMSTYLE records: the EED
  `extra_eed` merge replaced same-app blocks with
  remove-and-push-to-end, REORDERING the author's EED stream —
  her files carry the AcadAnnotative block FIRST, ours last
  (the blocks themselves byte-identical: the fetch trace showed
  the retention IS her bytes in her order — the merge flipped
  it); fixed: replace-in-place (an app with no retained block
  still appends). MEASURED: **211/211 byte-identical** — the
  conventional objects emission now reproduces the author's
  stream on circle.dwg with zero divergence (the α-era target
  fully reached). Corpus: 280 files, read 0/0, write 0/0,
  read key-gap 0, write-target 0 (the echo arms unchanged).
  Generation identity RE-RECORDED:
  `1a56bca0a56dff09511cec1659cfff1b` (the plotview form
  touches the generated LAYOUTs), 25,375 bytes, 30 OK / 0 FAIL —
  and identical md5 WITHOUT `--features serde`. Plus one
  engineering fix the maintainer's unflagged
  `cargo run --example gen_all_entities_all_versions_dwg`
  surfaced: a bare `#[serde(rename = "__version")]` on
  `DwgHeaderRaw::version` (unguarded — the crate could not
  build without the feature); cfg_attr-gated like its siblings,
  the no-feature build now compiles and runs byte-identically.
  REMAINING conventional-arm work: the MT-variant pinning
  (§F2.G — the crc_seed draws; a deeper algorithmic study, not
  attempted); ~~the record-identity result should extend to the
  other AC1021 corpus files the same way (circle's two form
  rules likely cover the class) — a future session's optional
  measurement~~ — **DONE, this row (H8h-ext) below**.

  **H8h-ext — the AC1021 record-identity survey + four landed
  form rules (2026-09-27; the named future-session measurement,
  executed)**: the survey instrument is permanent:
  `tests/gold_harness/record_identity_survey.py` (per file: the
  DWG_NO_ECHO conventional rewrite + the AC21_DIFF_RAW_DIR
  her/ours decompressed objects-stream dumps + the
  DWG_RECORD_TRACE maps; then a byte-wise walk of every record
  unit — her record bounded by ours, sizes verified equal,
  divergences reported with the first differing byte). ONE
  instrument bug found and fixed during the autopsy: the first
  parser read the DWG modular short as 7-bit bytes — the MS is
  **15 data bits per 16-bit LE word with bit 15 as the
  continuation flag** (so a record unit = [MS words][size
  span including the final pad byte][crc16 LE over MS..span];
  the §19.4.C frame confirmed — the crate's own reader/writer
  were always right, the survey tool was corrected). THE
  MEASUREMENT (58 AC1021 files): the "circle's two rules cover
  the class" hypothesis is FALSE — only 14/58 (the simple
  ODA-authored named specimens) were at 100%; 423 divergent
  records initial. THE TAXONOMY: (a) the author's **code-3
  owner-class refs** where our writer used 4/5 (the
  AcDbShHistory root's graph ref, the eval-graph node `evalexpr`,
  and further stragglers in example_2007/ATMOS); (b) the
  control-entries **author order + null deleted-slot tails**
  (Box's BLOCK_CONTROL `entries[1]=(2.0.0) abs:0`, its
  DIMSTYLE_CONTROL `[432 Standard, 311 Annotative, 27 ISO-25,
  null]` vs our ascending 3-slot emission) — the sub-genus-B
  26-vs-25 and the sub-genus-A +14 reorder; (c) the
  SH-history DICTIONARY stub (`size 70 vs 17` — gold prints the
  `ACAD_PARALLEL_BACKGROUND` item with a NULL target handle; our
  emission drops it); (d) the 3DSOLID/ACIS record tail ref
  (`her 06/64/03/19 vs our 08/84/04/21` — nibble-class/count
  deltas inside the SAT record's handle stream); (e) the
  assoc-pathparam/unknown-body classes on the M/CSurf stems; (f)
  the fillet/chamfer UNKNOWN bodies (1463/1509 vs 28) and the
  29-vs-27 class; (g) the gold-tree stragglers (VIEW_CONTROL
  27/18/22-v-10 — the views not in our model's control emission;
  Constraints 3E3 666-v-870; example_2007's marginal forms;
  PolyLine3D 1C2 16-vs-17 reversed; ATMOS's 64 + 84: the ACAS
  interior records and her controls). FOUR RULE-SETS LANDED:
  (1) `AcDbShHistory.owner` graph ref: SoftPointer(4) →
  **HardOwnership(3)** — gold prints `owner [3, ..]`
  corpus-wide across every genus that has the record (the 2027
  fixtures, the 2024-authored example_2007, ATMOS; verified —
  the 14 clean files have NO SH-family records, so no genus
  conflict); (2) the eval-graph node `evalexpr` ref:
  HardPointer(5) → **HardOwnership(3)** — the same census (gold
  prints `evalexpr [3, ..]` in all three genera); (3) the
  **authored table-control entry slots** capture:
  `document.table_control_entries` (`BTreeMap<control handle,
  Vec<Handle>>`, serde-skipped, semantic-inventory-excluded —
  the morehandles precedent — so the echo fingerprint is
  untouched) captured at DWG read for BLOCK/LTYPE/DIMSTYLE
  (entries verbatim: order + null tails; the LTYPE/RCu drains
  matching the gold spec forms) and echoed by the writers under
  a **same-universe gate** (`authored_control_entries` — the
  captured non-null set must equal the live table's handles;
  edited tables and DXF-built documents fall back to the model
  iteration); (4) the survey tool itself. **MEASURED: 423 → 185
  divergent records** (every fixture genus file at 2–3 residual
  records; the 14 clean files unchanged at 100% — circle still
  211/211). GATES: serde 1602/0, gold_roundtrip ok, issue80
  7/0, the four family smokes 0/0, the full corpus 280 files
  0/0/0/0, generation identity UNCHANGED
  (`1a56bca0a56dff09511cec1659cfff1b`, 25,375 bytes — the
  generator's programmatic documents never carry these fields;
  verified with AND without `--features serde`). The residual
  185 = classes (c)–(g) above — each a named, byte-level-scoped
  future packet.

  **H8h-ext-2 — three more form rules: 185 → 49 (2026-09-27,
  the same session's continuation; classes (c)+(d)+the
  VIEW slots)**: (1) **the null-target dictionary items** —
  the R2000+/same-version arm of `write_dictionary` filtered
  `!h.is_null()`, dropping the author's placeholder slots
  (the fixtures' `ACAD_PARALLEL_BACKGROUND → [2,0,0,0]`
  dictionaries, gold `numitems: 1` vs our 0); fixed: the arm
  keeps null-target items (`.filter(h.is_null() ||
  is_writable_object(h))` — a null target is authored state,
  not a dangling ref; the enqueue guard already skips nulls;
  the down-conversion arm untouched). (2) **the 3DSOLID-family
  `history_id` ref code**: our three write sites (3DSOLID/
  REGION/BODY, `solid_history_handle_value`) wrote
  SoftPointer(4); the author writes **code 3** — the gold
  census across the whole corpus prints history_id `[3, ..]`
  **128/128** non-null (and the nulls `[3,0,0,0]` — the
  code-3-count-0 `30` byte); the ExtrudeC walk: her ref2 =
  (3, 2, 02, E0) — the history root 0x2E0 — vs our (4, 2,
  02, E0); fixed: HardOwnership at all three sites (the
  Surface entity carries no history_id write — verified).
  (3) **the VIEW_CONTROL authored slots**: Leader's view
  control carries **16 null deleted slots** (gold:
  `num_entries: 16 [BL]`, entries `(2.0.0)` × 16) — the
  same authored-slots disease as the BLOCK/LTYPE/DIMSTYLE
  capture; fixed: the pass-1 OBJ_VIEW_CONTROL arm drains
  [BL count][refs] into `table_control_entries` and the
  view writer call site applies the same-universe gate (the
  all-null case passes: the filtered captured set equals
  the empty live table). **MEASURED: 185 → 49 divergent
  records; 32/58 files at 100%** — the whole
  extrude/revolve/polysolid/box/sphere/torus/union/wedge/
  cone/cylinder named-specimen + solid genus now FULLY
  byte-identical (ExtrudeC 206/206, Box 207/207, Leader
  245/245, circle 211/211 held). GENERATION IDENTITY
  RE-RECORDED: `84374e73ddcf1d6143877c4100b81e48`, 25,375
  bytes — the null history_id form changed `40` → `30`
  bytes on the generated R2007+ 3DSOLID-family records
  (same byte count, different nibble); verified identical
  WITHOUT `--features serde`. GATES: serde 1602/0,
  gold_roundtrip ok, issue80 7/0, four family smokes 0/0,
  corpus 280 files 0/0/0/0. THE RESIDUAL 49: ATMOS 5 (its
  controls h=2/h=3 + h=77 96v40 + h=352 25642v35 + h=541
  17v18) + 84 her-only orphans; Constraints 3E3 (666v870);
  PolyLine3D 1C2 (16v17, ours longer); example_2007's 9
  marginal forms; Chamfer/Fillet 2 each (the UNKNOWN
  bodies 1509/1463v28 + the 22v26/30); the Loft3/C/H/R/_
  29-vs-27 class ×5; the assoc/pathparam genus (ExtrudeCSurf/
  M, LoftCSurf/M, RevolveM) 24 — each still a named,
  byte-level-scoped packet.

  **H8h-ext-3 — the assoc genus + the *Model_Space block
  forms: 49 → 32 (2026-09-27, the continuation; the
  assoc/pathparam genus + the 1F class)**: (1) **the
  three assoc ref codes** — the gold -v9 walks name the
  fields and the corpus census decides: `pab.deps` →
  **HardOwnership(3)** (census {3: 27}, never 4/5; was
  SoftPointer), the compound `parameters` →
  **HardOwnership(3)** (census {3: 41}; was SoftPointer),
  `sab.assocdep` → **SoftPointer(4)** (census {4: 23},
  never 5; was HardPointer; the null case keeps the
  is_valid() guard — the PLANESURFACE [0,0] form). (2)
  **the Revolved body's trailing B(0)**: the RevolveM
  2E3 bit-diff (a clean 1-bit shift from record bit 250)
  proved her main stream one bit longer — one trailing
  '0' after class_version; the only corpus specimen of
  the class (no other Revolve file carries
  ASSOCREVOLVEDSURFACEACTIONBODY); landed as a
  Revolved-kind trailing bit. (3) **the *Model_Space
  BLOCK_HEADER (the 1F class)** — two independent drops:
  (a) the block's **xdicobjhandle** — the reader captures
  every xdic into `document.xdic_by_handle` (the
  normalizer's side-channel, already in the semantic
  inventory) but the writer hardcoded `&None`; now the
  block writer passes the side-map value (her fixtures:
  (3.2.1CE), 24 bits); (b) the **owned-list live-filter**
  — the caller filtered `entity_index.contains_key`, but
  the author's owned lists can reference non-graphical
  records (her entities[1] = 0x2DB, an ACDBASSOC* object
  gold prints as UNKNOWN_OBJ — our reader models it as an
  Unknown OBJECT with raw passthrough, so it IS
  serialized); the filter now keeps indexed entities OR
  writable objects (`is_writable_object`), so the count
  and the refs match her wire (num_owned 2, two refs).
  **MEASURED: 49 → 32 divergent; 42/58 files at 100%** —
  ExtrudeCSurf 215/216, LoftCSurf 219/221, RevolveM
  216/217, ExtrudeM 215/216 (each now holding ONLY the
  undocumented-subcurve stubs, below); circle 211/211 and
  Leader 245/245 held. GENERATION IDENTITY UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes,
  feature-independent — the generator's programmatic
  documents carry no block xdics, no assoc surface
  bodies, no owned-object refs). GATES: serde 1602/0,
  gold_roundtrip ok, issue80 7/0, four family smokes
  0/0, corpus 280 files 0/0/0/0. THE RESIDUAL 32: the
  **undocumented CALL_SUBCURVE bodies** (7 records:
  ExtrudeCSurf/ExtrudeM/RevolveM 2E6, LoftCSurf/LoftM
  2E7+2EA — the ACDBASSOCEDGEACTIONPARAM subcurve region;
  gold's own spec macro is an EMPTY TODO STUB, so the
  wire form must be reverse-engineered from the specimens
  — a named future packet needing the ODA PDF + the
  per-action_type dissection); ATMOS 5 + 84 her-only;
  Constraints 3E3; PolyLine3D 1C2; example_2007's 9;
  Chamfer/Fillet 4 (the UNKNOWN bodies + the 22v26/30);
  the Loft 29-vs-27 ×5.

  **H8h-ext-4 — the CALL_SUBCURVE bodies reverse-engineered:
  32 → 25 (2026-09-27, the continuation; the biggest named class
  closed)**: the subcurve wire — unknown to gold (its `CALL_SUBCURVE`
  spec macro an empty TODO stub in every commit that ever touched
  it) and to the ODA PDF (zero hits for any ACDBASSOC term) — was
  cracked from the seven corpus specimens. THE METHOD: a
  temporary reader trace (`DWG_SUBCURVE_TRACE`) printed the
  main-stream position after `action_type` and the main end per
  record (all seven: action_type 11 = ARC, region = span bits
  96..main_end); the regions were extracted and walked as standard
  BD sequences — **every region walks EXACTLY as twelve BDs**:
  center (3BD), normal (3BD), x-axis (3BD), radius (BD),
  start_angle (BD), end_angle (BD) — the standard BD forms
  (0.0/1.0 as the 2-bit shorts, other values as the full 66-bit
  LE double). The specimens' centers and radii match their source
  CIRCLE entities exactly (ExtrudeCSurf (2,3,0) r=1; RevolveM
  (2,0,0) r=0.8; LoftCSurf (3,4,0)/(3,4,7) r=1.5; the M-stems at
  the origin r=1); all seven are full circles (start 0.0, end 2π,
  the `18 2D 44 54 FB 21 19 40` LE tail) in the XY plane (normal
  (0,0,1), x-axis (1,0,0)). THE LANDING: (1) the model —
  `AssocArcSubcurve` (center/normal/x_axis/radius/start_angle/
  end_angle) + `AssocEdgeActionParam.subcurve: Option<…>`
  (serde-default; None for DXF-built documents and the
  unattested action types); (2) the reader — the twelve BDs
  after action_type for the attested ARC form; (3) the writer —
  the twelve BDs emitted when captured, plus the `param` ref
  code fix: the author writes the null param as **(4.0.0)**
  (SoftPointer) — the wire evidence uniform across all seven
  records, overriding gold's spec declaration of 3 (the
  HardOwnership emission produced the 2-bit tail delta the
  first survey pass showed). **MEASURED: 32 → 25 divergent; 47/58
  files at 100%** — ExtrudeCSurf 216/216, ExtrudeM 216/216,
  RevolveM 217/217, LoftCSurf 221/221, LoftM 221/221; circle
  211/211 held. GATES: serde 1602/0, gold_roundtrip ok,
  issue80 7/0, four family smokes 0/0, corpus 280 files
  0/0/0/0, generation identity UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes,
  feature-independent — the generator emits no EDGEACTIONPARAM
  records). THE RESIDUAL 25: ATMOS 5 + 84 her-only;
  example_2007's 9 marginal forms; Chamfer/Fillet 4 (the
  UNKNOWN bodies + the 22v26/30); the Loft 29-vs-27 ×5;
  Constraints 3E3; PolyLine3D 1C2 — the same named classes as
  the H8h-ext-3 halt minus the subcurve records.

  **H8h-ext-5 — the PersSubentManager v2 tail: 25 → 20
  (2026-09-27, the continuation; the Loft 29-vs-27 class
  closed)**: the divergent records are the NON-ASSOC
  `ACDBPERSSUBENTMANAGER` class (the maintainer's
  assoc/non-assoc distinction CONFIRMED: the corpus carries
  both classes — the ASSOC variant `ACDBASSOCPERSSUBENTMANAGER`
  has a gold spec block and its records were already
  byte-identical; the non-assoc variant has NO spec block, gold
  parks it raw, and our reader models it via the
  `PersSubentManagerStatic` path). THE AUTOPSY: the five loft
  specimens' class region = [5 BLs: 2, 0, 2, {2|1}, 1]
  [num_steps=7 + steps][num_subents=0][BL(1)][BL({2|1})] — the
  walk ends exactly at the main-stream CONTENT end; the
  following bit is the merged writer's no-text flag (the RL =
  content + 1; the first attempt misread the flag as a record
  B field — the +1-bit lesson: the merged writer's finalization
  writes `total_bits = main + text + 1` and the flag bit
  itself, so a tail must never include it). THE GATE (the
  regression that found it): the tail is present ONLY when
  `associative_subent_count != 0` — the first ungated landing
  captured garbage on the ~25 count-0 records (e.g. ExtrudeC
  2DC: 22v27, a 25→48 regression), because reading past the
  count-0 record's content end pulls the flag + handle-stream
  bits. THE LANDING: `PersSubentManager.v2_tail:
  Option<(i32, i32)>` (serde-default; None for DXF), the reader
  gated on `class_version == 2 && associative_subent_count !=
  0`, the writer emitting the two BLs when captured. **MEASURED:
  25 → 20 divergent; 49/58 files at 100%** — Loft 207/207,
  Loft3 206/206, LoftC 211/211, LoftH 206/206, LoftR 206/206;
  the regressed genus restored (ExtrudeC 206/206, Box 207/207,
  RevolveA 207/207); circle 211/211 held. GATES: serde 1602/0,
  gold_roundtrip ok, issue80 7/0, four family smokes 0/0,
  corpus 280 files 0/0/0/0, generation identity UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes,
  feature-independent). THE RESIDUAL 20: ATMOS 5 + 84
  her-only; example_2007's 9 marginal forms; Chamfer/Fillet 4
  (the UNKNOWN bodies 1509/1463v30 + the 22v26/30 — the same
  non-assoc class with the fillet/chamfer unknown_bits mass);
  Constraints 3E3; PolyLine3D 1C2.

  **H8h-ext-6 — the PersSubentManager genus completed:
  20 → 15 (2026-09-27, the continuation; the ASSOC-variant
  re-parse + the passthrough tails)**: (1) **the ASSOC
  variant re-parsed to the gold spec**: the old
  `ASSOCPERSSUBENTMANAGER` reader skipped the spec's
  `unknown_bl1`/`unknown_bl2` (the two BLs between the
  markers and num_steps) and read BLs until the stream end
  plus a final_flag bit — a value-level BL passthrough that
  was byte-SYMMETRIC (the loft records roundtripped) but
  desynced on the Chamfer/Fillet 2DE records (the 2-bit
  code-11 branch's 256 → our 30-byte garbage records).
  The re-parse reads the spec fields properly (bl1, bl2,
  num_steps, steps, num_subents, subents) — the FIRST
  attempt exposed the tail structure: the simple records
  end with ONE trailing B (the last content bit), but the
  LoftCSurf/LoftM 2DD records carry a variable BL run
  ([0,0,0,1,1,0]) before it. (2) **the passthrough tails**:
  both variants now capture the post-subents region
  verbatim — the ASSOC variant: BLs until one bit remains +
  the trailing B; the non-assoc variant: BLs until the
  content end (flush, no B). This subsumes the H8h-ext-5
  [BL][BL] tail (the loft records' two BLs are just a short
  run) AND the Chamfer/Fillet 2DF **~1224-BL history blobs**
  (the 1509/1463-byte records — pure BL sequences, 31
  distinct small values, ending flush). THE FRAME MATH that
  settled the boundaries: the RL (bitsize) = main content +
  text + 1 (the no-text flag); `main_remaining_bits()` is
  relative to the CONTENT end; the ASSOC records end
  [BLs][B], the non-assoc records end flush — the bit after
  the content is ALWAYS the flag, never a record field (the
  H8h-ext-5 lesson, now applied to both variants).
  **MEASURED: 20 → 15 divergent; 54/58 files at 100%** —
  the whole persubent genus (both variants, all tail forms,
  the 1.5KB blobs) byte-identical: Chamfer 207/207, Fillet
  207/207, Loft 207/207, LoftCSurf 221/221, LoftM 221/221;
  ATMOS dropped 5 → 4 (one of its records was persubent);
  circle 211/211 held. GATES: serde 1602/0, gold_roundtrip
  ok, issue80 7/0, four family smokes 0/0, corpus 280 files
  0/0/0/0, generation identity UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes,
  feature-independent). THE RESIDUAL 15: ATMOS 4 + 84
  her-only; example_2007's 9 marginal forms; Constraints
  3E3; PolyLine3D 1C2.

  **H8h-ext-7 — the SEQEND captured-flags fix: 15 → 14
  (2026-09-28, the continuation; the PolyLine3D 1C2
  single)**: the ODA-authored PolyLine3D SEQEND carries
  shadow_flags=0 with NO shadow ref (her Hdlsize 25 =
  owner 8 + layer 16 + 1 pad) where our writer emitted
  the era convention (3, 0) — `seqend_era_flags()`'s
  "verified per owner across the example corpus" was a
  per-AUTHOR form mistaken for an era law (the example_*
  corpus SEQENDs carry (3,0); the ODA test-data
  PolyLine3D carries (0,0)). THE FIX: the four
  polyline-family writers (Polyline2D/3D, PolyfaceMesh,
  PolygonMesh) now use the READ-CAPTURED flag pair
  (`e.seqend_shadow_flags`/`e.seqend_plotstyle_flags` —
  the pending.seqend_flags capture, transferred with the
  seqend handle) whenever the read retained a wire
  SEQEND; the era convention is the DXF-built fallback
  only. The INSERT path (literal (0,0), no corpus
  divergence) untouched. **MEASURED: 15 → 14; 55/58 at
  100%** — PolyLine3D 218/218; Polyline/Polygon/example_*
  held (their captured (3,0) equals the era form, so the
  switch is a no-op there). GATES: serde 1602/0,
  gold_roundtrip ok, issue80 7/0, four family smokes
  0/0, corpus 280 files 0/0/0/0, generation identity
  UNCHANGED (`84374e73…`, 25,375 bytes,
  feature-independent — the generator's programmatic
  polylines take the era fallback). THE RESIDUAL 14:
  example_2007's 9 marginal forms; ATMOS 4 + 84
  her-only; Constraints 3E3 (the ASSOC2DCONSTRAINTGROUP —
  ours 204 bytes LONGER; gold parks 5249 unknown bits in
  her record — a fresh dissection packet).

  **H8h-ext-8 — the Constraints 3E3 closure: the
  node-region wire capture: 13 → 12 (2026-09-28, the
  continuation session; the ASSOC2DCONSTRAINTGROUP
  single)**: the dissection completed against the ERA
  FAMILY — the same drawing exists as r14/2000/2004/
  2007/2010/2013 `Constraints.dwg` specimens (a corpus
  scan away from the AC1021 set), and the pre-2007
  records carry the node class names INLINE as TVs
  (`AcConstrainedCircle`, `AcConstrainedImplicitPoint`,
  `AcCenterPointConstraint`, `AcConstrainedBoundedLine`,
  `AcPointCurveConstraint`,
  `AcPointCoincidenceConstraint` — nine nodes, one name
  each, in walk order), which pins the real node wire:
  per node `[BLd id][RC status][TU class-name (AC1021:
  consumed from the record's TEXT stream in walk order;
  pre-2007: inline main TV)][class data arm][geometry
  handle refs]` — NOT gold's flat REPEAT (dwg2.spec
  5682), whose own -v9 walk desyncs at node[1] and parks
  5249 unknown bits. CROSS-ERA BIT-IDENTITY: the circle
  node's 320-bit data region is bit-identical between
  the 2007 and 2000 records once the inline TV is
  discounted — the class arms are era-stable. The
  recovered arms: the circle = `[BL 1][BL 1][BL 3][BL 0]
  [3BD center (11.319546, 16.063856, 0)][3BD (0,0,1)]
  [3BD (1,0,0)][BD radius 2.147789][BD 0.0][BD 2π]`;
  the implicit points carry the `00FFFFFFFF` BLd -1
  (point_idx) markers (three, one per ip node) + a
  curve_id BLd; the handle stream carries five node
  reads after the head's six (`[3E4][∅][3E5][∅][∅]` —
  the two ASSOCGEOMDEPENDENCY refs plus three nulls).
  THE AUTHORITY CLOSURE HOLDS: the ODA PDF has zero
  ConstraintGroup hits (grep-verified); libredwg's
  spec macros (`AcConstraintGeometry_fields` etc.) are
  defined but used by NO live block, and its model
  struct is base-only ("still in work"). THE FIX (the
  H8d/persubent-tail/MultiLeader-tail wire-capture
  doctrine): the AC1021 reader retains the region
  verbatim — the main bits from the end of num_nodes to
  the record's main-data end, the per-node class-name
  TUs (bounded by what the text stream holds), and the
  handle bits from the drain position after the
  record's own head reads to the record end MINUS the
  author's closing 1s pad (a ≤7-bit trailing-1s scan;
  the merged writer re-creates the pad at close — the
  first landing attempt double-padded, the measured
  666-vs-667 byte taught the trim); the writer re-emits
  the TUs into the text stream (walk order) + the raw
  bits + the raw handle tail, and the naive modeled
  REPEAT stays the DXF/programmatic fallback
  (`nodes_wire_main` absent; the naive semantic walk
  kept for the model). Gated to AC1021 — the only
  dissected frame; the pre-2007 era specimens keep
  their current behavior (echo-covered on the corpus
  axes, ungated by the AC1021 survey; a later packet if
  ever measured). **MEASURED: Constraints 219/219
  (0 divergent, streams 222306 = 222306); the residual
  12 = example_2007's 9 marginals + ATMOS 3 (the
  controls h=2 16v15, h=3 20v15, h=77 96v40) + 84
  her-only; 56/58 files at 100%.** THE HALT-DOC
  CORRECTION (honest bookkeeping, stash-rebuild
  verified): the ext-7 halt's "14" was actually 13 —
  its residual list carried the stale ext-6-era names
  h=352/h=541 for ATMOS (five names for "four"), and a
  direct re-measure of the halt's own committed HEAD
  shows ATMOS at 3 divergent (h=2/h=3/h=77 only): this
  landing's true delta is 13 → 12. GATES: serde 1602/0,
  gold_roundtrip ok, issue80 7/0, four family smokes
  0/0, corpus 280 files 0/0/0/0, generation identity
  UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes, feature-independent — the generator's
  programmatic groups take the naive fallback), the
  echo byte-identity held (circle cmp clean). THE
  RESIDUAL 12: example_2007's 9 marginals; ATMOS 3 +
  84 her-only.

  **H8h-ext-9 — the ATMOS closure: two reader-side
  conventions corrected: 12 → 9 (2026-09-28, the
  continuation; the ATMOS-DC22S three)**: the Russian
  drawing's three divergent records fell to two
  root causes, both reader-side. (1) **h=2/h=3
  (LAYER_CONTROL/STYLE_CONTROL)**: her entry vectors
  carry NULL DELETED-SLOT TAILS — [10, A5, 0] for two
  live layers; [11, 6B, 0×5] for two live styles —
  the same authored-slots phenomenon the H8h
  extension captured for BLOCK/LTYPE/VIEW/DIMSTYLE
  but not LAYER/STYLE, whose writers derived the
  count from the live tables. THE FIX: the builder
  captures the LAYER/STYLE entry slots (num_entries
  BL + the handle vector, keyed by the control
  handle — the VIEW_CONTROL pattern), and the two
  writers apply the same-universe gate
  (`authored_control_entries`: the captured non-null
  set must equal the live table's handles; edited
  tables and DXF-built documents fall back to the
  model). (2) **h=77 (DICTIONARY)**: the image
  dictionary's keys are CYRILLIC — "_Схема-1" …
  "_Схема-4" — and `clean_dict_key()` (the R13/R14
  mis-sized-key workaround: cut at the first
  non-printable/high byte, added for the
  ACAD_FILTER\u{80}0 xclip/gradient lookups) was
  applied to ALL eras, truncating the legitimate
  R2007 Unicode keys to "_". THE FIX: the cut is
  gated to `r13_14_only()` (the mis-sized era);
  R2000+ keys are exact-length and pass verbatim —
  the same lesson as the H8h-ext-7 SEQEND flags: an
  era-verified convention is not a law. (The
  read-fidelity gates never caught it:
  normalize_gold/normalize_silver drop DICTIONARY
  texts/items from the compare — only the byte
  survey sees the loss.) **MEASURED: ATMOS 340/340
  (0 divergent; the 84 her-only broken-map orphans
  remain, pre-existing); the residual 12 → 9; 57/58
  files at 100%** (only example_2007 below). GATES:
  serde 1602/0, gold_roundtrip ok, issue80 7/0,
  four family smokes 0/0, corpus 280 files 0/0/0/0,
  generation identity UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`, 25,375
  bytes, feature-independent — the generator's
  tables carry no captures and its dictionary keys
  are ASCII), the echo byte-identity held (circle
  cmp clean). THE RESIDUAL 9: example_2007's
  marginals only.

  **H8h-ext-10 — the example_2007 first fruits:
  four of the nine closed (the hasatts bit, the
  REGION history_id presence, the persubent-id
  tail): 9 → 5 (2026-09-28, the continuation)**:
  the last residual class opened; three root
  causes fell. (1) **h=1F (*Model_Space
  BLOCK_HEADER)**: her hasatts bit is 0 while
  the block carries ATTDEF entities — the writer
  DERIVED the bit from AttributeDefinition
  presence (a DXF-built heuristic applied to
  DWG reads). THE FIX: the captured model bit
  (`flags.has_attributes`) is authoritative on a
  DWG read; the heuristic stays the DXF-built and
  programmatic fallback (the generation
  identity's path). (2) **h=176/h=37D (REGION,
  +1 byte each)**: the writer emitted an explicit
  null history_id handle where the author OMITS
  it. Gold's rule (dwg.spec COMMON_3DSOLID): the
  ref is read only while ≥8 handle bits remain
  (the AVAIL_BITS guard) — and the wire census is
  PER-RECORD: ATMOS's native solids write
  explicit nulls (58×, matched by our emission),
  while example_2007's imported-ACIS REGIONs
  (an "ASM 223.0.1.1930 OSX" SAB blob) end
  their handle streams flush after the layer
  ref. THE FIX: the reader gates the read on
  the ≥8 rule and captures the wire PRESENCE
  (Some(NULL) = an explicit null on the wire;
  None = absent — the REGION reader previously
  never read the handle at all while the writer
  emitted it, the asymmetry that produced the
  +8 bits); the writer mirrors the presence for
  DWG reads and keeps the explicit null for
  DXF/programmatic documents. (3) **h=396
  (ASSOCGEOMDEPENDENCY, −46 bits)**: her record
  carries 46 undocumented main-stream bits after
  the persubent id's dependent_on_compound_
  object — gold's spec block (dwg2.spec 3148)
  ends there and its own walk parks them
  unknown; her handle stream holds only the five
  parsed refs plus the closing 1s pad, so the
  main tail is the whole delta. THE FIX (the
  H8h-ext-8 wire-capture doctrine): the reader
  retains the tail verbatim (`tail_bits` +
  `tail_bit_len` on AssocPersistentSubentId,
  peeked from the post-flag position to the
  main-data end); the writer re-emits the bits;
  records whose parse consumes the region
  exactly capture nothing. **MEASURED:
  example_2007 535/540 (the residual 5: h=1A9/
  h=430 — double low-mantissa diffs on fields
  that print identically, the recomputed-value
  suspicion; h=392/h=393 — single-bit diffs in
  the ACDBASSOC partial-parse classes' unknown
  regions; h=BF2 — a 14.5KB UNKNOWN_OBJ whose
  captured re-emission runs 203 bytes short and
  near-fully differs); ATMOS re-verified
  340/340; 57/58 files at 100%.** GATES: serde
  1602/0, gold_roundtrip ok, issue80 7/0, four
  family smokes 0/0, corpus 280 files 0/0/0/0,
  generation identity UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`, 25,375
  bytes, feature-independent), the echo
  byte-identity held (circle cmp clean). THE
  RESIDUAL 5: example_2007's h=1A9/h=392/h=393/
  h=430/h=BF2.

  **H8h-ext-11 — the wire-exactness landings:
  the RAY/XLine normalize and the ordinate
  measurement recompute: 5 → 3 (2026-09-28,
  the continuation)**: the two double records
  fell to RECOMPUTE-ON-ROUNDTRIP — the model
  APIs' semantic normalizations applied to the
  author's already-computed wire doubles,
  shifting them exactly 1 ulp. (1) **h=1A9
  (RAY)**: her vector = (0.8208410212999244,
  0.5711567365892636, 0); our model held
  (...42, ...35, 0) — `Ray::new` (and
  `XLine::new`) NORMALIZE the direction, and
  re-normalizing the author's unit vector
  recomputes it 1 ulp off. THE FIX: the
  builder constructs the entity WITHOUT the
  constructor normalize (the struct literal;
  the wire direction is authoritative); the
  normalize stays the programmatic-API
  semantic. (2) **h=430 (DIMENSION_ORDINATE)**:
  her act_measurement = 4630.519359082827
  (one ulp below her feature_location.x);
  BOTH sides recomputed it away — the
  builder's ordinate arm called
  `refresh_measurement()` after
  `map_dimension_common` had already set the
  wire value, and the writer's
  `write_dimension_ordinate` recomputed
  `d.measurement()` into a cloned base before
  emitting. THE FIX: both recomputes removed
  on the DWG path — the model's stored value
  is the semantic source of truth on every
  path (programmatic construction refreshes
  at `DimensionOrdinate::new`; edits refresh
  via the transform path). **MEASURED:
  example_2007 537/540; 57/58 files at
  100%.** GATES: serde 1602/0,
  gold_roundtrip ok, issue80 7/0, four family
  smokes 0/0, corpus 280 files 0/0/0/0,
  generation identity UNCHANGED
  (`84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes, feature-independent — the
  generator's rays take the constructor
  normalize and its ordinate the constructor
  refresh, so the emitted bytes are
  unchanged), the echo byte-identity held
  (circle cmp clean). THE RESIDUAL 3:
  example_2007's h=392/h=393 (the ACDBASSOC
  partial-parse single-bits) and h=BF2 (the
  14.5KB UNKNOWN_OBJ, 203 bytes short).

  **H8h-ext-12 — THE ZERO-RESIDUAL LANDING:
  the last three records closed (the two
  ASSOC ref-type nibbles + the TABLECONTENT
  wire capture): 3 → 0 — THE AC1021
  CONVENTIONAL ARM AT ZERO DIVERGENT RECORDS
  (2026-09-28, the continuation)**: (1)
  **h=392/h=393 — the handle-reference TYPE
  nibbles**: the byte dumps pinned both diffs
  to single ref-code nibbles in the handle
  streams. h=392 (ACDBASSOCALIGNEDDIMACTION-
  BODY): her empty-values pab dependency ref
  is (4.2.397) — SoftPointer — where our
  writer emitted 5 (gold's dwg2.spec pab
  block declares 5; the wire is the authority
  — the same spec-vs-wire split as the
  H8h-ext-3 deps census {3: 27} and the
  H8h-ext-4 edge param). h=393 (ACDBASSOC-
  OSNAPPOINTREFACTIONPARAM): her compound
  child.parameter (child id=0) is (4.0.0) —
  SoftPointer — where our writer emitted 3
  (gold's child_param block declares 3). THE
  FIX: both writer refs switched to
  SoftPointer; the child secondary/tertiary
  refs (child id != 0, no corpus specimen)
  keep gold's declared 3. (2) **h=BF2 — the
  TABLECONTENT closure (the ext-8 wire-capture
  doctrine, third application)**: the class
  (type 529, 14.5KB) has NO gold spec block
  ("Unknown Class object 529 TABLECONTENT")
  and no ODA documentation; our modeled
  AcDbLinkedTableData emission diverged
  structurally — her main region 17,587 bits
  vs our modeled 15,963 (203 bytes,
  near-full) — while the TEXT region (98,348
  bits, the cell texts) and the HANDLE stream
  (0xD8) re-emitted BIT-IDENTICAL, isolating
  the divergence to the main body. THE FIX:
  the AC1021 reader captures the body
  verbatim — the main bits from the body
  start (after the common fields) to the
  main-data end, the text-region bits, and
  the handle tail — and the writer re-emits
  all three raw (write_bit / write_text_bit /
  write_handle_bits); the modeled emission
  stays the DXF/programmatic fallback. (3)
  **THE PAD LESSON, EXTENDED**: the first
  capture landing produced the right SIZE
  but a wrong final byte (her 0x9b vs our
  0x98) — the merged writer's handle-stream
  close pads with ZEROS (write_spear_shift)
  while the author's final partial byte is
  1s (§19 H8d); every other corpus record's
  handle stream ends byte-aligned so only
  h=BF2 exposed it. THE FIX: the writer
  re-creates the author's 1s pad explicitly —
  the captured handle bits extended to the
  byte boundary with 1s (the reader's ≤7-bit
  trim cut exactly her pad; the corpus
  records' handle streams end in a 0 bit or
  aligned, so the trim never over-cuts). (4)
  **THE NORMALIZE-LAYER REGRESSION, caught
  by the family smokes**: the new wire_*
  model fields first leaked into the semantic
  compare as extra_in_silver rows on the
  R2000/R2004 smokes (6+6 diffs) — popped in
  the UNKNOWN_ENT path (the dwg_raw_tail_bits
  precedent: writer-side fidelity channels
  have no gold counterpart). **MEASURED:
  example_2007 540/540 (streams 417109 =
  417109); THE FULL 58-FILE AC1021 SURVEY AT
  ZERO DIVERGENT RECORDS — 58/58 files at
  100%; the only remaining her-only set is
  ATMOS's 84 broken-map orphans
  (pre-existing, documented).** GATES: serde
  1602/0, gold_roundtrip ok, issue80 7/0,
  four family smokes 0/0/0/0, corpus 280
  files 0/0/0/0, generation identity
  UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes, feature-independent), the
  echo byte-identity held (circle cmp clean).
  THE HONEST FRAMING, FINAL: the conventional
  arm is now record-identical to the author's
  stream on EVERY AC1021 corpus file — the
  residual named-record inventory is EMPTY;
  what remains outside the survey's reach:
  the ATMOS 84 her-only orphans (the
  broken-map pre-existing issue), the
  pre-2007 constraint-group conventional arms
  (echo-covered, ungated), and the
  unattested/dead rows (the subcurve action
  types, the MT crc_seed draws, LoftD, BREP).

  **H8h-ext-13 — the pre-2007
  constraint-group arms closed (the TwoStream
  capture extension + the H8d ownerhandle
  form rule extended to the pre-2007 slots)
  (2026-09-28, the continuation)**: the first
  residual task resumed after the
  zero-residual landing. THE MEASUREMENT
  PROBLEM: the record-identity survey is
  AC1021-only (the reader's record-trace
  gate + the token-diff's AC21 page-system
  raw dump) — the era proof was built via
  the gold -v9 TRACE-SEGMENT COMPARISON
  instead: the constraint-group object's
  full decode block (the field prints + the
  unknown_bits hex + the handle prints + the
  CRC print), her original vs our conv, the
  file-address lines stripped. The CRC-16
  covers MS+span, so matching CRCs = matching
  bytes — a record-level identity proof.
  THE DISSECTION FINDINGS: (a) our TwoStream
  close ALREADY mirrors the authored
  bit-continuous frame (§19.4.C) — the merge
  trace census: main_bits == handle_start_bits
  on 235/235 records of the R2000 specimen
  (the RL patch seeks back and the handle
  bytes overwrite bit-continuously into the
  partial byte; the final 1s pad closes); (b)
  her pre-2007 ownerhandle forms follow the
  SAME H8d rule as AC1021 — the census: (8.0)
  44×, (12.1) 70×, (4.1) 82×, (4.2) 17×,
  (4.0) 11×, (10.1) 5× = exactly the
  relative-iff-shorter + numeric-tie-break
  distribution; (c) the node region's inline
  class-name TVs (pre-2007) sit INSIDE the
  captured main bits — no separate names
  capture. THE FIXES: (1) the ext-8 capture's
  era gate extended from AC1021 to
  AC1015|AC1018|AC1021 (the names read stays
  AC1021-only — the TwoStream TVs are inline);
  (2) the constraint writer's handle-tail
  emission gets the ext-12 explicit-1s pad
  extension (a no-op for the byte-aligned
  AC1021 specimens — Constraints 219/219
  re-verified); (3) the H8d
  `write_first_ref_handle` ownerhandle form
  rule extended to the pre-2007 slots — the
  `r2007_plus` gates dropped in BOTH common
  writers (the entity entmode==0 slot and the
  non-entity owner slot). **MEASURED: the
  R2000 and R2004 Constraints.dwg
  constraint-group records round-trip
  BYTE-IDENTICAL (the CRCs match — 47A0 ==
  47A0, 4B0D == 4B0D; the unknown_bits hex
  identical; the only trace diffs are gold's
  file-address prints); the first attempt's
  +1-byte handle stream (her (12.1.3) vs our
  (4.2.3E0) ownerhandle) closed by the form
  rule.** GATES: serde 1602/0, gold_roundtrip
  ok, issue80 7/0, four family smokes 0/0,
  corpus 280 files 0/0/0/0 (the ownerhandle
  form change touches EVERY pre-2007 record —
  the semantic axes are form-agnostic and
  held), generation identity UNCHANGED
  (`84374e73…`, 25,375 bytes — the artifact
  is AC1032/R2018, outside the TwoStream
  change), the echo byte-identity held, the
  full AC1021 survey re-verified 58/58 / 0
  divergent. THE REMAINING ERA RESIDUAL: the
  R2010/R2013 constraint-group arms (the
  MC/BOT frames + the authored underlap need
  their own dissection) and the pre-2007
  FULL-FILE record identity (the era survey
  instrument = the token-diff's page-system
  extension — a future packet; today's proof
  is record-level).

  **H8h-ext-14 — the R2010/R2013
  constraint-group arms closed (the raw
  text-region capture; the era census
  instrument) (2026-09-28, the
  continuation)**: the ext-13 row's named
  residual fell. THE ERA FRAMES: the R2010/
  R2013 specimens carry the same nine-node
  group with `has_strings: 1` (R2010
  data_size 3546 = the 2007 stream exactly;
  R2013 2332 — DIFFERENT content, the
  drawing re-authored by a newer AutoCAD:
  node[0] id 0/status 0x20 vs the 2007
  9/0x40), and gold's own -v9 walk desyncs
  at node[1] on both (nconn 2800028726 /
  68456580 — the same ERROR print as
  AC1021). Without the capture our conv
  re-emitted the garbage model (the R2013
  record: 205,680 bits, 100,073 trace
  lines). THE FIX: (1) the ext-8 capture's
  era gate extended to AC1024/AC1027 (the
  MC handle-bits header, the BOT type, the
  flag at handle_start−1 — the bounds all
  frame-derived, the peek-based capture
  immune to the naive walk's desync); (2)
  the R2010+ TEXT region retained VERBATIM
  (`nodes_wire_text` — the ext-12
  TABLECONTENT `wire_text` pattern) instead
  of re-encoding class-name TUs: the AC21
  raw-stream dump instrument does not cover
  the R2010+ containers, so the stream
  content was never decoded (the names-read
  would be a guess; the raw capture is
  content-agnostic); AC1021 keeps the
  verified names path. THE ERA CENSUS
  INSTRUMENT (the handle-keyed -v9
  comparison): her original vs our conv,
  records matched by HANDLE (the index
  match is invalid — our conv rebuilds the
  section in the writer's order), each
  record's identity = (size, CRC-16) —
  gold prints the CRC check per record, so
  the census needs no raw-stream dump.
  **MEASURED: the R2010 constraint-group
  record round-trips byte-identically (CRC
  969C == 969C) and the R2013 likewise
  (36AD == 36AD); the era censuses: R2010
  215/216 records identical, R2013 157/160
  — the constraint group is in the
  identical set on both.** THE NEW NAMED
  RESIDUALS (the era full-file identity
  packet's territory): h=87 TABLESTYLE
  (both eras, ours +1 byte — the modern
  cell-style arms) and h=3E4/h=3E5
  ASSOCGEOMDEPENDENCY (R2013 only, ours +2
  bytes each — the ext-10 persubent-id
  tail capture is era-ungated but the
  R2013 records still diverge; a different
  root, likely the modeled prefix). GATES:
  serde 1602/0, gold_roundtrip ok, issue80
  7/0, four family smokes 0/0, corpus 280
  files 0/0/0/0, generation identity
  UNCHANGED (`84374e73…`, 25,375 bytes,
  feature-independent), the echo
  byte-identity held, the full AC1021
  survey re-verified 58/58 / 0 divergent.

  **H8h-ext-15 — THE ERA FULL-FILE
  RECORD IDENTITY CLOSED: all four era
  censuses at ZERO divergent records —
  seven systematic roots fell (2026-09-28,
  the continuation)**: the ext-14 census
  instrument turned on the R2000/R2004
  specimens for the first time: 164/235
  and 157/227 records diverged (vs the
  near-perfect R2010/R2013) — a
  never-measured surface. Seven roots
  fell, in census order: (1) **the
  pre-2007 TV terminator** — the authored
  wire counts the NUL in the length
  (BS(len+1) + chars + NUL; her LAYER "0"
  spans 26 bits vs our 18); the READER
  already parsed it right (count bytes →
  strip NULs — its own correctness proved
  the convention), only the WRITER's
  count-excluded form was wrong; the
  single fix closed 159+155 records. (2)
  **the VISUALSTYLE legacy_properties
  gate** — the pre-R2007 wires carry 23
  properties (the R2007-only bd2007_45 is
  absent) and the accessor's len==24-only
  gate fell back to CORE DEFAULTS (every
  VISUALSTYLE −25 bytes, our conv decoding
  face_opacity 0 where her wire carries
  −0.6); the accessor now synthesizes the
  never-emitted 24th slot at ACCESS time
  (the model keeps its faithful 23 — the
  semantic projections match gold; the
  first attempt padded the reader's model
  and the smokes caught the bd2007_45
  extra_in_silver leak). (3) **the
  PLACEHOLDER wire type** — PER-FILE
  variance (the R2000 specimen writes the
  class-based 501; R2004/R2010 the fixed
  80 — the ext-7 lesson again): the
  dispatched raw code is captured on the
  read (`wire_type_code`) and re-emitted;
  the fixed 80 stays the DXF/programmatic
  fallback. (4) **the entity chain
  handles** — the pre-2004 prev/next links
  follow the H8d first-ref form rule (her
  CIRCLE (12.2.1E0) = own−offset at equal
  length; the rule's `offset < handle`
  tie-break) — routed through
  write_first_ref_handle. (5) **the EED
  retained-block bytes** — the H8h
  replace-in-place overwrote the author's
  AcadAnnotative block with the synthesized
  marker (codepage 0 where her blocks carry
  30): a DWG read's retained raw block now
  keeps BOTH its position AND its bytes;
  the synthesis fires only for apps with
  no retained block (the DXF/programmatic
  path). (6) **the R2013 geomdep text
  stream** — her records carry
  has_strings: 0 (no stream; the classname
  TU reads "" at 0 bits) where our
  emission ran +18 bits; the stream's
  PRESENCE is PER-RECORD wire state: the
  first attempt (a blanket all-empty-
  stream drop in the merge) REGRESSED 22
  AC1021 records (their authors write
  has_strings: 1 with empty-only streams —
  the ext-7 lesson a third time) and
  changed the generation identity; REVERTED.
  The fix: the reader captures
  `wire_no_text_stream` at the classname TU
  (text_remaining_bits() ≤ 0, gated to
  R2007+) and the writer skips the TU so
  the merge emits no stream — the
  generator (DXF-built, flag false) is
  untouched. (7) **the
  TABLESTYLE modern_style resolution** —
  the writer resolved the null cell-style
  text_style into the "Standard" handle
  (the era censuses' single +1-byte
  divergence, h=87 on R2010/R2013); on a
  DWG read the retained NamedTableCellStyle
  is the wire truth — verbatim; the
  resolution rides the DXF/programmatic
  path. **MEASURED: the era censuses ALL
  AT ZERO — R2000 235/235, R2004 227/227,
  R2010 216/216, R2013 160/160 (every
  record byte-identical: size + CRC-16).**
  GATES: serde 1602/0 (the VisualStyle
  model keeps 23 — no test changes),
  gold_roundtrip ok, issue80 7/0, four
  family smokes 0/0/0/0 (the two
  wire-capture field leaks — the
  PlaceHolder wire_type_code popped in the
  normalize; the bd2007_45 leak fixed by
  the accessor-side pad), corpus 280 files
  0/0/0/0, the full AC1021 survey
  re-verified 58/58 / 0 divergent, the
  echo byte-identity held. **THE
  GENERATION IDENTITY UNCHANGED**
  (`84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes, feature-independent) —
  the blanket all-empty-stream drop (which
  changed the artifact to
  `3969f799…`) was REVERTED with the
  survey regression; the per-record
  `wire_no_text_stream` capture leaves
  the generator (DXF-built, flag false)
  byte-identical.

  **THE R2004_Header RE-SCOPE (2026-09-27, the maintenance-review
  probes — the accepted residue re-measured and re-scoped under
  the gold-vs-silver mechanism)**: the H7g AC18 container-shape
  mirror (above) EXISTS and already closed −2,007 rows; the 963
  is its RESIDUE, not untouched territory. The fresh corpus-report
  scan (per-file `struct_rt` reads) verifies the remaining rows'
  anatomy: **address/ID fields — `last_section_address`,
  `secondheader_address`, `section_map_address`, `last_section_id`
  — spread over 147 R2004+ files, including the R2010/R2013/R2018
  containers** (Box_2010/2013/2018 all carry the class; the key
  family covers the whole AC18–AC28 container). THE OLD
  "4 × 225 engaged + 7 × 9" DECOMPOSITION IS STALE: the scan
  reads ~598 address rows across the 147 files vs the census's
  963 leaf count — the delta is the page-map array leaves (the
  per-page fields) plus counting-class differences; the packet's
  opening measurement must reconcile the two views before any
  work starts. RESOLVABILITY (the maintenance question answered):
  the rows close by the SAME mechanism as the AC1021 address
  rows — the existing H7g mirror machinery engaging the remaining
  files, which requires (a) the AC18 mirror's objects fit — the
  AC18 form of the H8d placement wall, answered by the SAME
  raw-echo re-scope (retain her objects-section stream, echo it
  in the mirror arm: her page space then reproduces every address
  by construction); (b) the remaining `AC18_MIRROR_DEBUG` decline
  classes enumerated per file and cleared (the seven fallback
  files' trace is the starting map); (c) the R2010/R2013/R2018
  container variants covered by the same retention+mirror
  (the reader parses all of them — the read axis is 0/0 at every
  version, so retention is plumbing). No new instruments, no
  custom examples; the corpus's 147 files are the surface, and
  the zero-keeping gates apply unchanged. STATUS: still a
  maintainer decision whether to spend the extension — but no
  longer an unmeasured unknown: the cost, the machinery, and the
  row surface are all now on record.

### 19.3 Standing rules for the campaign

- Both axes' zero-keeping runs under the same gates: every packet lands
  with `cargo test --features serde` green, the smokes 0/0, and BOTH
  corpus counters unchanged (OBJECTS 0 frozen; struct moves only
  toward 0).
- The fixture surface is NOT extended by this campaign (the corpus of
  280 stands; version-class coverage comes from the existing R2000 →
  R2018 spans). If a metadata section needs a genuine fixture (e.g. a
  FileDepList with entries), that is a maintainer ask, recorded as a
  blocked row — the no-authoring stance holds.
- The §18 (SH) decoder rows and the 8757a7c AcDs/section census brief
  stay live: the AcDs row here and that brief share findings but keep
  their own queues.

### 19.4 The external-mechanism reference (self-contained; no ODA-spec or libredwg consultation needed)

Recorded 2026-09-27 so future zero-context agents work from this
record + the in-tree implementations alone. Every mechanism the
remaining queue touches is stated here explicitly and has a
working anchor; where a rule is subtle it was verified against
gold this campaign (the anchor note says so).

#### A. DWG bitcode primitives (the wire forms)

- Bits are packed MSB-first within each byte; RS/RL/RD are
  byte-wise little-endian, each byte read MSB-first.
- BS: prefix '00' → +RS16 raw; '01' → +RC8; '10' → 0; '11' → 256.
- BL: '00' → +RL32; '01' → +RC8; '10' → 0; '11' → 256.
- BD: '00' → raw f64; '01' → 1.0; '10' → 0.0; '11' → NaN.
- MS/MC (modular short/char): 7-bit groups, the LOW chunk
  first in stream order, high bit 0 terminates; value is the
  sum of (byte & 0x7F) << (7*i) over the chunks. CAREFUL — this
  7-bit-group form is NOT the objects-section record-head size
  field: that MS is **15 data bits per 16-bit LE word, bit 15 of
  a word the continuation flag** (a record unit = [MS words]
  [size span incl. the final pad byte] [crc16 LE over MS..span];
  for circle obj%1 the MS bytes are `11 00` and the BS type 48
  decodes cleanly only from byte 2 — the H8h-ext survey verified
  the frame exhaustively via the crc16 window; the crate's
  `write_modular_short_bytes` matches).
- TV strings live in the R2007+ per-record string stream (see
  D), not inline in the data.
- IN-TREE: every primitive is implemented in `bit_writer.rs`
  and the reader's bit chain; `dump_section_bytes` prints hex
  + the MSB-first bit string for hand-walking; gold's `-v9`
  trace prints per-field `@byte.bit` positions in the same
  convention (the §8.1.6 recipe).

#### B. Handle references (H)

- Frame: `[RC: (code<<4)|count][count value bytes]`, the value
  accumulated big-endian (first byte = high).
- Resolution (verified against gold): codes 2/3/4/5 are
  ABSOLUTE (the ownership classes); 6 = own handle + 1;
  8 = own − 1; 10 = own + value; 12 = own − value. The null
  handle is code 4, count 0.
- Declared-vs-wire TYPE codes (the §19 H8h-ext-12 census):
  gold's dwg2.spec guesses some ACDBASSOC reference TYPES, and
  the authored wires disagree — the empty-values
  AcDbAssocParamBasedActionBody dependency is declared 5
  (HardPointer) but written 4 (SoftPointer; example_2007
  h=392: (4.2.397)), and the compound child.parameter is
  declared 3 (HardOwnership) but written 4 (SoftPointer;
  example_2007 h=393: (4.0.0)). The same split as the
  H8h-ext-3 deps census {3: 27} and the H8h-ext-4 edge param
  (declared 3, written 4): the wire is the authority; fix the
  writer's type when the census is uniform, capture the code
  when authors diverge.
- First-in-stream ownerhandle form rule (the author's writer,
  verified 196/196 on circle_2007): choose the RELATIVE form
  iff its byte length is shorter, or at equal length iff the
  absolute offset is numerically less than the owner value.
  Lengths: rel = 1 if offset==1 else 1+bytecount(offset);
  abs = 1+bytecount(value).
- IN-TREE: `bit_writer.rs::write_handle` (absolute),
  `write_handle_relative` (codes 0x60/0x80/0xA0|n/0xC0|n —
  implemented); the `write_first_ref_handle` helper + the two
  call sites are the H8d-α re-apply package (recorded in
  NEXT_SESSION's H8d section).

#### C. Object record frame (R2004+; verified vs gold decode.c)

```
[MS size]                        size counts everything below
[R2010+: UMC handle-bits]        NOT counted in size
[BS (up to R2007) / BOT (R2010+) type]
[data bits ...]
[R2007+: string stream + flag bit (see D)]
[handle stream: ownerhandle first, packed starting at the
 DATA-END BIT — mid-byte, no alignment]
[1-bits padding to the byte boundary]   <- the author pads
                                          with ONES (verified
                                          AC15/AC18/AC21)
[crc16 0xC0C1 over MS..span — 2 bytes AFTER the span]
```
- The spec's "RS CRC inside the size" is a PHANTOM: gold
  never validates a per-object CRC on these paths (no
  per-object verdict lines at -v9); the only real per-record
  checksum is the after-span crc.
- R2004+ objects sections begin with the 4-byte 0x0DCA LE
  marker.
- The DWG CRC-16: `dx = ((dx>>8)&0xFF) ^ table[(byte ^
  (dx&0xFF)) & 0xFF]`, seed 0xC0C1; table[i] = 8 rounds of
  `crc = (crc>>1) ^ (0xA001 if crc&1 else 0)` (entry 1 =
  0xC0C1 verifies the table).
- IN-TREE: `merged_writer.rs::merge_two_stream` /
  `merge_three_stream` build exactly this shape (the final
  1s-pad variant is part of the α re-apply package);
- The pad × wire-capture interplay (the §19 H8h-ext-8/-12
  lesson): a verbatim capture that runs to the record end must
  TRIM the author's closing 1s pad (≤7 bits) — the writer
  re-creates it at close — and a capture re-emitted through the
  HANDLE stream must re-create the pad EXPLICITLY with 1s: the
  merged writer's own handle-stream close pads with ZEROS
  (`write_spear_shift`), so a trimmed capture replayed raw ends
  0-padded where the author pads 1s (the h=BF2 landing: the
  right record size with a wrong final byte, her 0x9b vs our
  0x98). Corpus records whose handle streams end byte-aligned
  never expose it.
  `crc.rs::crc16` + `crc::CRC16_SEED`; `register_object` /
  `register_raw_object` in `object_writer/common.rs`.

#### D. The R2007 per-record string stream

- The RL bitsize field after the type gives the data-end bit;
  when strings are present the flag bit at the data end is 1
  and a short size value sits 128 bits before the end (bit
  0x8000 flags a second short; combined 30-bit size); the
  string stream spans downward from there and holds ALL of the
  record's TV strings.
- IN-TREE: `merged_writer.rs::merge_three_stream` implements
  the placement (main/text/handle merge with the flag words);
  the reader's counterpart decodes it 0/0 corpus-wide.

#### E. AC21 LZ77 (the objects-section codec)

- Stream: `[literal-run opcode][reordered literals]` then
  match chains. Literal length opcode = count−8 (0x0F →
  23+continuation). The first opcode may be the 0x2X pseudo-op:
  the decoder skips 4 bytes and takes the leading literal
  count from byte[3]&7.
- Match classes: compact len 3–14 off ≤512 (2 bytes); short
  3–18 off ≤8192 (3); long 19–50 off ≤4096 (3, high nibble 0);
  extended len ≤255 with 16-bit offset (4); extended-long
  len ≥256 (5).
- After each match the next-op byte's low 3 bits are the
  trailing literal count: nonzero → literals follow and a
  FRESH chain block reads the next op (any class, including
  nibble 0); zero → the next op: nibble 0 EXITS the chain (a
  literal-length byte), nibble 0xF is REMASKED to 0 and read
  as an in-chain long match (the author's in-chain long
  form — her census uses it heavily).
- Literal runs ≥32 bytes are reordered by 8-byte group
  reversal; shorter tails via the 31-entry sub-32 swap table —
  both implemented verbatim in `decompressor_ac21.rs`
  (`decompress_copy_n_reordered` is pub) and byte-verified
  against gold's table.
- IN-TREE: `compressor_ac21.rs::compress_ac21` (measured
  BEATING the author's ratio on identical content),
  `decompressor_ac21.rs::decompress_ac21` (pub; pad the source
  +64), and the harness bin `ac21_token_diff`: per-page stream
  extraction (RS de-interleave, factor 1), a decoder-exact
  token walker, the H8b mirror-sim window scorer, and raw
  stream dumps via `AC21_DIFF_RAW_DIR`.

#### F. The containers (page systems)

R2007 (AC1021):
- Data pages: RS(255,251), repetition factor 1; a compressed
  page is de-interleaved at stride bc (blocks of 251). Stored
  vs compressed dispatches on SIZE ONLY: raw when
  `comp == uncomp || size == page_size_if_rs_coded(comp)`.
- System pages (the sections/pages maps): RS(255,239) with a
  repeat count; page_size = align8(ceil(pesize/239) x 255)
  with pesize = align8(compressed) x repeat — use the
  implemented `get_system_page_size`.
- rs_form for data pages:
  `page_size_if_rs_coded(len) = align32(ceil(align8(len)/251) x 255)`.
- The pages map: (id, on-disk size) pairs in physical order
  from file offset 0x480 (the running sum gives addresses);
  the sections map: 16-byte entries per the §18 record.
- IN-TREE: `dwg_reader.rs::decode_ac21_page` +
  `get_page_buffer_at`; the H8a retention
  (`document.dwg_ac21_shape`: map_order + per-section page
  plans); the H8b gate/emission/finalize
  (`file_header_ac21.rs::mirror_plan`, `write_mirrored_pages`,
  `write_file_mirrored`) with `AC21_MIRROR_DEBUG=1` decline
  traces; `reed_solomon.rs::reed_solomon_decode` (pub).

R2004/R2010/R2013/R2018 (the AC18–AC28 family):
- The H7g doctrine and machinery: per-descriptor maxdecomp
  frames (the author's custom caps measured: AppInfo 0x300,
  AppInfoHistory 0x580, Preview 0x7C00, SummaryInfo 0x80),
  box ids at data_count+3/+4, the SummaryInfo-first@0x100
  physical order, RS(255,239) system pages.
- IN-TREE: the H7g retention (`document.dwg_ac18_shape`),
  `ac18_mirror_plan` + `AC18_MIRROR_DEBUG=1`, the conventional
  `write_ac18`. All four container versions read 0/0.

#### G. The CRC random encoding (the ONE open algorithmic unknown)

- Purpose: some header CRC values are stored as UInt64s = 10
  data bits spread evenly + 54 pseudo-random bits.
- The engine (the ODA §5 pseudocode, reproduced in full; the
  spec's §5.11 tables follow exactly):
  `t[0] = (uint32)seed * 0x343fd + 0x269ec3`;
  `t[1] = (uint32)(seed >> 32) * 0x343fd + 0x269ec3`;
  `value = t[1]; t[i] (i >= 2) = (((value >> 0x1e) ^ value) *
  0x6c078965) + i` iterated for i in 2..0x270 — all stored
  little-endian; a 0x80-entry padding table is derived from
  the same table; the index loops with wrap at 0x270; the
  Encode method spreads the 10 bits and fills the rest from
  the table.
- IN-TREE: `file_header_ac21.rs::CrcRandomEncoder` (:95,
  with these exact constants) + the H8b mirror's raw table
  use.
- KNOWN STATE (recorded at H7g time): our sequence diverges
  from the author's draws at the same seed+crc_seed — the
  first draw already differs (measured on example_2007). The
  remaining work is pinning the author's MT-variant
  bit-exactly; every corpus file supplies (seed, her draws)
  as ground truth on disk, so NO fixture is needed. The
  3-per-file R2007_Header derive family
  (`sections_map_crc_seed`, `pages_map_crc_seed`,
  `crc_seed_encoded`) closes when this is pinned.

#### H. The harness surface (all implemented; the campaign loop)

- Gates: `cargo test --features serde`;
  `cargo test --features gold-harness --test gold_roundtrip`;
  `python3 tests/gold_harness/run_roundtrip.py FILE DIR`;
  `python3 tests/gold_harness/run_corpus.py` (the
  authoritative counters; report.md + per-file struct JSONs
  under `target/gold_harness_corpus/`).
- Record forensics: gold `dwgread -v9 FILE 2> trace.log`
  prints per-object `[Object number/size/type/address]`,
  `bitsize` [RL], `Hdlsize`, hdl/str extents in
  `@byte.bit` window coordinates, per-field positions, handle
  forms and the padding readings (the §8.1.6 walk); pair with
  `dump_section_bytes` (hex + MSB-first bit string) and
  `ac21_token_diff`.
- Decline traces: `AC21_MIRROR_DEBUG=1` /
  `AC18_MIRROR_DEBUG=1` prefixed on any run_roundtrip
  enumerates the per-file gate reasons.
- Zero-keeping: the stash-run corpus pattern (git stash the
  writer slice, run the corpus, pop) — measured evidence
  before any landing; both corpus counters must hold or
  improve (§19.3).
- The generation identity canary: after any writer change run
  `cargo run --example gen_all_entities_all_versions_dwg
  --features serde` and `md5sum
  gen_all_entities_all_versions.dwg`; record the new value in
  the halt — a tracked fact, not a frozen constant.
