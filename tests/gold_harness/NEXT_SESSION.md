# Zero-context prompt — TARGET ZERO held everywhere; the §20 genus
# queue is PENDING-ZERO after the EIGHTH QUEUE PACKET, and THE
# PRE-2007 RECORD-IDENTITY RESIDUE IS CLOSED (this continuation,
# three packets landed): (1) THE EIGHTH QUEUE PACKET — the gen_all
# cylinder wiring row (the BFS ordering packet's one re-opened row,
# "point before straight-curve", 2 occurrences) FELL: the gen_all's
# Solid3D/Body carried a hand-built SEAM cylinder whose only
# straight-curve hung on the seam edge (whose BFS token-walk lands
# after the vertex walks); the shape had NO authored counterpart
# (Cylinder_2018 carries no vertical seam), so the honest fix
# REMOVED the inauthentic shape — the gen_all now carries the
# authored-convention seam-less cylinder the primitive builder and
# the §20 fixtures use; sab_form 2 rows (0 pending / 2 TOLERATED) /
# sh_genus 1 (0/1) / acds 0, `--strict` passes, the identity moved
# to `bb9971a421733e8f09b114bf44b614ee`/25,473 (identical without
# serde); (2) THE MTEXT RECORD-IDENTITY PACKET (d81f864) — the
# pre-2007 MTEXT drift closed with the VERBATIM WIRE-TEXT CAPTURE:
# her example_2004 text `108\U+00B0` (escaped) vs example_2000's
# identical text RAW 0xB0 — the authored wire form is AUTHOR DATA,
# not a convention (the escape-always write rule was tried first
# and REFUTED by the R2000 census, then reverted);
# `read_variable_text_with_wire` returns the semantic text + the
# pre-MIF-decode wire string; `MText.dwg_wire_text` carries it; the
# writer's pre-2007 arm re-emits it verbatim (R2007+ keeps the
# decoded value); (3) THE DATATABLE PACKET (4f9cd18) — the LAST
# pre-2007 row fell with the FULL-RECORD RAW PASSTHROUGH (the
# CsacDocumentOptions/Unknown precedent on the same write
# dispatch, chosen over the write_wire_body regions replay
# because her 4337-bit handle stream runs bit-continuous to the
# byte-aligned record end with NO closing pad — a regions capture
# could not distinguish her trailing data bits from pad):
# `ClassObject` gains the `raw_dwg_data`/`raw_dwg_handle_bits`/
# `raw_dwg_version` envelope fields (serde(skip) — the JSON dump
# and the differ never see them, the fingerprint parity holds);
# the builder's generic class dispatch captures the whole record
# payload for the DATATABLE class at DWG read; the writer's
# ClassObject arm replays via `register_raw_object` BEFORE the
# typed emission, gated on `raw_passthrough_compatible`
# (same-version writes; conversions fall back to the modeled
# path — the no-authority typed layout stays the
# DXF/programmatic fallback); **ACCEPTANCE: the era censuses read
# example_2000 750/750 + example_2004 735/735 — ZERO divergent
# records; EVERY MEASURED ERA SPECIMEN NOW AT FULL RECORD
# IDENTITY** (R2000 750/750, R2004 735/735, Constraints_2010
# 216/216, example_2010 536/536, Constraints_2013 160/160,
# example_2018 474/474; the +LINE rewrite 473/474 + the edited
# BLOCK_HEADER); example_2018 carries NO DATATABLE instances so
# the rewrite-rejection acceptance stands; the battery green
# end-to-end (suite 52, mirrors, corpus 280 at 0/0/0/0 with the
# genus sections 2/1/0 pending-zero, identity UNMOVED, AC1021
# survey 0 divergent);
# THE CHIMERA BLAME-SPLIT — RUN AND DECISIVE (this continuation,
# the first experiment the closed rejection unlocked): the sab_swap
# chimeras re-built with the current code and probed under
# AutoCAD 2027 — **BOTH SIDES CARRY THE BLOCKER: the B-rep
# construction gap is AT LEAST TWO DEFECTS**: (A) the authored
# wrapper + the CONSTRUCTED region SAB → NO-SOLID (entity count 3,
# the first entity's bbox aborts "Automation Error. Invalid input")
# — the wrapper is proven good by the rewrite control (her file +
# LINE reads MODELED, census 3, real extents, through the same
# binary arm), so **the constructed SAB STREAM itself is a
# blocker**, below every measured genus invariant (the
# candidate-6 mirror region: the sense chain ffff, the vertex int
# 2, the era header, the BFS order — yet the modeler rejects it);
# (B) the constructed wrapper + the AUTHORED plain-region SAB →
# NO-SOLID — her SAB is proven good (the control models it), so
# **the constructed WRAPPER carries a blocker too** (the fixture's
# container/journal/entity emission — the G-C container is
# genus-correct yet her SAB dies inside it); the two chimeras
# OPEN cleanly at the file level (probe-end, censuses run — the
# rewrite-rejection closure extends to the chimeras);
# THE DESIGNED NEXT WORK — the two arms, one packet each:
# (1) THE SAB ARM: the below-invariant record-level pair diff —
# her plain region's SAB vs the constructed region's, token by
# token per class through the walk_sab surface with the
# face-chain-anchored isomorphism (the BFS-position pairing
# diverges at the first mention-order difference — the
# restore_gap_diffs.py instrument exists; the pre-divergence
# rows + the semantic pair-diff are the surfaces; the earlier
# suspects list's item (c) is the last unrun arm);
# (2) THE WRAPPER ARM: the container/journal graph investigation
# — the fixture's AcDs container + SH records vs her plain
# region's (the G-C extraction tools walk both): the
# ACAD_EVALUATION_GRAPH interposition (136/136 authored carriers
# carry it; the fixtures carry none — even the HistoryTree
# fixture, which carries the SH tree but still null-boxes, so
# the tree alone does not satisfy the wrapper requirement);
# the persubent attrib (journal-correlated) + the stream identity
# bytes; the asmheader version-string genus set {223.0.1.1930,
# 232.6.0.65535} (the slot's datblad/stream identity) remains a
# one-probe candidate on this arm; the probe improvement for the
# finer census: per-entity vl-catch-all trapping (the current
# census measures only entity[0]'s bbox — the loop aborts on the
# first failure, so a multi-entity file's later entities go
# unmeasured; the chimeras' verdicts stand by elimination via
# the control, but the per-entity census makes multi-entity
# probes first-class);
# THE REWRITE-REJECTION CAMPAIGN IS CLOSED (2026-09-29, the
# twelfth continuation): THE BYTE-PASSTHROUGH PACKET LANDED — the
# three divergent-objects' classes (ACDBASSOCALIGNEDDIMACTIONBODY
# 520, ACAD_TABLE 528, TABLECONTENT 529) replay their captured wire
# VERBATIM on the rewrite path (the reader marks the three body
# regions after the common parse — main bits from the body start to
# the main-data end, the text-region bits, the handle-tail from the
# post-common drain position with the author's closing 1s pad
# trimmed — and the writer re-emits the bits, re-creating the pad;
# the replay gates on the captured source DxfVersion so a
# conversion to another era falls back to the modeled emission);
# THE ACCEPTANCE LANDED LIVE: the example_2018+LINE rewrite OPENS
# IN AUTOCAD 2027 — census 3 (all solids/regions found), real
# extents, NO error dialog in the window transcript, probe-end
# reached, and the audit behavior is IDENTICAL to the authored
# original (dbmod 1 on BOTH files — the author's own audit trait,
# measured by the Original2018 baseline probe; the post-audit
# census keeps 3 with extents preserved); the rewrite's record
# census: 473/474 records BYTE-IDENTICAL (size + CRC-16), the one
# divergent record = the model-space BLOCK_HEADER carrying the
# added LINE (the edit's own footprint) + the our-only LINE handle;
# THE GATE WIDENED BY MEASUREMENT: the TableContent
# modeled-emission drift exists on EVERY era the class rides
# (AC1015 example_2000 h=9BC her 8317 bits vs our modeled 8114;
# AC1018 example_2004 h=ADB 8316 vs 8113 — the 203-byte class
# signature the AC1021 landing measured on h=BF2; AC1024+ the
# R2018 poison) — the capture covers them all: example_2000
# 749/750, example_2004 733/735, example_2010 536/536 (the file
# that carries ALL THREE classes on the AC1024 frame — the replay
# verified byte-exact there), Constraints_2010 216/216,
# Constraints_2013 160/160, example_2018 474/474; THE
# NEWLY-VISIBLE RESIDUE (the widened census instrument walks every
# record now — the old era censuses' smaller populations never
# saw these rows): DATATABLE (example_2000 754→1167;
# example_2004 753→1157 — a modeled over-emission) + MTEXT
# (example_2004 h=44C 100→94) — the NEXT record-identity packets,
# pre-existing (proven by the stash A/B), out of today's scope;
# THE SUSTAINER'S QUESTION ANSWERED (the same battery): the
# constructed fixtures re-probed under ACAD 2027 — every one
# NULL-BOX/NO-SOLID (the entity aborts the modeler with
# "Automation Error. Invalid input"; the audit purges) — the
# AcDs/wireframe fix did NOT move the null box: THE B-REP
# CONSTRUCTION GAP IS A SEPARATE ROOT on the conversion arm, still
# open (the candidates 1–6 genus work stands as genus-matching; the
# remaining measured lead: the asmheader version-string genus swap
# probe {223.0.1.1930 / 232.6.0.65535}); THE BATTERY GREEN
# END-TO-END: the suite 52 segments 0 failed — INCLUDING the four
# PRE-EXISTING reds the 34c75d0 spline packet left (deep_r2000/
# r2013/r2018 + rt_spline: the constructed model's dwg_wire_scenario
# None vs the wire-decoded Some(storage) asymmetry — proven
# pre-existing by the stash A/B, then FIXED test-side: the
# constructed doc is given the derived Some(storage) its own write
# emits, mirroring write_spline_data, so the value comparison stays
# armed while the None≡Some wire-capture artifact stops reading
# as a regression), the mirrors green, the four family smokes
# 0/0/0/0 (after the normalizer's wire-channel projection: the
# wire captures are writer-side codec channels with no gold
# counterpart — the source_version precedent — popped
# loop-universal for every object/entity so the read axis stays
# semantic), the corpus 280 at 0/0/0/0 with the genus sections
# 2/1/0 at PENDING-ZERO (the eighth packet — this continuation —
# closed the last pending row; every remaining count is the
# ADJUDICATED recorded state), the AC1021
# survey 58/58 / 0 divergent, the identity
# `bb9971a421733e8f09b114bf44b614ee`/25,473 (moved at the eighth
# packet's two SAB carriers — an intended content change; the
# byte-passthrough packet itself never touched a fresh write),
# the circle echo byte-identical; THE INSTRUMENT LANDED:
# tests/gold_harness/record_size_census.py — the handle-keyed
# record-identity census (size + hdlsize + bitsize + CRC-16) for
# ANY pair on ANY era (the pre-2010 [BS] block form + the R2010+
# [BOT]/UMC form; a new block header is the record-flush boundary
# for the pre-2010 logs that print no `< Next object:` separator),
# the era-census + rewrite-acceptance gate; DWG_NO_ECHO=1
# target/debug/dwgrewrite stages the conventional-arm rewrite to
# census against; the README step-7b + inventory entries updated

