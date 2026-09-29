# Gold-vs-Silver Roundtrip Harness

A repeatable test harness that closes the read/write fidelity gap between
**LibreDWG** (the *gold* oracle) and **cadcodec** (the *silver* implementation)
for DWG files. It decodes a DWG with both libraries, rewrites it with cadcodec,
decodes the rewrite again, and reports field-level differences in the
non-header object data. **Phase 2 (planned, §19 of
[`IMPLEMENTATION.md`](./IMPLEMENTATION.md)): the header and remaining file
structure come under a second, separately-gated comparison axis with its own
drive to 0 and a whole-structure audit matrix; the OBJECTS 0/0 stays frozen.**

The full plan, architecture rationale, and the autonomous fix-loop
specification live in [`IMPLEMENTATION.md`](./IMPLEMENTATION.md). This README
is the practical entry point: install, run, interpret.

---

## Oracles — the four validation layers

| # | Oracle | What it catches | Needs the gold oracle? |
|---|---|---|---|
| 1 | Deep unit gates — `cargo test --features serde` (47 ok segments; roundtrip asserts `diffs <= max_known`) | model/retention regressions; a new raw-retention field without a `normalize_entity_for_comparison` arm trips the budget and the failing test names it | **no** — fully hermetic |
| 2 | Harness integration test — `cargo test --features gold-harness --test gold_roundtrip` | harness self-check + prohibited storage-only `EntityCommon` fields | **optional** — skips (with a notice written to `target/gold_harness_oracle_skipped.txt`) when `GOLD_DWGREAD`/`GOLD_TESTDATA` are absent; set `GOLD_HARNESS_REQUIRE=1` to make absence a hard failure (CI) |
| 3 | Full corpus — `run_corpus.py` → `report.json` + the residue dump (`pk18a_all_rows.py`) | non-header field divergences; read/write 0/0 is the OBJECTS-axis campaign target (`per_file` is truth; the by-type tables truncate); the planned structure axis (§19) adds separate counters | yes |
| 4 | Authored-wire byte-fidelity — byte-compare silver's *rewrite* of an authored file against the authored original, record by record | writer **form** defects both decoders tolerate (legal-but-different bitcode choices — a BS short form where the authored wire used the raw-16 form, BD shortforms vs raw doubles, alpha-method nibbles) that strict CAD consumers (BricsCAD, AutoCAD) reject | yes |

Layers 1–3 keep gold-vs-silver **parser parity** at 0/0. Layer 4 closes their
structural blind spot: the rt pair compares two lenient reads of the *same*
silver bytes, so it can never flag an encoding *form* — only byte-for-byte
comparison against the authored wire can. Full procedure and worked examples:
[`IMPLEMENTATION.md` §18](./IMPLEMENTATION.md).

Layer 4's wire facts for record walking: the R2000+ object frame is
`[MS size][R2010+ UMC hdlsize][BOT type][window]` where the window starts at
the BOT byte; `bitsize = Size*8 − Hdlsize` is the handle-stream start, and
gold's `-v9` trace prints per-field `@byte.bit` positions in window
coordinates to walk against. Two harness instruments implement it:

- `dump_section_bytes` — record-framed raw byte dumps for the pair compare.
- `dump_proxy_graphics [--verify] FILE [HANDLE]` — derives + dumps the
  entity-common proxy-graphics metafile (the ODA "Proxy Entity Graphics"
  stream; format + type census in `acadrust::entities::proxy_graphics`), and
  with `--verify` asserts decode→encode byte-equality against the wire
  bytes.
- `ac21_token_diff [--section NAME] [--our-rt FILE] FILE` — the AC1021
  objects-layer differential (§19 H8c/H8d): extracts the author's on-disk
  compressed streams per page (RS de-interleave, factor 1, RS(255,251)),
  walks token sequences with a decoder-exact state machine (replay-validated
  per page against `decompress_ac21`), mirror-sims our rewrite's stream
  sliced at her window boundaries (the H8b gate rule per window:
  `page_size_if_rs_coded` vs her slot), maps both streams' anatomy, and —
  with `AC21_DIFF_RAW_DIR=dir` — dumps both reconstructed section streams
  for record pair byte-study.

