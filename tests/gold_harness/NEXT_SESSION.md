# Zero-context prompt — TARGET ZERO held; THE CONVENTIONAL ARM AT ZERO DIVERGENT RECORDS (H8h-ext-12: the zero-residual landing — the two ASSOC ref-type nibbles + the TABLECONTENT wire capture; the 58-file AC1021 survey at 58/58 files, 0 divergent)

> Campaign state 2026-09-28 (the halt after the H8h-ext-12 landing; the
> session continued the loop from the H8h-ext-11 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0,
> write-target 0** — verified after the landing). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. **THE RECORD-IDENTITY SURVEY IS AT ZERO: 58/58
> AC1021 files at 100%, 0 divergent records — the residual named-record
> inventory is EMPTY.** Read `tests/gold_harness/AGENTS.md` first,
> then §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d–H8h-ext-12 rows carry this arc's records), then this
> file top to bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-12)

1. **h=392/h=393 — the handle-reference TYPE nibbles**: the byte dumps
   pinned both diffs to single ref-code nibbles in the handle streams.
   h=392 (ACDBASSOCALIGNEDDIMACTIONBODY): her empty-values pab
   dependency ref is (4.2.397) — SoftPointer — where our writer
   emitted 5 (gold's dwg2.spec pab block declares 5; the wire is the
   authority — the same spec-vs-wire split as the H8h-ext-3 deps
   census {3: 27} and the H8h-ext-4 edge param). h=393 (ACDBASSOC-
   OSNAPPOINTREFACTIONPARAM): her compound child.parameter (child
   id=0) is (4.0.0) — SoftPointer — where our writer emitted 3
   (gold's child_param block declares 3). THE FIX: both writer refs
   switched to SoftPointer; the child secondary/tertiary refs (child
   id != 0, no corpus specimen) keep gold's declared 3.
2. **h=BF2 — the TABLECONTENT closure (the ext-8 wire-capture
   doctrine, third application)**: the class (type 529, 14.5KB) has
   NO gold spec block ("Unknown Class object 529 TABLECONTENT") and
   no ODA documentation; our modeled AcDbLinkedTableData emission
   diverged structurally — her main region 17,587 bits vs our
   modeled 15,963 (203 bytes, near-full) — while the TEXT region
   (98,348 bits, the cell texts) and the HANDLE stream (0xD8)
   re-emitted BIT-IDENTICAL, isolating the divergence to the main
   body. THE FIX: the AC1021 reader captures the body verbatim — the
   main bits from the body start (after the common fields) to the
   main-data end, the text-region bits, and the handle tail — and
   the writer re-emits all three raw (write_bit / write_text_bit /
   write_handle_bits); the modeled emission stays the
   DXF/programmatic fallback.
3. **THE PAD LESSON, EXTENDED**: the first capture landing produced
   the right SIZE but a wrong final byte (her 0x9b vs our 0x98) —
   the merged writer's handle-stream close pads with ZEROS
   (write_spear_shift) while the author's final partial byte is 1s
   (§19 H8d); every other corpus record's handle stream ends
   byte-aligned so only h=BF2 exposed it. THE FIX: the writer
   re-creates the author's 1s pad explicitly — the captured handle
   bits extended to the byte boundary with 1s (the reader's ≤7-bit
   trim cut exactly her pad; the corpus records' handle streams end
   in a 0 bit or aligned, so the trim never over-cuts).
4. **THE NORMALIZE-LAYER REGRESSION, caught by the family smokes**:
   the new wire_* model fields first leaked into the semantic
   compare as extra_in_silver rows on the R2000/R2004 smokes (6+6
   diffs) — popped in the UNKNOWN_ENT path (the dwg_raw_tail_bits
   precedent: writer-side fidelity channels have no gold
   counterpart).
5. **Measured: example_2007 540/540 (streams 417109 = 417109); THE
   FULL 58-FILE AC1021 SURVEY AT ZERO DIVERGENT RECORDS — 58/58
   files at 100%.** The gates: serde 1602/0, gold_roundtrip,
   issue80 7/0, four family smokes 0/0/0/0, corpus 280 files
   0/0/0/0. Generation identity UNCHANGED
   (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes, verified with
   AND without `--features serde`). The echo byte-identity held.

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
EXCLUDED from the fingerprint; `xdic_by_handle`, the modeled
subcurve fields, the persubent tails, the SEQEND flag captures, the
H8h-ext-8 constraint-group node captures, the H8h-ext-10
persubent-id tail captures, and the H8h-ext-12 TABLECONTENT wire
captures are INCLUDED (real state — an edit declines the echo).

**The honest framing, FINAL**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. **The conventional arm is record-identical to the author's
stream on EVERY AC1021 corpus file (58/58 at 100%, 0 divergent
records) — the residual named-record inventory is EMPTY.** What
remains outside the survey's reach: the ATMOS 84 her-only orphans
(the broken-map pre-existing issue), the pre-2007 constraint-group
conventional arms (echo-covered, ungated), and the
unattested/dead rows (the subcurve action types, the MT crc_seed
draws, LoftD, BREP).

## The remaining work (all optional — the target stays reached)

- **The pre-2007 constraint-group conventional arms** (the
  r14/2000/2004/2010/2013 `Constraints.dwg` specimens): the
  H8h-ext-8 capture is AC1021-gated; those eras keep their current
  behavior (echo-covered on the corpus axes, ungated by the AC1021
  survey).
- **The unattested subcurve action types** (17=ELLIPSE, 19=LINE,
  23=LINESEG3D, 42=NURB3D, 27=CURVE3D): no corpus specimens.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (the echo path never runs it for unedited roundtrips).
- **The dead/no-path rows**: `LoftD`; the SH revolve option shorts;
  **BREP stays deferred** (external authentic ACSH_BREP_CLASS
  specimen required).
- **The ATMOS 84 her-only orphans**: the broken-map pre-existing
  issue (documented; the object map itself is corrupt in her file —
  the records exist on the wire but not in her map).
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does); a DWG→DXF→DWG
  roundtrip of a tailed record would drop them — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-12). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- **The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): 58/58 files at 100%, 0 divergent
  records, 84 her-only orphans (ATMOS's broken map).** circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  Constraints 219/219; ATMOS 340/340; **example_2007 540/540**.
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
  libredwg `bits.c`. NEITHER documents the ACDBASSOC* classes, the
  subcurve wire, the non-assoc PersSubentManager, the
  constraint-group node classes, or the TABLECONTENT wire (the
  authority closure, commits `cc1a6c8` + the H8h-ext-8/ext-12
  greps); the ASSOC persubent variant HAS a gold spec block (the
  H8h-ext-6 re-parse follows it); the 3DSOLID history_id AVAIL_BITS
  rule IS in gold's dwg.spec COMMON_3DSOLID (the H8h-ext-10
  re-read follows it); gold's pab/child_param declared ref codes
  (5/3) are WRONG vs the wire (4/4) — the H8h-ext-12 census.
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). **The merged-stream
  frame math**: RL = main + text + 16 (the size ushort) + 1 (the
  flag); main_data_end = RL − text − 17; the text region =
  [main_data_end, RL − 17); the handle stream starts at RL. The
  record frame: [MS][data span][crc16 LE]; the MS value = the span
  INCLUDING the crc pair; gold's Hdlsize counts from RL to the
  span end minus 16 (its own accounting — the real handle stream +
  pad runs to the span end). The BL forms: 00 = 4-byte LE, 01 = 1
  byte, 10 = 0 (2 bits); the BD forms: 00 = full 66-bit LE double,
  01 = 1.0, 10 = 0.0. **The SEQEND lesson (H8h-ext-7)**:
  era-derived "conventions" verified on one corpus family are
  per-author forms — prefer the read capture, keep the convention
  as the DXF fallback only. **The pad lesson (H8h-ext-8, extended
  in H8h-ext-12)**: a wire capture that runs to the record end must
  TRIM the author's closing 1s pad (≤7 bits) — and the writer must
  re-create the pad explicitly with 1s (the merged writer's own
  handle-stream close pads with 0s; only a record whose handle
  stream ends mid-byte exposes it). **The era-gate lesson
  (H8h-ext-9)**: a workaround added for one era's wire quirk must
  be GATED to that era; the normalize layer can hide such losses —
  the byte survey is the only witness. **The presence lesson
  (H8h-ext-10)**: an author field that is sometimes-present on the
  wire needs the wire PRESENCE captured (Some(NULL) vs None); a
  writer-side DERIVATION is a DXF fallback only. **The recompute
  lesson (H8h-ext-11)**: a model-API semantic applied to a
  DWG-read value recomputes the author's stored double (1 ulp); the
  DWG read/write paths treat the wire f64s as exact. **The
  ref-code lesson (H8h-ext-12)**: gold's declared handle-reference
  codes are per-spec-guess, not per-wire — the author's ref-code
  nibble is wire state (her 4 vs gold's declared 5/3); when the
  census is uniform, fix the writer's type; when authors diverge,
  capture the code. **The normalize-lesson (H8h-ext-12)**: every
  new wire-capture model field must be popped in the
  normalize_silver compare paths (the dwg_raw_tail_bits precedent)
  — the family smokes catch the leak.

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use long timeouts on the bash tool for
the corpus — the full corpus takes ~4 minutes; the bash tool's
timeout parameter is capped at 120 s in practice — run the corpus
as a background process and read its logs).

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
# current state: 58/58 files at 100%, 0 divergent records
# (the only her-only set: ATMOS's 84 broken-map orphans)
# circle 211/211; PolyLine3D 218/218; Constraints 219/219;
# ATMOS 340/340; example_2007 540/540

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt)