# [THE BYTE-PASSTHROUGH MECHANISM — the packet's one design]
# The §19 H8h-ext-12 AC1021 TABLECONTENT capture was the template:
# for the classes whose typed re-encode drifts from the author's
# bytes (silver's model walks the frame but drops form content the
# model never retained — 528 loses the cell-style/border
# sub-structures it reads only "to stay positioned"; 529 loses
# the same content class; 520 loses 2 body bits), the read marks
# the record's class-specific regions BEFORE the typed walk
# (dwg_document_builder.rs wire_capture_marks: body_start = the
# main cursor after the common fields; main_end; text_len;
# handle_from = the handle cursor after the common handle reads)
# and peeks them AFTER (capture_wire_body — peek does not move
# the cursors); the write replays them verbatim
# (dwg_stream_writers/object_writer/common.rs write_wire_body:
# the main bits bit-for-bit, the text bits raw into the text
# stream, the handle bits with the author's final 1s pad
# re-created — the reader trimmed exactly her pad, corpus records
# end byte-aligned or in a 0 bit so the trim never over-cuts).
# THE GATES: the TABLE entity + the ALIGNEDDIM body capture on the
# R2010+ frames (AC1024/AC1027/AC1032 — where the drift was
# measured); TABLECONTENT on every era ≥ AC1015 (the widened
# census measured the drift on every era the class exists);
# AC1021 keeps its record-identity attested behavior (the 58/58
# survey ran with the capture in place). The writer replays only
# when the write targets the captured source DxfVersion
# (wire_dxf_version) — a conversion falls back to the modeled
# emission rather than emit foreign-frame bytes. The fields ride
# the models (Table gains wire_dxf_version; AssociativeObject
# gains the seven wire fields), serde-default so the fingerprint
# sees them (read-only captures: parity holds), and the dump side
# never sees them as comparison payload (the normalizer pops them
# loop-universal). THE DEFECT MAP, FINAL (V18/V26/ACAD2027):
# 1. THE FILE-LEVEL REJECTION — CLOSED (the wireframe synthesis +
# the SPLINE wire-scenario + this byte-passthrough packet — ACAD
# 2027 opens the edited R2018 rewrite with the census finding the
# entities and the audit indistinguishable from her own file);
# 2. THE B-REP CONSTRUCTION GAP (the conversion arm, OPEN): the
# fixtures null-box under V18/V26 (V26's audit purges) and under
# ACAD2027 the entity LISP aborts on them and the audit purges —
# re-probed at this halt: UNCHANGED by the file-level fixes (the
# separate-root answer); 3. THE AUTHORED CONTROLS: Box_2018
# MODELS in all three loaders — the discrimination holds.
> Campaign state 2026-09-29 (the halt after the seventh queue packet;
> the record-identity campaigns are ALL CLOSED: **THE CORPUS STAYS AT
> ZERO ON EVERY AXIS: 280 files, read-fidelity 0, write-fidelity 0,
> read key-gap 0, write-target 0** — re-verified at this halt WITH
> the genus sections attached as additional output; the AC1021 survey
> 58/58 / 0 divergent records and the era censuses R2000 235/235,
> R2004 227/227, R2010 216/216, R2013 160/160, last re-verified
> 2026-09-28 — the ordering packet's writer surface (the SAT-text→SAB
> conversion) is disjoint from every surveyed path: authored SAB
> carriers queue verbatim in the conventional arm, and pre-2007
> entities carry SAT text on the wire — the SAT text writer is
> untouched). The ACS/SH campaign stays COMPLETE at 0/0. The §19
> structure campaign's READ axis stays ZERO corpus-wide. **THE §20
> GENUS-GATE QUEUE IS PENDING-ZERO: the fifth layer is LANDED
> (§20.6), REVIEWED, and SEVEN PACKETS CLOSED — G-C (the container:
> `acds_genus_diffs` 11 rows → 0), the vertex role token, the
> asmheader record, the tolerance triple, the TOLERATED adjudication
> state (the ADJUDICATIONS table: rows close on recorded verdicts
> while keeping their counts), the persubent adjudication (the
> journal-correlated selection bias, pinned by the 2026-09-29 corpus
> scan: 72 carriers outside the fixture family, the attrib rides the
> doc's journal, journal-less authored carriers lack it), and the
> ordering genus normalization (the 60-row family: the rank table
> rewritten to a verified linearization of the pinned
> order_constraints — body, lump, transform, shell, face, loop, the
> surface family, coedge, edge, vertex, the curve family, point —
> and the write-site normalization widened to every SAT-text→SAB
> conversion; authored-shaped docs are genus-ordered so the stable
> sort is the identity on them). The sections stand at
> sab_form_diffs (4 rows — 0 pending / 4 TOLERATED) /
> sh_genus_diffs (1 row — 0 pending / 1 TOLERATED, the elide marker)
> / acds_genus_diffs (0 — CLOSED); `--strict` passes. THE BRICSCAD
> AUDIT IS MECHANIZED: strict_load_probe.py drove bricscad.exe /b
> headless and the verdict is recorded — the authored specimens
> model real extents in all three envelopes (2007/2013/2018), every
> constructed fixture yields the ±1e80 null box, and the old-vs-new
> discrimination shows the failure PRE-EXISTING and order-independent
> (the ordering packet is exonerated; the new queue head is the
> constructed-SAB restore gap).** Read
> `tests/gold_harness/AGENTS.md` first, then §20 (all of it — now
> with the §20.6 landed state and its seven closed packets), then
> §19.4 + §19.5, then §18.6 + §F2.1–F2.3, then this file top to
> bottom.

## The arc (2026-09-28, the §20 implementation session; continued
## 2026-09-29 with the fourth through seventh queue packets)

1. **The genus gates implemented, verbatim in §20.3's shape**:
   `genus_extract.py` (the expectation extractor — decodes the
   156-file sh_history family silver-side, walks every SAB blob with a
   framing mirror of `sab.rs`, projects the three families into the
   pinned `genus_expectations.json`; fixtures-only pin so it
   regenerates identically everywhere; `--corpus-scan` folds in the
   ACIS-bearing gold-tree files on demand), `genus_gates.py` (the gate
   run — regenerates the constructed corpus: the gen_all canonical,
   its md5 recorded as a fact, plus the `genus_constructed` fixture
   family: one solid per SAB surface family, one region, one body, one
   `create_solid_history` tree; decodes, asserts, emits the ranked
   sections), the `run_corpus.py` integration (the sections as
   ADDITIONAL output; the four axes and the differ untouched; a genus
   pipeline failure degrades to an error note), and the env-gated cargo
   mirror `tests/genus_gates.rs` (presence-gated, skip-pass with a
   marker file; asserts a fresh extraction EQUALS the pin — expectation
   drift is itself reviewable — and the report well-formed; the counts
   are NOT asserted zero).
2. **The day-one extraction is clean**: 136 SAB carriers (all four
   eras — the 2007/2010 in-entity SABs carry the 21200/21500
   flavors), 136 SH roots (uniform: the ACAD_EVALUATION_GRAPH
   interposition, 33/427, owner != ownerhandle, node ids resolve,
   node owner is the graph, the solid's history_handle links the
   root), 78 jard containers (ds_version 16/17, segidx-first 128,
   num_segidx 91/97, the two era tail patterns, the named-pointer slot
   types), ZERO extraction anomalies. The width sets carry the
   authored variance honestly: coedge {55, 60}, straight-curve
   {85, 103}, both terminators, four magic/version pairs.
 3. **The day-one queue (the initial counts — the designed nonzero)**:
    - **G-A (67 rows)**: the vertex short-width (34 vs the authored 39
      — the one topology class the conic/quadric completions did not
      cover), the missing asmheader, the `ACIS|700` flavor vs the
      authored 21200/21500/21800/22300 pairs, the header triple
      (0,1,0) vs the authored (0,2,*), the product strings
      ('acadrust'/'ACIS 7.0' vs 'Autodesk AutoCAD'/'ASM 232.6.0.65535
      NT'), the tolerance triple (spatial_resolution 10.0 vs the
      authored 1.0), the missing persubent-acadSolidHistory attribs,
      and the record-class ordering family (the writer's restore-file
      rank vs the authored order — with the point-rank rows its
      largest arm: the writer emits points FIRST, every authored
      specimen emits them LAST). **The completions' regression guard
      HOLDS: cone-surface 149, ellipse-curve 118, plane-surface 112,
      sphere-surface 122, torus-surface 130, straight-curve 85 all sit
      AT the authored genus.**
    - **G-B (1 row)**: the constructed-tree elide marker — the
      2646f05 contract (no ACSH records; the solid's history
      soft-pointer NULL) ranks as the tree's current genus divergence,
      adjudicated TOLERATED by the cylinder verdict; the topology
      invariants — including the solid-history-links-root arm — stay
      armed for any SH record that appears in a constructed decode.
      Tree-ABSENCE on a constructed solid is deliberately NOT ranked
      (the specimen family is selected for trees: 136 carriers = 136
      roots — selection bias, not genus).
    - **G-C (11 rows)**: ds_version 1 vs the authored 16/17, the
      segidx-last position vs the authored segidx-first 128,
      num_segidx 8 vs the authored 91/97, file_header_size 128 vs
      65664, the populated-slot tail pattern, the unpopulated prvsav
      slot — the ranked queue head, exactly as §20.2 predicted.
 4. **The review pass (the same day, the OCS lens — "review that the
    genus gates validate what OpenstudioCad constructs against the
    golden genus") found and fixed two instrument defects**: (a) the
    ordering genus computed `after ∩ keys` — a UNION — instead of
    always-after (`after − before`): variable pairs (coedge↔cone-
    surface, cone-surface↔plane-surface) were pinned as constraints
    BOTH ways (false positives against constructed orders the
    authored genus itself shows), and `point` — last in every
    authored first-appearance order — fell out of every constraint
    set, hiding the writer's points-first rank (the largest true
    ordering divergence) entirely; the fix recomputed after−before,
    regenerated the pin, and the corrected G-A queue grew 55→67 rows
    (false rows gone, the point-rank rows in). (b) the pin carried
    `solid_history_links_root: [True]` (the cylinder-audit
    dangling-link sibling: every authored root is named by its
    owning solid's history soft-pointer) but the gate never asserted
    it — the arm landed, armed for any constructed tree that
    survives save. Also verified in the pass: the constructed family
    exercises ONLY the public API surface (the OCS command path —
    the primitive builders, from_sat, add_entity,
    create_solid_history), and the unused-import residue was
    cleaned from both instruments.
 5. **The G-C container packet CLOSED (the first queue packet, the
    same day)**: `build_acds_prototype` gained the era profiles
    live-measured from the specimens (ds_version 16/17, segidx-FIRST
    at 128, the 91/97-row scales with the authored slot allocations,
    the named pointers, the populated prvsav — 48-byte header + zero
    body — and 2013's freesp, file_header_size 65664, unknown_1 8;
    the invented `_data_` id=3 thumbnail boilerplate dropped — the
    authored sections carry one `_data_` row). Two extractor arms
    extended (`unknown_1s`, `container_versions`), two regression
    tests pin both eras through construct → write → read-back with
    the SAB surviving. **`acds_genus_diffs`: 11 rows → 0.** The
    generation identity MOVED — the container bytes changed:
    `4265c04a19048e33295bc047cb0b2908`, 25,407 bytes (re-recorded
    here; an intended content change, never papered over).
 6. **The zero-keeping rule held through the G-C landing**: the full
    corpus re-ran at 280 files with ALL FOUR AXES 0 and the genus
    sections attached (the authored corpus rides the echo — the
    constructed container path is disjoint); the hermetic suites are
    green (52 segments, 0 failures, the two new era tests in); the
    genus cargo mirror green (fresh extraction == the regenerated
    pin); the four family smokes 0/0; the AC21 echo clean.
 7. **The vertex packet CLOSED (the second queue packet, the same
    day)**: the authored vertex's missing token — its role within
    its own edge (0 = start, 1 = end, 2 = both endpoints of a
    closed edge; the semantics pinned by a 580-vertex census: every
    authored vertex carries the 4-token payload, roles distribute
    216/268/96, the per-vertex edge-endpoint correlation is exact).
    `add_vertex` emits the role placeholder; `add_edge` fills it for
    the owning vertex records. **The vertex row's 28 occurrences
    gone: `sab_form_diffs` 67 → 66 rows, 521 → 493 occurrences.**
    The identity moved to
    `0e8b23cde1f2476198154cc317b16683`, 25,439 bytes. The suite
    green; the mirror green.
 8. **The asmheader packet CLOSED (the third queue packet, the same
    day)**: the authored SAB opens with
    `asmheader $-1 $-1 "232.6.0.65535"` (era-uniform, byte-identical
    across all 136 carriers). The record prepends at the SAB-write
    boundary (`prepend_asmheader`) with every wire pointer shifted
    by +1 — the SatDocument keeps the DXF-SAT body-at-0 convention
    (`new_body`'s recorded rule); the authored numbering is
    asmheader $0, body $1, …. Five SAB module tests updated to the
    shifted index convention. **The asmheader row's 11 occurrences
    gone: `sab_form_diffs` 66 → 65 rows, 493 → 482 occurrences.**
    The identity moved to
    `a8f227e09d60a17605043f9dedebc30c`, 25,439 bytes.
 9. **The tolerance packet CLOSED (the fourth queue packet,
    2026-09-29)**: the constructed SAB header carried
    `[10.0, 1e-06, 1e-10]` against the pinned authored genus
    `[[1.0, 1e-06, 1e-10]]` (uniform across all 136 carriers). The
    source was double: the `SatHeader::new()` DEFAULT pinned 10.0
    (`types.rs`), and the existing "normalize to 1.0 for SAB" pass —
    written for exactly this rule, with the code's own comment that
    IntelliCAD/AutoCAD native SAB data always uses 1.0 — sat AFTER
    `strip_for_sab`'s `nothing to strip` early return, so it never
    ran on the clean primitive documents the constructed corpus
    always takes. Both closed: the default is 1.0, and the normalize
    moved BEFORE the early return so it now applies unconditionally
    (a parsed older-ACIS source carrying 10.0 normalizes at
    conversion instead of leaking into SAB). **The tolerance row's 11
    occurrences gone: `sab_form_diffs` 65 → 64 rows, 482 → 471
    occurrences.** The generation identity moved to
    `a7c5f17080891a7c26a380a686e9298a`, 25,439 bytes (the res double
    changed in every constructed SAB; the compressed sections
    absorbed it, size unchanged). The suite green (52 segments);
    the mirror green (the pin untouched — the extraction surface
    never reaches the writer); the four smokes 0/0 with clean
    echoes; the corpus re-verified at 280/0/0/0 with the sections at
    64/1/0.
10. **The TOLERATED adjudication packet CLOSED (the fifth queue
    packet, 2026-09-29)**: §20.3's "close as TOLERATED with the row
    kept as the recorded state" gained its instrument — the
    ADJUDICATIONS table in `genus_gates.py` (a `(gate, field)` →
    recorded-verdict map with provenance; a listed row keeps its
    count, gains `status`/`verdict` annotations, a status column +
    an adjudication block in the markdown, and falls out of the new
    `pending_*` counts; `--strict` now asserts zero PENDING rows).
    Four rows adjudicated on their recorded verdicts: the three
    ACIS-700-family rows — `header-magic-version` (the ACIS|700
    binary flavor is silver's native output, BricsCAD-ACCEPTED per
    the 2026-09-21 strict-load zero, re-confirmed by the 2026-09-22
    region probe; the authored era stamps are the author's, not a
    genus defect), `header-triple` (the (0,1,0) triple rides the same
    accepted stance; the authored (0,2,4/12/24/26) semantics are
    unexplained constants — reproducing them would forge unmeasured
    fields), and `product-strings` (the author's identity rule: never
    forge Autodesk identity stamps) — plus G-B's `constructed-tree`
    elide marker (the 2646f05 cylinder verdict). **The primary counts
    stay 64/1/0; the PENDING queue is now G-A 61 rows / 438
    occurrences (the 60-row ordering family + the persubent class)
    and G-B 0.** The mirror green (the row shape is additive; the
    pin untouched); the instrument change touches no codec surface.
11. **The persubent packet CLOSED (the sixth queue packet,
    2026-09-29)**: the designed tree-correlation investigation,
    run as a live probe over the corpus decodes (73 SAB carriers
    outside the tree-selected fixture family, 72 walked clean): the
    persubent-acadSolidHistory attrib is JOURNAL-CORRELATED, not
    unconditional genus — 41 carriers carry it, every one in a file
    with journal/ACSH data (example_2004's solid + journaled Region
    carry it while its plain Regions do not; the entity's history
    soft-pointer is NOT the trigger — the DOC's journal is; ATMOS's
    11 pointer-bearing attribless carriers are the known broken-map
    exception). The fixture family's uniform 136/136 is the tree
    SELECTION. The constructed primitives are journal-less at save
    (the G-B elide) — emitting the marker would forge journal
    presence. **The row adjudicated TOLERATED (selection bias,
    journal-correlated) with the corpus evidence; pending G-A
    61 → 60 rows, 438 → 427 occurrences.** The pin untouched (the
    class stays in the specimen genus); the arm stays for any
    constructed journal that survives save.
12. **The ordering packet CLOSED (the seventh queue packet,
    2026-09-29)**: the 60-row ordering family fell. The
    investigation corrected the source map first: the rows came
    from the BUILDERS' assembly order (primitive-built docs carry
    body at index 0 — new_body's rule — so the reorder trigger
    never fired; the SAB inherited the construction order
    verbatim), not from `reorder_restore_file`. Two changes:
    (a) the rank table rewritten to the authored first-appearance
    genus — a verified linearization of the pinned
    order_constraints (zero violations, computed from the pin):
    body 0, lump 1, transform 2, shell 3, face 4, loop 5, the
    surface family (cone 6, sphere 7, torus 8, plane 9), coedge 10,
    edge 11, vertex 12, the curve family (ellipse 13, straight 14),
    point 15, else 16 — full-class matches for the disambiguated
    families with base fallbacks keeping unknown surfaces with
    plane and unknown curves with straight; (b) the write-site
    normalization widened to EVERY SAT-text→SAB conversion (the
    body-not-first trigger dropped; the asmheader + raw-Sab-token
    guards kept): authored-shaped documents are genus-ordered so
    the stable rank sort is the identity on them; builder-ordered
    and cadkernel-assembled documents re-rank; body keeps rank 0
    (the 2026-09-22 leading-top-level contract preserved). One SAB
    module test moved to the new convention (the transform now
    leads the face per the pinned order). **`sab_form_diffs`
    64 → 4 rows, 471 → 44 occurrences — the PENDING queue 0/0/0
    across all three families; `--strict` passes.** The identity
    moved to `e4cd19603fddc0eea8c91d407ed54af6`, 25,473 bytes
    (identical without `--features serde`). The battery green
    end-to-end: suite 52 segments, the mirror (fresh extraction ==
    the untouched pin), the four smokes 0/0 with clean echoes, the
    corpus 280 at 0/0/0/0 with the sections 4/1/0. THE OPEN
    VERIFICATION: the re-ranked constructed stream awaited the
    strict-loader audit (the revert rule recorded in §20.6's
    packet-seven record).
13. **The strict-loader probe MECHANIZED + the restore-gap finding
    (the second continuation session, 2026-09-29)**:
    `tests/gold_harness/strict_load_probe.py` — the /b-scripted
    BricsCAD audit (the LISP census in the post-OPEN document
    context; `vla-getboundingbox` forces the modeler — real
    extents = a healthy restore, ±1e80 = the null box; every
    launch bounded, a missing result = AMBIGUOUS; the build leaves
    LOGFILENAME unset so the restorer's text is not captured — the
    DB/modeler census is the measured surface). The paired probes:
    authored specimens model REAL extents in all three envelopes
    (Box_2007/2013/2018: 0,0,0..1,2,3); every constructed fixture
    reports entity-count 1 with the NULL BOX; the old-vs-new
    discrimination (the pre-rank cfe36f6 sab.rs, regenerated and
    probed identically) yields the SAME null box. **Two verdicts:
    (1) the ordering packet EXONERATED — the failure is
    pre-existing and order-independent, the revert rule's
    causation condition does not engage, the ordering rows stay
    closed on the genus axis; (2) the constructed SAB stream has
    NEVER constructed as an ACIS body in the strict kernel — §20.1's
    blindness a third time (the entity layer is sound; the B-rep
    never builds; the historical audits read the error surface,
    which went quiet after the 2026-09-28 fixes and was mistaken
    for acceptance). The NEW QUEUE HEAD: the constructed-SAB
    restore gap; the designed investigation is the record-level
    SAB diff (authored Box_2018 vs constructed Box, token-by-token
    per class through the shared walk_sab surface), with the probe
    as the acceptance gate.**
14. **The candidate-3 packet — the BFS emission (the fourth
    continuation session, 2026-09-29)**: the authored restore-file
    order was derived and PROVEN: BFS first-mention traversal
    (seed every body; each record emitted once at first mention —
    the attribute pointer first, then the token pointers in token
    order; FIFO; orphans in index order; the terminator last) —
    **136/136 authored carriers reproduce their EXACT record
    sequence** under it (the first "verification" was VACUOUS: the
    probe tokenizer was missing the 0x0C pointer tag in its size
    table, so every pointer read garbage and BFS degenerated to
    the identity — the lesson is recorded: fix the instrument
    before trusting its verdict). `reorder_restore_file`'s rank
    table replaced by the BFS (the remap machinery untouched);
    the SAB stream now carries the authored traversal-interleaved
    order. **The probe verdict: still the ±1e80 NULL BOX on every
    constructed fixture (the authored control MODELED — the
    SPECIMENS path bug that silently skipped the control is
    fixed)** — candidate 3 alone does not close the gap either.
    The BFS re-opened ONE queue row (2 occurrences): the gen_all
    cylinder's loop wiring reaches its base edges before the seam
    edge, so its points precede its straight-curve — a FIXTURE
    divergence (the authored Cylinder_2018 has NO vertical seam at
    all: its loops are single closed-coedge circles; the example's
    seam-cylinder shape has no authored counterpart) — the fix is
    the example's side-loop coedge chain order, a small
    follow-up. The probe also gained the DBMOD/ERRNO capture
    (pre/post-audit; gen_all's audit CHANGES the drawing — dbmod
    1 — while the standalone fixtures read 0).

15. **The byte-passthrough packet (the twelfth continuation,
    2026-09-29 — THE REWRITE-REJECTION CLOSURE)**: the designed
    packet landed as specified: the three divergent-objects'
    classes (520 ACDBASSOCALIGNEDDIMACTIONBODY, 528 ACAD_TABLE,
    529 TABLECONTENT) capture their class body at read
    (dwg_document_builder.rs wire_capture_marks + capture_wire_body —
    the H8h-ext-12 TableContent template generalized; Table gains
    wire_dxf_version; AssociativeObject gains the seven wire
    fields) and replay it verbatim at write
    (write_wire_body in the object_writer common — the main bits,
    the text-region bits, the handle tail with the author's 1s pad
    re-created; the replay gates on the captured source DxfVersion).
    The gates widened by measurement: TABLECONTENT's drift exists on
    every era the class rides — AC1015 example_2000 h=9BC (8317 vs
    our modeled 8114) and AC1018 example_2004 h=ADB (8316 vs 8113),
    the same 203-byte signature as the AC1021 h=BF2 landing — so the
    capture covers ≥ AC1015; the TABLE entity + the aligned-dim
    body capture on the R2010+ frames. THE ACCEPTANCE (the probe,
    AutoCAD 2027): the example_2018+LINE rewrite OPENS — census 3,
    real extents, no error dialog, probe-end; the authored-original
    baseline probe reads IDENTICALLY (dbmod 1 on both files — the
    author's own audit trait); the record census (the new
    record_size_census.py): 473/474 byte-identical, only the edited
    BLOCK_HEADER + the new LINE diverging. THE BATTERY: suite 52
    (the four pre-existing 34c75d0 deep/rt-spline reds fixed
    test-side — the wire-scenario sync), the mirrors green, the
    four smokes 0/0 (the normalizer's loop-universal wire-channel
    projection — the source_version precedent), the corpus 280 at
    0/0/0/0 with the genus sections 3/1/0, the AC1021 survey
    58/58, the era censuses 216+536+160+474+749/750+733/735 (the
    two residuals = the newly-visible pre-existing DATATABLE/MTEXT
    rows), the identity UNMOVED (0e953809…/25,473). THE SUSTAINER'S
    QUESTION ANSWERED: the constructed fixtures re-probed under
    ACAD 2027 — all NULL-BOX/NO-SOLID (the entity aborts the
    modeler; the audit purges) — the file-level fixes did NOT move
    the null box; the B-rep construction gap is a separate root on
    the conversion arm, still open.

16. **The eighth genus queue packet — the gen_all cylinder wiring
    row (this continuation, 2026-09-29): THE QUEUE AT
    PENDING-ZERO.** The BFS emission packet's one re-opened row
    ("point before straight-curve", 2 occurrences) attributed
    precisely: BOTH in the gen_all canonical (its Solid3D + Body),
    NONE in the primitive fixtures — the gen_all carried the
    example's hand-built SEAM cylinder (`build_cylinder_sat`),
    whose only straight-curve record hung on the seam edge, and
    the BFS walks the seam edge's tokens LAST (its first mentions
    arrive via the lateral coedge ring, whose walks queue after the
    cap/vertex walks): the two vertex points won the race against
    the seam's straight-curve (genus: point always after
    straight-curve). The shape had NO authored counterpart
    (Cylinder_2018 carries no vertical seam — its loops are single
    closed-coedge circles), so the fix REMOVED the inauthentic
    shape rather than re-wiring it: the gen_all's Solid3D/Body now
    carry the authored-convention seam-less cylinder the primitive
    builder and the §20 fixtures use; the local builder deleted.
    **sab_form 2 rows (0 pending / 2 TOLERATED) / sh_genus 1 (0/1)
    / acds 0 — `--strict` passes; the pin untouched (the cargo
    mirror green); the identity moved to
    `bb9971a421733e8f09b114bf44b614ee`/25,473, identical without
    --features serde.** The suite 52 green, the mirrors green,
    the corpus re-verified. The queue's remaining counts are the
    ADJUDICATED recorded state (the persubent/product-strings
    classes + the G-B elide) — the fifth layer's designed nonzero
    is now exactly its recorded verdicts everywhere.

