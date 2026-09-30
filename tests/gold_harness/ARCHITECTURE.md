# ARCHITECTURE.md — the gold_harness design

The structural reference for the gold-vs-silver roundtrip harness: what each
component is, why it exists, and how the pieces connect. Campaign history and
queue state live in `IMPLEMENTATION.md` (the plan) and `NEXT_SESSION.md` (the
halt records); this file describes the *standing design* those records
operate on. Entry points: `README.md` (workflow), `AGENTS.md` (durable rules).

---

## 1. The problem shape

`acadrust` ("silver") reads and writes native DWG. Two independent oracles
judge correctness:

- **gold** — LibreDWG (`dwgread`), a second, independent decoder. It can only
  *read*: it exposes parser-parity defects (fields silver drops or misparses)
  but shares silver's leniency — two tolerant decoders agree on streams a
  strict consumer would reject.
- **the strict loaders** — BricsCAD V26 and AutoCAD 2027 (the format's
  author). They *construct* (regenerate B-reps, audit records) and reject
  byte-legal-but-divergent encodings both decoders accept.

No single oracle suffices, so the harness is a **stack of five validation
layers plus a loader axis**, each catching what the ones below it cannot:

```mermaid
flowchart TD
    subgraph L1["Layer 1 — hermetic unit gates"]
        T["cargo test --features serde<br/>(roundtrip budgets: diffs <= max_known)"]
    end
    subgraph L2["Layer 2 — harness mirror"]
        G["cargo test --test gold_roundtrip<br/>(self-check + storage-only-field guard)"]
    end
    subgraph L3["Layer 3 — full corpus (280 files)"]
        C["run_corpus.py<br/>read/write fidelity + structure key-gaps<br/>→ report.json, four axes at 0"]
    end
    subgraph L4["Layer 4 — authored-wire byte fidelity"]
        R["record_size_census.py (size+CRC-16 per handle)<br/>dump_section_bytes / ac21_token_diff"]
    end
    subgraph L5["Layer 5 — genus gates (constructed content)"]
        GG["genus_extract.py → pin<br/>genus_gates.py → ranked sections<br/>cargo mirror: fresh == pin"]
    end
    subgraph LX["The loader axis (acceptance)"]
        P["strict_load_probe.py<br/>BricsCAD V26 + AutoCAD 2027 + accoreconsole<br/>census + bbox force + verbatim audit logs"]
        EV["entity_verification.py<br/>29 kinds x 5 axes matrix"]
    end
    L1 --> L2 --> L3 --> L4 --> L5 --> LX
    T -.->|"catches: model/retention regressions"| Q1["no oracle needed"]
    C -.->|"catches: field divergences"| Q2["needs gold"]
    R -.->|"catches: writer FORM defects<br/>both decoders tolerate"| Q3["needs the authored wire"]
    GG -.->|"catches: constructed-content drift<br/>(no gold counterpart exists)"| Q4["the authored genus is the oracle"]
    P -.->|"catches: construction/audit failures<br/>no decoder can see"| Q5["needs the real modelers"]
```

The blind-spot chain that motivated each layer:

