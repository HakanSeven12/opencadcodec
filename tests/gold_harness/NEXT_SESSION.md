# Zero-context prompt — TARGET ZERO held; the conventional arm at 32 residual records (H8h-ext-3: the assoc ref codes, the Revolved trailing bit, the *Model_Space block forms)

> Campaign state 2026-09-27 (the halt after the H8h-ext-3 landing; the
> session continued the loop from the H8h-ext-2 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0, write-target
> 0** — verified after the landing). The ACS/SH campaign stays COMPLETE
> at 0/0. The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3
> + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3 (§19.2's
> H8d–H8h-ext-3 rows carry this arc's records), then this file top to
> bottom.

## The arc (2026-09-27, the continuation session — H8h-ext-3)

1. **The assoc/pathparam genus autopsy** — the gold -v9 walks named
   the divergent refs: the ACTIONBODY's `pab.deps` (her `(3,2,2E5)`
   code 3 vs our 4), `sab.assocdep` (her code 4 vs our 5), the
   PATHPARAM's `params[0]` (her `(3,2,2E6)` code 3 vs our 4). The
   corpus census decided genus-safety: deps {3: 27}, params {3: 41},
   assocdep {4: 23} — no other code anywhere. Landed: HardOwnership
   for deps and compound parameters, SoftPointer for assocdep (the
   null case keeps the is_valid guard).
2. **The Revolved trailing bit** — RevolveM's 2E3 bit-diff showed a
   clean 1-bit shift from record bit 250: her main stream carries one
   trailing '0' after class_version. The only corpus specimen of
   ASSOCREVOLVEDSURFACEACTIONBODY (no other Revolve file has the
   class). Landed as a Revolved-kind trailing B(0).
3. **The *Model_Space BLOCK_HEADER (the 1F class)** — two
   independent drops found by the byte autopsies: (a) her record
   carries an xdicobjhandle (3.2.1CE) — our reader captures every
   xdic into `document.xdic_by_handle` (the normalizer's
   side-channel, already in the semantic inventory) but the block
   writer hardcoded `&None`; (b) her owned list references a
   non-graphical record (entities[1] = 0x2DB, an ACDBASSOC* object
   gold prints as UNKNOWN_OBJ — our reader models it as an Unknown
   OBJECT with raw passthrough) which the caller's live-filter
   (entity_index only) dropped — num_owned 1 vs her 2, one ref
   missing. Landed: the block writer passes the side-map xdic; the
   live-filter keeps indexed entities OR writable objects.
4. **Measured: 49 → 32 divergent records; 42/58 files at 100%.**
   ExtrudeCSurf 215/216, LoftCSurf 219/221, RevolveM 216/217,
   ExtrudeM 215/216 — each holding ONLY the undocumented-subcurve
   stubs now. circle 211/211, Leader 245/245 held.
5. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0.
   Generation identity UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
   25,375 bytes, verified with AND without `--features serde` — the
   generator's programmatic documents carry none of the touched
   fields).

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
EXCLUDED from the fingerprint; `xdic_by_handle` is INCLUDED (it was
always part of the inventory).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 42 of the 58 AC1021 corpus files, with **32 named
residual records** in five classes (below).

## The residual 32 (each a named, byte-level-scoped packet)

