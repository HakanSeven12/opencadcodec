# Zero-context prompt — TARGET ZERO held; the conventional arm at 49 residual records (H8h-ext-2: the null-slot dictionary, the history_id code 3, the VIEW slots)

> Campaign state 2026-09-27 (the halt after the H8h-ext-2 landing; the
> session continued from the H8h-ext halt: the review → land → verify →
> commit → push → continue loop, three more form-rule packets in ONE
> landing. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280 files,
> read-fidelity 0, write-fidelity 0, read key-gap 0, write-target 0.**
> The ACS/SH campaign stays COMPLETE at 0/0. The §19 structure
> campaign's READ axis stays ZERO corpus-wide. Read
> `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3 + §18.5–18.7
> in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's H8d–H8h-ext-2 rows
> carry this arc's records), then this file top to bottom.

## The arc (2026-09-27, the continuation session — H8h-ext-2)

1. **Class (c) — the null-target dictionary items**: the
   `write_dictionary` R2000+/same-version arm filtered
   `!h.is_null() && is_writable_object(h)` — dropping the author's
   placeholder slots (the fixtures' `ACAD_PARALLEL_BACKGROUND →
   [2,0,0,0]` dictionaries: gold `numitems: 1`, ours 0). The silver
   MODEL carried the entry (verified via the model dump) — the
   writer's filter was the sole drop point; no test asserted the
   drop (the filter predates the campaign). Fixed: keep null-target
   items (a null target is authored state, not a dangling ref; the
   enqueue guard already skips nulls; the down-conversion arm
   untouched).
2. **Class (d) — the 3DSOLID-family `history_id` ref code**: the
   ExtrudeC record-tail autopsy (the corrected bit-walk + gold's
   -v9: `layer: (5.1.10) abs:10 @2262.3` — the record-relative
   positions INCLUDE the 2-byte MS; the layer ref matched) isolated
   ref2: her `(3, 2, 02, E0)` — the history root 0x2E0 — vs our
   `(4, 2, 02, E0)`. The gold census across the WHOLE corpus:
   history_id `[3, ..]` **128/128** non-null, the nulls `[3,0,0,0]`
   (the code-3-count-0 `30` byte) — never 4/5. Fixed:
   HardOwnership at the three write sites (3DSOLID/REGION/BODY;
   the Surface entity carries no history_id write — verified).
3. **The VIEW_CONTROL authored slots**: Leader's view control
   carries **16 null deleted slots** (gold: `num_entries: 16 [BL]`,
   entries `(2.0.0)` × 16) — the same authored-slots disease as the
   BLOCK/LTYPE/DIMSTYLE capture. Fixed: the pass-1 OBJ_VIEW_CONTROL
   arm drains [BL count][refs] into `table_control_entries`; the
   view writer call site applies the same-universe gate (the
   all-null case passes: filtered captured set = empty live set).
4. **Measured: 185 → 49 divergent records; 32/58 files at 100%.**
   The whole extrude/revolve/polysolid/box/sphere/torus/union/
   wedge/cone/cylinder named-specimen + solid genus is now FULLY
   byte-identical (ExtrudeC 206/206, Box 207/207, Leader 245/245;
   circle 211/211 held).
5. **Generation identity RE-RECORDED**: `84374e73ddcf1d6143877c
   4100b81e48`, 25,375 bytes — the null history_id form changed
   (`40` → `30` bytes on the generated R2007+ 3DSOLID-family
   records; same byte count, different nibble); identical WITHOUT
   `--features serde`.
6. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0,
   four family smokes 0/0/0/0, the full corpus 280 files
   0/0/0/0 — the echo arms untouched (the conventional arm serves
   edited documents and conversions only).

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
BEFORE the prepare pipeline at the write entry. The
`table_control_entries` capture is EXCLUDED from the fingerprint
(the `dimstyle_morehandles` precedent) — wire-only state.

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm — the only path these form rules affect
— is now record-identical to the author's stream on 32 of the 58
AC1021 corpus files (every named specimen and the whole
solid-extrusion genus), with **49 named residual records** in six
small classes (below).

## The residual 49 (each a named, byte-level-scoped packet)

- **The assoc/pathparam genus (24)**: ExtrudeCSurf/M (4 each),
  LoftCSurf/M (6 each), RevolveM (4) — the
  ASSOCPATHACTIONPARAM/LOFTED-surface family forms: the
  `+82/+109 (her 32 vs our 42)` code-3-class refs, the
  `+43/+47 (23v24)` and `+53v54` count nibbles, the `size XX vs
  20` UNKNOWN-body stubs (our emission of the unmodeled assoc
  records), the `1F 60v57/63v60` BLOCK_HEADER forms.
- **The Loft 29-vs-27 class (5)**: Loft3/C/H/R/_ — her loft
  UNKNOWN_OBJ record carries 2 bytes ours lacks.
- **Fillet/Chamfer (4)**: the UNKNOWN bodies (`1509/1463 vs 28`)
  + the `22 vs 26/30` records — her fillet/chamfer node's
  unknown_bits mass.
- **example_2007 (9)**: the marginal forms — `1F +10 (01v21)`,
  `176 1726v1727` (ours +1), `1A9 +32 (a9va1)`, `37D 2269v2270`,
  `392 +17 (32v42 — another code-3-class ref)`, `393 +27 (0cv10)`,
  `396 91v85`, `430 +37 (27v28)`, +1 more.
- **ATMOS (5 + 84 her-only)**: its controls `h=2 16v15`,
  `h=3 20v15`, `h=77 96v40`, `h=352 25642v35` (a 25KB record we
  emit as 35 bytes — an ACIS mass), `h=541 17v18` (ours +1); the
  84 her-only = the broken-map orphans (pre-existing, documented).
- **The singles**: Constraints `3E3` (666 vs 870); PolyLine3D
  `1C2` (16 vs 17 — OURS longer).

## The remaining work (all optional — the target stays reached)

- **The residual classes above** — each an autopsy → census →
  rule → gates → re-survey packet in the H8h tradition; the
  §19.2 H8h-ext/H8h-ext-2 rows carry the map and the method.
- **The code-3 sweep**: example_2007's `h=392 +17 (32v42)` is
  another author-code-3 ref — a census of gold's JSON ref codes
  vs our writer's choices on the residual records extends the
  landed rule family.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (a deep algorithmic study of gold's random.c MT
  variant; the echo path never runs it for unedited roundtrips,
  so it gates only edited-document re-emission quality, not
  rows).
- **The dead/no-path rows**: `LoftD`; the SH revolve option
  shorts; **BREP stays deferred** (external authentic
  ACSH_BREP_CLASS specimen required).

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (RE-RECORDED at H8h-ext-2 — the null history_id
  form; was `1a56bca0…` from H8h through H8h-ext). The generator
  builds and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **32 files at 100%, 49 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245.
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
  libredwg `bits.c`.
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A).

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
# current state: 32 files at 100%, 49 divergent records total
# circle 211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
780fc37 <docs> the post-H8h-ext halt refresh (the prior halt head)
<feat> H8h-ext-2: the null-slot dictionary, the history_id code 3,
       the VIEW slots (185 -> 49; 32/58 at 100%; generation
       identity re-recorded 84374e73..)
<docs> this halt refresh — the post-H8h-ext-2 record
```

**PUSH STATE (2026-09-27)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext
halt → the class-(c) autopsy (the model-vs-writer split: the model
carried the null-target item; the writer's pre-campaign filter
dropped it) → the class-(d) autopsy (the record-tail bit-walk,
the gold -v9 record-relative lesson, the 128/128 code-3 census) →
the VIEW_CONTROL discovery (Leader's 16 null slots — the
authored-slots disease generalizes) → the three-rule landing →
the gates (serde 1602/0, gold_roundtrip, issue80, family smokes,
corpus 0/0/0/0) → the generation identity re-record
(`84374e73…`, feature-independent) → the full survey (185 → 49;
32/58 at 100%) → the docs (§19.2 H8h-ext-2 row; this halt
record). **The maintainer's loop instruction — "repeat process
until target = zero" — remains satisfied: the corpus is at zero
on every axis; the conventional arm is record-identical on 32 of
58 AC1021 files including every named specimen and the whole
solid-extrusion genus, and the residual 49 records are named,
byte-level-scoped packets in six classes.**
