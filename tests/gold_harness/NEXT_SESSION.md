# Zero-context prompt — the post-H8b halt: the AC21 mirror machinery landed at 3,576 (the encoder-ratio wall named)

> Campaign state 2026-09-26 (the halt after the H8b landing; the session
> ran the review → update → commit → push → continue loop from the
> 3,576 post-H8a halt). **The ACS/SH campaign stays COMPLETE at 0/0:
> corpus 280 files, read 0, write 0, read key-gap 0, write-target
> key-gap 3,576 (UNCHANGED — H8b landed as gate+machinery; every
> corpus AC1021 file declines on one measured wall).** The §19
> structure campaign's READ axis stays ZERO corpus-wide. The H8b
> write mirror is IN: the plan gate, the her-map-order emission, the
> mirrored finalize (a pure code-motion tail shared with the
> conventional path), the decline trace, 7 new hermetic tests — and
> the conventional path is stash-verified BYTE-IDENTICAL (196/196
> rewrites unchanged vs the pre-H8b baseline). The next packet is
> H8c: the AC21 LZ77 encoder ratio parity. Read
> `tests/gold_harness/AGENTS.md` first, then §F2.1–F2.3 + §18.5–18.7
> in `IMPLEMENTATION.md` (§18.6 carries the full decode record), then
> §19.1–19.3 (§19.2's H8b row carries this landing's record), then
> this file top to bottom.

## The oracle record FIX (this session's review finding — supersedes the prior halt's md5 string)

The prior halt recorded `programs/dwgread mtime Sep 15 20:57, md5
e01356f9e69839a61a5f4aa674ce53e5c` — **that md5 matches NO file in
the tree; it was a mis-record**. The true fingerprints (verified this
session at 34f02f54 with the tracked drift still ONLY `src/config.h.in`
— the autoheader requote, unchanged):

- `programs/dwgread` = the 6,316-byte **libtool wrapper script**
  (md5 `8dad57211b78f42e7594ba6c211cc0e5`, mtime 2026-09-15
  20:56:22) — `GOLD_DWGREAD` invokes it and it execs the real ELF;
- `programs/.libs/dwgread` = the oracle ELF (md5
  `d852da1db0894b866e86042d4b26b91d`, 230,232 bytes, mtime
  2026-09-15 20:56:22 — the newer fs-object timestamp class
  2026-09-15 20:56:22.x is fs-internal).
- **Continuity is behavioral, and it held**: the oracle reproduced
  EVERY recorded AC21 page-space fact bit-exact on example_2007
  (pages 22 entries, ids 3..22+25/26, slots 1024/160/307,936/800/
  1440/…/4864, "Invalid num_pages 0, skip" on the 13th descriptor,
  file 456,000). The anchors re-verified at their lines: rs_form
  @:692, read_data_page RS block :700-852, the dispatch @:856,
  read_pages_map @:1099 (the (size,id) walk + running sum), out_json
  R2007_Header record, dwg2.spec:3984, dwg.spec:5446,
  r2004_file_header.spec:41-53, decode.c:1432/1683, codepages.h.
  `decode_rs` @:553 pinned line-by-line: byte-interleave at stride
  block_count within bc×k bytes, the trailing bytes IGNORED (RS
  correction itself is commented out in gold) — her-slot pads are
  inert. `read_system_page` @:590 pinned: **correction = the repeat
  count** (pesize = align8(size_comp)×repeat; block_count =
  ceil(pesize/239) — RS k=239, page_size = align8(bc×255)); her
  sections-map slot 4864 holds the encoded 4080 + 784 inert pad.
  The gold tree is FROZEN at `34f02f54` + the config.h.in requote
  (recorded state); DO NOT touch anything there.

## What H8b landed (the machinery, verified)

