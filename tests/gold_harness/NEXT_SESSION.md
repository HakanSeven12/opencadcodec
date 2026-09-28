# Zero-context prompt — TARGET ZERO held; the conventional arm at ZERO DIVERGENT RECORDS (H8h-ext-13: the pre-2007 constraint-group arms closed — the TwoStream capture extension + the H8d ownerhandle form rule extended to the pre-2007 slots; the R2000/R2004 constraint records round-trip byte-identical)

> Campaign state 2026-09-28 (the halt after the H8h-ext-13 landing; the
> session continued the loop from the H8h-ext-12 zero-residual halt:
> review → land → verify → commit → push. **THE CORPUS STAYS AT ZERO ON
> EVERY AXIS: 280 files, read-fidelity 0, write-fidelity 0, read
> key-gap 0, write-target 0** — verified after the landing). The ACS/SH
> campaign stays COMPLETE at 0/0. The §19 structure campaign's READ
> axis stays ZERO corpus-wide. **THE RECORD-IDENTITY SURVEY STAYS AT
> ZERO: 58/58 AC1021 files at 100%, 0 divergent records — re-verified
> after the landing.** Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d–H8h-ext-13 rows carry this arc's records), then this
> file top to bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-13)

1. **The residual task resumed**: the pre-2007 constraint-group
   conventional arms (the first named residual after the ext-12
   zero-residual landing). THE MEASUREMENT PROBLEM: the
   record-identity survey is AC1021-only (the reader's record-trace
   gate + the token-diff's AC21 page-system raw dump) — the era proof
   was built via the gold -v9 TRACE-SEGMENT COMPARISON instead: the
   constraint-group object's full decode block (the field prints +
   the unknown_bits hex + the handle prints + the CRC print), her
   original vs our conv, the file-address lines stripped. The CRC-16
   covers MS+span, so matching CRCs = matching bytes — a record-level
   identity proof.
2. **THE DISSECTION FINDINGS**: (a) our TwoStream close ALREADY
   mirrors the authored bit-continuous frame (§19.4.C) — the merge
   trace census: main_bits == handle_start_bits on 235/235 records of
   the R2000 specimen (the RL patch seeks back and the handle bytes
   overwrite bit-continuously into the partial byte; the final 1s pad
   closes); (b) her pre-2007 ownerhandle forms follow the SAME H8d
   rule as AC1021 — the census: (8.0) 44×, (12.1) 70×, (4.1) 82×,
   (4.2) 17×, (4.0) 11×, (10.1) 5× = exactly the
   relative-iff-shorter + numeric-tie-break distribution; (c) the
   node region's inline class-name TVs (pre-2007) sit INSIDE the
   captured main bits — no separate names capture.
3. **THE FIXES**: (1) the ext-8 capture's era gate extended from
   AC1021 to AC1015|AC1018|AC1021 (the names read stays AC1021-only —
   the TwoStream TVs are inline); (2) the constraint writer's
   handle-tail emission gets the ext-12 explicit-1s pad extension (a
   no-op for the byte-aligned AC1021 specimens — Constraints 219/219
   re-verified); (3) the H8d `write_first_ref_handle` ownerhandle
   form rule extended to the pre-2007 slots — the `r2007_plus` gates
   dropped in BOTH common writers (the entity entmode==0 slot and the
   non-entity owner slot). The first attempt's +1-byte handle stream
   (her (12.1.3) vs our (4.2.3E0) ownerhandle) closed by the form
   rule.