## THE NEXT WORK: the file-level rewrite rejection (the payload-swap experiment's finding) — THEN the SAB question

**[CLOSED 2026-09-29, the twelfth continuation — the sections below
are the historical mid-state]**: the rewrite rejection fell in
three packets — the wireframe-block synthesis (1ad2509), the
SPLINE wire-scenario capture (34c75d0), and the byte-passthrough
packet (this session): the three divergent classes (520/528/529)
replay their captured wire verbatim on the rewrite path; the
example_2018+LINE rewrite opens in AutoCAD 2027 with the census
finding all 3 solids/regions and the audit indistinguishable from
her authored original (dbmod 1 on both — her own audit trait); the
record census: 473/474 byte-identical, the one divergence the
edited BLOCK_HEADER's own footprint. THE REMAINING WORK in this
territory: (1) the B-rep construction gap (defect 2 — re-probed at
this halt, the null box did NOT move; the asmheader version-string
genus swap probe {223.0.1.1930 / 232.6.0.65535} is the remaining
measured lead); (2) the newly-visible record-identity residue (the
widened census walks every record now, populations the old era
censuses never saw): DATATABLE (example_2000 754→1167, example_2004
753→1157 — a modeled over-emission) + MTEXT (example_2004 h=44C
100→94) — pre-existing per the stash A/B, the next censused
packets.