- `ac21_mirror_plan` (`DwgFileHeaderWriterAC21::mirror_plan`,
  file_header_ac21.rs, §19 H8b region): same-origin + shape + sys
  coherence gate → per-her-section pressure-vs-fit walk → the
  her-map-order payload preparation (every on-disk slot as an
  exact-size payload), with `AC21_MIRROR_DEBUG` tracing every
  decline reason per file. Decline classes implemented: system-id
  incoherence, gap/terminator or duplicate map entries, her-map
  terminator pairs (uncomp != 16×entries), duplicate section names,
  non-ascending page offsets, un-owned/missing map ids, our-content
  `does not reach`/`exceeds` her plan, per-page encode fit
  (rs_form(our comp) ≤ her slot — the contract's exact rule),
  real-content unused-buffer rule (the FileDepList STUB alone is
  droppable), system-page encode extent over her slot.
- The emission (`write_mirrored_pages`): her map order verbatim, the
  running sum reproduces her tiling; our honest section records
  (her offset/frame/id + OUR uncomp/comp/checksum/crc); her ids
  everywhere; `next_page_id` = her maxid+1 so the metadata's
  pages_amount/maxid land hers naturally.
- The finalize (`write_file_mirrored`): the metadata assembly over
  the mirror state + the SHARED two-pass tail extracted verbatim
  from the historical `write_file` (`write_two_pass_header`; pure
  code motion — the conventional `write_file` byte-identity is
  stash-proven).
- `write_ac21_impl` restructured: all section buffers built up
  front (hoisted pure builders), the gate, the mirror arm, then the
  conventional sequential arm unchanged. The XrefManifest raw
  (§19 H7g retention, ungated in the R2004+ read arm) is carried
  into the mirror buffers — an unregistered section (XrefManifest
  under AC21) falls back to HER table encoding for the honest
  declaration (our registry lacks it; gold never prints it; the
  section re-emits her raw bytes verbatim).
- Hermetic: 7 new tests in file_header_ac21.rs (the author-pinned
  rs_form table incl. the corrected `rs_form(302255) = 307,296`,
  stored-raw/pad, the stored-collision RS rule, the overflow
  decline, the compressed path, the map-bytes/prefix walks) — 42/42
  module green; full `cargo test --features serde` green;
  `gold_roundtrip` green; generation identity
  `40ab5d356cf05a71333f208e1651daf` 25,344 bytes RE-VERIFIED TWICE
  (the refactor is writer-path pure motion for programmatic docs).

## The engagement census (the load-bearing measurement)

All 58 corpus AC1021 files gate; **0 engage today**:
- **57 × objects-overflow**: every file declines on an
  `AcDb:AcDbObjects` window — our compressed chunk exceeds her
  page's on-disk size. Her slots are the EXACT rs_form of her
  compressed streams (zero slack): the fit requires our comp ≤ her
  comp (+bc-granularity), and OUR AC21 LZ77 encoder's ratio runs
  1.16–1.87× hers on those windows (measured: Box_2007 window 2
  ours 22,836 vs her ~12,2xx; example_2007 window 2 ours 16,725 vs
  her ~14.4xx; circle.dwg ours 17,993 vs her ~9.4xx; the
  constrained literal-run + greedy match choices are the suspect).
  EVERY non-objects section passes its fit on every file
  (AppInfoHistory/Preview/Summary/Handles/Classes/Template/… all
  fit — the whole gate holds except the objects windows).
- **1 × gap entry**: ATMOS-DC22S.dwg — her pages map carries a REAL
  negative-id gap entry (id −29, size 768) — the new legit decline
  class beyond the prior halt's speculation.
- The gold-tree minis and the sh-2007 fixtures behave IDENTICALLY
  (the author pack is uniform) — the wall is our encoder, not the
  corpus stratigraphy.

## H8c — the next packet: the AC21 encoder ratio parity

The wall is the LZ77 ENCODER (compressor_ac21.rs, the spec §5.10
implementation — 1,731 lines). The ground truth for the campaign:

- **The author's own compressed streams are ON DISK in every corpus
  file** (extractable per page: her page + the section-table comp
  sizes; her sections' DECOMPRESSED content is what our reader
  already derives 0/0). Token-stream differential: decode her comp
  with our existing `decompressor_ac21` instrumented to dump the
  token sequence (literal runs with their reorder pattern class,
  match opcodes/lengths/offsets), dump OURS over the same
  decompressed bytes, and align — the divergence classes name the
  encoder fixes (suspects: the 8-byte literal-run minimum forcing
  premature matches, the greedy longest-match vs lazy one-byte
  lookahead, the opcode class selection for 16-bit offsets, the
  min-match threshold per class).
- Acceptance = the mirror engage census (every engaged AC1021
  file closes its FILEHEADER AC1021 + THUMBNAILIMAGE + R2007
  pages-map family rows automatically — the machinery is IN and
  stash-safe; the H8b row in §19.2 carries the predicted field
  list).
- The workspace probe pattern (`AC21_MIRROR_DEBUG=1 python3
  tests/gold_harness/run_roundtrip.py …` per file) reports the
  exact over-slot margins per page as the landing proceeds.
- ALTERNATIVE unlock (record-only for now): matching the objects
  stream byte-identically (the AC18 layer-4 doctrine) also fits her
  slots, but that is the larger campaign — the encoder ratio work
  is the cheaper first move and is required for mismatched-content
  rewrites anyway.

## The remaining queue (3,576 — unchanged by this halt)

- **R2007_Header 925**: the H8b mirror column is now LIVE (engages
  when H8c lands); the content-coupled family (sections-map CRCs,
  six CRCs, file_size/header2, the 3-per-file MT-derive draws) +
  the pages-map crc_compressed rows stay open until byte-identity
  of the streams.
- **FILEHEADER 103** (= AC1021 82 + R2000 7 + fallback 14) and
  **THUMBNAILIMAGE 50**: close on the AC1021 side through H8c;
  the R2000 seeker pair needs flat-layout parity.
- **FileDepList 1,055 + SecondHeader 386 + AuxHeader 94**: the
  PARALLEL SESSION's rows (untracked probes `h7_probe1.sh`/
  `h7_rows.sh` in the repo root — not ours to commit).
