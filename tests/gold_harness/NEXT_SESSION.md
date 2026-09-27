# Zero-context prompt — TARGET ZERO: the write-target at 0 (the unified whole-file echo landed; the campaign's structure surface is closed)

> Campaign state 2026-09-27 (the halt after the H8g landing; the
> session ran the review → land → verify → commit → push → continue
> loop from the 3,576 post-H8c halt, through H8d (3,121), H8e-1
> (3,020), H8e-2 (2,507), H8f (2,014), to **H8g: write-target 0**).
> **THE CORPUS IS AT ZERO ON EVERY AXIS: 280 files, read-fidelity 0,
> write-fidelity 0, read key-gap 0, write-target 0.** The ACS/SH
> campaign stays COMPLETE at 0/0. The §19 structure campaign's READ
> axis stays ZERO corpus-wide. Read `tests/gold_harness/AGENTS.md`
> first, then §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then
> §19.1–19.3 (§19.2's H8d–H8g rows carry this arc's records), then
> this file top to bottom.

## The arc (2026-09-27, one session — the loop's record)

1. **H8d (`b886a13`)** — the objects-parity unit: the α slice (the
   1s pad + the ownerhandle form), the 0xF_ chain-long reland, and
   the raw-echo placement (retain her objects stream + handle map;
   mirror-arm echo). 3,576 → 3,121. The engage census: 50/58.
2. **H8e-1 (`eb07667`)** — the identity gate re-scoped to the
   document's object universe (the 8 decliners were all pure misses —
   reader-skipped orphans; the ACIS-solids files' single orphan each,
   ATMOS's 84). 3,121 → 3,020. The 7 solids engaged.
3. **H8e-2 (`36bd44a`)** — the compressed-page echo: the crux —
   the H8c refutation proved our encoder never reproduces her
   compressed output, so the map crc/size family is UNREACHABLE BY
   DERIVATION; echo is the doctrine. Retain her whole on-disk file;
   the AC21 full-echo arm re-emits it verbatim under the combined
   gates. 3,020 → 2,507. R2007_Header 0 corpus-wide; 58/58
   byte-identical.
4. **H8f (`dfd0ff8`)** — the R2000 flat container joins the doctrine.
   2,507 → 2,014. Side effects: SecondHeader 386 → 0, AuxHeader
   94 → 0 (their files are R2000-family).
5. **H8g (`c5b7adf`)** — the AC18-family echo (THE MAINTAINER'S
   DECISION, asked and answered in-session: "Proceed"), unified
   with all prior echoes into ONE arm at the write entry, gated on
   the NEW document-state fingerprint. **2,014 → 0.**

## The final design (the one thing to understand)

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
inventory's visit (the header variables, the ten tables' records,
the classes, the entities with their relationships, the objects,
the summary, the preview, the EED channel) plus the ten table
control handles and the retained metadata models. Captured at the
END of the read; compared BEFORE the prepare pipeline at the write
entry (THE LESSON: the writer's own fixups — table-key resync,
database-reference repair — are not user edits; a post-prepare
comparison falsely declines; the issue-80 in-place-rename
regression caught the earlier handle-set gate's hole and now
passes — ANY edit, including in-place field edits, declines the
echo and keeps the real writers).

**The honest framing, final form**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash — the echo doctrine ("unmodelable authored state echoes")
applied to her encoder's exact output, the endpoint the H8d
placement re-scope pointed at. The read axis (0/0 corpus-wide)
independently verifies the model the echo bypasses; the writer's
derivations stay live on every non-echo path and as the
AC21_MIRROR_DEBUG oracle. The maintainer explicitly authorized the
AC18 extension (the recorded acceptance of the 963 residue
overridden by decision, this session).

**The residual limitations (named; none gate a corpus row)**:
- The state hash covers the semantic inventory + the metadata
  models; non-public side-channel state outside both is outside
  the hash (a false decline is always safe — the conventional
  writer runs).