| Blind spot | Closed by |
|---|---|
| Unit tests assert silver-vs-silver — a shared codec bug passes | layer 3 (gold parity) |
| Both decoders are lenient — a *form* defect (legal bitcode choice A vs the author's B) passes | layer 4 (byte-compare the rewrite against the authored wire) |
| Constructed documents have no authored counterpart — layers 1–4 see nothing | layer 5 (the authored *genus* — measured invariants across the specimen family — becomes the expectation) |
| Decoders accept what modelers reject — construction never tested | the loader axis (the modeler's own census/audit words are the verdict) |

---

## 2. The corpus pipeline (layer 3)

`run_corpus.py` walks every corpus file through a read→diff→rewrite→diff
cycle. The differ (`diff_fields.py` + `ignore_fields.toml`) is **frozen during
the fix loop** — fixes belong in the codec, never in the differ.

```mermaid
flowchart LR
    F["corpus file<br/>(280 files, mixed genus)"] -->|"dwgread -O JSON"| GJ["gold JSON"]
    F -->|"silver read"| SJ["silver model"]
    SJ -->|"dwg2json"| SJ2["silver JSON"]
    GJ --> NG["normalize_gold.py<br/>(faithful projections only)"]
    SJ2 --> NS["normalize_silver.py"]
    NG --> D["diff_fields.py<br/>+ ignore_fields.toml (frozen)"]
    NS --> D
    D --> R1["*_diff_orig.json<br/>READ fidelity"]
    SJ -->|"silver write (rewrite)"| RT["rewritten DWG"]
    RT -->|"dwgread -O JSON"| GJ2["gold JSON of rewrite"]
    GJ2 --> D2["diff vs original's gold JSON"]
    D2 --> R2["*_diff_rt.json<br/>WRITE fidelity"]
    R1 --> REP["report.json<br/>(four axes, per-file truth)"]
    R2 --> REP
    SJ -.->|"structure walk"| SA["struct_axis.py<br/>(17 header/section keys)"]
    SA --> REP
```

Design invariants:

- **Normalizers are faithful projections, never data-hiding.** A normalizer
  may re-shape a value for comparison (e.g. pop writer-side codec channels
  that have no gold counterpart) but may not delete a real semantic
  difference. Every pop is loop-universal and documented.
- **`per_file` is truth; by-type tables truncate.** Corpus counts are
  stem-collision inflated (280 files → 196 unique stems) — use them for
  ranking only.
- **Header data is out of scope** (compared laxly); only the `OBJECTS` array
  is diffed exactly.
- **The four axes**: read fidelity, write fidelity, read structure key-gap,
  write structure key-gap — all held at 0.

The corpus is **mixed-genus on purpose** (ODA FileConverter stamps, AutoCAD
stamps, unmarked pre-R2004): wire conventions are content- and
generation-class facts, so byte attribution keys on a specimen's stamp and
content class, never on "which writer said so".

---

## 3. The write path (what layer 4 byte-compares)

Silver's DWG writer has two arms, chosen at the write entry:

```mermaid
flowchart TD
    W["write_to_writer(document, version)"] --> FP{"document-state<br/>fingerprint holds?<br/>(captured at read end)"}
    FP -->|"yes + same-version DWG source"| ECHO["THE UNIFIED WHOLE-FILE ECHO<br/>re-emit her on-disk bytes VERBATIM<br/>(raw_acds_data, serde-skipped containers)"]
    FP -->|"no (edited / converted / constructed)"| PRE
    subgraph CONV["THE CONVENTIONAL ARM"]
        PRE["prepare_database_references<br/>(output-copy repairs: Table/MLine/MLeader/MText<br/>style + pointer normalization — caller's doc untouched)"] --> OBJ["DwgObjectWriter<br/>per-record emission"]
        OBJ --> SEC["section assembly<br/>(AcDs container, SH, objects, entities)"]
    end
    ECHO --> OUT["output DWG"]
    CONV --> OUT
```

- **The echo** is a byte-copy gated on a full-content hash: for an unedited
  same-version roundtrip the writer is a copier. The fingerprint
  (`document_state_fingerprint`) hashes the semantic inventory, the ten table
  control handles, and the retained metadata models; wire-only captures are
  excluded so codec-channel bookkeeping never blocks the echo.
- **The conventional arm** serves every other case. Its record emission is
  held to **record identity**: on every surveyed era specimen the rewrite's
  records match the author's on size + CRC-16 (`record_size_census.py`), with
  byte-passthrough replay for classes whose typed re-encode drifts (the
  520/528/529 wire-capture classes, DATATABLE raw passthrough, pre-2007
  MText wire text).
- **The output-copy repairs** fix *invalid authored states* (a null
  MLeader style pointer, a corrupt MText attachment repeat) only in the
  write's copy — the caller's document stays untouched, and authored
  captures that already carry valid values pass through verbatim.

### 3.1 The AcDs container (R2013+ modeler data)

Constructed documents' 3DSOLID/REGION/BODY geometry lives in an
`AcDb:AcDsPrototype_1b` section — a `jard` container of named segments. The
layout (era-shared Form A, measured from the authored specimens):

```mermaid
flowchart LR
    subgraph JARD["jard container (16-byte aligned)"]
        H["48-byte file header<br/>ds_version 16/17, segidx-first @128,<br/>91/97-row scale"]
        SI["segidx table<br/>(slot: offset, size)"]
        subgraph SEGS["segments (slot ids = table positions)"]
            S1["schdat-A 384B<br/>(schema bytes, authored verbatim)"]
            S2["schdat-B 256B<br/>(EIGHT-column ASM_Data schema)"]
            S3["schidx 512B<br/>(schema list; ASM_Data = slot 5)"]
            S4["datidx<br/>rows: (slot_data, row*20, schidx)"]
            S5["search 256B<br/>(per-schema lookup indexes)"]
            S6["_data_ (content-sized)<br/>row table + blob chain"]
            S7["prvsav / freesp"]
        end
        H --> SI --> SEGS
    end
```

The `_data_` record table is the multi-record heart (the 2026-09-30
row-locator design):

```
[48-byte segment header]
[20-byte row] × n:  (0x14, 1, owner_handle, 0, LOC)
                  LOC = cumulative chunk offset in the aligned blob area
[0x62 fill to 16-alignment]
[blob area]: [len u32][blob] × n   — row 0 = the Model Layout thumbnail PNG,
                                     rows 1..n = the SABs (datidx schidx 5)
```

The `search` segment carries per-schema lookup indexes in the authored
grammar — per block: `schema_namidx`, `(row<<32)` sorted keys,
`num_ididxs=0`, `unknown=1` (her constant), `zero`, then `(handle, 1, row)`
triples **row-synced with the keys**. The modeler resolves
handle→record through this table; a mis-synced search mispairs
multi-record geometry (entity A renders entity B's B-rep).

### 3.2 The SAB stream (the B-rep payload)

The ACIS/SAT model is serialized to SAB (binary SAT) at the write boundary:

```mermaid
flowchart LR
    subgraph SRC["source"]
        SAT["SAT text (DXF/programmatic)"]
        CAP["captured SAB (DWG read: is_binary + sab_data)"]
    end
    SAT -->|"parse → strip_for_sab → normalize"| MODEL["SatDocument<br/>(records, tokens)"]
    CAP -->|"verbatim queue (no re-encode)" | Q["AcDs queue"]
    MODEL -->|"BFS first-mention reorder<br/>(authored traversal genus)"| MODEL2["ordered records"]
    MODEL2 -->|"SabEra::write_modern"| SAB["SAB bytes"]
    subgraph SAB2["SAB stream layout"]
        H2["file header: magic (era-coded) + version u32<br/>+ records/bodies/history + product strings<br/>+ date + tolerance triple"]
        A["asmheader record (the era's version string)"]
        B["body → lump → transform → shell → face → loop<br/>→ surfaces → coedges → edges → vertices<br/>→ curves → points (BFS first-mention)"]
        T["terminator: [0e 03 End][0e 02 of][0e flavor][0d 04 data]<br/>flavor era-coded: ACIS (21200/21500) / ASM (21800/22300)"]
        H2 --- A --- B --- T
    end
    SAB --> SAB2
    SAB2 -->|"R2013+: AcDs _data_ row"| DS["_data_ blob"]
    SAB2 -->|"R2007/R2010: in-entity"| IE["entity record inline"]
```

The era profile (`SabEra`) pins the magic/version/triple per target era; the
product strings stay silver's own (the author-identity rule: never forge
Autodesk identity stamps). The terminator's segmented, era-coded form is
load-bearing: ACAD's modeler rejects the single-tag form at open-time
regeneration (the 65010), while BricsCAD tolerates both.

---

## 4. The genus gates (layer 5)

Constructed documents have no authored counterpart, so layers 1–4 are blind
to their content. The genus gates turn the authored specimen family's
*measured invariants* into the expectation:

```mermaid
flowchart LR
    subgraph EXTRACT["genus_extract.py (fixtures-only)"]
        SPEC["specimen family<br/>(sh_history: SAB/SH/AcDs carriers)"] --> W1["SAB walker (mirrors sab.rs framing)"]
        SPEC --> W2["SH root walker<br/>(ACAD_EVALUATION_GRAPH topology)"]
        SPEC --> W3["AcDs container decoder<br/>(per-segment ds/size/align)"]
        W1 --> PIN["genus_expectations.json<br/>(THE PIN — never hand-edited)"]
        W2 --> PIN
        W3 --> PIN
    end
    subgraph GATES["genus_gates.py"]
        GEN["regenerate the constructed corpus<br/>(gen_all canonical + fixture family)"] --> DEC["decode + assert against the pin"]
        DEC --> RANK["ranked sections:<br/>sab_form / sh_genus / acds_genus diffs"]
        RANK --> ADJ["ADJUDICATIONS table:<br/>rows close on recorded verdicts,<br/>keep their counts as the recorded state"]
    end
    PIN -->|"expectations"| GATES
    RANK -->|"--strict asserts pending == 0"| VERDICT
    MIRROR["cargo test --test genus_gates<br/>(fresh extraction == pin)"] -.->|"drift is itself reviewable"| PIN
```

Design rules:

- **The pin is fixtures-only** and regenerates identically everywhere; the
  cargo mirror asserts a fresh extraction equals it, so *expectation drift*
  is caught as a diff, not silently absorbed.
- **The gate counts are a ranked work queue, not pass/fail** — the script
  exits 0 with nonzero counts by design. `--strict` asserts the *pending*
  queue is zero; adjudicated rows (recorded verdicts with provenance) keep
  their counts as documented state.
- **A genus expectation changes only through a recorded strict-loader
  verdict** (§20.4): an oracle trace alone never re-pins a row.
- The gates never touch the differ, normalizers, or fidelity totals — they
  are additional output riding the corpus run.

---

## 5. The loader axis (the acceptance instruments)

### 5.1 strict_load_probe.py

Drives the real modelers headless (`/b <script>`) over the constructed corpus
plus authored controls, and records the evidence a verdict needs:

```mermaid
sequenceDiagram
    participant P as probe (python)
    participant L as loader (bcad / acad / core)
    participant D as staged copy
    P->>D: copy fixture to Windows-visible staging
    P->>L: launch /b script (WorkingDirectory pinned)
    L->>L: _.LOGFILEON → OPEN → LISP census
    Note over L: modeler-family aggregate:<br/>ssget 3DSOLID,REGION,BODY + vla-getboundingbox<br/>(real extents = restored; ±1e80 = null box)
    Note over L: widened per-kind census:<br/>CENSUS_KINDS (all 29 kinds),<br/>trapped bbox force per entity
    L->>L: AUDIT (dbmod pre/post) → re-census
    L->>L: _.LOGFILEOFF → QUIT _N (saves staged copy)
    L-->>P: result file (LOGSEC lines survive any kill)
    P->>P: harvest {run}_audit.log verbatim<br/>(the loader's own words)
    P->>P: classify: MODELED / NULL-BOX / NO-SOLID /<br/>AUDIT-REPORT / AMBIGUOUS
```

Guardrails baked into the instrument:

- **Launch-time LISP assertions** — every generated `.scr` must
  parenthesize to depth 0 (the one-paren-short stall class), written
  `encoding="ascii"` (a UTF-8 em-dash would reach the loader's ANSI codepage
  as a curly quote).
- **Bounded launches** — a missing result is AMBIGUOUS, never clean; stray
  loader processes are killed by PID, never by name.
- **The audit reports are first-class evidence** — the modeler's own failure
  text ("Data stream is empty", "LeaderStyle Id is Null",
  "Modeling Operation Error: 65010") names the defect class before any
  byte-level investigation starts.
- **The core console** (`accoreconsole.exe`) is the dialog-free channel: its
  stdout transcript is the evidence (its ActiveX bridge is nil, so no bbox
  force there — verdicts read AUDIT-REPORT).
- `QUIT _N` **saves the staged copy**, so an audit's *repairs* are diffable
  against the pre-audit bytes — the empirical path that named the MText and
  MLeader defects field-exactly.

### 5.2 entity_verification.py

The per-kind matrix: BUILD (public API constructs) / SILVER-READ / GOLD-READ
(census + ERROR lines) / REWRITE (conventional-arm survival) / MODELER (the
probe axis), all measured on the gen_all canonical (one AC1032 document
carrying all ~29 kinds).

### 5.3 The chimera instrument (examples/sab_swap.rs)

The blame-splitting design: swap one arm of a constructed-vs-authored pair
while holding the other fixed, and probe both chimeras plus controls.

```mermaid
flowchart TD
    subgraph MATRIX["the chimera matrix (any defect localizes)"]
        A["our wrapper + our SAB"]
        B["our wrapper + HER SAB<br/>(chimera B: swap into our container)"]
        C["HER wrapper + our SAB<br/>(chimera A: swap into her file)"]
        D["her wrapper + her SAB<br/>(the authored control)"]
    end
    A -->|"fails, B fails"| W1["defect in the WRAPPER"]
    A -->|"fails, C fails"| W2["defect in the SAB STREAM"]
    A -->|"fails, B+C pass"| W3["defect needs both arms (coherence)"]
```

The swaps are **coherent** — the SAB travels with its owning entity's 3BD
anchor (the modeler cross-checks the anchor against the SAB's actual
geometry), so the experiment never creates its own artifact.

### 5.4 The record-identity instruments

- `record_size_census.py` — handle-keyed (size + hdlsize + bitsize + CRC-16)
  record comparison for ANY pair on ANY era; `DWG_NO_ECHO=1 dwgrewrite`
  stages the conventional-arm rewrite to census against.
- `record_identity_survey.py` — the AC1021 corpus survey.
- `dump_section_bytes` / `ac21_token_diff` — layer-4 pair-compare and the
  AC1021 compressed-stream differential.
- `restore_gap_diffs.py` — the SAB record-level pair diff (face-chain
  anchored isomorphism) for constructed-vs-authored stream study.

---

## 6. The zero-keeping workflow

Every change runs the gate in order; only commit when each step holds its
expected value:

```mermaid
flowchart TD
    S["scope the change"] --> H["hermetic: cargo test --features serde"]
    H --> M["mirrors: gold_roundtrip + genus_gates + issue80"]
    M --> SM["family smokes (one per era)"]
    SM --> CO["full corpus (280, four axes 0) + genus sections"]
    CO --> ID["generation identity<br/>(md5 of gen_all output; serde-parity required)"]
    ID --> L4{"writer form change?"}
    L4 -->|yes| RC["record-identity census on touched specimens"]
    L4 -->|no| OK
    RC --> OK["commit (message carries before→after counts)"]
```

The standing battery values live in `NEXT_SESSION.md`'s verification gate;
the identity history there records every intended content change as a
one-way chain.

---

## 7. Component map

| Component | Role | Layer |
|---|---|---|
| `run_corpus.py` | the 280-file cycle → `report.json` | 3 |
| `run_roundtrip.py` | single-family smoke | 3 |
| `diff_fields.py` + `ignore_fields.toml` | the differ (**frozen**) | 3 |
| `normalize_gold.py` / `normalize_silver.py` | faithful comparison projections | 3 |
| `struct_axis.py` | the 17 header/section keys | 3 |
| `record_size_census.py` | handle-keyed size+CRC identity, any era | 4 |
| `record_identity_survey.py` | the AC1021 survey | 4 |
| `src/bin/dump_section_bytes.rs` | record-framed raw byte dumps | 4 |
| `src/bin/ac21_token_diff.rs` | AC1021 compressed-stream differential | 4 |
| `src/bin/dwg2json.rs` | silver decode → JSON (serde-gated) | 3 |
| `src/bin/dwgrewrite.rs` | the conventional-arm staging writer | 4 |
| `genus_extract.py` → `genus_expectations.json` | the expectation pin (fixtures-only) | 5 |
| `genus_gates.py` | the constructed-corpus gate + ranked sections | 5 |
| `tests/genus_gates.rs` | the cargo mirror (fresh == pin) | 5 |
| `strict_load_probe.py` | the loader audit (bcad/acad/core) | loader |
| `entity_verification.py` | the 29-kind × 5-axis matrix | loader |
| `restore_gap_diffs.py` | SAB record-level pair diff | loader |
| `check_env.py` / `bootstrap_oracle.sh` | environment + on-demand gold build | — |

## 8. Design principles (the load-bearing ones)

1. **Gold is read-only and never edited** — verify every field against its
   spec (`dwg.spec`/`dwg2.spec` + the common blocks) before changing code.
2. **The differ never changes to make a diff pass** — fixes belong in the
   codec, the dump, or the normalizers (as faithful projections).
3. **Attribution is measured, not assumed** — decode the authored bytes,
   survey the whole specimen family, and only then design the fix; the
   corpus is the authority where the spec undersides the runtime.
4. **Author identity is never forged** — product strings stay silver's own;
   format fields (magics, triples, terminators) follow the authored genus.
5. **Captured author data re-emits verbatim** — wire captures (SABs, wire
   text, raw passthrough classes) are replayed, not re-encoded; repairs fire
   only on degenerate values (nulls, zeros) an authored file never carries.
6. **Every verdict is mechanized or recorded as a manual procedure's
   evidence** — the probe's audit logs, the census counts, the identity
   chain: numbers a future session can re-verify, not prose it must trust.
