# AGENTS.md — instructions for AI coding agents

This file is read first by a coding agent in a fresh session. It holds the
**durable rules** that must always stay true for the gold-vs-silver harness
work in this repository. Anything task-specific, dated, or counts-bearing
belongs in `IMPLEMENTATION.md` (the living plan, in this same directory), not
here — keep this file free of transient state so it never goes stale.

---

## What this repo is

`acadrust` — a pure-Rust crate for reading/writing CAD files (ASCII + binary
DXF R12–R2018+, native binary DWG R13–R2018+). The current development focus is
the **gold-vs-silver roundtrip harness** in this directory, which diffs this
crate ("silver") against LibreDWG ("gold", the read-only oracle) to drive
read/write fidelity to zero non-header field diffs.

**Start here:** `IMPLEMENTATION.md` (this directory) is the single source of
truth for the harness. Its §7 "How to start cold" block is the entry point;
§8.1 is the self-contained operating manual. Read those before editing.

---

## Always-true rules (do not violate)

### Sources of truth
- **LibreDWG is the gold oracle and is read-only.** Never edit anything under
  the LibreDWG checkout (`~/work/libredwg`). Verify every field against its
  spec *before* changing code: grep both `dwg.spec` **and** `dwg2.spec`
  (R2000+ objects live in `dwg2.spec`), plus `common_entity_data.spec` /
  `common_entity_handle_data.spec` / `common_object_handle_data.spec` for
  shared fields. Gold spec names for `3DFACE`/`3DSOLID`/`3DLINE` carry a
  leading underscore (`_3DFACE`, `_3DSOLID`, `_3DLINE`) — grep the alias if the
  plain name finds nothing.
- The harness's differ and ignore-list are **frozen during the fix loop**:
  never edit `diff_fields.py` or `ignore_fields.toml` (this directory) to make
  a diff pass. Fixes belong in the silver codec (reader/writer/struct), the
  dump (`src/bin/dwg2json.rs` under this directory), or the silver/gold
  normalizers — as faithful projections, never as data-hiding.

### DWG field work
- A grep hit for `DWG_ENTITY/DWG_OBJECT/DWG_TABLE (NAME)` is only meaningful
  if the block is **live**: blocks inside `#if defined (DEBUG_CLASSES) ||
  defined (IS_FREE)`, inside `#if 0`, or comment-mentioned only are compiled
  out of the built `dwgread` — gold then decodes such objects as raw
  `UNKNOWN_ENT`/`UNKNOWN_OBJ` and never emits the typed name. Check the
  enclosing preprocessor frame before designing a packet (frame map and
  liveness rule: IMPLEMENTATION.md §8.1.1).
- Gate every DWG field by the **exact version predicate the gold spec uses**
  (`PRE`/`VERSIONS`/`SINCE`/`UNTIL`/`LATER_VERSIONS`), in both reader and
  writer. Fields guarded by `DXF { … }` in the spec are DXF-only — do **not**
  emit them on the binary-DWG path.
- Never touch version dispatch, decompression, CRC, or the section map. Edit
  only per-field read/write bodies, struct fields, or normalizer projections.
- Handles compare by **resolved target identity**, never by raw handle value
  (handles are reassigned on rewrite). Do not merge gold's distinct
  `UNKNOWN_ENT`/`UNKNOWN_OBJ` types into one bucket — they have different
  common-field shapes and merging them breaks ordinal alignment.

### Regression gates (the authoritative checks)
- `cargo test --features serde` must stay green — **this is the authoritative
  gate** (baseline: 1556 passed / 0 failed).
- `cargo test --features gold-harness --test gold_roundtrip` must stay `ok`.
- Plain `cargo test` fails to compile `examples/entity_atlas.rs` (it imports
  `serde_json`, which is optional behind the `serde` feature). This is a
  **pre-existing, unrelated** condition — do not treat that specific `E0432`
  as a regression you caused, and do not "fix" it as part of an unrelated
  packet.
- After a harness change, the corpus non-header diff totals must strictly
  decrease (or stay flat when the packet targets a type absent from the
  representative files). Corpus counts are stem-collision inflated — use them
  for *ranking*, and verify the true per-file count before committing to a
  packet.
- The §20 genus gates (constructed content) are ADDITIONAL output: they never
  modify the differ, the normalizers, or the fidelity totals. Expectations
  come from the specimen family via `genus_extract.py` — never hand-edit
  `genus_expectations.json`; regenerate it and review the drift (the
  `genus_gates` cargo mirror asserts the pin matches a fresh extraction).
  The gate counts are a ranked work queue, not pass/fail: `genus_gates.py`
  exits 0 with nonzero counts by design; a genus expectation for a surface
  where gold is known-wrong changes only through a recorded strict-loader
  probe verdict, never an oracle trace alone (§20.4).
- The strict-loader verdicts come from `strict_load_probe.py` (README step
  7b): the default loader is BricsCAD V26; AutoCAD 2027 — the format's
  author, the strongest oracle — is passed via `--bcad "C:\Program
  Files\Autodesk\AutoCAD 2027\acad.exe"`. A stalled loader instance is
  killed BY PID, never by name (`acad.exe` is shared with other Autodesk
  sessions). Verdict vocabulary: MODELED (real extents) / NULL-BOX (the
  ±1e80 sentinel — the B-rep did not construct) / NO-SOLID / AMBIGUOUS;
  an AutoCAD "Open Drawing - Errors found" dialog is a file-level defect
  verdict. Every probe launch holds the window visible 10 s before QUIT.

### Commits
- Commit message format: `fix(harness): <packet> — <gold spec ref>` (or
  `fix(dwg):` / `docs(harness):` as appropriate), with before→after corpus
  counts in the body.
- Leave the build green at every checkpoint; never commit a broken tree.

---

## Session workflow (summary — details in IMPLEMENTATION.md §8.1)

Two campaigns are closed at zero (parser parity 0/0 + the strict-load
zero); the ACS/SH solid-history campaign is complete through its
differential queue (§18.5–18.7; corpus 280 files at 0/0, the
maintainer fixture surface empty, the surface-parser row landed —
the 20 surface-twin files un-quarantined). The operative
procedure is the **zero-keeping regression gate** in `README.md`
("The zero-keeping workflow"): scope the change, run the required gate
steps in order (hermetic cargo tests → harness self-check →
touched-entity pair smoke → full corpus → deterministic generation
identity → layer-4 byte oracle for writer form changes), and only
commit when every step holds its expected value.

1. Read §7 "How to start cold" + §8.1.6 (work queue) of `IMPLEMENTATION.md`
   (this directory).
2. Verify the environment (§8.1.0).
3. Pick exactly ONE `(type, field)` packet; consult its gold spec block.
4. Make the minimal, version-gated fix; build with
   `cargo build --features serde --bins`.
5. Verify on every DWG version the field's gate covers, then run the corpus.
6. Pass the regression gates above; commit; update the plan's queue.