The layer-4 campaign result it instrumented (closed 2026-09-21): silver's
rewrite of `2018/Leader.dwg` reproduces the LEADER and MULTILEADER records
byte-identical to the authored wire, and the strict-load target
(`gen_all_entities_all_versions.dwg`, 30 entities, the LEADER warn-class and
the MULTILEADER fatal-class both rooted through form mismatches both decoders
tolerate) opens in BricsCAD via plain `_open` with zero warnings.

---

## Origin quality of the gold specimens

The libredwg `test/test-data` corpus is **mixed-genus on purpose** — a
property to keep, not a defect to fix. Each R2004+ specimen self-identifies
in its `AcDb:SummaryInfo` ProductInformation string, and the census
(2026-09-22) splits it:

| corpus family | self-stamped writer | notes |
|---|---|---|
| named specimen set — `2018/Leader.dwg`, Line, circle, Point, Arc, Ellipse, Spline, Text, Polygon, Donut, Helix, Multiline, Polyline, PolyLine3D, RAY, ConstructionLine, Constraints, … | **ODA FileConverter**: `Teigha® CompanyName="Open Design Alliance" build 2.0 / registry 4.3, install "ODA"` | `2018/Leader.dwg`'s comments field literally reads *"This file was last saved by an Open Design Alliance (ODA) application or an ODA licensed application."* |
| the Leader drawing family's down-saves (`2007/`, `2010/`, `2013/Leader.dwg`) | **AutoCAD** `N.51.M.5 reg 21.0` (2017) / `O.48.M.294 reg 22.0` (2018) | one drawing exported across versions — carrying the SAME wire conventions as the ODA-converted 2018 variant |
| issue / real-world files — `gh44-error.dwg`, ATMOS-DC22S, gh109_1, `sample_2018`, `LiveSection1`, `example_*` | **AutoCAD**, various builds (C.608.0/17.2, M.x/20.1 2015-era, …) | different writer-generation conventions (e.g. 2015-era sequential LEADER tails vs the 2017/2018 underlap genus) |
| pre-R2004 specimens (`2000/`, `2004/` dirs) | no marker exists in the format | provenance decidable only structurally (byte-compare conventions) |

Two consequences for harness work:

1. **Wire conventions are content- and generation-class facts, not writer
   fingerprints.** The 17-bit MULTILEADER tail group and the LEADER underlap
   appear in AutoCAD-2017/2018-saved files and the ODA FileConverter output
   alike; the 9-bit group belongs to fresh simple-content mleaders from
   every writer. Attribute a byte by its specimen's stamp and content
   class, never by "the ODA file said so".
2. **The published ODA spec undersides the runtime by design**: every
   authored file — ODA-written or AutoCAD-written — carries wire details
   (hidden tail groups, the underlap layout, proxy-graphics metafiles) that
   the Open Design Specification does not document. Conformance to the
   spec's field model is necessary; the layer-4 byte oracle against the
   authored wire is the only sufficient check.

---

## What it does

For each DWG file the harness produces three diffs:

| Diff | Compares | Finds |
|---|---|---|
| **Read fidelity** (`*_diff_orig.json`) | gold's decode vs silver's decode of the original | fields cadcodec's *reader* drops or misparses |
| **Write fidelity** (`*_diff_rt.json`) | gold's decode of the original vs gold's decode of cadcodec's rewrite | fields cadcodec's *writer* omits or corrupts |
| **Internal consistency** (stubbed) | silver's decode of original vs rewrite | cadcodec read→write→read self-check |

Header data is **out of scope** (compared laxly); only the top-level `OBJECTS`
array (entities + non-entity objects) is diffed exactly.

---

## Requirements