4. **Measured: the R2000 and R2004 Constraints.dwg constraint-group
   records round-trip BYTE-IDENTICAL** (the CRCs match — 47A0 ==
   47A0, 4B0D == 4B0D; the unknown_bits hex identical; the only trace
   diffs are gold's file-address prints).
5. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0 (the
   ownerhandle form change touches EVERY pre-2007 record — the
   semantic axes are form-agnostic and held), the full AC1021 survey
   re-verified 58/58 / 0 divergent. Generation identity UNCHANGED
   (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes, verified with
   AND without `--features serde` — the artifact is AC1032/R2018,
   outside the TwoStream change). The echo byte-identity held.

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
H8h-ext-8/-13 constraint-group node captures, the H8h-ext-10
persubent-id tail captures, and the H8h-ext-12 TABLECONTENT wire
captures are INCLUDED (real state — an edit declines the echo).

**The honest framing, FINAL**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. **The conventional arm is record-identical to the author's
stream on EVERY AC1021 corpus file (58/58 at 100%, 0 divergent
records) — the residual named-record inventory is EMPTY — and the
R2000/R2004 constraint-group records round-trip byte-identically
(the ext-13 trace-CRC proof).** What remains outside the surveys'
reach: the ATMOS 84 her-only orphans (the broken-map pre-existing
issue), the R2010+ constraint-group arms, the pre-2007 full-file
record identity, and the unattested/dead rows (the subcurve action
types, the MT crc_seed draws, LoftD, BREP).

## The remaining work (all optional — the target stays reached)

- **The R2010/R2013 constraint-group arms**: the H8h-ext-8/-13
  capture is gated to AC1015/AC1018/AC1021; the R2010+ frames (the
  MC handle-bits header, the BOT type, the authored underlap) need
  their own dissection before the capture extends.
- **The pre-2007 FULL-FILE record identity**: the era survey
  instrument (the reader record-trace lift + the token-diff's
  page-system extension beyond AC21) — a future packet; today's
  proof is record-level via the trace-CRC comparison.
- **The unattested subcurve action types** (17=ELLIPSE, 19=LINE,
  23=LINESEG3D, 42=NURB3D, 27=CURVE3D): no corpus specimens.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (the echo path never runs it for unedited roundtrips).
- **The dead/no-path rows**: `LoftD`; the SH revolve option shorts;
  **BREP stays deferred** (external authentic ACSH_BREP_CLASS
  specimen required).
- **The ATMOS 84 her-only orphans**: the broken-map pre-existing
  issue (documented; the records exist on the wire but not in her
  map).
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does); a DWG→DXF→DWG
  roundtrip of a tailed record would drop them — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-13; the artifact is
  AC1032/R2018). The generator builds and runs identically WITH or
  WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **58/58 files at 100%, 0 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  Constraints 219/219; ATMOS 340/340; example_2007 540/540.
  **The era specimens (the trace-CRC proof): the R2000/R2004
  Constraints.dwg constraint-group records round-trip
  byte-identically (47A0/4B0D).**
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
  INCLUDING the crc pair. **The TwoStream frame (pre-2007,
  §19.4.C)**: the handle stream is BIT-CONTINUOUS at the RL (no
  alignment) — our merge already mirrors it (the RL patch seeks
  back; the handle bytes overwrite into the partial byte; the
  final 1s pad closes; the merge-trace census: main_bits ==
  handle_start_bits on 235/235 R2000 records). The BL forms: 00 =
  4-byte LE, 01 = 1 byte, 10 = 0 (2 bits); the BD forms: 00 =
  full 66-bit LE double, 01 = 1.0, 10 = 0.0. **The SEQEND lesson
  (H8h-ext-7)**: era-derived "conventions" verified on one corpus
  family are per-author forms — prefer the read capture, keep the
  convention as the DXF fallback only. **The pad lesson
  (H8h-ext-8, extended in ext-12/-13)**: a wire capture that runs
  to the record end must TRIM the author's closing 1s pad (≤7
  bits) — and the writer must re-create the pad explicitly with
  1s. **The era-gate lesson (H8h-ext-9)**: a workaround added for
  one era's wire quirk must be GATED to that era; the normalize
  layer can hide such losses — the byte survey is the only
  witness. **The presence lesson (H8h-ext-10)**: an author field
  that is sometimes-present on the wire needs the wire PRESENCE
  captured (Some(NULL) vs None); a writer-side DERIVATION is a
  DXF fallback only. **The recompute lesson (H8h-ext-11)**: a
  model-API semantic applied to a DWG-read value recomputes the
  author's stored double (1 ulp); the DWG read/write paths treat
  the wire f64s as exact. **The ref-code lesson (H8h-ext-12)**:
  gold's declared handle-reference codes are per-spec-guess, not
  per-wire — the author's ref-code nibble is wire state. **The
  normalize-lesson (H8h-ext-12)**: every new wire-capture model
  field must be popped in the normalize_silver compare paths —
  the family smokes catch the leak. **The form-rule lesson
  (H8h-ext-13)**: the H8d ownerhandle form rule
  (relative-iff-shorter + the numeric tie-break) holds on the
  pre-2007 eras too (the R2000/R2004 census matches the rule's
  output distribution exactly) — an era-bound gate on a verified
  wire convention is a bug waiting to surface; and the era
  record-identity proof without the survey: the gold -v9
  trace-segment comparison (the CRC print is the byte-identity
  witness — the CRC-16 covers MS+span).

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

