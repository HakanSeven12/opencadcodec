# Zero-context prompt — the post-H8d halt: the loop BROKEN (the objects-parity unit LANDED — 3,576 → 3,121; the mirror ENGAGED 50/58)

> Campaign state 2026-09-27 (the halt after the H8d landing; the session
> ran the review → land-the-unit → verify → commit → push loop from the
> 3,576 post-H8c halt). **The measure→verify→revert loop is BROKEN: the
> H8d objects-parity packet landed as the ONE UNIT (α slice + 0xF_
> chain-long reland + the raw-echo placement layer) and moved the first
> rows since H8b. The ACS/SH campaign stays COMPLETE at 0/0: corpus 280
> files, read 0, write 0, read key-gap 0, write-target 3,121 (WAS 3,576
> — the landing closed 455 rows: R2007_Header 925→569, FILEHEADER
> 103→37, THUMBNAILIMAGE 50→17; every other key UNCHANGED — zero-keeping
> satisfied).** The §19 structure campaign's READ axis stays ZERO
> corpus-wide. Read `tests/gold_harness/AGENTS.md` first, then
> §F2.1–F2.3 + §18.5–18.7 in `IMPLEMENTATION.md`, then §19.1–19.3
> (§19.2's H8d row carries this landing's record), then this file top
> to bottom.

## The H8d landing (this session — the unit, all verified)

**The packet landed as the one unit per the prior halt's re-apply
package:**

1. **The α slice (record-level form parity)** — re-applied per the
   recorded 4-edit package: `write_spear_shift_ones` (bit_writer) on
   the merge's FINAL pad in both `merge_two_stream` and
   `merge_three_stream` (intermediate pads stay zero — verified
   byte-identical); `write_first_ref_handle` (the ownerhandle rule:
   relative iff (rel_len < abs_len) OR (rel_len == abs_len AND
   |offset| < value)) wired into the entity entmode==0 path and the
   non-entity internal path, both r2007_plus-gated; nulls keep code-4;
   the explicit relative_owner request paths keep write_handle_relative.
2. **The 0xF_ chain-long reland** — `emit_chained_match`'s nibble-0
   branch now emits `enc.bytes[0] |= 0xF0` (the author's remap form,
   3 bytes; the decoder masks 0xF_→0x0_ at exactly the chain position,
   gold decode_r2007.c:537-538) instead of the 4-byte extended
   fallback; the after-gap position keeps the class-0 form. Hermetic
   test `test_chained_long_match_rides_0xf_form` added.
3. **The placement layer (the raw-echo re-scope)** — the loop's
   deadlock resolution: READER retains her reconstructed
   objects-section stream + her handle map + the identity fingerprint
   (`objects_handle_set_fingerprint`: sorted handle set — the
   classes-verbatim doctrine; adds/deletes/renumberings decline the
   echo) on AC1021 files (`raw_acdb_objects_data` /
   `raw_acdb_objects_handles` / `raw_acdb_objects_fingerprint` on the
   document, serde-skipped, the H8a-retention pattern); WRITER's mirror
   arm passes her raw as the `Ac21MirrorBuffers.objects` buffer and
   re-emits her handle pairs through `write_handles` (her offsets
   address her echoed stream) when identity is unchanged. The
   conventional arm keeps our own emission untouched; an
   identity-changed document falls back to it by construction.

**The verification battery (all green at this halt):**

- `cargo test --features serde`: 1,337 lib tests green.
- `cargo test --features gold-harness --test gold_roundtrip`: ok.
- Family smokes 0/0: circle 21 (was 25), example_2007 11 (was 25),
  Box_2007 12 (was 25) — all three ENGAGED; example_2004 (AC18) at 4
  and example_2000 (AC15) at 80, both UNCHANGED (the 1s-pad and
  ownerhandle edits moved no AC15/AC18 row).
- **Generation identity RE-RECORDED** (the writer changed — the gate
  says re-run): `39e51dfe3fe281b2373994e2ebd82453`, 25,375 bytes (was
  `40ab5d356cf05a71333f208e1651daf`, 25,344 — the 1s-pad + ownerhandle
  forms touch the generated file's records; the generator reports
  30 OK / 0 FAIL).
- Full corpus: 280 files, read 0/0, write 0/0, write-target **3,121**.

**The acceptance census (AC21_MIRROR_DEBUG=1 per AC1021 corpus file —
58 files): 50 ENGAGED / 8 DECLINED.**