| Dependency | Purpose | Provided via |
|---|---|---|
| Built LibreDWG `dwgread` | gold oracle (`-O JSON`) | `GOLD_DWGREAD` env var — **oracle-optional**: `cargo test` skips gold checks without it; fetch/build on demand with `bash tests/gold_harness/bootstrap_oracle.sh` |
| LibreDWG `test/test-data` | DWG corpus | `GOLD_TESTDATA` env var |
| Rust toolchain (`cargo`) | builds silver binaries | system |
| Python 3.11+ (with `tomllib`; 3.8–3.10 need `tomli`) | normalizers + differ | system |

The harness itself (normalizers, differ, drivers, the two Rust binaries under
`src/bin/`) is self-contained in this directory. Nothing else in the repo is
required to run it, and the default `cargo test` suite stays hermetic (the
harness is behind the off-by-default `gold-harness` feature).

---

## Installation

These steps build the gold oracle and the silver binaries. They assume a
Linux/WSL environment with a standard build toolchain.

### 1. Build LibreDWG (gold oracle)

```bash
cd ~/work
git clone https://github.com/LibreDWG/libredwg.git
cd libredwg
sudo apt install build-essential autoconf automake libtool texinfo perl python3 jq
sh ./autogen.sh
./configure --disable-bindings --disable-docs
make -j"$(nproc)"
# Smoke test:
programs/dwgread -O JSON test/test-data/2000/Line.dwg | jq .
```

`programs/dwgread` is the only artifact the harness needs. The corpus comes
with the clone under `test/test-data/`.

### 2. Point the harness at LibreDWG

```bash
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
```

### 3. Build the silver binaries

```bash
cd ~/work/cadcodec
cargo build --features serde --bins
```

This builds `dwg2json` (silver JSON dump) and `dwgrewrite` (read→write), both
registered as Cargo `[[bin]]` targets with sources in `src/bin/`.

### 4. Verify the environment

```bash
python3 tests/gold_harness/check_env.py
```

It confirms `GOLD_DWGREAD` is executable and supports `-O JSON`, that
`GOLD_TESTDATA` has the `2000`…`2018` version folders, and that `cargo` is on
`PATH`.

---

## Running — manual

All commands assume the env vars from step 2 are set and you are in the
cadcodec repo root.

### Single file

```bash
python3 tests/gold_harness/run_roundtrip.py \
    "$GOLD_TESTDATA/2000/Line.dwg"
```

Outputs land in `target/gold_harness/` (or a directory you pass as the second
argument): `*_gold_orig.json`, `*_silver_orig.json`, `*_rt.dwg`,
`*_gold_rt.json`, `*_silver_rt.json`, `*_diff_orig.json`, `*_diff_rt.json`,
and a human-readable `*_report.md`.

### Whole corpus (batch)

```bash
python3 tests/gold_harness/run_corpus.py
```

Iterates the in-scope corpus (`test/test-data/{2000,2004,2007,2010,2013,2018}/*.dwg`
plus top-level `example_*`/`sample_*`) and writes an aggregated
`target/gold_harness_corpus/report.json` + `report.md` ranking divergent
`(entity_type, field)` pairs by frequency.

### Cargo integration test

```bash
cargo test --features gold-harness --test gold_roundtrip
```

Oracle-optional: shells out to the driver for a representative subset
(`2000/Line.dwg`, `2000/circle.dwg`) and asserts the harness runs cleanly and
that no prohibited `EntityCommon` storage-only fields appear in the diffs.
Without a LibreDWG checkout the test skip-passes and writes
`target/gold_harness_oracle_skipped.txt`; `GOLD_HARNESS_REQUIRE=1` turns
absence into a hard failure (CI), and
`bash tests/gold_harness/bootstrap_oracle.sh` clones and builds the oracle
online when you want the real fidelity run.

**Strict mode** — assert zero `missing_in_silver` fields on the subset:

```bash
GOLD_HARNESS_STRICT=1 cargo test --features gold-harness --test gold_roundtrip
```

---

## The zero-keeping workflow — the regression gate for upstream changes

