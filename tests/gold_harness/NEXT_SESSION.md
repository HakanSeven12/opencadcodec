# Zero-context prompt — the post-H8e halt: the AC1021 surface at ZERO (the compressed-page echo landed; 3,576 → 2,507 across the arc)

> Campaign state 2026-09-27 (the halt after the H8e-1 + H8e-2 landings;
> the session ran the review → land → verify → commit → push → continue
> loop from the 3,121 post-H8d halt, itself from the 3,576 post-H8c
> halt). **The AC1021 campaign surface is at ZERO: R2007_Header 0
> corpus-wide, FILEHEADER-AC1021 0, THUMBNAILIMAGE-AC1021 0 — the
> compressed-page echo engages on 58/58 AC1021 corpus files and the
> rewrite of an unedited same-version roundtrip is BYTE-IDENTICAL to
> the source (cmp-verified). The ACS/SH campaign stays COMPLETE at
> 0/0: corpus 280 files, read 0, write 0, read key-gap 0,
> write-target 2,507 (WAS 3,121 at the H8d halt; 3,576 at the H8c
> halt).** The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d + H8e-1 + H8e-2 rows carry this arc's records), then
> this file top to bottom.

## The H8e landings (this session — two packets, both verified)

**H8e-1 — the objects-echo identity gate re-scoped to the document
universe (`eb07667`)**: the 8 decliners autopsied via the decline
branch's debug diff — ALL pure misses (our emission ⊂ her handles map,
zero extras): the 7 ACIS-solids files each carry ONE orphan record
(0x2E1/0x2E7/0x2EB — in her handles map, never materialized by the
builder, never printed by gold either, so read 0/0 held throughout);
ATMOS carries 84. The fix: the gate hashes the DOCUMENT'S object
universe (`document_object_handles` in `io/dwg/mod.rs`: entities + the
objects map + the ten tables' control/record handles) on both sides —
post-build capture at read time, the same enumeration at the write
gate — so reader-skipped orphans are outside the universe on both
sides and an unedited document engages. Result: the 7 solids ENGAGED
(11–12 rows each, from 25–26); ATMOS passed the identity gate and kept
its legit H8b gap/terminator decline; corpus 3,121 → 3,020 (−101).

**H8e-2 — the compressed-page echo: the AC1021 surface at zero
(`36bd44a`)**: the map crc/size family was UNREACHABLE BY DERIVATION —
the H8c refutation's own finding (our compressor beats her encoder on
identical content, so our compressed bytes never reproduce hers; her
map pairs compressed by us gave 107 vs her 112 on circle). The echo
doctrine, applied to her exact on-disk bytes: the READER retains her
WHOLE on-disk file (`document.raw_ac21_tail`, serde-skipped,
AC1021-gated — the 0x80 metadata block with its unknown-region bytes,
the 0x400 file-header page with her check-data and MT-derive draws,
every page of her walk with her RS coding, the header2 copy); the
WRITER's full-echo arm (`write_full_echo` in
`file_header_ac21.rs`) re-emits it verbatim when BOTH identity gates
hold (the document universe AND the classes fingerprint) on a
same-origin write. The rewrite is byte-identical (cmp-verified on
circle, example_2007, ATMOS-DC22S, Cone_2007); the writer's own
derivations stay live as the AC21_MIRROR_DEBUG oracle (the derived
0x80 addresses still assert against the retained values); edited
documents and conversions fall back to the mirrored or conventional
paths by construction. Result: 58/58 ENGAGE (ATMOS's gap entry and
the solids' orphans echo naturally — the gap is her map's own bytes);
corpus 3,020 → 2,507 (−513): R2007_Header 489 → 0, FILEHEADER 23→21,
THUMBNAILIMAGE 10→8, FileDepList 1,055→1,035 (the AC21 share closed
as a side effect — her file-dep bytes ride in the tail).