**THE CANDIDATE-6 RESULTS (2026-09-29, the eighth continuation)**:
the sheet mirror LANDED in `build_region_sat` (the constructed
region now matches the authored R2018 region on EVERY measured
invariant: the sense chain `ffff` + the reversed ring, the vertex
int 2, the era-profiled header, the BFS order) — **the probe STILL
reads the ±1e80 NULL BOX**. Both bisect arms are negative: the
measured SAB stream is not the (whole) blocker.

**THE PAYLOAD-SWAP EXPERIMENT (examples/sab_swap.rs — committed)**:
two chimeras (authored example_2018 wrapper + constructed SAB;
constructed wrapper + authored SAB) plus a pure-rewrite control
(the authored doc + one LINE, region untouched). **ALL THREE FAIL
TO OPEN IN BRICSCAD** — the window-lifecycle transcript (the
scraper) proves it: the main-window title never leaves
`[Drawing1]`; the censuses were measuring a BLANK drawing (the
"zero entities" readings). Silver's own reader decodes the same
files fine (the control: 243 entities, the regions' SABs intact);
gold reads them (the corpus axes' design). The FRESH-document
writes (the fixtures) open fine — the fixture probes' transcripts
show the real title transitions.

**THE REFRAMED DEFECT MAP**: (1) **the file-level rewrite
rejection** — the conventional arm's read→rewrite output for
edited R2018 documents is rejected by the strict loader at the
FILE level, silently in script mode. NEVER CAUGHT because every
corpus axis runs unmodified roundtrips (the echo — byte-identical
by construction) and the record-identity surveys covered the
echo too; the edited-document write path was never strict-loader-
probed. This is a real-world correctness defect (an edited R2018
document does not open in BricsCAD) AND it blocks the chimera
experiments (the SAB-vs-wrapper blame split). (2) The SAB-level
question (the fixtures open but the B-rep does not construct)
remains second — the candidates 1–6 genus work stands as
genus-matching; the null-box blocker is below the measured
invariants or in the wrapper.

**THE DESIGNED NEXT PACKET — the AcDs two-arm slot diff**: the
file-level bisect COMPLETED (the ninth continuation's elimination
matrix, below); the poison arm is precise now: **the AcDs-section
emission for PRE-DECODED (is_binary, pre-carried sab_data)
entities in EDITED documents**. The two arms diverge in
`src/io/dwg/dwg_stream_writers/object_writer/entities.rs`:
- The conversion arm (`queue_sab_entry`: `acis.is_binary == false`
  + SAT text → parse→strip→write_for_era→the AcDs slot) — the
  fixtures ride it, and they OPEN (their entities carry SAT text;
  the SAB is built at write).
- The binary arm (`acis.is_binary && sab_data` → verbatim queue of
  the pre-decoded SAB into the AcDs slot) — every rejected file
  rode it: a read document's entities carry is_binary SAB, and
  ANY edit (a LINE, an SAB swap, a cloned region) forces the
  conventional arm through the binary queue → the emitted file
  is BricsCAD-rejected at the file level.

The minimal pair for the byte-level diff:
`target/genus_gates/constructed/Region.dwg` (the conversion arm,
OPENS) vs `/mnt/c/Users/SebastianSchoeller/AppData/Local/Temp/
kilo/swap2/rewrite_control.dwg` (Region read + LINE = the binary
arm, REJECTED — same document content, same version, 22,660 vs
22,724 bytes). Gold parses both clean (headers, CRCs, classes,
section_info verified; zero warnings) — the divergence is
BricsCAD-strict-only and lives in the AcDs container's bytes:
diff the two files' `AcDs:Prototype_1b` section payloads slot by
slot (the genus gates' AcDs container extractor decodes both:
`tests/gold_harness/genus_gates.py` / the G-C tools; the container
sits in the section whose decoding the era censuses verified).
The suspects: the slot header framing for pre-decoded SAB (the
size fields, the section/slot metadata), the target-version/
datavalue pairs around the slot, and the entity's history/
soft-pointer fields on the edit path.

**THE ELIMINATION MATRIX (the probes, all /b + LOGSEC-lisp
censuses + the window-transcript title check)**:
- REJECTED (file never opens — the title stays `[Drawing1]`):
  sab_swap's two chimeras + read(example_2018)+LINE (this
  campaign's original control); `read(Region.dwg)+LINE`
  (swap2/rewrite_control — the pure minimal rewrite);
  `fresh-doc + read-cloned binary region + LINE` (fresh_pair's
  fresh_region_plus_line — the fresh-document control that
  killed the read-state hypothesis); the read_region_plus_line
  twin.
- OPENS: the byte echoes (swap2/swap_* — unedited → the echo,
  byte-identical, 22,660 bytes, census 1 ✓ sanity);
  `line_only.dwg` (a FRESH doc with ONLY a LINE — 21,113 bytes,
  census 1 ✓ the LINE is innocent); every fresh fixture (the
  conversion arm); every authored original.
- Also cleared along the way: stale .dwl2 locks (none in the
  staging dir); the section_info descriptors (identical modulo
  content growth); the classes; the sampled header vars; the
  tails/section-frame (gold's -v5 full-dump diff shows only
  address/CRC/size deltas).

The instruments for the next session (committed): the sab_swap /
fresh_pair / line_only examples; the probe scripts' pattern; the
minimal pair on disk. Acceptance for the fix: the binary-arm
writes OPEN in BricsCAD (the rewrite control's census finds the
region), and the full corpus still 0/0/0/0.

**AFTER THE AcDs FIX — the null-box question returns**: the
fixtures (conversion arm) OPEN but their B-reps do not construct
(±1e80). Candidates 1–6 closed every measured SAB genus
divergence; the null box persists. Two live leads: (1) the AcDs
fix may also move the null box (the container's slot metadata may
gate the modeler — re-probe the fixtures immediately after);
(2) if not, the remaining measured divergence is the asmheader
version-string genus set {223.0.1.1930, 232.6.0.65535} — the
slot's datblad/stream identity bytes — worth one swap probe.

**The state after candidates 1–4 (2026-09-29, the fifth
continuation)**: all four landed and genus-verified — (1) the
era-profiled coedge form, (2) the identity transform, (3) the
BFS first-mention emission (136/136 authored carriers verified),
(4) the era-profiled SAB header (`SabEra`: the ASM|22300 flavor +
the 0x34 trailing byte + the constant-2 bodies field + the
era-coded history field 26/24/12/4; `write_for_era` at the three
call sites; the two header rows FELL — sab_form 5 → 3 rows, the
remaining three all adjudicated: product-strings, persubent, +
the 1-row gen_all wiring pending). **The probe still reads the
±1e80 NULL BOX on every constructed fixture; the authored
control reads MODELED.** The era-profile packet's FULL BATTERY is
IN FLIGHT at this halt — finish it (suite + mirrors + smokes +
echoes + corpus + identity), commit, push. The console mechanism
LANDED alongside (bac227f): the window-lifecycle scraper
(bricscad_console_scraper.ps1 — the tested channel map: LOGFILENAME
read-only, WM_GETTEXT empty on the wxWidgets UI, UIA empty,
GetWindowText works; the transcript captures the run's window
lifecycle with timestamps).

**THE CANDIDATE-6 FINDINGS (2026-09-29, the seventh continuation
— the same-era minimal pair)**: the suspects (a) and (b) were
cleared — (a) an authored journal-less, attrib-less carrier MODELS
(example_2004/2013/2018's plain Region, 28 records, 0 attribs:
real extents, all three eras — "attribs absent" does NOT block
modeling); (b) the coedge wiring symmetry — 0 violations in both
streams (partner/next/prev all correct). **THE DECISIVE SAME-ERA
PAIR then landed: the authored R2018 REGION (MODELS) vs the
CONSTRUCTED REGION (NULL-BOX) — identical 28-record topology,
identical ASM|22300 header triple, and exactly THREE divergences
remained**:
1. **The coedge sense chain: authored `ffff` (all reversed) vs
   constructed `TTTT` (all forward)** — the two loops travel the
   same rectangle in OPPOSITE directions. The authored BOX top
   face travels CCW (TTTT, verified); the authored REGION travels
   CW with the same +z plane normal — ACIS's sheet (region-face)
   handedness appears LEFT-HANDED relative to the material-side
   normal, and the constructed region emits the wrong direction.
2. **The vertex int token: authored `2` on every sheet vertex vs
   constructed `0`** — the 580-vertex census's 0/1/2 semantics
   were derived from SOLIDS; the authored SHEET convention is
   demonstrably 2 (this region MODELS with role 2 on all four
   vertices, both endpoints of distinct-parent edges).
3. The asmheader's version string is a GENUS SET, not one value:
   example_2018's region carries "223.0.1.1930" while the
   Box_2018 specimens carry "232.6.0.65535" — BOTH model, so not
   the blocker, but the asmheader row's pin (era-uniform) came
   from the tree-selected fixtures only.

**THE DESIGNED EXPERIMENT (candidate 6, the next packet)**: in
`build_planar_body` — (1) flip each face loop's coedge senses so
the loop travels CW viewed from the face's material-side normal
(mirror `Sense::Forward`/`Reversed` in the per-segment
assignment; verify with `restore_gap_diffs.py`'s travel audit
against the authored region), and (2) set the vertex int to the
authored sheet value 2. Regenerate the constructed corpus, then
`strict_load_probe.py`: acceptance is the CONSTRUCTED REGION
reading MODELED (the box fixtures are the follow-up — their
senses/vertex ints need the same authored-mirror treatment
class by class: authored box top TTTT / sides Tfff / bottom
TTTT with vertex roles 0/1). One change at a time is fine —
bisect senses-first (the biggest divergence), then the vertex
int.
the orientation audits, landed as
`tests/gold_harness/restore_gap_diffs.py`)**: the full structural
diff (BFS-position paired, persubent attribs skipped) surfaced the
sense-flag family as the suspects, then the three-level audit
CLEARED every computable invariant in BOTH streams: (1) the face
orientation — every authored face effectively outward (sense
compensates the surface normal: fwd+outward or rev+inward), every
constructed face outward directly (uniform fwd, outward normals)
— coherent in both; (2) the loop traversal — every loop closed
and vertex-consistent in both; (3) the travel direction — every
loop travels CCW around its face's effective normal in both.
The sense PATTERNS differ (authored top/bottom TTTT, sides Tfff;
constructed bottom ffff, top TTTT, sides TTff) without either
being invalid. **The constructed B-rep is demonstrably valid at
every computable level — the blocker is subtler still.** THE
REMAINING SUSPECTS, in test order: (a) **probe an authored
journal-less, attrib-less carrier** — every MODELED control so
far carries persubent attribs (the sh_history family is
tree-selected), so "attribs absent is fine" is UNPROVEN;
example_2004's plain Regions (attrib-less, journal-less) are the
candidates — if they MODEL, attribs are cleared; if they
NULL-BOX, BricsCAD's modeler may require the subentity/journal
machinery and the persubent adjudication reopens; (b) the coedge
partner/next/prev SYMMETRY invariants (partner.edge == edge,
partner.sense == !sense, next.prev == self) checked on both
streams; (c) the semantic pair-diff via a face-chain-anchored
isomorphism (the BFS-position pairing diverges at the first
mention-order difference — the pre-divergence rows still
surfaced: vertex role ints 1 vs 0 at aligned positions). The
probe (with the window transcript) is the acceptance gate:
MODELED with the authored controls green.

**The BricsCAD audit is MECHANIZED and its verdict is recorded
(2026-09-29, the second continuation session)**:
`tests/gold_harness/strict_load_probe.py` drives `bricscad.exe /b`
headless; the script's LISP censuses the 3DSOLID/REGION entities,
forces the modeler via `vla-getboundingbox`, audits, re-censuses,
and quits without saving (nothing outlives the OPEN document
switch; a missing result file = AMBIGUOUS, never clean; the build
leaves LOGFILENAME unset, so the restorer's TEXT is not captured —
the DB/modeler census is the measured surface). Run:

```
python3 tests/gold_harness/genus_gates.py    # fresh constructed corpus
python3 tests/gold_harness/strict_load_probe.py
```

**The maintainer's user-run verdicts (2026-09-29, opening the
constructed fixtures by hand)**: ConstructedCylinder "not showing
correctly", and ConstructedBox reports **"General modeling failure /
AcDb3dSolid(31)"** at OPEN — the strict loader's own words, the
entity layer resolving (AcDb3dSolid identified by id) while the
modeler rejects the B-rep. The failure MESSAGE BLOCKS the scripted
runs: the prompt stalls the /b script mid-sequence (the LISP result
file opens — `#<FILE …>` — but the buffered write-lines never
reach `(close)` before the launcher's timeout kill), which is why
strict_load_probe.py's staged runs read 0 bytes; the maintainer
closed the blocking instances by hand. The tool's next revision
needs an error-dialog discipline (EXPERT/sysvar suppression or a
post-error continuation arm) before its runs read clean; the
recorded verdicts below stand on the validated manual procedure
plus these user-run confirmations. **The record now carries the
verbatim kernel verdict — the restore gap is named by the strict
loader itself.**

**The recorded verdicts**: the authored specimens MODEL across all
three envelopes (Box_2007 in-entity SAB; Box_2013/Box_2018 AcDs —
real extents 0,0,0..1,2,3), while every constructed fixture
reports entity-count 1 with the ±1e80 NULL BOX, and the old-vs-new
discrimination (the pre-rank `cfe36f6` sab.rs, regenerated and
probed identically) yields the SAME null box. **The ordering
packet is EXONERATED** (the failure is pre-existing and
order-independent; the revert rule's causation condition does not
engage; the ordering rows stay closed on the genus axis). **The
new head of the work queue is the restored gap itself: the
constructed SAB stream has NEVER constructed as an ACIS body in
the strict kernel** — the entity layer is sound (opens, censuses,
survives audit), the B-rep never builds, and every historical
BricsCAD reading was the error surface; once the 2026-09-28 fixes
landed the surface went quiet, which was read as acceptance while
the body silently failed to restore.

**The restore-gap campaign, mid-state (2026-09-29, the third
continuation)**: candidate 1 LANDED — the era-profiled coedge form
(the authored 2013/2018 streams' 60-byte coedges carry the
parameter-space slot as `int 0, ptr −1`; 2007/2010 carry the
8-field form — measured on the specimen corpus, 55/60 by magic):
`SabWriter::write_modern` inserts the pair at the SAB boundary
(SAT-text emission untouched), the AcDs queue paths and the
R2013+ in-entity branch call it. Candidate 2 LANDED — the identity
transform: the per-family census is definitive (Box/Wedge/Pyramid/
Cylinder/Cone/Sphere/Torus/Chamfer/Fillet/Union ALL exactly 1
transform, all eras; the operation families 0), so the seven
primitive builders call `set_placement(identity)` — the
constructed Box now opens `asmheader, body, lump, transform,
shell, face…`, byte-shaped like the authored genus. **Both fixes
are genus-verified (gates 0-pending) but the probe STILL reads the
NULL BOX** — the modeler still rejects the stream. The probe
itself was fixed (the 0-byte mystery was BUFFERING, not stalls:
every section now opens→writes→closes via a LOGSEC helper, so
evidence survives any kill). **CANDIDATE 3 LANDED (the BFS emission, arc item 14)**: the
authored order proven 136/136, `reorder_restore_file` now emits
the BFS first-mention traversal, the probe still reads the NULL
BOX — and the packet re-opened ONE queue row (the gen_all
cylinder's loop-wiring order, 2 occurrences — a fixture
divergence; the fix is the example's side-loop coedge chain).
The candidates-1+2 packet is committed (66d18f8) with its battery
green (corpus 280 at 0/0/0/0, the identity 0b813a68…/25,473).

**CANDIDATE 4 — THE DESIGNED NEXT PACKET: the era-profiled SAB
header.** The constructed stream's header is the last measured
divergence class: the flavor ACIS|700 + the triple (0, 1, 0),
against the authored era genus (measured on the specimens):
R2007 (AC1021) ACIS|21200 + (0,2,26); R2010 (AC1024) ACIS|21500
+ (0,2,24); R2013 (AC1027) ACIS|21800 + (0,2,12); R2018
(AC1032) ASM|22300 + (0,2,4). The second field is the CONSTANT 2
across every authored carrier (even one-body files); the third
is era-coded (26/24/12/4); the constructed writes 1 and 0. The
designed fix: `write_modern` (the R2013 in-entity and AcDs
paths) emits the era-correct magic + version + triple — R2018:
ASM|22300 + (0,2,4); R2013: ACIS|21800 + (0,2,12). The PRODUCT
STRINGS stay silver's own (the author-identity rule — the flavor
and the triple are format fields, not identity). The
header-triple adjudication (TOLERATED on the 2026-09-21
load-acceptance) is SUPERSEDED by the finer probe measurement:
the ACIS|700 stance LOADS (the entity layer resolves) but has
never MODELED — the §20.4 rule applies (a genus expectation
changes through a recorded probe verdict, and the probe now
measures modeling). The acceptance gate: the probe reads MODELED
with the authored controls green. If the header closes the gap,
the gen_all wiring row and the flavor/triple re-pin close as
follow-ups.

- headers: authored ASM|22300 (0,2,4); constructed ACIS|700
  (0,1,0) — the adjudicated rows.
- record counts: authored 114 (with 26 persubent attribs + ONE
  transform); constructed 87 (zero attribs — the adjudicated
  journal gap; **zero transforms**).
- the topology ladder is FULLY WIRED in both (body→lump tok1,
  lump→shell, shell→face, face→loop/surface, loops, coedges,
  vertices) — the earlier "null lump pointer" reading was the
  transform slot misread (body tok3: authored ptr 3 = its
  transform record; constructed ptr -1).

**The three root-cause candidates, in test order**: (1) **the
coedge tail form** — authored coedges end
`… ptr <loop> true, ptr <pc-int-slot>, int 0, ptr -1` (9 tokens)
while constructed end `… ptr <loop> false, ptr 10, ptr -1`
(8 tokens): the constructed stream's final pcurve slot is ONE
`ptr` where the authored carries `int 0, ptr -1` — the
(int,ptr) parameter-space pair the kernel's coedge reader expects;
candidate-fix in the SAB writer's coedge pcurve arm. (2) **the
missing transform record** — the authored body links an identity
transform (body tok3 → transform record; the constructed carries
NO transform records at all; `set_body_transform` exists
(`types.rs` ~2596) and the builders never call it); candidate-fix:
new_body (or the builders) emits the identity transform + wires
body tok3, mirroring the authored genus (verify transform is
authored-uniform in the pin's class census first). (3) **the
stream structure** — the authored stream is TOPOLOGY-TRAVERSAL
interleaved (face → its loop → its surface → its coedges → its
edges → vertices/curves), the constructed is CLASS-GROUPED (both
satisfy the pinned first-appearance constraints — the gates'
zero-pending was correct — but the restorer may expect traversal
shape; a reorder to authored interleaving would be per-consumer
assembly, a different design than the class-rank sort).

The test protocol: fix one candidate, regenerate, run
`strict_load_probe.py` (a healthy fixture should OPEN WITHOUT the
"General modeling failure" prompt — which also un-sticks the
scripted runs); the acceptance gate reads **MODELED** (real
extents) with the authored controls green.

The standing guards stay armed regardless: the G-C container
regression tests + gate rows (any writer change that regresses
ds_version, the segidx position, the row scale, the slot allocation,
the pointers, prvsav, or the two extended header fields re-ranks
immediately); the vertex role, asmheader, and tolerance arms; the
G-B topology invariants for any constructed tree that survives
save; the pin's fresh-extraction mirror.

**The residual work beyond the queue** (the standing future-work
rows, unchanged): the version-parity tiers (§19.5: R13/R14
implemented-but-divergent; pre-R13 unsupported — maintainer
decisions); the era censuses across the full era corpora (today's
proofs cover the constraints specimens); the unattested subcurve
action types (17/19/23/42/27); the MT-variant pinning (§19.4.G);
the dead rows (LoftD, the SH revolve shorts, BREP deferred); the
ATMOS 84 her-only orphans (the broken-map pre-existing issue); the
DXF writer's non-assoc PersSubentManager tail-BL gap (outside the
campaign's gates, noted for completeness).

## The final design (the one thing to understand — unchanged)

**The unified whole-file echo** (`write_to_writer` in
`src/io/dwg/dwg_writer.rs`): when the document came from a
same-version DWG read, her whole on-disk file is retained
(`document.raw_ac21_tail`, serde-skipped, every container family),
and the document-state hash holds, the rewrite re-emits her bytes
VERBATIM. Everything else — the H8b/H7g mirrors, the conventional
arms, the prepare pipeline — serves edited documents and conversions,
unchanged.

**The document-state fingerprint**
(`document_state_fingerprint` in `src/io/dwg/mod.rs`): the sorted
per-part hash of the semantic inventory's visit plus the ten table
control handles and the retained metadata models. Captured at the END
of the read; compared BEFORE the prepare pipeline at the write entry.
The wire-only captures (`table_control_entries`, `dimstyle_morehandles`)
are EXCLUDED from the fingerprint; `xdic_by_handle`, the modeled
subcurve fields, the persubent tails, the SEQEND flag captures, the
constraint-group node captures, the persubent-id tail + stream-
presence captures, and the TABLECONTENT wire captures are INCLUDED.

**The honest framing, FINAL**: for an unedited same-version roundtrip
the DWG writer is a byte-copy gated on a full-content hash. The
conventional arm is record-identical to the author's stream on EVERY
AC1021 corpus file (58/58, 0 divergent) AND every record of all four
era specimens (the era censuses all-zero).

## The remaining work (after the queue opens)

- **The genus-gate queue itself** (above) — the initial NONZERO
  counts are the queue; each closes through the §8.1.2 packet workflow
  with the strict-loader verdicts adjudicating.
- **The version-parity tiers outside the scope** (§19.5): R13/R14
  implemented-but-divergent (89/153/149 diffs on the three r14
  specimens); pre-R13 unsupported (gold decodes nothing there either).
  Both pathed in §19.5; maintainer decisions.
- **The era censuses across the full era corpora**: today's proofs
  cover the Constraints specimens (1 file/era).
- **The unattested subcurve action types** (17/19/23/42/27): no corpus
  specimens. **The MT-variant pinning (§19.4.G)**: the crc_seed draws.
  **The dead rows**: LoftD; the SH revolve shorts; BREP deferred.
  **The ATMOS 84 her-only orphans**: the broken-map pre-existing issue.
- NOTE: the DXF writer's non-assoc PersSubentManager arm does not
  emit the captured tail BLs (the DWG writer does) — outside the
  campaign's gates, noted for completeness.

## The standing facts

- The corpus workdirs are STEM-KEYED (280 files → 196 unique stems);
  report.json totals are authoritative: all four axes 0, WITH the
  genus sections (`sab_form_diffs` 4 rows — 0 pending / 4
  adjudicated-TOLERATED; `sh_genus_diffs` 1 row — 0 pending / 1
  TOLERATED, the elide marker; `acds_genus_diffs` 0 — CLOSED) as
  additional output. **The queue's PENDING axis is ZERO; the counts
  that remain are the five adjudicated rows' recorded state
  (§20.4).**
- The generation identity is `bb9971a421733e8f09b114bf44b614ee`,
  25,473 bytes (moved at the eighth genus queue packet — the
  gen_all's Solid3D/Body cartridge changed from the hand-built
  seam cylinder to the authored-convention seam-less shape, an
  intended content change; UNMOVED through the rewrite-campaign
  packets — the wireframe fix, the spline wire-scenario, and the
  byte-passthrough packet never touch a fresh write's bytes; the
  identity history: `84374e73…`/25,375 through the §20
  landing → `4265c04a…`/25,407 at G-C → `0e8b23cd…`/25,439 at the
  vertex packet → `a8f227e0…`/25,439 at the asmheader packet →
  `a7c5f170…`/25,439 at the tolerance packet → `e4cd1960…`/25,473
  at the ordering packet → `0e953809…`/25,473 at the era-profiled
  SAB header packet d1dd5ee → `bb9971a4…`/25,473 at the eighth
  genus queue packet). The generator
  builds and runs identically WITH or WITHOUT `--features serde`.
- The genus expectations pin
  (`tests/gold_harness/genus_expectations.json`) is FIXTURES-ONLY:
  regenerate with `python3 tests/gold_harness/genus_extract.py` and
  review the drift; the cargo mirror asserts fresh == pin.
- The record-identity state (the 58-file AC1021 survey): **58/58
  files at 100%, 0 divergent records, 84 her-only orphans (ATMOS's
  broken map)**. circle 211/211; ExtrudeC 206/206; Box 207/207;
  Leader 245/245; Chamfer 207/207; Fillet 207/207; Loft 207/207;
  PolyLine3D 218/218; Constraints 219/219; ATMOS 340/340;
  example_2007 540/540. **The era censuses: R2000 235/235, R2004
  227/227, R2010 216/216, R2013 160/160.**
- The gold tree sits at `34f02f54` FROZEN with ONE tracked
  generated-file drift (`src/config.h.in`, autoheader requote) — the
  freeze rule stands. Oracle fingerprints unchanged: the 6,316-byte
  libtool wrapper `programs/dwgread` (md5
  `8dad57211b78f42e7594ba6c211cc0e5`) and the ELF
  `programs/.libs/dwgread` (md5
  `d852da1db0894b866e86042d4b26b91d`, 230,232 bytes). NO
  `encode_r2007.c` exists — gold has no R2007 writer.
- Spec authority on file: the ODA spec PDF
  (`~/work/OpenDesign_Specification_for_.dwg_files.pdf`, 270pp) +
  libredwg `bits.c`. NEITHER documents the ACDBASSOC* classes, the
  subcurve wire, the non-assoc PersSubentManager, the
  constraint-group node classes, or the TABLECONTENT wire (the
  authority closure); the ASSOC persubent variant HAS a gold spec
  block; the 3DSOLID history_id AVAIL_BITS rule IS in gold's
  dwg.spec COMMON_3DSOLID; gold's pab/child_param declared ref codes
  (5/3) are WRONG vs the wire (4/4).
- The hermetic suites: serde green (the full suite at this halt), 
  gold_roundtrip green, genus_gates green, at every landing.
- Autopsy tooling notes: gold's `-v9` `@byte.bit` positions are
  RECORD-relative; the survey's merge-trace positions are
  SPAN-relative. The record-head MS is 15 data bits per 16-bit LE
  word. **The merged-stream frame math**: RL = main + text + 16 + 1;
  main_data_end = RL − text − 17; the handle stream starts at the RL.
  **The pre-2007 TV wire**: BS(len+1) + chars + NUL. **The
  TwoStream frame**: the handle stream is BIT-CONTINUOUS at the RL.
  The BL forms: 00 = 4-byte LE, 01 = 1 byte, 10 = 0; the BD forms: 00
  = full 66-bit LE double, 01 = 1.0, 10 = 0.0. **The SAB walker
  facts** (genus_extract.py mirrors `sab.rs` framing): the
  ENTITY_TYPE 0x0D and SUBTYPE 0x0E tags carry a RAW 1-byte length
  (not a string tag); the ASM magic is 14 bytes + 1 trailing byte
  with the version u32 at offset 15 either way; the resfit double is
  optional; the End-of-* terminator carries no attribute pointer and
  no EOR. **The campaign lessons (the full set lives in the §19.2
  rows + §19.4)**: the SEQEND era-convention lesson; the pad lesson;
  the era-gate lesson; the presence lesson; the recompute lesson;
  the ref-code lesson; the normalize lesson; the form-rule lesson;
  the content-agnostic lesson; the reader-correctness proof; the
  accessor-gate pattern.

## Environment (complete)

The repo lives in WSL. From Windows:
`\\wsl.localhost\Ubuntu-24.04\home\sebastianschoeller\work\cadcodec`.
Shell commands run via
`wsl.exe -d Ubuntu-24.04 -- bash <script>` — write scripts with the
write tool and run by absolute path (PowerShell quoting caveats:
inline `&&`, `$var`, pipes, nested quotes and multi-word grep
alternations are all broken; ONE COMMAND PER LINE in script files;
`sleep` is capped at 120 s — use long timeouts on the bash tool for
the corpus — the full corpus takes ~4 minutes + ~2 for the genus
stage; the bash tool's timeout parameter is capped at 120 s in
practice — run the corpus as a background process and read its
logs).

```bash
# Environment (source this):
export PATH="$HOME/.cargo/bin:$PATH"
export GOLD_DWGREAD="$HOME/work/libredwg/programs/dwgread"
export GOLD_TESTDATA="$HOME/work/libredwg/test/test-data"
```

## Verification gate (the zero-keeping rule — all four axes; the genus sections are additional)

```bash
# 1. Build gates
cargo test --features serde          # green at this halt (the full suite)
cargo test --features gold-harness --test gold_roundtrip
cargo test --features gold-harness --test genus_gates   # the NEW mirror
cargo test --features serde --test issue80   # the edit-survival gate

# 2. Family smokes (all four families, write-target 0)
python3 tests/gold_harness/run_roundtrip.py \
    <FIXTURE>.dwg /tmp/smoke
# AC21: $GOLD_TESTDATA/2007/circle.dwg
# R2000: $GOLD_TESTDATA/example_2000.dwg
# AC18: $GOLD_TESTDATA/example_2004.dwg
# R2018: $GOLD_TESTDATA/2018/Dynblocks.dwg

# 3. Full corpus (280 files; ALL FOUR AXES 0 — the genus sections
#    are ADDITIONAL output; these totals must not move)
python3 tests/gold_harness/run_corpus.py
#    ... genus gates (the §20 queue — PENDING-ZERO): sab_form 4 rows
#        (0 pending / 4 TOLERATED), sh_genus 1 row (0 pending / 1
#        TOLERATED), acds_genus 0 (G-C CLOSED; the vertex, asmheader,
#        and tolerance rows closed at packets two through four; the
#        ACIS-700 family, the G-B elide marker, and the persubent
#        class adjudicated at packets five and six; the 60-row
#        ordering family fell at packet seven)

# 4. Generation identity (re-run if the writer changes)
cargo run --example gen_all_entities_all_versions_dwg --features serde
md5sum gen_all_entities_all_versions.dwg
# bb9971a421733e8f09b114bf44b614ee, 25,473 bytes (moved at the
# eighth genus queue packet — the gen_all cylinder cartridge
# change; UNMOVED through the wireframe/spline/byte-passthrough
# rewrite-campaign packets — those never touch a fresh write;
# earlier history: e4cd1960… at the ordering packet → 0e953809… at
# the era-profiled SAB header packet d1dd5ee)
# (identical without --features serde)

# 4b. The record-identity survey (58/58, 0 divergent)
python3 tests/gold_harness/record_identity_survey.py \
    "$GOLD_TESTDATA"/2007/*.dwg "$GOLD_TESTDATA"/example_2007.dwg \
    tests/gold_harness/tests/sh_history/*_2007.dwg

# 4c. The era/record-identity censuses (record_size_census.py,
#     handle-keyed, size+hdlsize+bitsize+CRC-16; DWG_NO_ECHO=1
#     dwgrewrite stages the conventional rewrite)
python3 tests/gold_harness/record_size_census.py \
    <ORIG.dwg> <REWRITE.dwg>
# measured at this halt: Constraints_2010 216/216; example_2010
# 536/536 (all three poison classes on AC1024); Constraints_2013
# 160/160; example_2018 474/474 (+LINE rewrite: 473/474 + the
# edited BLOCK_HEADER); example_2000 750/750 + example_2004
# 735/735 — ZERO divergent records (the MTEXT rows fell at the
# wire-text-capture packet d81f864; the DATATABLE row fell at the
# full-record raw-passthrough packet 4f9cd18; EVERY MEASURED ERA
# SPECIMEN AT FULL RECORD IDENTITY)

# 5. The byte-identity check (the echo's acceptance, per family)
cmp "$GOLD_TESTDATA/2007/circle.dwg" <RT_DIR>/circle_rt.dwg
# clean (no output)

# 6. The H8c instrument (analysis-only, unchanged)
cargo build --bin ac21_token_diff --features serde

# 7. THE GENUS GATES (the §20 layer — LANDED)
python3 tests/gold_harness/genus_extract.py   # regenerate expectations
python3 tests/gold_harness/genus_gates.py     # the ranked report
# the queue is PENDING-ZERO (the eighth packet closed the last
# pending row — the gen_all wiring fixture divergence); the rows
# that remain are the ADJUDICATED recorded verdicts (persubent +
# product-strings + the G-B elide) kept as counts on purpose;
# `--strict` asserts the pending-zero state

# 7b. THE STRICT-LOAD PROBE (the mechanized loader audit)
#     (Windows-visible host required; the DEFAULT run exercises BOTH
#      loaders — BricsCAD V26 [DEFAULT_BCAD] and AutoCAD 2027
#      [DEFAULT_ACAD, the format author] — every fixture under each,
#      per-loader verdicts; --loader bcad/acad narrows)
python3 tests/gold_harness/strict_load_probe.py
# the constructed fixtures still read NULL-BOX/NO-SOLID under ACAD
# 2027 (re-probed at this halt — the B-rep construction gap is the
# OPEN campaign, the null box did NOT move with the file-level
# fixes; the sentinel is ±1e80 under BricsCAD, ±1e20 under
# AutoCAD); the authored controls read MODELED; the
# example_2018+LINE rewrite reads MODELED — census 3, real
# extents, the audit indistinguishable from her authored original
```

## Commit inventory (this halt)

```
<docs> the halt refresh - the CHIMERA BLAME-SPLIT RUN (this
       commit): the sab_swap chimeras re-built + probed under
       ACAD 2027 — BOTH NO-SOLID with each arm proven by the
       control: the constructed SAB stream is a blocker (A:
       her wrapper + constructed SAB) AND the constructed
       wrapper carries one (B: her SAB + constructed wrapper;
       the HistoryTree fixture still null-boxes so the tree
       alone is not it) — the B-rep gap is AT LEAST TWO
       DEFECTS; the next work pinned one packet per arm (the
       below-invariant SAB pair-diff + the container/journal
       graph investigation); the per-entity census noted as the
       probe improvement
c24bc24 <docs> the halt refresh - the pre-2007 record-identity
       residue CLOSED (the DATATABLE packet 4f9cd18; every
       measured era specimen at full record identity; the B-rep
       gap the next work)
4f9cd18 <feat> the DATATABLE record-identity packet - the
       full-record raw passthrough (the CsacDocumentOptions/
       Unknown precedent on the ClassObject path; the
       raw_dwg_data/raw_dwg_handle_bits/raw_dwg_version envelope
       fields, serde(skip); register_raw_object replay gated on
       raw_passthrough_compatible); ACCEPTANCE: example_2000
       750/750 + example_2004 735/735 - ZERO divergent records;
       the battery green end-to-end (suite 52, mirrors, corpus
       280 0/0/0/0 genus 2/1/0 pending-zero, identity UNMOVED,
       AC1021 survey 0 divergent)
0c2b9e9 <docs> the halt refresh - the queue-pending-zero + MTEXT packet
       state (the DATATABLE next-packet design)
d81f864 <feat> the MTEXT record-identity packet - the verbatim
       pre-2007 wire-text capture (read_variable_text_with_wire +
       MText.dwg_wire_text + the writer's pre-2007 verbatim arm);
       the escape-always attempt REFUTED by the R2000 census and
       reverted (the author-data lesson); era censuses 749/750 +
       734/735 (MTEXT rows fell), corpus 280 0/0/0/0, suite 52,
       identity UNMOVED; the DATATABLE row = the designed next
       packet (the byte-passthrough class)
2128b7d <feat> the eighth genus queue packet - the gen_all cylinder
       wiring row closes and THE QUEUE IS PENDING-ZERO (the
       inauthentic seam-cylinder shape removed; the gen_all carries
       the authored-convention seam-less cylinder; --strict passes;
       the identity to bb9971a4.../25,473)
<docs> the post-closure halt refresh (the twelfth continuation:
the rewrite-rejection campaign CLOSED - the top state + the arc
item 15 + the verification gate + the standing facts re-recorded)
29fa2cf <feat> the R2018 record-identity packet - byte-passthrough
       for the unmodeled drift classes (the rewrite-rejection
       closure): the 520/528/529 classes replay their captured wire
       verbatim; TableContent widened to >= AC1015 by measurement;
       record_size_census.py the new instrument; the 34c75d0
       spline test asymmetry fixed test-side; the normalizer's
       wire-channel projection; ACCEPTANCE: the example_2018+LINE
       rewrite opens in AutoCAD 2027 (census 3, real extents, the
       audit identical to her authored original; record census
       473/474 + the edited BLOCK_HEADER); corpus 280 at 0/0/0/0,
       suite 52 green, era censuses all-identical except the
       newly-visible pre-existing DATATABLE/MTEXT rows; the
       fixtures re-probed NULL-BOX unchanged (the B-rep gap a
       separate root)
574a59e <docs> the post-H8h-ext-6 halt refresh (the prior halt head)
a1a1506 <feat> H8h-ext-7: the SEQEND captured-flags fix (15 -> 14)
b5e16b5 <docs> the post-H8h-ext-7 halt refresh
195ef3b <docs> the Constraints 3E3 dissection-state handover
d6b0431 <feat> H8h-ext-8: the Constraints 3E3 closure (13 -> 12)
d3d2258 <feat> H8h-ext-9: the ATMOS closure (12 -> 9)
0625f6c <feat> H8h-ext-10: the example_2007 first fruits (9 -> 5)
3dd3362 <feat> H8h-ext-11: the wire-exactness landings (5 -> 3)
000f3b2 <feat> H8h-ext-12: the zero-residual landing (3 -> 0)
d9bf5f3 <docs> the post-H8h-ext-12 maintenance review
6712327 <feat> H8h-ext-13: the pre-2007 constraint-group arms closed
dc3737a <feat> H8h-ext-14: the R2010/R2013 arms closed + the census
0f2bbfa <feat> H8h-ext-15: the era full-file record identity closed
2bafbe4 <docs> the post-H8h-ext-15 maintenance review
a99ccc6 <docs> the version-parity matrix as future work (§19.5)
274fcf4 <docs> the version-parity matrix in the README
2646f05 fix(dwg): the 2026-09-28 cylinder verdict - conic/quadric
       completions + the constructed-tree elide + §20 (the design)
da78f79 docs(harness): the IMPLEMENTATION.md review + the stale-claim
       reconciliation (§1 goal, §5 coverage, §7 title, §8.1.6 pointer,
       §17 F1/F2) + the genus-gates handover
ba12a2f <feat> §20 genus gates: genus_extract.py + genus_gates.py + the
       genus_constructed fixture-family bin + the pinned
       genus_expectations.json + the run_corpus additional sections +
       the tests/genus_gates.rs cargo mirror + §20.6 (the landed state)
       + the README step 7 + the AGENTS.md fifth-layer contract +
       this halt record
c21fdff fix(harness): the genus-gate review pass - the ordering genus
       recomputed as always-after (after − before, not the union that
       pinned variable pairs both ways and hid the points-first rank;
       the pin regenerated, G-A 55 -> 67 rows — false rows out, the
       point-rank rows in) + the solid-history-links-root arm gated +
       the unused-import residue cleaned + the docs reconciled
c617761 fix(dwg): the G-C genus packet - the constructed AcDs container
       at the authored genus (acds_genus_diffs 11 -> 0; ds_version
       16/17, segidx-first at 128, the 91/97-row scales, the authored
       slot allocations + pointers, prvsav + 2013 freesp, the header
       constants, the invented thumbnail dropped; two extractor arms
       extended; two era regression tests; the identity to 4265c04a...)
864450d fix(dwg): the G-A vertex packet - the authored vertex role token
       (0=start, 1=end, 2=closed-edge; the 580-vertex census) —
       add_vertex emits the placeholder, add_edge fills the owning
       records' roles; the vertex row's 28 occurrences gone
       (sab_form 67 -> 66 rows, 521 -> 493 occurrences); the identity
       moved to 0e8b23cd... (25,439 bytes)
021ca52 fix(dwg): the G-A asmheader packet - the authored SAB's opening
       record (asmheader $-1 $-1 "232.6.0.65535", era-uniform 136/136)
       prepended at the SAB-write boundary with the +1 wire-pointer
       shift (the DXF-SAT body-at-0 convention preserved); five SAB
       module tests updated to the shifted indices; the asmheader
       row's 11 occurrences gone (sab_form 66 -> 65 rows,
       493 -> 482 occurrences); the identity moved to a8f227e0...
       (25,439 bytes)
590209d docs(harness): the post-queue-session halt refresh - three
       packets landed (G-C, vertex, asmheader), the queue at 65/1/0
b9cdd83 fix(dwg): the G-A tolerance packet - the authored 1.0
       spatial_resolution (sab_form 65 -> 64 rows) — the SatHeader::new()
       default fixed AND the strip_for_sab normalize moved before the
       nothing-to-strip early return so it always runs; the tolerance
       row's 11 occurrences gone (sab_form 65 -> 64 rows,
       482 -> 471 occurrences); the identity moved to a7c5f170...
       (25,439 bytes)
a8fe585 fix(harness): the TOLERATED adjudication state - the genus
       queue and its recorded verdicts (the pending queue 61/0/0) —
       the ADJUDICATIONS table in genus_gates.py (recorded
       strict-loader verdicts with provenance; a listed row keeps its
       count as the recorded state, gains status/verdict annotations +
       a markdown status column and adjudication block, and falls out
       of the new pending_* counts; --strict now asserts zero PENDING
       rows) — the ACIS-700 family (header-magic-version,
       header-triple, product-strings; the 2026-09-21 strict-load
       zero + the 2026-09-22 region probe + the author's identity
       rule) and G-B's constructed-tree elide marker (the 2646f05
       cylinder verdict) adjudicated; the primary counts stay 64/1/0,
       the pending queue 61/0/0
cfe36f6 fix(harness): the persubent adjudication - the journal-
       correlated selection bias (G-A pending 61 -> 60 rows) — the
       2026-09-29 corpus scan (72 walked SAB carriers outside the
       tree-selected fixture family): the attrib rides the doc's
       journal (example_2004's journaled solid + Region carry it,
       its plain Regions do not; ATMOS's 11 exceptions are the known
       broken map); the constructed primitives are journal-less at
       save — emitting the marker would forge journal presence; the
       row adjudicated TOLERATED with the evidence recorded
2ebc90f fix(dwg): the G-A ordering packet - the authored
       first-appearance genus rank (sab_form 64 -> 4 rows, the
       pending queue 0) — the rank table rewritten to a verified
       linearization of the pinned order_constraints (body, lump,
       transform, shell, face, loop, cone, sphere, torus, plane,
       coedge, edge, vertex, ellipse, straight, point) + the
       write-site normalization widened to every SAT-text->SAB
       conversion (authored-shaped docs are genus-ordered: the
       stable sort is the identity on them); one SAB test moved to
       the new convention; the 60-row ordering family fell —
       pending 0/0/0, --strict passes; the identity moved to
       e4cd1960... (25,473 bytes); the battery green end-to-end;
       OPEN: the user-run BricsCAD audit of the re-ranked stream
       (a failed audit REVERSES the rank change)
c760014 docs(harness): the halt refresh - the queue PENDING-ZERO
       (sab_form 4 / sh_genus 1 / acds_genus 0, all rows
       adjudicated), the one open verification the user-run
       BricsCAD audit
<feat> the strict-loader probe MECHANIZED + the restore-gap
       finding: tests/gold_harness/strict_load_probe.py (the
       /b-scripted headless BricsCAD census - the LISP probes in
       the post-OPEN document context, vla-getboundingbox forces
       the modeler, real extents vs the +-1e80 null box, every
       launch bounded, a missing result = AMBIGUOUS) + the paired
       verdicts recorded (authored specimens model real extents in
       all three envelopes 2007/2013/2018; every constructed fixture
       the null box; the old-vs-new discrimination pre-existing
       and order-independent - the ordering packet EXONERATED, the
       revert rule's causation condition not engaged) + the new
       queue head: the constructed-SAB restore gap (the stream has
       never constructed as an ACIS body in the strict kernel; the
       record-level SAB diff is the designed next packet; the probe
       is the acceptance gate)
       (THIS SESSION'S PROBE LANDING)
```

**PUSH STATE (2026-09-29)**: push after each landing per the
maintainer's loop instruction (`git push origin gold-vs-silver`).

**Session arc, for context**: the 2026-09-28 session — the
continuation from the documentation halt → §20 read in full + the
extraction surface located (the SAB walker facts pinned by live
probing: the raw-length type tags, the ASM header layout, the
optional resfit) → the specimens decoded and the genus measured (136
SAB carriers, 136 SH roots, 78 jard containers, zero anomalies) →
the constructed-fixture family built (`genus_constructed`:
Box/Sphere/Cylinder/Cone/Torus/Region/Body/HistoryTree, the elide
verified live) → the extractor + gates + pin landed → the
ordering-check inversion caught and fixed on the first report → the
corpus re-run at 280/0/0/0 with the sections attached → the identity
+ echo + suites green → the halt record written → **the review pass
(the OCS lens)**: the ordering genus's union defect found by probing
the pin's constraint symmetry (variable pairs pinned both ways =
false positives; `point` dropped from every constraint set = the
writer's points-first rank invisible), recomputed as after−before,
the pin regenerated, the ungated solid-history-links-root invariant
armed, the corrected queue verified, the docs reconciled → **the
first three queue packets** (G-C the era-profiled AcDs container: 11
rows → 0; the vertex role token: 28 occurrences gone; the asmheader:
11 gone) — the queue left at 65/1/0. The 2026-09-29 session (the
maintainer's "continue implementation, repeat until fully
implemented" directive, continued): **the fourth queue packet — the
tolerance triple** — landed from the halt's own designed next step:
the SatHeader default 10.0 → the authored 1.0 plus the
strip_for_sab normalize hoisted before the nothing-to-strip early
return (the dead pass that never ran on clean primitive documents);
the row's 11 occurrences gone (sab_form 65 → 64 rows, 482 → 471),
the identity re-recorded at the intended move
(`a7c5f170…`, 25,439 bytes), the suite green (52 segments), the
genus mirror green (fresh extraction == the untouched pin), the four
family smokes 0/0 with clean echoes, the corpus re-verified at
280/0/0/0 with the sections at 64/1/0 → **the fifth queue packet —
the TOLERATED adjudication state**: §20.3's "close as TOLERATED with
the row kept as the recorded state" gained its instrument (the
ADJUDICATIONS table: recorded verdicts with provenance; the listed
rows keep their counts, gain status annotations, and fall out of the
pending_* counts; --strict asserts zero pending) and its first four
adjudications — the ACIS-700 family (header-magic-version,
header-triple, product-strings: the 2026-09-21 strict-load zero, the
2026-09-22 region probe, and the author's identity rule) and G-B's
elide marker (the cylinder verdict) — the primary counts stay
64/1/0, the pending queue now 61/0/0, the mirror green. **The
maintainer's loop instruction — "repeat process until target =
zero" — remains satisfied on its own terms: the corpus is at zero
on every axis, the conventional arm is record-identical everywhere
surveyed, and the fifth layer's queue is MEASURED, REVIEWED, and
WORKED DOWN — five packets closed across the two sessions
(G-C, the vertex role token, the asmheader record, the tolerance
triple, the adjudication state). The 2026-09-29 continuation (the
"review, update, commit, push, then continue implementation …
repeat process until fully implemented" directive): the review pass
re-verified the docs' consistency and the tree state → **the sixth
packet** — the persubent adjudication, from a live corpus probe
(72 walked carriers: the attrib is journal-correlated; the fixture
uniformity is the tree selection; emitting it on journal-less
primitives would forge journal presence) → **the seventh packet** —
the ordering genus normalization: the source map corrected (the
rows came from the BUILDERS' assembly order, not the reorder the
trigger never fired), the rank table rewritten to a verified
linearization of the pinned order_constraints, and the write-site
normalization widened to every SAT-text→SAB conversion (authored-
shaped documents sort to themselves; body keeps rank 0 per the
2026-09-22 contract) — the 60 ordering rows fell, the PENDING queue
is 0/0/0, `--strict` passes, the identity moved to
`e4cd1960…`/25,473, and the full battery held green end-to-end
(suite + mirrors + smokes + echoes + corpus 280 at 0/0/0/0) → the
halt, with the one open item the strict-loader audit → the second
continuation (the "review, update, commit, push, then continue"
loop): the audit MECHANIZED (strict_load_probe.py — the /b-scripted
headless BricsCAD census) and its verdict recorded through the
paired probes: authored specimens model real extents in all three
envelopes, every constructed fixture yields the ±1e80 null box,
and the old-vs-new discrimination shows the failure PRE-EXISTING
and order-independent — **the ordering packet exonerated (the revert
rule's causation condition does not engage), and the new queue
head is the constructed-SAB restore gap: the stream has never
constructed as an ACIS body in the strict kernel, invisible to
every corpus axis and every genus invariant — the record-level
SAB diff is the designed next packet, with the probe as the
acceptance gate.**
