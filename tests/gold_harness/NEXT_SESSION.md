# Zero-context prompt — TARGET ZERO held; the era full-file record identity CLOSED (H8h-ext-15: all four era censuses at ZERO divergent records — seven systematic roots; the AC1021 survey re-verified 58/58; the generation identity UNCHANGED)

> Campaign state 2026-09-28 (the halt after the H8h-ext-15 landing; the
> session continued the loop from the H8h-ext-14 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0,
> write-target 0** — verified after the landing). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. **THE RECORD-IDENTITY SURVEY STAYS AT ZERO: 58/58
> AC1021 files at 100%, 0 divergent records — re-verified after the
> landing. THE ERA CENSUSES (the ext-14 handle-keyed instrument, the
> Constraints specimens): R2000 235/235, R2004 227/227, R2010 216/216,
> R2013 160/160 — ALL AT ZERO DIVERGENT RECORDS.** Read
> `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3 + §18.5–18.7
> in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's H8d–H8h-ext-15 rows
> carry this arc's records), then this file top to bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-15)

1. **The era full-file record identity resumed** (the ext-14 census's
   named residual). The census instrument turned on the R2000/R2004
   specimens for the first time: **164/235 and 157/227 records
   diverged** (vs the near-perfect R2010 1/216, R2013 3/160) — a
   never-measured surface. Seven systematic roots fell:
2. **(1) The pre-2007 TV terminator** — the authored wire counts the
   NUL in the length (BS(len+1) + chars + NUL; her LAYER "0" spans 26
   bits vs our 18); the READER already parsed it right (count bytes →
   strip NULs — its own correctness proved the convention), only the
   WRITER's count-excluded form was wrong; the single fix closed
   159+155 records.
3. **(2) The VISUALSTYLE legacy_properties gate** — the pre-R2007
   wires carry 23 properties (the R2007-only bd2007_45 is absent) and
   the accessor's len==24-only gate fell back to CORE DEFAULTS (every
   VISUALSTYLE −25 bytes, our conv decoding face_opacity 0 where her
   wire carries −0.6); the accessor now synthesizes the never-emitted
   24th slot at ACCESS time (the model keeps its faithful 23 — the
   semantic projections match gold; the first attempt padded the
   reader's model and the smokes caught the bd2007_45 extra_in_silver
   leak).
4. **(3) The PLACEHOLDER wire type** — PER-FILE variance (the R2000
   specimen writes the class-based 501; R2004/R2010 the fixed 80 — the
   ext-7 lesson again): the dispatched raw code is captured on the
   read (`wire_type_code`) and re-emitted; the fixed 80 stays the
   DXF/programmatic fallback (popped in the normalize).
5. **(4) The entity chain handles** — the pre-2004 prev/next links
   follow the H8d first-ref form rule (her CIRCLE (12.2.1E0) =
   own−offset at equal length; the rule's `offset < handle`
   tie-break) — routed through write_first_ref_handle.
6. **(5) The EED retained-block bytes** — the H8h replace-in-place
   overwrote the author's AcadAnnotative block with the synthesized
   marker (codepage 0 where her blocks carry 30): a DWG read's
   retained raw block now keeps BOTH its position AND its bytes; the
   synthesis fires only for apps with no retained block.
7. **(6) The R2013 geomdep text stream** — her records carry
   has_strings: 0 (no stream; the classname TU reads "" at 0 bits)
   where our emission ran +18 bits; the stream's PRESENCE is
   PER-RECORD wire state: the first attempt (a blanket all-empty-
   stream drop in the merge) REGRESSED 22 AC1021 records (their
   authors write has_strings: 1 with empty-only streams — the ext-7
   lesson a third time) and changed the generation identity; REVERTED.
   The fix: the reader captures `wire_no_text_stream` at the
   classname TU (text_remaining_bits() ≤ 0, gated to R2007+) and the
   writer skips the TU so the merge emits no stream — the generator
   (DXF-built, flag false) is untouched.
8. **(7) The TABLESTYLE modern_style resolution** — the writer
   resolved the null cell-style text_style into the "Standard" handle
   (the era censuses' single +1-byte divergence, h=87 on R2010/
   R2013); on a DWG read the retained NamedTableCellStyle is the wire
   truth — verbatim; the resolution rides the DXF/programmatic path.
9. **Measured: the era censuses ALL AT ZERO — R2000 235/235, R2004
   227/227, R2010 216/216, R2013 160/160** (every record
   byte-identical: size + CRC-16). The gates: serde 1602/0,
   gold_roundtrip, issue80 7/0, four family smokes 0/0/0/0, the full
   corpus 280 files 0/0/0/0, the full AC1021 survey re-verified 58/58
   / 0 divergent. **Generation identity UNCHANGED**
   (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes, verified with
   AND without `--features serde` — the reverted drop restored it).
   The echo byte-identity held.

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
persubent-id tail captures, the H8h-ext-15 geomdep stream-presence
captures, and the H8h-ext-12 TABLECONTENT wire captures are INCLUDED
(real state — an edit declines the echo).

**The honest framing, FINAL**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. **The conventional arm is record-identical to the author's
stream on EVERY AC1021 corpus file (58/58, 0 divergent) AND every
record of all four era specimens (R2000 235/235, R2004 227/227,
R2010 216/216, R2013 160/160 — size + CRC-16 identical).** What
remains outside the surveys' reach: the era censuses cover the
Constraints specimens only (one file per era — the full era corpora
are the next surface), the ATMOS 84 her-only orphans (the broken-map
pre-existing issue), and the unattested/dead rows.

## The remaining work (all optional — the target stays reached)

- **The version-parity tiers outside the scope** (the §19.5 audit,
  2026-09-28): R13/R14 is implemented but divergent (89/153/149
  read+write diffs on the three r14 specimens — record-count
  mismatches + era fields); pre-R13 is unsupported in silver and
  gold decodes nothing there either (identification only). Both
  pathed in §19.5; maintainer decisions.
- **The era censuses across the full era corpora**: today's proofs
  cover the Constraints specimens (1 file/era). The instrument (the
  handle-keyed -v9 census) applies to any era file; the r2000/
  2004/2010/2013 corpus directories are the surface — a future
  packet per era, the same autopsy → census → rule loop.
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
  25,375 bytes (UNCHANGED through H8h-ext-15 — the blanket
  all-empty-stream drop briefly changed it to `3969f799…`; reverted
  with the AC1021 survey regression). The generator builds and runs
  identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **58/58 files at 100%, 0 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  Constraints 219/219; ATMOS 340/340; example_2007 540/540.
  **The era censuses (the ext-14/-15 instrument): R2000 235/235,
  R2004 227/227, R2010 216/216, R2013 160/160 — the constraint-group
  records identical across all five dissected eras.**
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
  INCLUDING the crc pair. **The pre-2007 TV wire**: BS(len+1) +
  chars + NUL (the count INCLUDES the terminator; the reader reads
  count bytes and strips NULs). **The TwoStream frame (pre-2007,
  §19.4.C)**: the handle stream is BIT-CONTINUOUS at the RL. The
  BL forms: 00 = 4-byte LE, 01 = 1 byte, 10 = 0 (2 bits); the BD
  forms: 00 = full 66-bit LE double, 01 = 1.0, 10 = 0.0. **The
  SEQEND lesson (H8h-ext-7)**: era-derived "conventions" verified
  on one corpus family are per-author forms — prefer the read
  capture, keep the convention as the DXF fallback only. **The pad
  lesson (H8h-ext-8, extended in ext-12/-13)**: a wire capture that
  runs to the record end must TRIM the author's closing 1s pad (≤7
  bits) — and the writer must re-create the pad explicitly with
  1s. **The era-gate lesson (H8h-ext-9)**: a workaround added for
  one era's wire quirk must be GATED to that era; the normalize
  layer can hide such losses — the byte survey is the only
  witness. **The presence lesson (H8h-ext-10, extended in
  ext-15)**: an author field that is sometimes-present on the wire
  needs the wire PRESENCE captured (Some(NULL) vs None; the
  text-stream's has_strings flag) — and the presence is PER-RECORD
  wire state, never a blanket merge rule (the ext-15 regression:
  the all-empty-stream drop broke 22 AC1021 records whose authors
  DO write empty-only streams). **The recompute lesson
  (H8h-ext-11)**: a model-API semantic applied to a DWG-read value
  recomputes the author's stored double (1 ulp); the DWG read/write
  paths treat the wire f64s as exact. **The ref-code lesson
  (H8h-ext-12)**: gold's declared handle-reference codes are
  per-spec-guess, not per-wire. **The normalize-lesson
  (H8h-ext-12/-15)**: every new wire-capture model field must be
  popped in the normalize_silver compare paths — the family smokes
  catch the leak. **The form-rule lesson (H8h-ext-13)**: the H8d
  ownerhandle form rule holds on the pre-2007 eras too. **The
  content-agnostic lesson (H8h-ext-14)**: when a capture region's
  content cannot be decoded, retain the bits VERBATIM. **The
  reader-correctness proof (H8h-ext-15)**: when the reader parses
  a wire form correctly, its own parse logic reveals the author's
  convention (the TV count-includes-NUL proved by the
  read-count-bytes-strip-NULs path); and the accessor-gate
  pattern: a length-gated accessor that falls back to defaults
  silently re-emits garbage for length variants (the VisualStyle
  23-vs-24) — pad at ACCESS time, keep the model faithful.

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

# 4c. The era censuses (the ext-14/-15 instrument: the handle-keyed
#     -v9 comparison; identity = (size, CRC-16) per handle)
for era in 2000 2004 2010 2013; do
  DWG_NO_ECHO=1 ./target/debug/dwgrewrite \
      "$GOLD_TESTDATA/$era/Constraints.dwg" /tmp/conv_$era.dwg
  "$GOLD_DWGREAD" -v9 "$GOLD_TESTDATA/$era/Constraints.dwg" 2> her.log
  "$GOLD_DWGREAD" -v9 /tmp/conv_$era.dwg 2> our.log
  # census: match records by handle, compare (size, CRC)
done
# current state: R2000 235/235, R2004 227/227,
#                R2010 216/216, R2013 160/160 — all zero divergent

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
dc3737a <feat> H8h-ext-14: the R2010/R2013 constraint-group arms
       closed — the raw text-region capture + the era census
       instrument; the R2010/R2013 constraint records
       byte-identical (969C/36AD)
0f2bbfa <feat> H8h-ext-15: the era full-file record identity closed —
       seven systematic roots (the pre-2007 TV terminator, the
       VISUALSTYLE accessor gate, the PLACEHOLDER wire-type capture,
       the entity chain form rule, the EED retained-block bytes, the
       geomdep stream-presence capture, the TABLESTYLE verbatim
       write); all four era censuses at zero (235/227/216/160)
       [this commit also carried the §19.2 H8h-ext-15 row and the
       post-ext-15 halt refresh of this file]
<docs> the post-H8h-ext-15 maintenance review — the §19 header state
       annotation (the era censuses), the §19.4.B form-rule call-site
       census, the §19.4.D string-stream reference (the pre-2007 TV
       form + the stream-presence lesson), and this file's
       commit-inventory hashes
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-14
halt → the era censuses turned on R2000/R2004 for the first time
(164/235 + 157/227 divergent — a never-measured surface) → the
seven roots fell in census order (the TV terminator closing 159+155
records in one fix; the VisualStyle accessor gate; the PLACEHOLDER
per-file type; the chain-handle form rule; the EED retained-block
bytes; the geomdep stream presence — with the blanket-drop attempt
REGRESSED by the survey and reverted to the per-record capture; the
TABLESTYLE verbatim write closing the last record) → the two
wire-capture field leaks caught by the smokes and popped → the
gates (serde 1602/0, gold_roundtrip, issue80, family smokes,
corpus 0/0/0/0, the full AC1021 survey re-verified 58/58, the
generation identity UNCHANGED after the revert, the echo
byte-identity held) → the docs (§19.2 H8h-ext-15 row; this halt
record). **The maintainer's loop instruction — "repeat process
until target = zero" — remains satisfied in full: the corpus is at
zero on every axis, the conventional arm is record-identical on
every AC1021 corpus file, and all four era censuses stand at ZERO
divergent records.** The remaining residuals: the era censuses'
extension to the full era corpora, the unattested subcurve types,
the MT crc_seed draws, the dead rows, and ATMOS's 84 pre-existing
broken-map orphans.