# 4c. The era record-identity proof (the ext-13 instrument:
#     the trace-segment comparison — her original vs our conv)
DWG_NO_ECHO=1 ./target/debug/dwgrewrite \
    "$GOLD_TESTDATA/2000/Constraints.dwg" /tmp/conv2000.dwg
"$GOLD_DWGREAD" -v9 /tmp/conv2000.dwg 2> our.log
# extract the ASSOC2DCONSTRAINTGROUP block from her/our logs,
# strip the Address lines, diff — the CRC line must match
# (the era specimens: R2000 47A0, R2004 4B0D)

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
       ref-type nibbles + the TABLECONTENT wire capture + the
       explicit 1s pad; 3 -> 0 (58/58 at 100%)
d9bf5f3 <docs> the post-H8h-ext-12 maintenance review — the
       reference-record addenda and the state annotations
<feat> H8h-ext-13: the pre-2007 constraint-group arms closed — the
       TwoStream capture extension (AC1015/AC1018, the inline TVs
       inside the captured bits) + the H8d ownerhandle form rule
       extended to the pre-2007 slots (both common writers); the
       R2000/R2004 constraint records round-trip byte-identical
       (the trace-CRC proof: 47A0/4B0D)
<docs> the post-H8h-ext-13 halt refresh (this file)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-12
halt → the residual list resumed (the pre-2007 constraint-group
arms) → the measurement problem (the survey is AC1021-only; the
token-diff is AC21-page-specific) → the era proof built via the
gold -v9 trace-segment comparison (the CRC print as the
byte-identity witness) → the TwoStream frame verified (the merge
trace census: our close already bit-continuous, 235/235) → the
ownerhandle census (her pre-2007 forms match the H8d rule exactly)
→ the fixes (the capture era gate extended; the explicit-1s pad in
the constraint writer; the H8d form rule's r2007_plus gates
dropped in both common writers) → the first landing's +1 byte (her
(12.1.3) relative vs our (4.2.3E0) absolute ownerhandle) closed by
the form rule → the proof (the CRCs match on both eras; the hex
identical) → the gates (serde 1602/0, gold_roundtrip, issue80,
family smokes, corpus 0/0/0/0, the full AC1021 survey re-verified
58/58, the generation identity unchanged — the artifact is
AC1032, the echo byte-identity held) → the docs (§19.2
H8h-ext-13 row; this halt record). **The maintainer's loop
instruction — "repeat process until target = zero" — remains
satisfied in full: the corpus is at zero on every axis, the
conventional arm is record-identical on every AC1021 corpus file,
and the first named residual (the pre-2007 constraint-group arms)
is closed with a byte-level proof.** The remaining residuals: the
R2010+ constraint-group arms, the pre-2007 full-file record
identity, the unattested subcurve types, the MT crc_seed draws,
the dead rows, and ATMOS's 84 pre-existing broken-map orphans.
