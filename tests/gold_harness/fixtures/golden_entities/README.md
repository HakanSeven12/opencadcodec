# golden_entities — the per-version entity fixture campaign (TODO C4 reopened)

The 2026-10-03 maintainer call ("generate per version supported in
cadcodec all supported entities separately to replace libredwg
test-files and beyond") reopened the C4 direction. The campaign authors
**every cadcodec-supported entity family per DWG version headless**,
per the README_HEADLESS.md mechanism, sorted **per version**:

```
golden_entities/AC1012/  Point_AC1012.dwg + .scr + .txt   (R13 - BricsCAD)
golden_entities/AC1014/  ...                              (R14 - AutoCAD floor)
golden_entities/AC1015/  ...                              (R2000)
golden_entities/AC1018/  ...                              (R2004)
golden_entities/AC1021/  ...                              (R2007)
golden_entities/AC1024/  ...                              (R2010)
golden_entities/AC1027/  ...                              (R2013)
golden_entities/AC1032/  ...                              (R2018)
```

30 families × 8 versions = 239 version-cells; **221 fixtures landed**,
8 recorded refusals, 10 version-gated cells (below). Every `.dwg` sits
with its `.scr` (the exact replication record) and `.txt` (provenance:
engine, seed, op, save token, qualification, gold census).

## The matrix outcome (2026-10-03, all runs transcript-recorded)

| family | AC1012 | AC1014 | AC1015 | AC1018 | AC1021 | AC1024 | AC1027 | AC1032 |
|---|---|---|---|---|---|---|---|---|
| Point..Tolerance (18 families) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Shape | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Viewport | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| HatchSolid / HatchLines | **R14+** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| MLine, Polyface, Region, Solid3d, Insert | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| LWPolyline | ✓* | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Multileader | gated | gated | gated | gated | ✓ | ✓ | ✓ | ✓ |
| Mesh | gated | gated | gated | gated | gated | ✓ | ✓ | ✓ |
| Body | REFUSED | REFUSED | REFUSED | REFUSED | REFUSED | REFUSED | REFUSED | REFUSED |

*LWPolyline at AC1012: the R13 downsave converts the light polyline to
the era-native heavy `POLYLINE_2D` (+`VERTEX_2D`s +`SEQEND`) — the B3
campaign's two-class datum; the surviving class per era IS the
measurement, recorded per file.

## Engines (AutoCAD preferred, BricsCAD fallback — the measured map)

- **AC1012 (R13)**: BricsCAD V26 hidden `/Automation /b` — AutoCAD's
  save floor is R14 (the B3 toolchain record). All 25 R13 families are
  BricsCAD-authored, including `Region`/`Solid3d` (the ACIS solid
  SURVIVES the R13 downsave — gold reads both typed) and `Shape`
  (BricsCAD LOADs AutoCAD's `ltypeshp.shx` and places the `BAT` shape).
- **AC1014+**: AutoCAD 2027 core console (`/i seed /s scr`), all
  recipes pre-validated by the probe battery: entmake for the 14
  direct families, REGEN+ename for Region, typed-point commands for
  the rest (the `Shape` LOAD+SHAPE route with a QUOTED shape-file path
  — a bare path splits at spaces; `Viewport` via `CTAB`+`MVIEW` on
  Layout1; `Mesh` via `MESH _BO` first-corner/opposite-corner/height).
- **Body**: refused on BOTH engines at every version — INTERFERE is
  non-functional in accoreconsole (the `(command)` returns with no
  prompt output — the constraint-family/module-absent signature) and
  BricsCAD's INTERFERE completes but creates no BODY (`BODY=0`; only
  the extruded solid persists). Consistent with B4's no-user-facing-
  lever class; the 8 refusal `.txt`+`.scr` records carry the verbatim
  measurement. Re-opens only if an authentic specimen surfaces.

## Version gates (era truth, all measured in-campaign)

- **HATCH is R14+**: at R13 the downsave explodes the hatch (the B3/
  r13_r14 datum) — the two hatch families are gated out of AC1012.
- **MULTILEADER is AC1021+**: the MLEADER command (2008) writes the
  2007 format — confirmed by the pass at AC1021.
- **MESH is AC1024+**: the mesh primitives (2010) — confirmed by the
  pass at AC1024.

## Qualification gates (every landed file)

1. in-app census via the LOGSEC result file (`ssget` count + `last=`),
2. `dwgread -O JSON`: exit 0, zero `Error` lines,
3. **file-level** gold entity census: the family's class present in
   the saved file (catches save-down conversions),
4. magic bytes == the version code (`AC1012`..`AC1032`).

## Corpus note

These stems carry version codes (`Point_AC1012`), not the `r13`/`r14`
substrings — the corpus driver's parked-campaign guard does not
exclude them, so the golden files are IN-SCOPE for the fixtures walk
(the maintainer's "replace the libredwg test-files" intent; the parked
`r13_r14` B3 wave keeps its own guard).