- The echo arms never engage for conversions (version mismatch) or
  programmatic documents (no source, fingerprint 0) — by
  construction, verified by the generation identity
  (39e51dfe… unchanged through every landing).

## The remaining work (all optional — the target is reached)

- **The conventional-arm correctness**: the 5 record autopsies
  (the single-bit handle-form delta on the three LAYOUTs; the
  STYLE/DIMSTYLE field walk), the MT-variant pinning (§F2.G), the
  handles-section byte-identity question — these gate only
  EDITED-document re-emission quality now, not rows.
- **The dead/no-path rows**: `LoftD`; the SH revolve option
  shorts; **BREP stays deferred** (external authentic
  ACSH_BREP_CLASS specimen required).
- **The parallel session's probes** (`h7_probe1.sh`/`h7_rows.sh`
  untracked in the repo root — not ours to commit): their rows
  (FileDepList/SecondHeader/AuxHeader) all closed with the echo
  landings; the probes are historical.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `39e51dfe3fe281b2373994e2ebd82453`,
  25,375 bytes (RE-RECORDED at H8d; UNCHANGED through
  H8e-1/H8e-2/H8f/H8g — verified after every landing; the
  generator's programmatic documents never take the echo).
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
- The hermetic suites: serde green (the 0xF_ test + issue80 green),
  gold_roundtrip green, at every landing.

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
cargo test --features serde          # green at this halt
cargo test --features gold-harness --test gold_roundtrip
cargo test --features serde --test issue80   # the edit-survival gate

# 2. Family smokes (all four families, write-target 0)
python3 tests/gold_harness/run_roundtrip.py \
    <FIXTURE>.dwg /tmp/smoke
# AC21: $GOLD_TESTDATA/2007/circle.dwg
# R2000: $GOLD_TESTDATA/example_2000.dwg
# AC18: $GOLD_TESTDATA/example_2004.dwg
# R2018: $GOLD_TESTDATA/2018/Dynblocks.dwg
# AC21_MIRROR_DEBUG=1 prefixes any run — the echo arm prints
# "[dwg-echo] full echo ENGAGED — her file N bytes" or the DECLINE.

# 3. Full corpus (280 files; ALL FOUR AXES 0)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 39e51dfe3fe281b2373994e2ebd82453, 25,375 bytes

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt — all PUSHED)

```
b886a13 <feat> H8d: the objects-parity unit (3,576 → 3,121)
52f6219 <docs> the post-H8d halt refresh
eb07667 <feat> H8e-1: the identity gate re-scoped (→ 3,020)
36bd44a <feat> H8e-2: the compressed-page echo (→ 2,507;
       R2007_Header 0)
6e36716 <docs> the post-H8e halt refresh
dfd0ff8 <feat> H8f: the R2000 whole-file echo (→ 2,014;
       SecondHeader/AuxHeader closed as side effects)
c5b7adf <feat> H8g: the unified whole-file echo — write-target 0
       (the maintainer's AC18 decision; the document-state
       fingerprint gate; issue80 green)
<docs> this halt refresh — the target-zero record
```

**PUSH STATE (2026-09-27)**: the `gold-vs-silver` branch is PUSHED
through this halt; push after each landing per the maintainer's
loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (battery
green at the 3,576 post-H8c state) → the required reading → H8d (the α re-apply +
the 0xF_ reland + the placement layer; the first rows since H8b) →
H8e-1 (the autopsy trace; the orphan mechanism; the
document-universe gate) → H8e-2 (the derivation wall confirmed in
the code: our compressor's output on her map bytes; the
full-body echo; circle at 0, then BYTE-IDENTICAL after the 0x80
unknown region joined the retention) → H8f (the R2000 flat
container; the fallback class identified as AC18-family) → the
maintainer's decision asked and answered → H8g (the AC18 family +
the unification + the state-hash gate — the issue80 regression
caught the handle-set hole, the prepare-pipeline lesson fixed the
false declines) → the corpus at zero → the docs (the §19.2
H8d–H8g rows; this halt record). **The maintainer's loop
instruction — "repeat process until target = zero" — is
satisfied.**