```
574a59e <docs> the post-H8h-ext-6 halt refresh (the prior halt head)
a1a1506 <feat> H8h-ext-7: the SEQEND captured-flags fix — the four
       polyline-family writers use the read-captured flag pair, the
       era convention demoted to the DXF fallback (15 -> 14; 55/58 at
       100%)
b5e16b5 <docs> the post-H8h-ext-7 halt refresh
195ef3b <docs> the Constraints 3E3 dissection-state handover
d6b0431 <feat> H8h-ext-8: the Constraints 3E3 closure — the AC1021
       node-region wire capture for the ASSOC2DCONSTRAINTGROUP (the
       class-name TUs, the main bits, the handle tail; naive as the
       DXF fallback; Constraints 219/219; 13 -> 12)
d3d2258 <feat> H8h-ext-9: the ATMOS closure — the LAYER/STYLE
       authored-slots capture + the dictionary-key cut gated to
       r13_14_only (the Cyrillic keys pass verbatim on R2000+);
       ATMOS 340/340; 12 -> 9
0625f6c <feat> H8h-ext-10: the example_2007 first fruits — the
       hasatts captured bit, the REGION history_id wire presence,
       the persubent-id tail capture; 9 -> 5
3dd3362 <feat> H8h-ext-11: the wire-exactness landings — the
       RAY/XLine read-side normalize removed, the ordinate
       measurement recompute removed on both sides; 5 -> 3
000f3b2 <feat> H8h-ext-12: the zero-residual landing — the two ASSOC
       ref-type nibbles (the pab empty-values dependency + the
       compound child.parameter, both SoftPointer per the wire) +
       the TABLECONTENT wire capture (the ext-8 doctrine, third
       application) + the explicit 1s pad; 3 -> 0 (58/58 at 100%)
       [this commit also carried the §19.2 H8h-ext-12 row and the
       post-ext-12 halt refresh of this file]
<docs> the post-H8h-ext-12 maintenance review — the §19 header
       state annotation (PHASE 2 landed; the survey at 58/58 /
       0 divergent), the §19.4.B declared-vs-wire ref-type census,
       the §19.4.C pad × wire-capture interplay, and this file's
       commit-inventory hashes
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-11
halt → the h=392/h=393 byte dumps (the handle streams parsed
ref-by-ref: her (4.2.397)/(4.0.0) vs our 5/3 — single ref-code
nibbles; gold's declared 5/3 wrong vs the wire) → the ref-type
fixes (both SoftPointer; the unattested child secondary/tertiary
keep gold's 3) → the h=BF2 autopsy (TABLECONTENT: no gold spec
block; the text region + handle stream bit-identical, isolating
the 203-byte divergence to the main body) → the wire capture (the
ext-8 doctrine: main + text + handles verbatim, AC1021-gated) →
the pad lesson extended (the first landing's right-size-wrong-byte:
the merged writer's handle close pads 0s, the author 1s; the
explicit 1s-pad extension in the writer) → the normalize-layer
regression caught by the family smokes (the wire_* fields popped
in the UNKNOWN_ENT path) → the gates (serde 1602/0,
gold_roundtrip, issue80, family smokes, corpus 0/0/0/0, generation
identity unchanged and feature-independent, echo byte-identity
held) → the full survey (**58/58 files at 100%, 0 divergent
records — the residual named-record inventory EMPTY**) → the docs
(§19.2 H8h-ext-12 row; this halt record). **The maintainer's loop
instruction — "repeat process until target = zero" — is satisfied
IN FULL: the corpus is at zero on every axis, AND the conventional
arm is record-identical to the author's stream on every AC1021
corpus file.** The loop's residual work is now entirely outside
the survey's reach: the pre-2007 constraint-group arms, the
unattested subcurve types, the MT crc_seed draws, the dead rows,
and ATMOS's 84 pre-existing broken-map orphans.
