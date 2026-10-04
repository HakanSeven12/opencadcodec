# brep — the B4 ACSH_BREP_CLASS dataset (FROM SCRATCH, fully self-authored)

**The 2026-10-03 replacement** (the maintainer's call): the earlier
carrier re-emission files are gone — this dataset is authored **from a
naked `acadiso.dwt` seed**, no legacy content anywhere in any chain.
B4's "external-only" verdict is falsified by measurement: the class is
mintable from scratch, headless, in one script.

## The recipe (the mint discovery)

```
(setvar "SOLIDHIST" 1)              ; THE switch — profile default 0
                                    ; strips boolean history (the D7
                                    ; control; the confound behind B4's
                                    ; entire negative map)
_.BOX  5,5,0 → 15,15 → 10          ; the parametric operand
_.MESH _BO 0,0,0 → 10,10 → 10      ; the raw-geometry source
_.CONVTOSOLID <mesh>                ; mesh → framework-less naked solid
_.UNION <box> <solid>               ; the framework records the chain —
                                    ; the raw operand's node IS
                                    ; ACSH_BREP_CLASS
```

The saved file: one `3DSOLID` (both operands consumed) + the four-node
SH chain `ACSH_BOX_CLASS` + **`ACSH_BREP_CLASS`** + `ACSH_BOOLEAN_CLASS`
+ `ACSH_HISTORY_CLASS` — a minimal, fully typed census. The UNION is
the fixture's operation; the operand constructions are its
prerequisites (the Hatch-fixture precedent). Each `.dwg` sits with its
`.scr` (the exact replication script — self-contained, no external
files) and `.txt` provenance.

## The retention map (measured 2026-10-03, fresh content)

| version | ACSH_BREP_CLASS | BREP record `major/minor` | silver pair (read/write) |
|---|---|---|---|
| AC1012 (R13) | **REFUSED — both engines closed** (AutoCAD: no R14-below save; BricsCAD: its boolean records a bare HISTORY root only, no retention switch exists — measured at R13 *and* 2018) | — | refusal record |
| AC1014 (R14) | **minted** | **64 / 0** | **0 / 0** |
| AC1015 (2000) | **minted** | **64 / 0** | **0 / 0** |
| AC1018 (2004) | **dropped** — the downsave strips the whole SH object family (same as the carrier re-emission; a container property, not content-dependent) | — | refusal record |
| AC1021 (2007) | **minted** | **3528495168 / 83** | **0 / 0** |
| AC1024 (2010) | **minted** | **3528495168 / 83** | **0 / 0** |
| AC1027 (2013) | **minted** | 3528495168 / 83 | 6 / 6 — the R2013+ trailing region (below) |
| AC1032 (2018) | **minted** | **3545534528 / 32** | 7 / 7 — the R2013+ trailing region (below) |

Notes:
- **The `major` field is container-coded, not content-coded**: the
  fresh mints reproduce the exact per-version values the carrier
  re-emissions showed (64 at R14/2000, 3528495168 at 2007–2013,
  3545534528 at 2018) — the derailed-field dissection datum, now
  double-sourced.
- **The 0/0 quartet** (R14/2000/2007/2010): silver round-trips the
  fresh-minted records record-exactly — a materially stronger result
  than the carrier re-emissions (which carried 21/22/43-row era
  residues). The fresh content is minimal and fully inside silver's
  typed model.
- **The R2013+ trailing region** (6–7 rows, `missing_in_silver`):
  the BREP record's `materials` + `has_revision_guid` + `revision_*` +
  `end_marker` fields — the R2013+ trailing form silver's typed
  `ACSH_BREP_CLASS` model does not emit yet. A named raw-remainder
  packet with two clean per-version specimens (the B2/A8 pattern
  applies: dissection → typed model → bit-exact pin).
  **CLOSED 2026-10-04 (the raw-remainder packet)**: the records'
  wire version sits OUTSIDE {1,2} (0 at 2007–2013, 38438 at 2018) —
  gold's unstable-class walk reads no body and walks the
  COMMON_3DSOLID tail from the modeler blob's first bits. Silver
  mirrors that walk (typed, projection-only) and captures the whole
  tail verbatim as the write authority (`raw_tail` on `AcisData`,
  serde-skipped); the BREP class joined the elide allowlist; the
  normalizer projects the materials `[0]*count` + the six revision
  keys. The six files read 0/0/0/0 and the conventional rewrites
  are record-identical (153/153, 141/141, 206/206, 203/203 — the
  R13/R2000 SAT-era mints carry a +4-bit typed-field delta on the
  conventional arm, the era campaign's Tier-2 surface).
- **The 2004 drop** is a container property (the same wholesale SH
  strip the carrier re-emission showed) — recorded as the refusal, not
  a writer question.

## Engine notes (the measured dialect map this campaign adds)

- AutoCAD 2027 core console authors everything (its MESH →
  CONVTOSOLID door needs the `SOLIDHIST=1` switch to mint; with the
  default 0 even a plain two-box union saves as a bare HISTORY root —
  the D7 control that explains B4's original seven failures).
- BricsCAD V26: writes R13, and its `CONVTOSOLID` accepts **polyface
  meshes** where AutoCAD refuses them ("Object cannot be converted" —
  AutoCAD's converter wants MESH objects) — but its boolean modeler
  records no operand history at all (bare `ACSH_HISTORY_CLASS` at R13
  and 2018 alike; `SOLIDHIST` does not exist; even the parametric
  operand's node is discarded). The R13 era stays unreachable for this
  class on both engines.

## Corpus note

Stems are AC-coded — the corpus driver's parked-campaign guard does
not exclude them; the six files are IN-SCOPE for the fixtures walk.
The dataset satisfies §F2.1 cleanliness (naked seed, self-authored
chain, minimal census) — unlike the carrier re-emissions it replaces,
these are first-class one-op fixtures.
