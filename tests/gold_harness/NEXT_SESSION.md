# Zero-context prompt — TARGET ZERO held; the conventional arm at 3 residual records (H8h-ext-11: the wire-exactness landings — the RAY/XLine read-side normalize removed, the ordinate measurement recompute removed on BOTH the read and write sides)

> Campaign state 2026-09-28 (the halt after the H8h-ext-11 landing; the
> session continued the loop from the H8h-ext-10 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0,
> write-target 0** — verified after the landing). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d–H8h-ext-11 rows carry this arc's records), then this
> file top to bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-11)

1. **The two double records fell to RECOMPUTE-ON-ROUNDTRIP** — the
   model APIs' semantic normalizations applied to the author's
   already-computed wire doubles, shifting them exactly 1 ulp.
2. **h=1A9 (RAY)**: her vector = (0.8208410212999244,
   0.5711567365892636, 0); our model held (...42, ...35, 0) —
   `Ray::new` (and `XLine::new`) NORMALIZE the direction, and
   re-normalizing the author's unit vector recomputes it 1 ulp off.
   THE FIX: the builder constructs the entity WITHOUT the constructor
   normalize (the struct literal; the wire direction is
   authoritative); the normalize stays the programmatic-API semantic.
3. **h=430 (DIMENSION_ORDINATE)**: her act_measurement =
   4630.519359082827 (one ulp below her feature_location.x); BOTH
   sides recomputed it away — the builder's ordinate arm called
   `refresh_measurement()` after `map_dimension_common` had already
   set the wire value, and the writer's `write_dimension_ordinate`
   recomputed `d.measurement()` into a cloned base before emitting.
   THE FIX: both recomputes removed on the DWG path — the model's
   stored value is the semantic source of truth on every path
   (programmatic construction refreshes at `DimensionOrdinate::new`;
   edits refresh via the transform path).
