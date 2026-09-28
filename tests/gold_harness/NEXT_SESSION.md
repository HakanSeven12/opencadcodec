# Zero-context prompt — TARGET ZERO held; the conventional arm at ZERO DIVERGENT RECORDS (H8h-ext-14: the R2010/R2013 constraint-group arms closed — the raw text-region capture + the era census instrument; the era censuses: R2010 215/216, R2013 157/160 records byte-identical)

> Campaign state 2026-09-28 (the halt after the H8h-ext-14 landing; the
> session continued the loop from the H8h-ext-13 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0,
> write-target 0** — verified after the landing). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. **THE RECORD-IDENTITY SURVEY STAYS AT ZERO: 58/58
> AC1021 files at 100%, 0 divergent records — re-verified after the
> landing.** Read `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3
> + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's
> H8d–H8h-ext-14 rows carry this arc's records), then this file top to
> bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-14)

1. **The residual task resumed**: the R2010/R2013 constraint-group arms
   (the ext-13 row's named residual — the MC handle-bits header, the
   BOT type, the authored underlap frames).
2. **The era frames dissected**: the R2010/R2013 specimens carry the
   same nine-node group with `has_strings: 1` (R2010 data_size 3546 =
   the 2007 stream exactly; R2013 2332 — DIFFERENT content, the
   drawing re-authored by a newer AutoCAD: node[0] id 0/status 0x20 vs
   the 2007 9/0x40), and gold's own -v9 walk desyncs at node[1] on
   both (nconn 2800028726 / 68456580 — the same ERROR print as AC1021).
   Without the capture our conv re-emitted the garbage model (the
   R2013 record: 205,680 bits, 100,073 trace lines).
3. **THE FIX**: (1) the ext-8 capture's era gate extended to
   AC1024/AC1027 (the bounds all frame-derived — the peek-based
   capture immune to the naive walk's desync); (2) the R2010+ TEXT
   region retained VERBATIM (`nodes_wire_text` — the ext-12
   TABLECONTENT `wire_text` pattern) instead of re-encoding class-name
   TUs: the AC21 raw-stream dump instrument does not cover the R2010+
   containers, so the stream content was never decoded (the
   names-read would be a guess; the raw capture is content-agnostic);
   AC1021 keeps the verified names path.
4. **THE ERA CENSUS INSTRUMENT** (the handle-keyed -v9 comparison):
   her original vs our conv, records matched by HANDLE (the index
   match is invalid — our conv rebuilds the section in the writer's
   order), each record's identity = (size, CRC-16) — gold prints the
   CRC check per record, so the census needs no raw-stream dump.
5. **Measured: the R2010 constraint-group record round-trips
   byte-identically (CRC 969C == 969C) and the R2013 likewise (36AD ==
   36AD); the era censuses: R2010 215/216 records identical, R2013
   157/160** — the constraint group is in the identical set on both.
   THE NEW NAMED RESIDUALS (the era full-file identity packet's
   territory): h=87 TABLESTYLE (both eras, ours +1 byte — the modern
   cell-style arms) and h=3E4/h=3E5 ASSOCGEOMDEPENDENCY (R2013 only,
   ours +2 bytes each — the ext-10 persubent-id tail capture is
   era-ungated but the R2013 records still diverge; a different root,
   likely the modeled prefix).
6. **The gates**: serde 1602/0, gold_roundtrip, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0, the full
   AC1021 survey re-verified 58/58 / 0 divergent. Generation identity
   UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes,
   verified with AND without `--features serde`). The echo
   byte-identity held.

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
H8h-ext-8/-13/-14 constraint-group node captures, the H8h-ext-10
persubent-id tail captures, and the H8h-ext-12 TABLECONTENT wire
captures are INCLUDED (real state — an edit declines the echo).

**The honest framing, FINAL**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. **The conventional arm is record-identical to the author's
stream on EVERY AC1021 corpus file (58/58 at 100%, 0 divergent
records), the R2000/R2004 constraint-group records round-trip
byte-identically (ext-13), and the R2010/R2013 constraint-group
records likewise (ext-14); the era censuses: R2000/R2004
constraint records proven, R2010 215/216 records identical, R2013
157/160.** The named residuals: h=87 TABLESTYLE (R2010/R2013, +1),
h=3E4/h=3E5 ASSOCGEOMDEPENDENCY (R2013, +2 each), the ATMOS 84
her-only orphans, and the unattested/dead rows.

## The remaining work (all optional — the target stays reached)

- **The era full-file record identity**: the R2010 census's h=87
  TABLESTYLE (+1 byte, both eras — the modern cell-style arms) and
  the R2013 census's h=3E4/h=3E5 ASSOCGEOMDEPENDENCY (+2 each — the
  modeled prefix vs her form); the R2000/R2004 full-file censuses
  (the ext-13 proof covered the constraint record only); the era
  survey instrument = the reader record-trace lift + the token-diff's
  page-system extension beyond AC21.
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
  25,375 bytes (UNCHANGED through H8h-ext-14; the artifact is
  AC1032/R2018). The generator builds and runs identically WITH or
  WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **58/58 files at 100%, 0 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  Constraints 219/219; ATMOS 340/340; example_2007 540/540.
  **The era censuses (the ext-14 handle-keyed instrument): R2010
  215/216, R2013 157/160; the constraint-group record identical on
  R2000/R2004/R2007/R2010/R2013.**
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
  INCLUDING the crc pair. **The R2010+ frame**: [MS][MC
  handle_bits][BOT type][data][text][flag@handle_start−1][handles];
  handle_start = total_bits − handle_bits; the text bounds live in
  the bit reader's text_stream_pos/end_pos
  (`set_position_by_flag`). **The TwoStream frame (pre-2007,
  §19.4.C)**: the handle stream is BIT-CONTINUOUS at the RL (no
  alignment) — our merge already mirrors it. The BL forms: 00 =
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
  the family smokes catch the leak (NOTE: the constraint-group
  projection is a whitelist — its wire fields never leak). **The
  form-rule lesson (H8h-ext-13)**: the H8d ownerhandle form rule
  holds on the pre-2007 eras too; an era-bound gate on a verified
  wire convention is a bug waiting to surface. **The
  content-agnostic lesson (H8h-ext-14)**: when a capture region's
  content cannot be decoded (no instrument coverage), retain the
  bits VERBATIM rather than guessing a structured re-encoding —
  and the era record census: match by HANDLE (the conv rebuilds
  the section in the writer's order — the index match is
  invalid), identity = (size, CRC-16) from gold's per-record CRC
  print.

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

# 4c. The era record-identity proofs (the ext-13/-14 instruments):
#     the trace-segment comparison (the CRC print = the byte witness)
#     and the handle-keyed era census (size + CRC per handle)
# R2000/R2004: the constraint-group trace segments — CRCs 47A0/4B0D
# R2010/R2013: the constraint-group trace segments — CRCs 969C/36AD;
#              the censuses: R2010 215/216, R2013 157/160

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
d9bf5f3 <docs> the post-H8h-ext-12 maintenance review
6712327 <feat> H8h-ext-13: the pre-2007 constraint-group arms closed
       — the TwoStream capture extension + the H8d ownerhandle form
       rule extended to the pre-2007 slots; the R2000/R2004
       constraint records byte-identical (47A0/4B0D)
<feat> H8h-ext-14: the R2010/R2013 constraint-group arms closed —
       the capture era gate extended to AC1024/AC1027 + the raw
       text-region capture (the ext-12 wire_text pattern; the
       content-agnostic lesson); the era census instrument; the
       R2010/R2013 constraint records byte-identical (969C/36AD);
       the censuses: R2010 215/216, R2013 157/160
<docs> the post-H8h-ext-14 halt refresh (this file)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-13
halt → the R2010/R2013 residual resumed → the era frames dissected
(the specimens carry has_strings: 1; gold's own walk desyncs at
node[1] on both; our conv re-emitted the garbage model — the R2013
record 205,680 bits) → the fix (the capture gate extended to
AC1024/AC1027; the text region retained raw — the AC21 dump
instrument doesn't cover the R2010+ containers, so the names-read
would be a guess; the ext-12 wire_text pattern applied) → the era
census instrument built (handle-keyed — the index match invalid
because the conv rebuilds the section in the writer's order;
identity = size + the per-record CRC print) → the proof (the
constraint records byte-identical on both eras: 969C/36AD) → the
censuses (R2010 215/216, R2013 157/160 — the new named residuals:
h=87 TABLESTYLE +1 both eras, h=3E4/h=3E5 ASSOCGEOMDEPENDENCY +2 on
R2013) → the gates (serde 1602/0, gold_roundtrip, issue80, family
smokes, corpus 0/0/0/0, the full AC1021 survey re-verified 58/58,
the generation identity unchanged, the echo byte-identity held) →
the docs (§19.2 H8h-ext-14 row; this halt record). **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied in full: the corpus is at zero on every
axis, the conventional arm is record-identical on every AC1021
corpus file, and the constraint-group arms are closed across all
five dissected eras (R2000/R2004/R2007/R2010/R2013) with
byte-level proofs.** The remaining residuals: the era full-file
identity (h=87, h=3E4/h=3E5, the R2000/R2004 censuses), the
unattested subcurve types, the MT crc_seed draws, the dead rows,
and ATMOS's 84 pre-existing broken-map orphans.