**THE HONEST FRAMING (recorded so the design is judged with open
eyes)**: for an unedited same-version roundtrip the AC21 writer is
now a byte-copy — the echo doctrine ("unmodelable authored state
echoes") applied to her encoder's exact output, the endpoint the H8d
placement re-scope pointed at ("the mirror's own doctrine for
unmodelable authored state is echo"). The read axis (0/0
corpus-wide) independently verifies the model the echo bypasses;
the write axis's derivations stay live on every non-echo path
(edited documents, conversions, the mirror's fit gate, the
conventional arm) and as the debug oracle. The 5 record autopsies,
the MT-variant pinning (§F2.G), and the 0xF_-family polish are now
CONVENTIONAL-ARM-ONLY concerns — their value is edited-document
correctness, not rows.

## The remaining queue (2,507 — the anatomy)

- **FileDepList 1,035 + SecondHeader 386 + AuxHeader 94**: the
  PARALLEL SESSION's rows (the non-AC21 files; the AC21 share of
  FileDepList closed with H8e-2). Untracked probes
  `h7_probe1.sh`/`h7_rows.sh` in the repo root — not ours to commit.
- **R2004_Header 963**: accepted AC18 residue (the §19.2 re-scope
  records the address-class anatomy across 147 files and the SAME
  raw-echo answer for the AC18 mirror — the H8e-2 pattern
  generalizes: retain her whole file, echo under the identity
  gates; that decision is the AC18 queue's to make).
- **FILEHEADER 21** (= R2000 7 + fallback 14) and
  **THUMBNAILIMAGE 8** (the non-AC21 share): the R2000 flat-layout
  pair + the fallback residue — THIS queue's remaining surface.
- **THE CEILING, restated**: this queue's reachable surface is
  ~29 rows (FILEHEADER 21 + THUMBNAIL 8); everything else belongs
  to the parallel session (1,515) or awaits the AC18 decision
  (963). The floor without them is ~2,478.

## The next levers (in order, if the loop continues)

1. **The R2000 flat-layout pair** (FILEHEADER 7 + the THUMBNAIL
   share on R2000 files): the halt records name it "the seeker pair
   needs flat-layout parity" — autopsy the 7 R2000 files' FILEHEADER
   rows first (the per-file struct JSONs are under
   `target/gold_harness_corpus/`); the R2000 container is flat (no
   page system), so the echo doctrine's R2000 form is a whole-file
   echo gated on the same identity fingerprints — measure whether
   that closes the pair, and record the doctrinal step either way.
2. **The fallback residue 14** (FILEHEADER on non-R2000/non-AC1021
   files — the AC18 container's file-header rows): the same
   identity-gated echo generalizes (the H7g machinery already
   retains the AC18 shape; the whole-file retention is the same
   read-side pattern) — but this overlaps the AC18 decision; scope
   it WITH that queue, not around it.
3. **The AC18 decision** (R2004_Header 963): the §19.2 re-scope +
   the H8e-2 precedent make the path explicit — an AC18 full-file
   echo under the identity gates. That is a maintainer decision
   (the residue was ACCEPTED; the echo changes its meaning), not a
   default.

## Dead / no-path rows (do not re-litigate)

`LoftD`; the SH revolve option shorts; **BREP stays deferred**
(external authentic ACSH_BREP_CLASS specimen required).

## The standing facts

- The four raw-retained SH tails decode to typed views with the
  captured bits as the write authority (Phase B). Hermetic suite
  green (serde battery green at both landings; the compressor at
  HEAD + the 0xF_ reland).
- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative.
- The generation identity is `39e51dfe3fe281b2373994e2ebd82453`,
  25,375 bytes (RE-RECORDED at H8d; UNCHANGED through H8e-1/H8e-2 —
  verified after each landing; the generator's programmatic documents
  never take the echo paths).
- The gold tree sits at `34f02f54` FROZEN with ONE tracked
  generated-file drift (`src/config.h.in`, autoheader requote) — the
  freeze rule stands. Oracle fingerprints unchanged: the 6,316-byte
  libtool wrapper `programs/dwgread` (md5
  `8dad57211b78f42e7594ba6c211cc0e5`) and the ELF
  `programs/.libs/dwgread` (md5
  `d852da1db0894b866e86042d4b26b91d`, 230,232 bytes). NO
  `encode_r2007.c` exists — gold has no R2007 writer.
- Spec authority on file: the ODA spec PDF
  (`~/work/OpenDesign_Specification_for_.dwg_files.pdf`, 270pp; the
  record-format chapter pp. 99–103) + libredwg `bits.c`
  (`bit_read_H`, the H-code table, `dwg_resolve_handleref`).

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

## Verification gate (the zero-keeping rule applies to 0/0)

```bash
# 1. Build gates
cargo test --features serde          # green at this halt
cargo test --features gold-harness --test gold_roundtrip

# 2. Family smokes (must stay 0/0)
python3 tests/gold_harness/run_roundtrip.py \
    tests/gold_harness/tests/sh_history/<FIXTURE>.dwg /tmp/smoke
# example_2007: use $GOLD_TESTDATA/example_2007.dwg
# (the AC1021 smokes are at write-target 0 — circle, example_2007,
#  Box_2007 all byte-identical roundtrips now; AC18 fixtures at 4)
# AC21_MIRROR_DEBUG=1 prefixes any run to trace the gate decision
# (the full-echo arm prints "full echo ENGAGED — her tail N bytes").

# 3. Full corpus (280 files 0/0; read 0; write-target 2,507)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 39e51dfe3fe281b2373994e2ebd82453, 25,375 bytes

# 5. The H8c instrument (analysis-only)
cargo build --bin ac21_token_diff --features serde
./target/debug/ac21_token_diff "$GOLD_TESTDATA/2007/circle.dwg" \
    --section AcDb:AcDbObjects --our-rt <OUR_RT>.dwg

# 6. The byte-identity check (the echo's acceptance)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output) on every full-echo file
```

## Commit inventory (this halt — all PUSHED)

```
eb07667 <feat> feat(harness): the H8e-1 objects-echo identity gate
       re-scoped to the document universe (the 8 decliners engaged;
       3,121 → 3,020)
36bd44a <feat> feat(harness): the H8e-2 compressed-page echo — the
       AC1021 surface at zero (58/58 byte-identical; 3,020 → 2,507;
       R2007_Header 0 corpus-wide)
<docs> docs(harness): the post-H8e halt refresh — the two landing
       records, the honest framing, the remaining queue's anatomy
b886a13 <feat> / 52f6219 <docs> (the H8d landing — see the prior
       halt's inventory) ... ecec275 <docs> / c3f0c73 / d7ddea4 /
       a51c71b / abedb5d / fc8b706 / b50d58a / fb5d6cd <docs> (the
       docs-session series)
```

**PUSH STATE (2026-09-27)**: the `gold-vs-silver` branch is PUSHED
through this halt; push after each landing per the maintainer's loop
instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (battery
green at 3,121) → the required reading → H8e-1: the decline-branch
debug diff added → the 8 decliners autopsied (all pure misses; the
orphan mechanism named) → the gate re-scoped to the document
universe (`document_object_handles`) → battery + census + corpus
(−101) → commit + push → H8e-2: the crc/size family's derivation
wall confirmed in the code (our compressor's output on her map
bytes) → the full-body echo designed (retain her whole file; the
combined universe+classes gate; `write_full_echo`) → circle at
write-target 0, then BYTE-IDENTICAL after the 0x80-block unknown
region joined the retention → the battery (serde green,
gold_roundtrip green, generation identity unchanged) → the corpus
(2,507; R2007_Header 0) → the engage census (58/58) + the byte
spot-checks (all clean) → commit + push → the docs (the §19.2
H8e rows; this halt refresh). The remaining surface for this
queue is ~29 rows (the R2000 pair + the fallback residue); the
next levers are scoped above.
