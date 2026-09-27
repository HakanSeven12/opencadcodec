# Zero-context prompt — TARGET ZERO held; the conventional arm at 25 residual records (H8h-ext-4: the CALL_SUBCURVE bodies reverse-engineered from the specimens)

> Campaign state 2026-09-27 (the halt after the H8h-ext-4 landing; the
> session continued the loop from the H8h-ext-3 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0, write-target
> 0** — verified after the landing). The ACS/SH campaign stays COMPLETE
> at 0/0. The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3
> + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's
> H8d–H8h-ext-4 rows carry this arc's records), then this file top to
> bottom.

## The arc (2026-09-27, the continuation session — H8h-ext-4)

1. **The authority closure** (the hints session, committed `cc1a6c8`):
   the subcurve wire is unknown to BOTH authorities — libredwg's full
   history (5 commits touch SUBCURVE, none implementing; `subent` a
   bare placeholder; the frozen HEAD parks the region in unknown_bits)
   and the ODA PDF (zero hits for any ACDBASSOC term). The corpus
   specimens are the only authority.
2. **The dissection**: a temporary reader trace
   (`DWG_SUBCURVE_TRACE`) printed the main-stream position after
   `action_type` and the main end per record — all seven records are
   action_type 11 (ARC), region = span bits 96..main_end (88–344
   bits). The region sizes are all ≡ 24 (mod 64) — the first hint of
   the BD structure. Walking each region as a sequence of standard
   BDs (2-bit prefix; 00 = full 66-bit LE double; 01 = 1.0; 10 = 0.0):
   **every region walks EXACTLY as twelve BDs** — center (3BD),
   normal (3BD), x-axis (3BD), radius (BD), start_angle (BD),
   end_angle (BD). The values match the fixtures' known geometry
   exactly (centers and radii = the source CIRCLEs; all seven are
   full circles: start 0.0, end 2π — the `18 2D 44 54 FB 21 19 40`
   LE tail; normal (0,0,1), x-axis (1,0,0)).
3. **The landing**: the `AssocArcSubcurve` model (serde-default
   Option on `AssocEdgeActionParam`); the reader parses the twelve
   BDs after action_type (the attested ARC form; other action types
   have no specimens and stay unread); the writer emits them when
   captured. PLUS the `param` ref code fix: the first survey pass
   after the subcurve landing showed a 2-bit tail delta — the
   handle-stream decode proved her null param = **(4.0.0)**
   (SoftPointer) where our writer emitted (3.0.0) (HardOwnership);
   the wire evidence is uniform across all seven records,
   overriding gold's spec declaration of 3.
4. **Measured: 32 → 25 divergent records; 47/58 files at 100%.**
   ExtrudeCSurf 216/216, ExtrudeM 216/216, RevolveM 217/217,
   LoftCSurf 221/221, LoftM 221/221 — the whole assoc genus closed.
   circle 211/211 held.
5. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0.
   Generation identity UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
   25,375 bytes, verified with AND without `--features serde` — the
   generator emits no EDGEACTIONPARAM records).

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
EXCLUDED from the fingerprint; `xdic_by_handle` and the modeled
subcurve fields are INCLUDED (real geometry — an edit declines the
echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 47 of the 58 AC1021 corpus files, with **25 named
residual records** in six classes (below).

## The residual 25 (each a named, byte-level-scoped packet)

- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
- **ATMOS (5 + 84 her-only)**: the controls h=2 16v15, h=3 20v15,
  h=77 96v40, h=352 25642v35 (a 25KB ACIS mass we emit as 35
  bytes), h=541 17v18; the 84 her-only = the broken-map orphans
  (pre-existing, documented).
- **Chamfer/Fillet (4)**: the UNKNOWN bodies (1509/1463 vs 28) +
  the 22v26/30 records (the fillet/chamfer node's unknown_bits
  mass).
- **The Loft 29-vs-27 class (5)**: Loft3/C/H/R/_ (2DD/2DE/2E4/2EF)
  — her loft UNKNOWN_OBJ record carries 2 bytes ours lacks.
- **The singles**: Constraints `3E3` (666 vs 870); PolyLine3D `1C2`
  (16 vs 17 — OURS longer).

## The remaining work (all optional — the target stays reached)

- **The residual classes above** — each an autopsy → census →
  rule → gates → re-survey packet in the H8h tradition; the
  §19.2 H8h-ext-4 row carries the map and the method.
- **The unattested subcurve action types** (17=ELLIPSE, 19=LINE,
  23=LINESEG3D, 42=NURB3D, 27=CURVE3D): no corpus specimens — the
  twelve-BD ARC form is the only attested layout; a future
  authentic specimen extends the match arm.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (the echo path never runs it for unedited roundtrips).
- **The dead/no-path rows**: `LoftD`; the SH revolve option shorts;
  **BREP stays deferred** (external authentic ACSH_BREP_CLASS
  specimen required).

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-4). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **47 files at 100%, 25 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  ExtrudeCSurf 216/216; LoftCSurf 221/221.
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
  libredwg `bits.c`. NEITHER documents the ACDBASSOC* classes or
  the subcurve wire (the authority closure, commit `cc1a6c8`).
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). The `bitsize` RL =
  main+text bits = the handle-stream start (R2007); the record
  span = bitsize + hdlsize + pad. The BD forms: 00 = full 66-bit
  LE double, 01 = 1.0, 10 = 0.0 (the author uses the shorts —
  our writer matches). The subcurve dissection recipe: the
  `DWG_SUBCURVE_TRACE` reader trace (removed after the landing —
  re-add at the EDGEACTIONPARAM arm if a new action type appears).

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
# current state: 47 files at 100%, 25 divergent records total
# circle 211/211; ExtrudeCSurf 216/216; LoftCSurf 221/221

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
cc1a6c8 <docs> the subcurve authority-search closure (the prior halt head)
<feat> H8h-ext-4: the CALL_SUBCURVE bodies reverse-engineered — the
       twelve-BD ARC form (center/normal/x-axis/radius/angles) +
       the param ref code 4 (32 -> 25; 47/58 at 100%)
<docs> this halt refresh — the post-H8h-ext-4 record
```

**PUSH STATE (2026-09-27)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-3
halt → the authority closure committed → the dissection (the
DWG_SUBCURVE_TRACE positions; the region sizes ≡ 24 mod 64 hint;
the BD walk cracking the twelve-field form; the geometry match
against the fixtures' known circles) → the landing (the model +
reader + writer + the param ref code fix found by the handle-stream
decode of the 2-bit tail delta) → the gates (serde 1602/0,
gold_roundtrip, issue80, family smokes, corpus 0/0/0/0, generation
identity unchanged and feature-independent) → the full survey
(32 → 25; 47/58 at 100% — the whole assoc genus closed) → the docs
(§19.2 H8h-ext-4 row; this halt record). **The maintainer's loop
instruction — "repeat process until target = zero" — remains
satisfied: the corpus is at zero on every axis; the conventional
arm is record-identical on 47 of 58 AC1021 files, and the residual
25 records are named, byte-level-scoped packets in six classes.**