- **R2004_Header 963 = 4 × 225 + 9 × 7**: accepted AC18 residue.

**Dead / no-path rows (do not re-litigate)**: `LoftD`; the SH revolve
option shorts; **BREP stays deferred** (external authentic
ACSH_BREP_CLASS specimen required).

## The standing facts

- The four raw-retained SH tails decode to typed views with the
  captured bits as the write authority (Phase B). Hermetic suite
  green after H8b (42 module tests incl. the 7 new mirror tests).
- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative; the STASH directory
  `/tmp/baseline_rts` (the 196 pre-H8b `_rt.dwg` md5s) was compared
  196/196 byte-identical after the landing — the conventional path
  did not move a byte.
- The generation identity is UNTOUCHED (`40ab5d356cf05a71333f208e16
  51daf`, 25,344 bytes — verified twice after H8b; the write_file
  tail extraction + buffer hoisting are pure motion).
- The gold tree sits at `34f02f54` with ONE tracked generated-file
  drift (`src/config.h.in`, autoheader requote) — recorded above
  with the corrected oracle fingerprints; the freeze rule stands.
- No parallel-board activity at this halt (main-only).

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use the tracked background process for
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
# (AC18 fixtures at write-target 4; Box_2007/example_2007 at 25 —
#  all AC1021 smokes hold their engage-pending numbers until H8c)
# AC21_MIRROR_DEBUG=1 prefixes any run to trace the gate decision.

# 3. Full corpus (280 files 0/0; read 0; write-target 3,576)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 40ab5d356cf05a71333f208e1651daf, 25,344 bytes
```

## Commit inventory (this halt — all PUSHED)

```
<feat> feat(dwg): the H8b AC21 container-shape mirror — the plan
       gate, the her-map-order emission, the mirrored finalize (§19
       H8b; 0/58 engage at the measured encoder-ratio wall; the
       conventional path stash-verified byte-identical, 3,576
       unchanged)
<docs> docs(harness): the post-H8b halt refresh — the landing
       record, the engage census, the H8c encoder-parity packet,
       the oracle record fix
37cc8ad <docs> / 06c12a7 <feat> ... (the prior halt series — see the
       previous NEXT_SESSION inventories: the H8a retention, the
       H7g mirror, the H7a-H7f landings, the §18 walks)
```

**PUSH STATE (2026-09-26)**: the `gold-vs-silver` branch is PUSHED
through this halt; push after each landing per the maintainer's loop
instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (battery:
serde green, gold_roundtrip green, corpus 280 @ 0/0 at 3,576, the
baseline stash captured) → the gold re-scan (the md5 mis-record
found and FIXED with the true wrapper/ELF fingerprints + the
behavioral identity check reproducing her page space exactly; the
decode contracts re-pinned incl. decode_rs/read_system_page line
level) → the required reading (AGENTS, §F2, §18.5-18.7, §19.1-19.3)
→ H8b implemented (the plan gate + emission + mirrored finalize +
the write_ac21_impl restructure + the XrefManifest carriage + the
encoding fallback) → the smokes (every AC1021 spec declined — the
objects windows measured 1.16–1.87× over her exact-rs_form slots;
the XrefManifest gate fixed; the unregistered-name `?`-fail fixed)
→ the battery (hermetic 42/42, corpus 0/0 @ 3,576, 196/196
byte-identity, generation identity verified twice) → the halt
refresh. The H8c encoder-parity packet is fully scoped above with
its on-disk ground truth; the mirror machinery ends this halt
LIVE and stash-safe, waiting on the encoder.
