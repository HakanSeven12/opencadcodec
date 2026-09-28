# Zero-context prompt — TARGET ZERO held; the conventional arm at 9 residual records (H8h-ext-9: the ATMOS closure — the LAYER/STYLE authored-slots capture + the dictionary-key era gate; the residual is example_2007's marginals only)

> Campaign state 2026-09-28 (the halt after the H8h-ext-9 landing; the
> session continued the loop from the H8h-ext-8 halt: review → land →
> verify → commit → push. **THE CORPUS STAYS AT ZERO ON EVERY AXIS: 280
> files, read-fidelity 0, write-fidelity 0, read key-gap 0,
> write-target 0** — verified after the landing). The ACS/SH campaign
> stays COMPLETE at 0/0. The §19 structure campaign's READ axis stays
> ZERO corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d–H8h-ext-9 rows carry this arc's records), then this
> file top to bottom.

## The arc (2026-09-28, the continuation session — H8h-ext-9)

1. **The ATMOS autopsy**: the Russian drawing's three divergent
   records (the ext-8 halt's residual ATMOS class) fell to TWO
   reader-side root causes.
2. **h=2/h=3 (LAYER_CONTROL/STYLE_CONTROL)**: her entry vectors carry
   NULL DELETED-SLOT TAILS — the layer control [10, A5, 0] for two
   live layers; the style control [11, 6B, 0×5] for two live styles
   (gold's -v9 walk prints the full vectors). The H8h-extension
   authored-slots capture existed for BLOCK/LTYPE/VIEW/DIMSTYLE but
   NOT LAYER/STYLE, whose writers derived num_entries from the live
   tables. THE FIX: the builder captures the LAYER/STYLE entry slots
   (num_entries BL + the handle vector, keyed by the control handle —
   the VIEW_CONTROL pattern), and the two writers apply the
   same-universe gate (`authored_control_entries`: the captured
   non-null set must equal the live table's handles; edited tables
   and DXF-built documents fall back to the model).
3. **h=77 (DICTIONARY)**: the image dictionary's keys are CYRILLIC —
   "_Схема-1" … "_Схема-4" (her text stream: plain TUs, count=8,
   UTF-16LE, 552 bits) — and `clean_dict_key()` (the R13/R14
   mis-sized-key workaround: cut at the first non-printable/high
   byte, added for the ACAD_FILTER\u{80}0 xclip/gradient lookups)
   was applied to ALL eras, truncating the legitimate R2007 Unicode
   keys to "_" (the silver model carried "_"; gold's JSON carried
   the full text). THE FIX: the cut is gated to `r13_14_only()` (the
   mis-sized era); R2000+ keys are exact-length and pass verbatim —
   the same lesson as the H8h-ext-7 SEQEND flags: an era-verified
   convention is not a law. (The read-fidelity gates never caught
   it: normalize_gold/normalize_silver drop DICTIONARY texts/items
   from the compare — only the byte survey sees the loss.)
4. **Measured: ATMOS 340/340 (0 divergent; the 84 her-only
   broken-map orphans remain, pre-existing, documented); the
   residual 12 → 9; 57/58 files at 100%** (only example_2007 below).
5. **The gates**: serde 1602/0, gold_roundtrip ok, issue80 7/0, four
   family smokes 0/0/0/0, the full corpus 280 files 0/0/0/0.
   Generation identity UNCHANGED (`84374e73ddcf1d6143877c4100b81e48`,
   25,375 bytes, verified with AND without `--features serde` — the
   generator's tables carry no captures and its dictionary keys are
   ASCII). The echo byte-identity held (circle cmp clean).

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
subcurve fields, the persubent tails, the SEQEND flag captures, and
the H8h-ext-8 constraint-group node captures are INCLUDED (real
state — an edit declines the echo).

**The honest framing, extended**: for an unedited same-version
roundtrip the DWG writer is a byte-copy gated on a full-content
hash. The conventional arm is record-identical to the author's
stream on 57 of the 58 AC1021 corpus files, with **9 named
residual records** in one class (below).

## The residual 9 (each a named, byte-level-scoped packet)

- **example_2007 (9)**: 1F +10 (01v21), 176 1726v1727, 1A9 +32
  (a9va1), 37D 2269v2270, 392 +20 (42v52 — another assoc-dep-class
  ref), 393 +30 (10v0c), 396 91v85, 430 +37 (27v28), +1 more.
  (ATMOS's 84 her-only = the broken-map orphans, pre-existing,
  documented — not counted in the 9.)

## The remaining work (all optional — the target stays reached)

- **example_2007's 9 marginals** — the last residual class; each an
  autopsy → census → rule → gates → re-survey packet in the H8h
  tradition.
- **The pre-2007 constraint-group conventional arms** (the
  r14/2000/2004/2010/2013 `Constraints.dwg` specimens): the
  H8h-ext-8 capture is AC1021-gated; those eras keep their current
  behavior (echo-covered on the corpus axes, ungated by the AC1021
  survey). A later packet could extend the capture (the inline-TV
  forms and the R2010+ framing need their own dissection).
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
  25,375 bytes (UNCHANGED through H8h-ext-9). The generator builds
  and runs identically WITH or WITHOUT `--features serde`.
- The record-identity state (the 58-file AC1021 survey,
  `record_identity_survey.py`): **57 files at 100%, 9 divergent
  records, 84 her-only orphans (ATMOS's broken map)**. circle
  211/211; ExtrudeC 206/206; Box 207/207; Leader 245/245;
  Chamfer 207/207; Fillet 207/207; Loft 207/207; PolyLine3D 218/218;
  Constraints 219/219; **ATMOS 340/340**.
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
  a gold spec block (the H8h-ext-6 re-parse follows it).
- The hermetic suites: serde green (the 0xF_ test + issue80
  green), gold_roundtrip green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative (they INCLUDE the 2-byte MS head); the survey's
  merge-trace positions are SPAN-relative. The record-head MS is
  15 data bits per 16-bit LE word (§19.4.A). **The merged-stream
  frame math**: the RL (bitsize) = main content + text + 1 (the
  no-text flag); `main_remaining_bits()` is relative to the
  CONTENT end; the bit after the content is ALWAYS the flag —
  never a record field. The ASSOC persubent records end [BLs][B];
  the non-assoc records end flush. The BL forms: 00 = 4-byte LE,
  01 = 1 byte, 10 = 0 (2 bits); the BD forms: 00 = full 66-bit LE
  double, 01 = 1.0, 10 = 0.0. **The SEQEND lesson (H8h-ext-7)**:
  era-derived "conventions" verified on one corpus family are
  per-author forms — prefer the read capture, keep the convention
  as the DXF fallback only. **The pad lesson (H8h-ext-8)**: a
  wire capture that runs to the record end must TRIM the author's
  closing 1s pad (≤7 bits) — the writer re-creates it at close, so
  an untrimmed capture double-pads (+1 byte, the measured
  666-vs-667). **The era-gate lesson (H8h-ext-9)**: a workaround
  added for one era's wire quirk (the R13/R14 mis-sized dictionary
  keys) must be GATED to that era — applied universally it mangles
  legitimate later-era content (the R2007 Cyrillic dictionary
  keys); the normalize layer can hide such losses (DICTIONARY
  texts are dropped from the semantic compare) — the byte survey
  is the only witness.

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
# current state: 57 files at 100%, 9 divergent records total
# circle 211/211; PolyLine3D 218/218; Chamfer 207/207;
# Constraints 219/219; ATMOS 340/340

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
<feat> H8h-ext-9: the ATMOS closure — the LAYER/STYLE authored-slots
       capture + the dictionary-key cut gated to r13_14_only (the
       Cyrillic keys pass verbatim on R2000+); ATMOS 340/340;
       12 -> 9
<docs> the post-H8h-ext-9 halt refresh (this file)
```

**PUSH STATE (2026-09-28)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the continuation from the H8h-ext-8
halt → the ATMOS autopsy (the three records: h=2/h=3 the table
controls, h=77 the image dictionary) → the null deleted-slot tails
discovered in her LAYER/STYLE entry vectors (the authored-slots
capture missing for those two tables) → the Cyrillic dictionary
keys discovered truncated to "_" (the `clean_dict_key` R13/R14
workaround applied to all eras; the normalize layer had hidden the
loss from the read-fidelity gates — the byte survey was the only
witness) → the fixes (the VIEW_CONTROL-pattern capture + the
same-universe gate for both writers; the cut gated to
`r13_14_only()`) → the gates (serde 1602/0, gold_roundtrip,
issue80, family smokes, corpus 0/0/0/0, generation identity
unchanged and feature-independent, echo byte-identity held) → the
full survey (ATMOS 340/340; the residual 12 → 9; 57/58 at 100%) →
the docs (§19.2 H8h-ext-9 row; this halt record). **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied: the corpus is at zero on every axis;
the conventional arm is record-identical on 57 of 58 AC1021 files,
and the residual 9 records are one named class (example_2007's
marginals), each a byte-level-scoped packet.**