- The 50 engaged: all the uniform minis (Arc/Constraints/
  ConstructionLine/Donut/Ellipse/Helix/Leader/Line/Multiline/Point/
  PolyLine3D/Polygon/Polyline/RAY/Spline/Text/circle), example_2007
  (417,109 bytes / 540 handles), Box_2007 (178,375/207), and the whole
  Extrude*/Loft*/Polysolid*/Revolve*/Sphere/Union solids family
  (~171–188K each). Every engaged file's 0x80 addresses derive
  == retained (summaryinfo 3200, thumbnail 3360 on the minis).
- The 8 decliners, all at the OBJECT-IDENTITY gate (our emission's
  handle set ≠ her handles map) then the H8b objects-overflow class:
  **Chamfer/Cone/Cylinder/Fillet/Pyramid/Torus/Wedge_2007** (the
  ACIS-solids pattern — the skipped/undecoded records mechanism is the
  prime suspect) + **ATMOS-DC22S** (also carries the gap/terminator
  map entry, size 768 — its old legit decline). The echo's
  conservative doctrine is correct by construction: an edited object
  set must not be resurrected by the echo.

**THE HONEST FORECAST RECONCILIATION (the prior halt projected ~1,057
rows → ~2,519; the actual is −455 → 3,121 — recorded so the next
forecast is judged by the same standard):**

1. Only the pages-map family rows closed on the engaged files. The
   content-coupled crc/size family STAYS OPEN because the OTHER
   sections' bytes are still ours: the handles section is re-encoded
   from her pairs (same content, OUR chunk layout — byte-identity to
   her section bytes UNVERIFIED), and template/classes/aux/header are
   our emissions. The sections-map records carry our computed
   sizes/corrections; the pages-map crc_compressed/crc_seed and the
   MT-derive draws derive from the full map bytes. The engaged files
   carry ~10-12 remaining rows each (FileDepList 11 + the crc/size
   family).
2. The 8 decliners keep their full old families (25–26 rows each;
   ATMOS 48).

## The H8e queue (the next packet — the three levers, in order)

1. **The object-set parity on the 8 decliners**: autopsy why our
   emission's handle set differs from her handles map. The census
   message is precise (`objects echo DECLINED — object identity
   changed`); the instrument (`ac21_token_diff --our-rt`) plus a
   per-file handle-set diff (her handles-section map vs our
   `handle_map_u32`) names the dropped/added handles. The ACIS-solids
   pattern (7 of 8 are solids) suggests the failsafe skip drops
   records our document never materialized.
2. **The other-sections byte-identity** (the crc/size family's
   unlock): FIRST verify whether `write_handles(her_pairs)` is already
   byte-identical to her handles-section bytes (the instrument
   compares them directly — no new code needed to check); if not,
   retain her raw handles-section bytes (the same echo pattern) and
   pass them in the mirror arm. Then the map-bytes derivation: the
   sections-map/pages-map crc/size rows close when every section's
   compressed size matches hers (template/classes/aux/header are our
   emissions — their comp sizes entering the map records are the
   remaining divergence).
3. **The 5 record autopsies** (the α-era polish): the three LAYOUTs
   share ONE single-bit delta (bit 2 of a handles-tail byte: her
   0x94/0x14 vs our 0x90/0x10 at +227/+93/+93 — one common
   handle-form field, likely one fix for all three); obj 69 STYLE +
   obj 74 DIMSTYLE diverge beyond the head (+7: her 0x12/0x52 vs our
   0x11/0x50, same-length different-form field patterns — the §8.1.6
   spec-ordered field walk names them). These now gate ONLY the
   conventional arm and the decliners' re-encode path — the echo
   bypasses our emission on the engaged files.

## The remaining queue (3,121)

- **R2007_Header 569**: the engaged files' content-coupled crc/size
  family (sections-map CRCs/sizes/correction, pages-map
  crc_compressed/crc_seed/size_comp, the 3-per-file MT-derive draws)
  + the 8 decliners' full families. The H8e levers 1+2 above are the
  path.
- **FILEHEADER 37** (= AC1021 residue ~16 + R2000 7 + fallback 14):
  the AC1021 side closes through H8e lever 2; the R2000 seeker pair
  needs flat-layout parity.
- **THUMBNAILIMAGE 17**: the AC1021 residue closes through H8e lever
  2 (the engaged files' preview is already her raw container).
- **FileDepList 1,055 + SecondHeader 386 + AuxHeader 94**: the
  PARALLEL SESSION's rows (untracked probes `h7_probe1.sh`/
  `h7_rows.sh` in the repo root — not ours to commit).