Both campaigns are closed at zero: gold-vs-silver **parser parity**
(read 0 / write 0 across all 124 corpus files) and the **strict-load
zero** (the generated 30-entity file opens in BricsCAD via plain
`_open` with no modal error and no warnings). Every upstream change
must keep both at zero. Run this gate, in order, before committing
any reader or writer change (all commands from the repo root with the
oracle env from [Installation](#installation) step 2 set):

### Step 0 — scope the change

| change class | gate required |
|---|---|
| docs / probes only | nothing (this file's sections) |
| reader / model fields | steps 1 – 3 |
| writer (bitcode emission, forms, streams) | steps 1 – 6 |
| entity construction / examples that emit DWG | steps 1, 2, 5 (+4 if the corpus inputs' rewrite bytes change output) |

### Step 1 — hermetic layer (no oracle needed)

```bash
cargo test --features serde
```

**Expected:** every segment `ok`, zero `FAILED`. This layer names its
own failure class: a new raw-retention field without a matching
`normalize_silver.py` arm trips the `diffs <= max_known` budgets and
the failing test prints the field. A deep-roundtrip asymmetry (a
written default the walk does not read back symmetrically) fails the
`dwg_roundtrip_deep_*` tests — the only sanctioned filter class is a
version-inherent diff (see the `dwg_raw_tail_bits` filter for the
canonical example).

### Step 2 — harness self-check (oracle-optional)

```bash
cargo test --features gold-harness --test gold_roundtrip
```

**Expected:** `ok` with the oracle present, or the skip-pass notice
`target/gold_harness_oracle_skipped.txt` without. CI sets
`GOLD_HARNESS_REQUIRE=1` to make oracle absence a hard failure;
`bash tests/gold_harness/bootstrap_oracle.sh` brings the oracle online
on demand.

### Step 3 — touched-entity pair smoke

For every entity type your change touches, run the representative
authored file through the pair compare:

```bash
cargo build --features serde --bins
python3 tests/gold_harness/run_roundtrip.py \
    "$GOLD_TESTDATA/2018/Leader.dwg"      # replace with your family's specimen
```

**Expected:** `..._diff_orig.json: 0` and `..._diff_rt.json: 0` —
gold-vs-silver on the original AND gold-vs-gold on the rewrite. Any
nonzero count means the change dropped or corrupted a field; read the
`missing_in_silver` / `wrong_value` / `extra_in_silver` rows in the
diff, fix version-gated, re-run.

### Step 4 — the full corpus (the parity zero)

```bash
python3 tests/gold_harness/run_corpus.py
```

**Expected** in `target/gold_harness_corpus/report.md` (and
`report.json` — use the `per_file` totals there, never the truncating
by-type tables):

```
Files: 124
Read-fidelity diffs: 0
Write-fidelity diffs: 0
```

Also inspect one diff JSON for residues: any leftover row class the
normalizers don't host (gold-only `unknown_bits` kept residuals are
the sanctioned exception class — see the comments in
`normalize_silver.py`).

### Step 5 — deterministic generation identity (writer/example output)

```bash
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
```

**Expected:** both runs identical (the current zero-file identity is
`40ab5d356cf05a71333ff208e1651daf`, 25344 bytes — re-verified twice
on 2026-09-26 and unchanged by the H7e/H7f landings and the §18
walks (programmatic documents keep the historical bytes); it moved
from the earlier `0217fbac…` / 24986 at an H5–H7 landing whose
re-record landed only in `NEXT_SESSION.md`. LEADER 0x41 /
MULTILEADER 0x51 — recompute and re-record the identity in
`NEXT_SESSION.md` when an intended content change moves it, never to
paper over a regression). Spot-verify the mleader's metafile survives
the writer:

```bash
cargo run --bin dump_proxy_graphics -- --verify \
    gen_all_entities_all_versions.dwg 51
```

**Expected:** `verify: decode -> encode is byte-identical`.

### Step 6 — layer-4 byte oracle (writer form changes only)

Byte-compare silver's *rewrite* of the authored pair against the
authored original, record by record — the only instrument that catches
legal-but-different bitcode forms both decoders tolerate and strict
consumers reject:

```bash
python3 tests/gold_harness/run_roundtrip.py \
    "$GOLD_TESTDATA/2018/Leader.dwg"          # rewrite lands in target/gold_harness/
target/debug/dump_section_bytes "$GOLD_TESTDATA/2018/Leader.dwg" 4907 256 > /tmp/a.txt
target/debug/dump_section_bytes target/gold_harness/*Leader_rt.dwg 1959 256 > /tmp/b.txt
# leader record: window [Address..Address+Size) = [4916..5156); MS at
# Address-3 (4913), UMC at Address-1 (4915); the rewrite's Address is in
# the trace frames — re-dump around (Address-9) for the current tree —
# then bit-compare both window byte-ranges.
```

**Expected** for the current tree: the LEADER record (240 / 0x6A)
and the MULTILEADER record (833 / 0x76) are byte-identical to the
authored wire, CRC included. Any divergence means the writer changed
a *form* — find the first divergent bit, walk it against gold's
`-v9` trace (frame facts in [Oracles](#oracles--the-four-validation-layers)),
and remember the golden rule: the harness layers 1–3 are form-blind, so
only this step protects the strict-load zero.

### Step 7 — the §20 genus gates (constructed content)

The fifth validation layer (IMPLEMENTATION.md §20): constructed
content — documents silver authors from scratch — has no gold
counterpart, so layers 1–4 cannot see it. The genus gates decode the
constructed corpus silver-side and assert it against the
authored-specimen genus:

```bash
python3 tests/gold_harness/genus_extract.py   # regenerate + re-pin expectations
python3 tests/gold_harness/genus_gates.py     # the ranked report
```

**Expected:** the pipeline completes and the report's three sections
(`sab_form_diffs`, `sh_genus_diffs`, `acds_genus_diffs`) are
well-formed. The counts are the **work queue** — they land NONZERO on
purpose (§20.3) and close row by row through the packet workflow with
strict-loader verdicts adjudicating (§20.4: the gates rank divergence,
they do not decide fatality). The four fidelity axes and the differ
are untouched (§20.5); the corpus report carries the sections as
ADITIONAL output. The cargo mirror
`cargo test --features gold-harness --test genus_gates` asserts a
fresh extraction equals the pinned `genus_expectations.json` — when
the decode changes, regenerate the pin and review the drift.

### Step 7b — the strict-load probe (the mechanized BricsCAD audit)

The §20.4 verdict instrument: `strict_load_probe.py` drives
`bricscad.exe /b <script>` headless over the constructed corpus plus
the authored specimen controls, and the script's LISP records the
evidence a verdict needs — the 3DSOLID/REGION census, the
modeler-forced bounding box (real extents = a restored body; the
±1e80 sentinel = the null box), the post-audit census:

```bash
python3 tests/gold_harness/genus_gates.py        # fresh constructed corpus first
python3 tests/gold_harness/strict_load_probe.py  # the verdict table
```

**The LOGSEC flush discipline (the evidence-survival rule).** The
script's LISP writes every result section through a `LOGSEC` helper
that opens the result file in APPEND mode, writes its lines, and
CLOSES the handle — so each line is on disk the moment it is
written:

```lisp
(defun LOGSEC (lines / f)
  (setq f (open RESULT "a"))
  (foreach l lines (write-line l f))
  (close f))
```

A fixture that trips BricsCAD's modeling-failure prompt (the dialog
blocks the /b script engine mid-sequence) or a launcher timeout kill
still leaves its partial evidence on disk. The earlier
single-handle form (`(setq rf (open … "w"))` … one final
`(close rf)`) buffered everything in the handle and lost ALL evidence
when the run died before the close — the staged results read 0 bytes
and a recorded verdict looked AMBIGUOUS. With LOGSEC, a result that
stops before `probe-end` is itself evidence: the surviving lines tell
the census story and the classifier reads the truncation. The tool
unlinks each result file before its launch, so append mode never
accumulates across runs.

**Expected:** the authored controls read MODELED (real extents —
e.g. Box_2007 `0,0,0..1,2,3`); a constructed fixture reads MODELED
when its restore gap closes, NULL-BOX (the ±1e80 sentinel) while the
gap stands, NO-SOLID when the entity layer fails, and AMBIGUOUS
(missing `probe-end`) when the run was cut short — the surviving
LOGSEC lines still rank the failure. A Windows-visible host and the
BricsCAD install are required; stray `bricscad.exe` processes poison
subsequent launches — kill them before re-running.

**Console capture — the window-lifecycle scraper (landed, with the
tested limits).** BricsCAD's console text is not reachable through
the classic channels on this build — all tested: `LOGFILENAME` is
read-only (a diag run trapped the setvar), `WM_GETTEXT` returns
empty on BricsCAD windows (the UI is Qt; text is painted, not
stored in window text slots), and UI Automation exposes no
Text/Value patterns and an empty Name tree (no accessibility
bridge). What works is `GetWindowText` for window TITLES — so the
probe ships `bricscad_console_scraper.ps1`: the launcher spawns it
with the BricsCAD PID and it polls every 250 ms, logging each
top-level window's class + title at first appearance with
timestamps into `<name>_console.log`. The transcript captures
WHICH dialogs appear during a run (the modeling-failure dialog is
a top-level window), WHEN, and their titles — alongside the
LOGSEC LISP census and the DBMOD/ERRNO pre/post-audit record,
that is the complete programmatic evidence surface on this build.
The probe's analyzer digests the transcript against the strict
loader's message vocabulary (deduped, with repetition counts).
The console text itself needs a build with a working log channel
or accessibility bridge; a hand-run transcript from the maintainer
remains the highest-fidelity console evidence.

### When a layer trips

- **Step 1 fails** — model/normalizer/budget mismatch: the failing
  test names the field; fix version-gated (never widen the budget to
  silence it).
- **Steps 3–4 fail** — parser parity regression: run the per-file
  diff, read the offending `(entity_type, field)` rows, fix
  version-gated against the gold spec (`libredwg src/dwg.spec /
  dwg2.spec`). The convergence recipe — context budgeting, per-field
  decision tree, blocked-handling — lives in
  [`IMPLEMENTATION.md` §8.1](./IMPLEMENTATION.md#81-subagent-execution-guide-128k-context).
- **Step 5 md5 drifts without an intended change** — nondeterminism
  (a hash-ordered collection in a writer path: the
  `Mesh::compute_edges` BTreeSet fix is the canonical example).
- **Step 6 diverges** — encoding-form regression: re-derive the
  authored convention from the census (attribute by specimen origin
  stamp + content class, per [Origin quality](#origin-quality-of-the-gold-specimens)),
  keep the value-preserving guards, and re-verify the affected
  strict-consumer behavior.
- **Step 7's mirror fails on the pin compare** — the decode changed
  the projections: regenerate `genus_expectations.json` with
  `genus_extract.py` and review the drift (expectation drift is itself
  reviewable, §20.3); never hand-edit the pin.

### Hard rules (inherited, unchanged)

- Never edit LibreDWG — it is a read-only oracle.
- Never edit `ignore_fields.toml` or `diff_fields.py` to make a diff pass.
- Never touch version dispatch, decompression, CRC, or the section map.
- Gate every fix behind the exact version predicate gold uses.
- Each checkpoint must leave the build green.
- Header comparison stays lax.

---

## File inventory

Every tracked file in this directory, plus the adjacent tracked scripts the
harness relies on. The harness bins below are registered in the root
`Cargo.toml` as `[[bin]]` targets (`dwg2json` with `required-features =
["serde"]`; the rest unconditional); the `entity_atlas` example is gated
there with `required-features = ["serde"]` alongside them.

| Path | Role |
|---|---|
| `AGENTS.md` | Durable rules for agents working the harness (the frozen files, the version-gate rule, loop invariants, session workflow); the honor-system contract behind every commit |
| `README.md` | This file — entry point: oracle layers, origin quality, setup, the zero-keeping workflow |
| `NEXT_SESSION.md` | The cold-start handover brief for the next session (durable findings + census tables); self-replaced at each campaign halt |
| `IMPLEMENTATION.md` | The single source of truth for the plan (§7 the completed fidelity campaign; §8.1 the fix-loop manual; §18 the validation layers and the strict-load campaign resolution) |
| `run_roundtrip.py` | Single-file driver (the three diffs) |
| `run_corpus.py` | Batch driver + aggregated report |
| `normalize_gold.py` | LibreDWG JSON → canonical records |
| `normalize_silver.py` | cadcodec JSON → canonical records (holds the live type/field maps) |
| `diff_fields.py` | Diff engine (`missing_in_silver`, `wrong_value`, `extra_in_silver`, `count_mismatch`) |
| `ignore_fields.toml` | Curated fields the differ skips (**frozen** during the fix loop) |
| `check_env.py` | Environment sanity checker |
| `bootstrap_oracle.sh` | On-demand LibreDWG checkout + build (the gold oracle), prints the env exports |
| `src/bin/dwg2json.rs` | Silver JSON dump (re-injects serde-skipped `EntityCommon` fields under `_common_dwg`) |
| `src/bin/dwgrewrite.rs` | Silver read→write binary |
| `src/bin/dump_section_bytes.rs` | Record-framed raw byte dumps (the layer-4 pair-compare instrument) |
| `src/bin/dump_proxy_graphics.rs` | Proxy-graphics metafile derivation + `--verify` byte-roundtrip proof |
| `src/bin/genus_constructed.rs` | The §20 constructed-fixture family generator (one solid per SAB surface family, one region, one body, one `create_solid_history` tree) |
| `genus_extract.py` | The §20 expectation extractor — decodes the specimen family silver-side, projects the SAB/SH/AcDs genus into the pinned expectations |
| `genus_expectations.json` | The pinned genus expectations (regenerate with `genus_extract.py`; the cargo mirror diffs a fresh extraction against this copy) |
| `genus_gates.py` | The §20 gate run — decodes the constructed corpus, asserts against the pin, emits the ranked `sab_form_diffs` / `sh_genus_diffs` / `acds_genus_diffs` sections |

**Adjacent tracked scripts the harness does not own** — documented for
completeness:

| Path | Role |
|---|---|
| root `Cargo.toml` | Workspace manifest; registers the harness bins and the feature-gated `entity_atlas` example (`[[bin]]` / `[[example]]` blocks) |
| `tests/roundtrip.rs`, `tests/gold_roundtrip.rs`, `tests/genus_gates.rs` | The Rust test surfaces the workflow's steps 1–2 and 7 gate on (the harness integration tests are feature-gated `gold-harness`) |
| `tests/issue64/validate_ezdxf.py` | Issue-64 DXF validation helper (fixture pair for `issue64.rs`; not part of the gold-vs-silver harness) |

---

## Output interpretation

- `missing_in_silver` — gold has a field silver lacks (reader gap or
  projection gap).
- `wrong_value` — both have the field but values differ (check bitcode type
  and version gating against the gold spec).
- `extra_in_silver` — silver emits a field gold doesn't have for this version
  (usually a dump/normalizer version-gating issue).
- `count_mismatch` — gold and silver decoded different numbers of a type
  (coverage gap, e.g. proxy/unknown objects).

Handles are compared by **resolved target type**, not raw id, because handles
are reassigned on rewrite. Records are aligned by `(type, ordinal-within-type)`.

---

## Troubleshooting

| Symptom | Likely cause | Action |
|---|---|---|
| `GOLD_DWGREAD is not set` | env var missing | re-export per Installation step 2 |
| `dwgread` non-zero exit on a file | gold can't decode it (advanced R2010+ object) | treated as "skip file", not a silver failure |
| harness diff count jumps after an edit | cross-version regression | re-run all 6 versions; the version gating is wrong |
| `cargo test` fails with harness off | unrelated to harness | harness is feature-gated; check the core change |

---

For the design decisions, corpus coverage analysis, the Phase 6 loop
specification, the planned header & structure campaign (IMPLEMENTATION.md
§19), and authoring coverage-gap fixtures, see
[`IMPLEMENTATION.md`](./IMPLEMENTATION.md).