4. **Measured: example_2007 537/540; 57/58 files at 100%.** The
   gates: serde 1602/0, gold_roundtrip, issue80 7/0, four family
   smokes 0/0/0/0, corpus 280 files 0/0/0/0. Generation identity
   UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`, 25,375 bytes,
   verified with AND without `--features serde` — the generator's
   rays take the constructor normalize and its ordinate the
   constructor refresh, so the emitted bytes are unchanged). The
   echo byte-identity held.

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
H8h-ext-8 constraint-group node captures, and the H8h-ext-10
persubent-id tail captures are INCLUDED (real state — an edit
declines the echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 57 of the 58 AC1021 corpus files, with **3 named
residual records** in one class (below).

## The residual 3 (each a named, byte-level-scoped packet)

All in example_2007 (537/540):

- **h=392 (ACDBASSOCALIGNEDDIMACTIONBODY, unhandled class)**: ONE
  bit differs (record byte +20: her 0x42 our 0x52; in the
  unknown_bits TF coords: her A1 our A9 at TF byte 8). We re-emit
  the unknown region NOT verbatim — the class is parsed as an
  AnnotationActionBody partial parse and re-serialized. THE PATH:
  find the ASSOCALIGNEDDIMACTIONBODY reader/writer arms
  (src/io/dwg/dwg_stream_readers/object_reader/associative.rs,
  read_annotation_action; the writer's AnnotationActionBody arm),
  decode the 51 unknown bits around the diff, and either fix the
  one field or capture the unknown region verbatim (the ProxyObject
  path already does this for unhandled classes — check why this
  class takes the parsed path instead).
- **h=393 (ACDBASSOCOSNAPPOINTREFACTIONPARAM, unstable class)**:
  ONE byte differs at +30 (her 0x10 our 0x0c) — in the handle
  stream tail (the record's last data byte; the handle stream @24.2
  ..@30.0, 46 bits). The parsed fields print identically
  (num_params 1, params[0] 394, status 0, osnap_mode 0xa0, param
  2.0646e-255 — gold's own garbage prints). THE PATH: dump both
  handle streams (the parsed refs: ownerhandle (8.0.0) abs:391,
  params[0] (3.2.394)), find the extra/shorter handle read, and
  check the writer's handle order for this class. NOTE: the parsed
  handles account for 32 of her 46 handle bits — 14 unparsed bits
  (≈ one short handle + the 1s pad) differ.
- **h=BF2 (UNKNOWN_OBJ, 14.5KB)**: ours 203 bytes SHORTER and
  near-fully differing (14,167 of 14,322 bytes). The biggest fish:
  a large unhandled-class object whose capture/re-emission goes
  wrong wholesale. THE PATH: identify the class (object number 465
  — check the classes section for its DXF name), then compare our
  ProxyObject payload capture vs her record (the payload bit_count
  vs her unknown region size — a 203-byte shortfall suggests the
  capture stops early or the re-emission drops a trailing block).
  NOTE: this record's divergence may share a root cause with the
  h=392 class (both UNKNOWN_OBJ-decoded).

## The remaining work (all optional — the target stays reached)

- **example_2007's 3 residuals** — the packets above; each an
  autopsy → census → rule → gates → re-survey in the H8h tradition.
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
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does); a DWG→DXF→DWG
  roundtrip of a tailed record would drop them — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-11). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **57 files at 100%, 3 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  Constraints 219/219; ATMOS 340/340; **example_2007 537/540**.
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
  subcurve wire, the non-assoc PersSubentManager, or the
  constraint-group node classes (the authority closure, commits
  `cc1a6c8` + the H8h-ext-8 grep); the ASSOC persubent variant HAS
  a gold spec block (the H8h-ext-6 re-parse follows it); the
  3DSOLID history_id AVAIL_BITS rule IS in gold's dwg.spec
  COMMON_3DSOLID (the H8h-ext-10 re-read follows it).
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). **The merged-stream
  frame math**: the RL (bitsize) = main content + text + 1 (the
  no-text flag); `main_remaining_bits()` is relative to the
  CONTENT end; the bit after the content is ALWAYS the flag —
  never a record field. The record frame: [MS][BS type][RL
  bitsize][H own handle][data …][handle stream][1s pad][crc16 LE];
  the MS value = the data size INCLUDING the crc pair; gold's
  Hdlsize = the handle-stream bit count including the closing 1s
  pad. The ASSOC persubent records end [BLs][B]; the non-assoc
  records end flush. The BL forms: 00 = 4-byte LE, 01 = 1 byte,
  10 = 0 (2 bits); the BD forms: 00 = full 66-bit LE double, 01 =
  1.0, 10 = 0.0. **The SEQEND lesson (H8h-ext-7)**: era-derived
  "conventions" verified on one corpus family are per-author
  forms — prefer the read capture, keep the convention as the DXF
  fallback only. **The pad lesson (H8h-ext-8)**: a wire capture
  that runs to the record end must TRIM the author's closing 1s
  pad (≤7 bits) — the writer re-creates it at close, so an
  untrimmed capture double-pads (+1 byte, the measured
  666-vs-667). **The era-gate lesson (H8h-ext-9)**: a workaround
  added for one era's wire quirk must be GATED to that era; the
  normalize layer can hide such losses (DICTIONARY texts are
  dropped from the semantic compare) — the byte survey is the only
  witness. **The presence lesson (H8h-ext-10)**: an author field
  that is sometimes-present on the wire needs the wire PRESENCE
  captured, not just the value — Option<Handle> with Some(NULL)
  vs None — or the writer cannot mirror the author's form; and a
  writer-side DERIVATION is a DXF fallback only, never a DWG-read
  rule. **The recompute lesson (H8h-ext-11)**: a model-API
  semantic (the Ray::new normalize, the ordinate
  refresh_measurement) applied to a DWG-read value recomputes the
  author's stored double — a 1-ulp shift invisible to the
  %g-printed traces and hidden from the semantic compare by the
  normalize layer's double tolerance; the DWG read/write paths
  must treat the wire f64s as exact, and the semantic recompute
  stays on the programmatic/edit paths only. The double-diff
  technique: scan the record for raw BDs near the printed value
  (the %g print gives ~7 digits), decode the full IEEE hex on
  both sides — a 1-ulp diff shows as adjacent shortest-reprs
  (...827 vs ...828).

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
# current state: 57 files at 100%, 3 divergent records total
# circle 211/211; PolyLine3D 218/218; Chamfer 207/207;
# Constraints 219/219; ATMOS 340/340; example_2007 537/540

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
<feat> H8h-ext-11: the wire-exactness landings — the RAY/XLine
       read-side normalize removed (the builder constructs the
       struct literal), the ordinate measurement recompute removed
       on BOTH the read (refresh_measurement after the map) and
       write (d.measurement() into the cloned base) sides; 5 -> 3
<docs> the post-H8h-ext-11 halt refresh (this file)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-10
halt → the h=1A9/h=430 double autopsy (the %g-identical prints vs
the byte diffs; the raw-BD scan technique: scan the record for
BDs near the printed value, decode the full IEEE hex — her
vector.x `...6a59b663...` vs ours `...6859b663...`, exactly 1 ulp)
→ the RAY root cause (Ray::new/XLine::new normalize the direction;
the re-normalization recomputes the author's unit vector 1 ulp off;
the builder now constructs the struct literal) → the ordinate root
cause (BOTH sides recomputed: the builder's refresh_measurement
after map_dimension_common, AND the writer's d.measurement() into
the cloned base; both removed — the model's stored value is the
truth on every path) → the gates (serde 1602/0, gold_roundtrip,
issue80, family smokes, corpus 0/0/0/0, generation identity
unchanged and feature-independent, echo byte-identity held) → the
full survey (example_2007 537/540; the residual 5 → 3; 57/58 at
100%) → the docs (§19.2 H8h-ext-11 row; this halt record). **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied: the corpus is at zero on every axis;
the conventional arm is record-identical on 57 of 58 AC1021 files,
and the residual 3 records are one named class (example_2007's
ACDBASSOC partial-parse single-bits + the big UNKNOWN_OBJ), each a
byte-level-scoped packet with a recorded next-session path.**