- **R2004_Header 963**: accepted AC18 residue (the §19.2 re-scope
  records the address-class anatomy across 147 files and the SAME
  raw-echo answer for the AC18 mirror — the H8e levers generalize).

**The ceiling, stated plainly (updated)**: this queue's reachable
surface is the H8d-family residue — R2007_Header 569 + FILEHEADER-AC1021
~16 + THUMBNAILIMAGE 17 ≈ **602 rows**; FileDepList 1,055 + SecondHeader
386 + AuxHeader 94 = 1,535 are the PARALLEL SESSION's rows, and
R2004_Header 963 is accepted AC18 residue pending an AC18 decision —
the number's floor without the parallel session and that decision is
~2,498, and every halt record should say so.

## Dead / no-path rows (do not re-litigate)

`LoftD`; the SH revolve option shorts; **BREP stays deferred**
(external authentic ACSH_BREP_CLASS specimen required).

## The standing facts

- The four raw-retained SH tails decode to typed views with the
  captured bits as the write authority (Phase B). Hermetic suite green
  (1,337 + the new 0xF_ test; the compressor module at HEAD + the
  reland).
- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative.
- The generation identity is `39e51dfe3fe281b2373994e2ebd82453`,
  25,375 bytes (RE-RECORDED this halt — the writer changed; verified
  30 OK / 0 FAIL).
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
# (engaged AC1021 smokes hold their new numbers: circle 21,
#  example_2007 11, Box_2007 12; AC18 fixtures at 4)
# AC21_MIRROR_DEBUG=1 prefixes any run to trace the gate decision.

# 3. Full corpus (280 files 0/0; read 0; write-target 3,121)
python3 tests/gold_harness/run_corpus.py

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# 39e51dfe3fe281b2373994e2ebd82453, 25,375 bytes

# 5. The H8c instrument (analysis-only)
cargo build --bin ac21_token_diff --features serde
./target/debug/ac21_token_diff "$GOLD_TESTDATA/2007/circle.dwg" \
    --section AcDb:AcDbObjects --our-rt <OUR_RT>.dwg
```

## Commit inventory (this halt — all PUSHED)

```
<feat> feat(harness): the H8d objects-parity packet landed as the
       unit — α slice (1s pad + ownerhandle form) + the 0xF_
       chain-long reland + the raw-echo placement (reader retention
       + identity fingerprint + mirror-arm echo); the mirror ENGAGED
       50/58; write-target 3,576 → 3,121 (−455); read 0/0 held
<docs> docs(harness): the post-H8d halt refresh — the landing
       record, the engage census, the forecast reconciliation, the
       H8e queue
ecec275 <docs> / c3f0c73 / d7ddea4 / a51c71b / abedb5d / fc8b706 /
       b50d58a / fb5d6cd <docs> (the docs-session series — the loop
       diagnosis, the zero-reachability audit, the R2004_Header
       re-scope, the strengthened zero-audit, the §19.4 reference,
       the README instrument index, the H8d five-row autopsy — see
       the prior NEXT_SESSION inventories for the earlier series)
```

**PUSH STATE (2026-09-27)**: the `gold-vs-silver` branch is PUSHED
through this halt; push after each landing per the maintainer's loop
instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the halt-state verification (battery
green, the halt's 3,576 confirmed as the stale-report value after the
corpus re-run) → the required reading (AGENTS, §F2, §18.5-18.7,
§19.1-19.3) → the code sites read (bit_writer, merged_writer,
common.rs, dwg_reader, dwg_writer, compressor, handle_writer,
file_header_ac21) → the α re-apply (4 edits) → the 0xF_ reland + the
hermetic test (first assertion corrected: the finder's match choice
rides 0xF4, the FORM is what's asserted) → the placement layer (the
document retention fields + the fingerprint + the reader capture + the
mirror-arm echo with the identity gate) → the battery (serde 1,337
green, gold_roundtrip green, the 0xF_ test green) → the smokes (circle/
example_2007/Box_2007 all ENGAGED at 21/11/12; AC18/AC15 unchanged) →
the generation identity re-recorded → the full corpus (3,121, every
non-AC1021 key unchanged) → the engage census (50/58, the 8 decliners
autopsied at the gate messages) → the docs (the §19.2 H8d row after
the parallel docs session finished; this halt refresh) → the commit +
push. The H8e queue is scoped above with its three levers; the H8b
mirror machinery is LIVE and engaged on 50 files.