- **The undocumented CALL_SUBCURVE bodies (7)** — THE IN-FLIGHT
  PACKET, dissection started (2026-09-27, the notes for the next
  session): ExtrudeCSurf/ExtrudeM/RevolveM `2E6` (47/31/47 vs our
  20), LoftCSurf/LoftM `2E7`+`2EA` (55/63, 31/39 vs 20) — the
  ACDBASSOCEDGEACTIONPARAM subcurve region after action_type. The
  concrete findings: (a) the frame math holds per record — span =
  bitsize + hdlsize (ExtrudeCSurf 2E6: 331 + 45 = 376 = 47 bytes);
  (b) our modeled reader parses the record through action_type
  (main-stream bit 96 on ExtrudeCSurf: type 18 + bitsize 32 +
  handle H 24 + eed 2 + reactors 2 + is_xdic 1 + is_r2013 2 +
  dep_cv 2 + cv 2 + has_action 1 + action_type 10); (c) her
  subcurve region = main bits 96..330 (234 bits): 56 zero bits,
  then a BD candidate `2.000000000000001` (2.0 + 1 ulp — the
  author's computed extrusion distance), then ~120 further bits
  (the `054B4C0C16A2...` bytes in gold's unknown_bits print);
  (d) gold's own spec macro `CALL_SUBCURVE` is an EMPTY TODO stub
  — the wire form must be reverse-engineered from the specimens
  (the ODA PDF's AcDbAssocSubcurveGeometry + the per-action_type
  dissection: 11=ARC, 17=ELLIPSE, 19=LINE, 23=LINESEG3D, 42=NURB3D,
  27=CURVE3D). The six specimens' hex is in the -v9 walks
  (`DWG_NO_ECHO=1` + `$GOLD_DWGREAD -v9 <fixture>`).
- **ATMOS (5 + 84 her-only)**: the controls h=2 16v15, h=3 20v15,
  h=77 96v40, h=352 25642v35 (a 25KB ACIS mass we emit as 35
  bytes), h=541 17v18; the 84 her-only = the broken-map orphans
  (pre-existing, documented).
- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
- **Chamfer/Fillet (4)**: the UNKNOWN bodies (1509/1463 vs 28) +
  the 22v26/30 records.
- **The Loft 29-vs-27 class (5)**: Loft3/C/H/R/_ (2DD/2DE/2E4/2EF).
- **The singles**: Constraints `3E3` (666 vs 870); PolyLine3D `1C2`
  (16 vs 17 — OURS longer).

## The remaining work (all optional — the target stays reached)

- **The subcurve reverse-engineering packet** — the biggest named
  class (7 records): decode her unknown_bits per action_type (11 =
  ARC, 17 = ELLIPSE, 19 = LINE, ...) and complete the
  ASSOCEDGEACTIONPARAM reader+writer. Needs the ODA spec PDF study;
  the specimens' hex is in the -v9 walks.
- **The other residual classes above** — each an autopsy → census →
  rule → gates → re-survey packet in the H8h tradition.
- **The MT-variant pinning (§F2.G)**: the crc_seed draws — NOT
  attempted (the echo path never runs it for unedited roundtrips).
- **The dead/no-path rows**: `LoftD`; the SH revolve option shorts;
  **BREP stays deferred** (external authentic ACSH_BREP_CLASS
  specimen required).

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique
  stems); report.json totals are authoritative: all four axes 0.
- The generation identity is `84374e73ddcf1d6143877c4100b81e48`,
  25,375 bytes (UNCHANGED through H8h-ext-3). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **42 files at 100%, 32 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245.
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
  libredwg `bits.c`. NOTE: gold's `CALL_SUBCURVE` spec macro is an
  EMPTY TODO stub — the ASSOCEDGEACTIONPARAM subcurve wire is
  undocumented there.
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). The `bitsize` RL =
  main+text bits; the record span = bitsize + hdlsize (verified on
  the 1F autopsy: 330 + 150 = 480 = 60 bytes).

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
# current state: 42 files at 100%, 32 divergent records total
# circle 211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde
```

## Commit inventory (this halt — all PUSHED)

```
53c9b11 <docs> the post-H8h-ext-2 halt refresh (the prior halt head)
c3f28c3 <feat> H8h-ext-3: the assoc ref codes (deps/params -> 3,
       assocdep -> 4), the Revolved trailing bit, the *Model_Space
       block xdic + owned-object refs (49 -> 32; 42/58 at 100%)
7585711 <docs> the post-H8h-ext-3 halt refresh
<docs> the subcurve-packet handover notes (the in-flight dissection
       state: the frame math, the bit-96 start, the BD candidate)
```

**PUSH STATE (2026-09-27)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-2
halt → the assoc-genus autopsies (the -v9 walks naming the ref
fields; the corpus census deciding genus-safety: deps {3:27},
params {3:41}, assocdep {4:23}) → the RevolveM bit-diff (the clean
1-bit shift; the trailing B(0); the single-specimen caveat) → the
1F autopsies (the bitsize+hdlsize frame identity 330+150=480; the
xdic side-map discovery — the reader always captured it, the
writer hardcoded None; the owned-list live-filter hole — her
blocks own non-graphical records, our Unknown objects ARE
serialized, the filter needed is_writable_object) → the landing →
the gates (serde 1602/0, gold_roundtrip, issue80, family smokes,
corpus 0/0/0/0, generation identity unchanged and
feature-independent) → the full survey (49 → 32; 42/58 at 100%) →
the docs (§19.2 H8h-ext-3 row; this halt record). **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied: the corpus is at zero on every axis;
the conventional arm is record-identical on 42 of 58 AC1021 files,
and the residual 32 records are named, byte-level-scoped packets —
the largest being the undocumented CALL_SUBCURVE bodies (gold's
own spec macro is an empty TODO stub).**
