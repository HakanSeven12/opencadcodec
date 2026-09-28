//! Phase B blob-autopsy tests: the raw-retained solid-history node
//! tails decode into the typed views, the decode is identical across all
//! four DWG versions of each family (the tails are bit-identical, the
//! Phase A cross-version anchor), and the Phase B write rule holds:
//! untouched records re-emit verbatim while a decoded-field edit lands
//! bit-locally in its raw span (same tail and record length).
//!
//! Specimen provenance: the fixture DWGs authored in the 2026-09-23
//! campaign (Extrude/Polysolid/Loft/Revolve, DWG 2007-2018; see the
//! .txt companions and IMPLEMENTATION.md F2.3). The embedded tails are
//! the captured post-`op.minor` payloads; gold gives no oracle walk for
//! these classes (DEBUGGING_CLASS), so the pinned semantics come from
//! the cross-specimen + live-oracle instruments documented there.

use acadrust::entities::{solid3d::Solid3D, EntityType};
use acadrust::io::dwg::sh_tail_decode::{
    loft_tail_view, revolve_tail_view, sweep_tail_view,
};
use acadrust::objects::{
    SolidHistoryLoft, SolidHistoryLoftSection, SolidHistoryLoftTail, SolidHistoryNodeBase,
    SolidHistoryOperation, SolidHistoryRevolve, SolidHistoryRevolveTail, SolidHistorySweep,
    SolidHistorySweepTail,
};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

/// Mark the constructed SH tree of `entity` as byte-captured content.
///
/// The Phase B tests verify the re-emit rule on records whose tails are
/// verbatim specimen spans (the unhex'ed captured payloads) — the
/// save-guard reserves the DWG write path for byte-captured genus
/// (constructed trees elide by default; the 2026-09-28 cylinder
/// verdict: the factory genus audits as "Duplicate ownership of
/// reference"). Marking keeps these write-path assertions exercising
/// the re-emit pipeline.
fn mark_tree_captured(document: &mut CadDocument, entity: acadrust::types::Handle) {
    let graph = document.solid_history_graph(entity).expect("tree exists");
    let mut handles = graph.nodes.clone();
    handles.push(graph.root);
    for handle in handles {
        if let Some(acadrust::objects::ObjectType::DynamicBlock(value)) =
            document.objects.get_mut(&handle)
        {
            value.captured = true;
        }
    }
}

fn unhex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
        .collect()
}

const POLYSOLID_SWEEP_HEX: &str = "AAA9A812056ACE738225AB193DB3F8346875DD7FE0ECBF8346875DD7FE0ECBF8E738225AB193DBBFA6AA4E738225AB193DB3F8346875DD7FE0ECBF8346875DD7FE0ECBF8E738225AB193DBBFA6AA54D2A008000001020D1A1D775FF83B2FCE738225AB193DBBF9040000000000000440000000000000000000000000000004C0000000000000000000000000000004C0000000000000004000000000000004400000000000000040314D160040001040000000000000510102000000000000000000000000000000005C5B3A1004F2A740684B9EAA0BDE964000";
const POLYSOLID_SWEEP_BITS: u32 = 1738;

// The §18.7 PolysolidX/W/L differential quads (AC2018 twins of the
// fixture set; bit-identical across the 2007/2010/2013/2018 versions).
// X: profile 3 wide / 5 high, path to (12, 0) with drag residue
// 12.00001; W: profile 7/3, clean 12.0; L: profile 3/5, path to (5, 0).
const POLYSOLID_SWEEP_X_HEX: &str = "AAA9A812056AE8000000000000F0BF8000000000000F0BFAA6AA68000000000000F0BF8000000000000F0BFAA6AA54D1A008000001020000000000003C2FE904000000000000F83F0000000000000000000000000000F8BF0000000000000000000000000000F8BF0000000000001440000000000000F83F0000000000001440314D160040001040000000000000210102000000000000000000000000000000008E588B4F01002840000000000000000000";
const POLYSOLID_SWEEP_X_BITS: u32 = 1418;

const POLYSOLID_SWEEP_W_HEX: &str = "AAA9A812056AE8000000000000F0BF8000000000000F0BFAA6AA68000000000000F0BF8000000000000F0BFAA6AA54D1A008000001020000000000003C2FE9040000000000000C4000000000000000000000000000000CC000000000000000000000000000000CC000000000000008400000000000000C400000000000000840314D160040001040000000000000710102000000000000000000000000000000000000000000002840000000000000000000";
const POLYSOLID_SWEEP_W_BITS: u32 = 1418;

const POLYSOLID_SWEEP_L_HEX: &str = "AAA9A812056AE8000000000000F0BF8000000000000F0BFAA6AA68000000000000F0BF8000000000000F0BFAA6AA54D1A008000001020000000000003C2FE904000000000000F83F0000000000000000000000000000F8BF0000000000000000000000000000F8BF0000000000001440000000000000F83F0000000000001440314D160040001040000000000000210102000000000000000000000000000000000000000000001440000000000000000000";
const POLYSOLID_SWEEP_L_BITS: u32 = 1418;

const EXTRUDE_SWEEP_HEX: &str = "A00000000000000102A9B2052A9AA6A9AA5AA6A9AA512442A692";
const EXTRUDE_SWEEP_BITS: u32 = 208;
const EXTRUDE_T_HEX: &str = "A0000000000000010065732D3852C1D03FA9B2052A9AA6A9AA5AA6A9AA512442A692";
const EXTRUDE_T_BITS: u32 = 272;

const EXTRUDE_R_HEX: &str = "A00000000000000102A9B2052A9AA6A9AA5AA6A9AA512542A0000000000000250292";
const EXTRUDE_R_BITS: u32 = 272;

const EXTRUDE_P_HEX: &str = "A00000000000000102A9B2052A9A1829074AC411BF8FE61DAB10061A71FD8FE9AA5AA6A9AA54D0800800000002410000000000000000000000000000000000000000000004100000000000000000000000000000041000000000000002100000000000000000000000000000021032";
const EXTRUDE_P_BITS: u32 = 888;


const LOFT_HEX: &str = "442A6911204004000000000000000000400000000000000010000000000000014400CCCCCCCCCCCF4CFE928182D4454FB21F93F060B51153EC87E4FE9D60";
const LOFT_BITS: u32 = 492;

// The §18.7 loft differential fixtures (AC2018 twins; bit-identical
// across 2007-2018). The §18 loft container walk's named-section
// witnesses: Loft3 (the z-only mid sections), LoftR ((5, 2.5)),
// LoftH ((7,) with the radius elided), LoftC (the world-offset
// double section with the z-elided first).
const LOFT3_HEX: &str = "442A691125428000000000000044069112542800000000000014406928182D4454FB21F93F060B51153EC87E4FE9D6";
const LOFT3_BITS: u32 = 376;

const LOFTC_HEX: &str = "7400000000000000840000000000000041020000000000003E0FE91120400400000000000000000840000000000000041000000000000001C400000000000003E0FE928182D4454FB21F93F060B51153EC87E4FE9D60";
const LOFTC_BITS: u32 = 684;

const LOFTH_HEX: &str = "442A6911254280000000000001C406928182D4454FB21F93F060B51153EC87E4FE9D60";
const LOFTH_BITS: u32 = 276;

const LOFTR_HEX: &str = "442A691126428000000000000144000000000000001102928182D4454FB21F93F060B51153EC87E4FE9D60";
const LOFTR_BITS: u32 = 340;

const REVOLVE_HEX: &str = "AA634884CDFDF3644902AA912541A26A666666666724FE90";
const REVOLVE_BITS: u32 = 190;
const REVOLVE_A_HEX: &str = "AA6060B51153EC842502AA9126400000000000000040A26A6666666667A4FE90";
const REVOLVE_A_BITS: u32 = 254;

const REVOLVE_R_HEX: &str = "AA6060B51153EC842502AA9126400000000000000040A0000000000003D0FE90";
const REVOLVE_R_BITS: u32 = 254;



#[test]
fn polysolid_sweep_tail_decodes_the_pinned_semantics() {
    let bytes = unhex(POLYSOLID_SWEEP_HEX);
    let view = sweep_tail_view(&bytes, POLYSOLID_SWEEP_BITS)
        .expect("the polysolid sweep tail must decode");
    // The six named spine slots (the SweepOptions order) + the two
    // all-short sweep extras.
    assert_eq!(view.draft_angle, Some(0.0));
    assert_eq!(view.draft_start_distance, Some(0.0));
    assert_eq!(view.draft_end_distance, Some(0.0));
    assert_eq!(view.twist_angle, Some(0.0));
    assert_eq!(view.scale_factor, Some(1.0));
    assert_eq!(view.align_angle, Some(0.0));
    assert_eq!(view.option_doubles, vec![0.0, 0.0]);
    assert_eq!(view.profile, None);
    // Mid-region raw BD entries: the path unit direction components
    // ([+u_y, -u_x, -u_x, -u_y], twice) — confirmed against the R2010
    // live-oracle anchor geometry (segment end (3065.007936309483,
    // 1463.5113930448078), unit (0.9024047208179184, 0.4308894520007826)).
    assert_eq!(
        view.raw_doubles,
        vec![
            0.4308894520007826,
            -0.9024047208179184,
            -0.9024047208179184,
            -0.4308894520007826,
            0.4308894520007826,
            -0.9024047208179184,
            -0.9024047208179184,
            -0.4308894520007826,
            -0.9024047208179184,
            -0.4308894520007826,
        ]
    );
    // Byte-aligned profile corners: the 5x2 rectangle of the polysolid
    // cross-section ((x, height) pairs; the 3DSOLID anchor z = 1.0 is
    // the pair-height mid).
    assert_eq!(
        view.profile_corners,
        vec![[2.5, 0.0], [-2.5, 0.0], [-2.5, 2.0], [2.5, 2.0]]
    );
    // The record constant (the trailing unpaired corner-block entry) and
    // the post-corner single of the gap (the profile-coupled 32+n/32
    // frame; this fixture's width is 5 → the fraction 5/32).
    assert_eq!(view.record_constant, Some(4.00024414192312));
    assert_eq!(view.post_corner_single, Some(32.15625));
    // Segment end in the record frame; gold's R2010 wireframe anchor
    // reads exactly its half (the live-oracle overlap).
    assert_eq!(
        view.segment_end,
        Some([3065.007936309483, 1463.5113930448078])
    );
}

#[test]
fn polysolid_xwl_quads_decode_the_post_corner_singles() {
    // The §18.7 singles walk, pinned across the X/W/L differentials:
    // the record constant is bit-invariant (third P-probe
    // confirmation); the post-corner single tracks the profile
    // (X/L share the 3x5 profile → 32 + 2/32; W's 7x3 → 32 + 7/32);
    // the segment end is the only path-coupled entry (12.00001 drag
    // residue on X, clean 12.0 on W, 5.0 on L).
    let x = sweep_tail_view(&unhex(POLYSOLID_SWEEP_X_HEX), POLYSOLID_SWEEP_X_BITS)
        .expect("the PolysolidX quad must decode");
    let w = sweep_tail_view(&unhex(POLYSOLID_SWEEP_W_HEX), POLYSOLID_SWEEP_W_BITS)
        .expect("the PolysolidW quad must decode");
    let l = sweep_tail_view(&unhex(POLYSOLID_SWEEP_L_HEX), POLYSOLID_SWEEP_L_BITS)
        .expect("the PolysolidL quad must decode");
    for quad in [&x, &w, &l] {
        assert_eq!(quad.record_constant, Some(4.00024414192312));
    }
    assert_eq!(
        x.profile_corners,
        vec![[1.5, 0.0], [-1.5, 0.0], [-1.5, 5.0], [1.5, 5.0]]
    );
    assert_eq!(
        w.profile_corners,
        vec![[3.5, 0.0], [-3.5, 0.0], [-3.5, 3.0], [3.5, 3.0]]
    );
    // X and L share the profile: identical corner block and post-corner
    // single; the segment-end pair alone carries the path difference.
    assert_eq!(x.profile_corners, l.profile_corners);
    assert_eq!(x.post_corner_single, l.post_corner_single);
    assert_eq!(x.post_corner_single, Some(32.0625));
    assert_eq!(w.post_corner_single, Some(32.21875));
    assert_eq!(x.segment_end, Some([12.00001, 0.0]));
    assert_eq!(w.segment_end, Some([12.0, 0.0]));
    assert_eq!(l.segment_end, Some([5.0, 0.0]));
}

#[test]
fn extrude_tail_decodes_the_pinned_semantics() {
    let view = sweep_tail_view(&unhex(EXTRUDE_SWEEP_HEX), EXTRUDE_SWEEP_BITS)
        .expect("the extrusion tail must decode");
    // The direction is the extrusion length vector (0, 0, 2.0): the
    // circle-extrude fixture stands 2 tall (live-oracle wires z 0..2).
    // `direction` itself is the SolidHistorySweep model field, reused
    // by the writer's render compare.
    // The six named spine slots: all defaults, scale 1.0.
    assert_eq!(view.draft_angle, Some(0.0));
    assert_eq!(view.draft_start_distance, Some(0.0));
    assert_eq!(view.draft_end_distance, Some(0.0));
    assert_eq!(view.twist_angle, Some(0.0));
    assert_eq!(view.scale_factor, Some(1.0));
    assert_eq!(view.align_angle, Some(0.0));
    assert!(view.option_doubles.is_empty());
    // The embedded profile circle (the S18.7 CALL grammar): the
    // fixture's circle is center (0, 0, 0) radius 1.0 — the radius
    // exactly 1.0 takes the two-bit short BD — with the plan normal
    // (0, 0, 1); the CALL bit-length 16 covers the body + 2 flag bits.
    let call = view.profile.as_ref().expect("the profile CALL decodes");
    assert_eq!(call.kind, 18);
    assert_eq!(call.bit_len, 16);
    let circle = call.circle.as_ref().expect("the circle body decodes");
    assert_eq!(circle.center, [0.0, 0.0, 0.0]);
    assert_eq!(circle.radius, 1.0);
    assert_eq!(circle.normal, [0.0, 0.0, 1.0]);
    // The extrusion tail has no frame entries, profile corners or
    // segment end.
    assert!(view.raw_doubles.is_empty());
    assert!(view.profile_corners.is_empty());
    assert_eq!(view.segment_end, None);
}

#[test]
fn loft_tail_decodes_the_pinned_semantics() {
    let view = loft_tail_view(&unhex(LOFT_HEX), LOFT_BITS)
        .expect("the loft tail must decode");
    assert_eq!(view.option_doubles, vec![1.0]);
    // Raw entries in stream order: the loft top height 5.0 (the wires
    // run z 0..5) and the two 90-degree draft angles (pi/2).
    assert_eq!(
        view.raw_doubles,
        vec![
            2.0,
            2.0,
            5.0,
            0.3,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::FRAC_PI_2,
        ]
    );
}

#[test]
fn loft_fixtures_walk_the_named_sections() {
    // The §18 loft container walk: the raw frame stream's contiguous
    // groups attribute to the closed per-section reading
    // [center.x][center.y][height][radius] — the canonical elisions
    // (a 0.0 center component or height, a 1.0 radius) land as 2-bit
    // shorts between the frames, `None` in the model — and the final
    // 2-frame group names the trailing draft-angle pair. The leading
    // region (an origin section's shorts plus per-record state) and
    // the inter-section gaps stay documented-verbatim: the frames'
    // named fields never claim their bits.
    let section = |cx, cy, h, r| SolidHistoryLoftSection {
        center: [cx, cy],
        height: h,
        radius: r,
    };
    let pair = [
        Some(std::f64::consts::FRAC_PI_2),
        Some(std::f64::consts::FRAC_PI_2),
    ];

    // Loft_ (the landed original): the bottom origin section contributes
    // no frames; the top section carries all four fields raw.
    let view = loft_tail_view(&unhex(LOFT_HEX), LOFT_BITS)
        .expect("the loft tail must decode");
    assert_eq!(
        view.sections,
        vec![section(Some(2.0), Some(2.0), Some(5.0), Some(0.3))]
    );
    assert_eq!(view.draft_angles, pair);

    // LoftR: (0, 0, 5, 2.5) — the center elided in the leading region,
    // [z][r] as raws.
    let vr = loft_tail_view(&unhex(LOFTR_HEX), LOFTR_BITS)
        .expect("the LoftR tail must decode");
    assert_eq!(
        vr.sections,
        vec![section(None, None, Some(5.0), Some(2.5))]
    );
    assert_eq!(vr.draft_angles, pair);

    // LoftH: (0, 0, 7, 1) — only the height frame survives elision.
    let vh = loft_tail_view(&unhex(LOFTH_HEX), LOFTH_BITS)
        .expect("the LoftH tail must decode");
    assert_eq!(vh.sections, vec![section(None, None, Some(7.0), None)]);
    assert_eq!(vh.draft_angles, pair);

    // Loft3: the two mid sections at z 2.5 and 5 (both origin-centered,
    // r = 1 elided) walk as [z] singles.
    let v3 = loft_tail_view(&unhex(LOFT3_HEX), LOFT3_BITS)
        .expect("the Loft3 tail must decode");
    assert_eq!(
        v3.sections,
        vec![
            section(None, None, Some(2.5), None),
            section(None, None, Some(5.0), None),
        ]
    );
    assert_eq!(v3.draft_angles, pair);

    // LoftC — the decisive world differential: section 1
    // (3, 4, z 0 elided, 1.5), section 2 (3, 4, 7, 1.5), all four
    // fields raw in section 2.
    let vc = loft_tail_view(&unhex(LOFTC_HEX), LOFTC_BITS)
        .expect("the LoftC tail must decode");
    assert_eq!(
        vc.sections,
        vec![
            section(Some(3.0), Some(4.0), None, Some(1.5)),
            section(Some(3.0), Some(4.0), Some(7.0), Some(1.5)),
        ]
    );
    assert_eq!(vc.draft_angles, pair);
}

#[test]
fn loft_named_section_edits_land_bit_locally() {
    // The §18 container walk's write rule: a programmatic edit through
    // the NAMED fields (not the positional raw run) splices only the
    // edited frames — the radius frame of LOFT_HEX (value bits
    // [270..334), bytes [33..41]) and the first draft frame
    // (value bits [348..412), bytes [43..51]) — nothing else moves and
    // the tail keeps its length.
    let loft_tail = unhex(LOFT_HEX);
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(
            entity,
            SolidHistoryOperation::Loft(SolidHistoryLoft {
                base: SolidHistoryNodeBase::new(1),
                raw_tail: loft_tail.clone(),
                raw_tail_bit_len: LOFT_BITS,
                tail_decode: loft_tail_view(&loft_tail, LOFT_BITS),
                ..SolidHistoryLoft::default()
            }),
        )
        .unwrap();
    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Loft(loft) = &mut replacement else {
        panic!("expected a loft node");
    };
    let section = loft
        .tail_decode
        .get_or_insert_with(SolidHistoryLoftTail::default);
    section.sections[0].radius = Some(1.25);
    section.draft_angles[0] = Some(0.75);
    document.update_solid_history_step(entity, replacement).unwrap();

    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Loft(loft) =
        roundtrip.solid_history_operations(entity).unwrap()[0].clone()
    else {
        panic!("expected a loft node");
    };
    assert_eq!(loft.raw_tail.len(), loft_tail.len());
    assert_eq!(loft.raw_tail_bit_len, LOFT_BITS);
    let differing: Vec<usize> = loft
        .raw_tail
        .iter()
        .zip(loft_tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty(), "the edits must land somewhere");
    for index in &differing {
        let in_radius = (270 / 8 <= *index) && (*index <= 334 / 8);
        let in_draft = (348 / 8 <= *index) && (*index <= 412 / 8);
        assert!(
            in_radius || in_draft,
            "edits must stay inside the radius + first-draft frames, got {differing:?}"
        );
    }
    let view = loft.tail_decode.as_ref().unwrap();
    assert_eq!(view.sections[0].radius, Some(1.25));
    assert_eq!(view.draft_angles[0], Some(0.75));
    assert_eq!(view.sections[0].center, [Some(2.0), Some(2.0)]);
    assert_eq!(view.sections[0].height, Some(5.0));
    assert_eq!(view.draft_angles[1], Some(std::f64::consts::FRAC_PI_2));
}

#[test]
fn revolve_tail_decodes_the_pinned_semantics() {
    let view = revolve_tail_view(&unhex(REVOLVE_HEX), REVOLVE_BITS)
        .expect("the revolve tail must decode");
    // The head is the AXIS PAIR (the REVOLVEDSURFACE twin's order):
    // axis_point then axis_vector - every landed specimen revolves
    // about the +Y axis through the origin.
    assert_eq!(view.axis_point, Some([0.0, 0.0, 0.0]));
    assert_eq!(view.axis_vector, Some([0.0, 1.0, 0.0]));
    // The revolve sweep angle: the original is 3*pi/2 (270 degrees);
    // the typed A/R quads read pi - two independent values.
    assert_eq!(view.revolve_angle, Some(3.0 * std::f64::consts::FRAC_PI_2));
    // The all-zero option run between the angle and the profile CALL.
    assert_eq!(view.option_doubles, vec![0.0; 6]);
    // The embedded profile circle (CALL: [BL 18 = OBJ_CIRCLE]
    // [BL 80][circle]): the original's circle is centered (1.0, 0, 0)
    // - the x a two-bit SHORT form (BD '01' encodes exactly 1.0) -
    // with radius 0.2 (the raw BD the earlier raw-scan mislabeled as
    // "the 0.2 entry") and the plan normal (0, 0, 1). This is the
    // as-drawn profile: the wire regression's torus M 1.0 / m 0.2.
    assert_eq!(view.profile_center, Some([1.0, 0.0, 0.0]));
    assert_eq!(view.profile_radius, Some(0.2));
    assert_eq!(view.profile_normal, Some([0.0, 0.0, 1.0]));
}

#[test]
fn revolve_matrix_stems_decode_the_structural_profile() {
    // The §18.7 RevolveA/R quads (torus M 2.0, m 0.8/1.25 about +Y,
    // wire-ladder-verified): the same CALL grammar with the circle
    // center's x a raw 2.0 (not short-encodable) and the radius raw.
    let view = revolve_tail_view(&unhex(REVOLVE_A_HEX), REVOLVE_A_BITS)
        .expect("the RevolveA tail must decode");
    assert_eq!(view.axis_point, Some([0.0, 0.0, 0.0]));
    assert_eq!(view.axis_vector, Some([0.0, 1.0, 0.0]));
    assert_eq!(view.revolve_angle, Some(std::f64::consts::PI));
    assert_eq!(view.option_doubles, vec![0.0; 6]);
    assert_eq!(view.profile_center, Some([2.0, 0.0, 0.0]));
    assert_eq!(view.profile_radius, Some(0.8));
    assert_eq!(view.profile_normal, Some([0.0, 0.0, 1.0]));

    let view = revolve_tail_view(&unhex(REVOLVE_R_HEX), REVOLVE_R_BITS)
        .expect("the RevolveR tail must decode");
    assert_eq!(view.revolve_angle, Some(std::f64::consts::PI));
    assert_eq!(view.profile_center, Some([2.0, 0.0, 0.0]));
    // The radius-only variable: R's 1.25 lands at A's radius span.
    assert_eq!(view.profile_radius, Some(1.25));
    assert_eq!(view.profile_normal, Some([0.0, 0.0, 1.0]));
}

#[test]
fn revolve_profile_edits_land_bit_locally() {
    // The CALL fields are splice-backed: editing the profile radius
    // on the RevolveA tail lands inside its raw value span only
    // (bits [182..246), bytes 22..=30).
    let tail = unhex(REVOLVE_A_HEX);
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(
            entity,
            SolidHistoryOperation::Revolve(SolidHistoryRevolve {
                base: SolidHistoryNodeBase::new(1),
                raw_tail: tail.clone(),
                raw_tail_bit_len: REVOLVE_A_BITS,
                tail_decode: revolve_tail_view(&tail, REVOLVE_A_BITS),
                ..SolidHistoryRevolve::default()
            }),
        )
        .unwrap();
    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Revolve(revolve) = &mut replacement else {
        panic!("expected a revolve node");
    };
    revolve
        .tail_decode
        .get_or_insert_with(SolidHistoryRevolveTail::default)
        .profile_radius = Some(0.75);
    document.update_solid_history_step(entity, replacement).unwrap();

    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Revolve(revolve) =
        &roundtrip.solid_history_operations(entity).unwrap()[0]
    else {
        panic!("expected a revolve node");
    };
    let view = revolve.tail_decode.as_ref().unwrap();
    assert_eq!(view.profile_radius, Some(0.75), "the radius edit re-reads");
    // Every grammar neighbor is untouched.
    assert_eq!(view.axis_point, Some([0.0, 0.0, 0.0]));
    assert_eq!(view.axis_vector, Some([0.0, 1.0, 0.0]));
    assert_eq!(view.revolve_angle, Some(std::f64::consts::PI));
    assert_eq!(view.profile_center, Some([2.0, 0.0, 0.0]));
    assert_eq!(view.profile_normal, Some([0.0, 0.0, 1.0]));
    assert_eq!(revolve.raw_tail.len(), tail.len());
    let differing: Vec<usize> = revolve
        .raw_tail
        .iter()
        .zip(tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty());
    assert!(
        differing.iter().all(|index| (22..31).contains(index)),
        "radius edits must stay inside the radius span (bytes 22..=30), \
         got {differing:?}"
    );
}

#[test]
fn malformed_and_empty_tails_do_not_decode() {
    // No tail, truncated seed, or a head that never forms — all keep
    // the verbatim-only behavior (None view, no typed claims).
    assert!(sweep_tail_view(&[], 0).is_none());
    assert!(sweep_tail_view(&[0b10101010], 4).is_none());
    assert!(loft_tail_view(&[], 0).is_none());
    assert!(revolve_tail_view(&[], 0).is_none());
    // A reserved pair at the direction head cannot anchor a sweep view.
    assert!(sweep_tail_view(&[0b11000000], 8).is_none());
}

fn sweep_op(tail: &[u8], bit_len: u32) -> SolidHistoryOperation {
    SolidHistoryOperation::Sweep(SolidHistorySweep {
        base: SolidHistoryNodeBase::new(1),
        shsw_raw_tail: tail.to_vec(),
        shsw_raw_tail_bit_len: bit_len,
        ..SolidHistorySweep::default()
    })
}

#[test]
fn raw_tails_round_trip_verbatim_and_decoded() {
    // Full-stack: a DWG-written sweep op with the captured polysolid
    // tail re-reads bit-identically (Phase A rule) and re-derives the
    // Phase B typed view (semantic field parity throughout the stack).
    let tail = unhex(POLYSOLID_SWEEP_HEX);
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(entity, sweep_op(&tail, POLYSOLID_SWEEP_BITS))
        .unwrap();
    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let ops = roundtrip.solid_history_operations(entity).unwrap();
    let SolidHistoryOperation::Sweep(sweep) = ops.first().expect("the sweep node survives") else {
        panic!("expected a sweep operation node");
    };
    assert_eq!(sweep.shsw_raw_tail, tail);
    assert_eq!(sweep.shsw_raw_tail_bit_len, POLYSOLID_SWEEP_BITS);
    let view = sweep.tail_decode.as_ref().expect("typed view re-decodes");
    assert_eq!(
        view.segment_end,
        Some([3065.007936309483, 1463.5113930448078])
    );
    assert_eq!(view.profile_corners.len(), 4);
    assert_eq!(view.raw_doubles.len(), 10);
    assert_eq!(view.scale_factor, Some(1.0));
    assert_eq!(view.option_doubles, vec![0.0, 0.0]);
}

#[test]
fn edited_segment_end_lands_bit_locally() {
    // Write rule: a programmatic edit of a decoded field re-encodes ONLY
    // its raw span — every other byte of the tail (and thus the record)
    // stays identical, so the re-encoded record keeps its size.
    let tail = unhex(POLYSOLID_SWEEP_HEX);
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(entity, sweep_op(&tail, POLYSOLID_SWEEP_BITS))
        .unwrap();

    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Sweep(sweep) = &mut replacement else {
        panic!("expected a sweep operation node");
    };
    let view = sweep.tail_decode.get_or_insert_with(SolidHistorySweepTail::default);
    view.segment_end = Some([1000.0, 2000.0]);
    document.update_solid_history_step(entity, replacement).unwrap();

    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Sweep(sweep) =
        roundtrip.solid_history_operations(entity).unwrap()[0].clone()
    else {
        panic!("expected a sweep operation node");
    };
    assert_eq!(sweep.shsw_raw_tail.len(), tail.len());
    assert_eq!(sweep.shsw_raw_tail_bit_len, POLYSOLID_SWEEP_BITS);
    // The decoded edit is visible through the reader...
    assert_eq!(
        sweep.tail_decode.as_ref().unwrap().segment_end,
        Some([1000.0, 2000.0])
    );
    // ...and bit-locally: the changed bits lie inside the segment-end
    // span (the final 16 bytes of the tail), nothing else moves.
    let differing: Vec<usize> = sweep
        .shsw_raw_tail
        .iter()
        .zip(tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty(), "the edit must land somewhere");
    assert!(
        differing.iter().all(|index| *index >= 201 && *index <= 216),
        "edits must stay inside the segment-end span, got {differing:?}"
    );
    // The other decoded anchors keep their values (splices are local).
    let view = sweep.tail_decode.as_ref().unwrap();
    assert_eq!(view.profile_corners.len(), 4);
    assert_eq!(view.raw_doubles.len(), 10);
}

#[test]
fn edited_post_corner_singles_land_bit_locally() {
    // The post-corner singles walk (§18.7): a programmatic edit of the
    // record constant or the post-corner single re-encodes only its own
    // 64 value bits; the tail keeps its size and every other byte.
    let tail = unhex(POLYSOLID_SWEEP_X_HEX);
    let original = sweep_tail_view(&tail, POLYSOLID_SWEEP_X_BITS)
        .expect("the PolysolidX quad must decode");
    assert_eq!(original.record_constant, Some(4.00024414192312));
    assert_eq!(original.post_corner_single, Some(32.0625));

    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(entity, sweep_op(&tail, POLYSOLID_SWEEP_X_BITS))
        .unwrap();
    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Sweep(sweep) = &mut replacement else {
        panic!("expected a sweep operation node");
    };
    let view = sweep.tail_decode.get_or_insert_with(SolidHistorySweepTail::default);
    view.record_constant = Some(4.25);
    view.post_corner_single = Some(40.5);
    document.update_solid_history_step(entity, replacement).unwrap();

    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Sweep(sweep) =
        roundtrip.solid_history_operations(entity).unwrap()[0].clone()
    else {
        panic!("expected a sweep operation node");
    };
    assert_eq!(sweep.shsw_raw_tail.len(), tail.len());
    assert_eq!(sweep.shsw_raw_tail_bit_len, POLYSOLID_SWEEP_X_BITS);
    let view = sweep.tail_decode.as_ref().unwrap();
    assert_eq!(view.record_constant, Some(4.25));
    assert_eq!(view.post_corner_single, Some(40.5));
    // Un-edited anchors keep their bits (splices are local): the corners,
    // the record-constant neighbors, and the segment end.
    assert_eq!(
        view.profile_corners,
        vec![[1.5, 0.0], [-1.5, 0.0], [-1.5, 5.0], [1.5, 5.0]]
    );
    assert_eq!(view.segment_end, Some([12.00001, 0.0]));
}

#[test]
fn loft_and_revolve_tails_round_trip_and_edits_land() {
    // Same full-stack rule for the other two families: verbatim survival
    // plus local re-encode of the edited raw spans.
    let loft_tail = unhex(LOFT_HEX);
    let revolve_tail = unhex(REVOLVE_HEX);

    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    // A loft node and a revolve node on the same solid history chain.
    document
        .create_solid_history(
            entity,
            SolidHistoryOperation::Loft(SolidHistoryLoft {
                base: SolidHistoryNodeBase::new(1),
                raw_tail: loft_tail.clone(),
                raw_tail_bit_len: LOFT_BITS,
                tail_decode: loft_tail_view(&loft_tail, LOFT_BITS),
                ..SolidHistoryLoft::default()
            }),
        )
        .unwrap();
    document
        .append_solid_history(
            entity,
            SolidHistoryOperation::Revolve(SolidHistoryRevolve {
                base: SolidHistoryNodeBase::new(2),
                raw_tail: revolve_tail.clone(),
                raw_tail_bit_len: REVOLVE_BITS,
                tail_decode: revolve_tail_view(&revolve_tail, REVOLVE_BITS),
                ..SolidHistoryRevolve::default()
            }),
        )
        .unwrap();

    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let ops = roundtrip.solid_history_operations(entity).unwrap();
    assert_eq!(ops.len(), 2);
    let SolidHistoryOperation::Loft(loft) = &ops[0] else {
        panic!("expected a loft node");
    };
    let SolidHistoryOperation::Revolve(revolve) = &ops[1] else {
        panic!("expected a revolve node");
    };
    assert_eq!(loft.raw_tail, loft_tail);
    assert_eq!(revolve.raw_tail, revolve_tail);
    assert_eq!(
        loft.tail_decode.as_ref().unwrap().raw_doubles,
        vec![2.0, 2.0, 5.0, 0.3, std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2]
    );
    assert_eq!(
        revolve.tail_decode.as_ref().unwrap().revolve_angle,
        Some(3.0 * std::f64::consts::FRAC_PI_2)
    );

    // Edit the revolve sweep angle: the splice must only touch the 8
    // bytes of its raw span (bits [14..78) -> bytes [1..10) of the tail).
    let mut replacement = ops[1].clone();
    let SolidHistoryOperation::Revolve(revolve) = &mut replacement else {
        panic!("expected a revolve node");
    };
    let view = revolve
        .tail_decode
        .get_or_insert_with(SolidHistoryRevolveTail::default);
    view.revolve_angle = Some(std::f64::consts::PI);
    document.update_solid_history_step(entity, replacement).unwrap();
    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Revolve(revolve) =
        &roundtrip.solid_history_operations(entity).unwrap()[1]
    else {
        panic!("expected a revolve node");
    };
    assert_eq!(revolve.raw_tail.len(), revolve_tail.len());
    assert_eq!(
        revolve.tail_decode.as_ref().unwrap().revolve_angle,
        Some(std::f64::consts::PI)
    );
    let differing: Vec<usize> = revolve
        .raw_tail
        .iter()
        .zip(revolve_tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty());
    assert!(
        differing.iter().all(|index| (1..10).contains(index)),
        "revolve-angle edits must stay inside the angle span, got {differing:?}"
    );

    // Edit a loft raw entry (the 5.0 top height): splice lands inside
    // its 8-byte raw span only.
    let mut replacement = roundtrip.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Loft(loft) = &mut replacement else {
        panic!("expected a loft node");
    };
    let view = loft
        .tail_decode
        .get_or_insert_with(SolidHistoryLoftTail::default);
    view.raw_doubles = loft_tail_view(&loft_tail, LOFT_BITS)
        .unwrap()
        .raw_doubles;
    view.raw_doubles[2] = 6.0;
    document.update_solid_history_step(entity, replacement).unwrap();
    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Loft(loft) =
        &roundtrip.solid_history_operations(entity).unwrap()[0]
    else {
        panic!("expected a loft node");
    };
    assert_eq!(loft.raw_tail.len(), loft_tail.len());
    assert_eq!(loft.tail_decode.as_ref().unwrap().raw_doubles[2], 6.0);
    let differing: Vec<usize> = loft
        .raw_tail
        .iter()
        .zip(loft_tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty());
    // The 5.0 entry's raw span: value bits [204..268) -> bytes [25..34].
    assert!(
        differing.iter().all(|index| (25..34).contains(index)),
        "loft edits must stay inside the 5.0 entry span, got {differing:?}"
    );
}

#[test]
fn undecodable_tails_stay_verbatim_never_modeled() {
    // Review regression: a captured tail whose layout does NOT decode
    // (the reserved pair heads this sample) must keep the Phase A
    // behavior — verbatim re-emission — and NEVER fall through to the
    // modeled fallback, which would replace the captured bits with the
    // guess sequence and corrupt the record.
    let opaque: Vec<u8> = vec![0b1100_0000, 0b0101_1010, 0x33, 0x55, 0x11, 0x7F];
    let opaque_bits: u32 = 48;
    assert!(sweep_tail_view(&opaque, opaque_bits).is_none());

    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(
            entity,
            SolidHistoryOperation::Sweep(SolidHistorySweep {
                base: SolidHistoryNodeBase::new(1),
                shsw_raw_tail: opaque.clone(),
                shsw_raw_tail_bit_len: opaque_bits,
                ..SolidHistorySweep::default()
            }),
        )
        .unwrap();
    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Sweep(sweep) =
        &roundtrip.solid_history_operations(entity).unwrap()[0]
    else {
        panic!("expected a sweep operation node");
    };
    assert_eq!(sweep.shsw_raw_tail, opaque, "the captured bits survive verbatim");
    assert_eq!(sweep.shsw_raw_tail_bit_len, opaque_bits);
    assert!(sweep.tail_decode.is_none(), "no typed view is claimed");
}

#[test]
fn extrude_t_names_the_sweep_option_spine() {
    // The §18.7 ExtrudeT quad (typed 15-degree taper): the draft lands
    // as a raw BD in the FIRST spine slot — the SweepOptions order
    // [draft_angle][draft_start_distance][draft_end_distance]
    // [twist_angle][scale_factor][align_angle] — and the profile
    // circle is the untouched r 1.0 default.
    let view = sweep_tail_view(&unhex(EXTRUDE_T_HEX), EXTRUDE_T_BITS)
        .expect("the ExtrudeT tail must decode");
    assert_eq!(view.draft_angle, Some(0.2617993877991494)); // 15 deg
    assert_eq!(view.draft_start_distance, Some(0.0));
    assert_eq!(view.draft_end_distance, Some(0.0));
    assert_eq!(view.twist_angle, Some(0.0));
    assert_eq!(view.scale_factor, Some(1.0));
    assert_eq!(view.align_angle, Some(0.0));
    let call = view.profile.as_ref().expect("the profile CALL decodes");
    assert_eq!(call.kind, 18);
    assert_eq!(call.bit_len, 16);
    let circle = call.circle.as_ref().expect("the circle body decodes");
    assert_eq!(circle.radius, 1.0);
    assert_eq!(circle.center, [0.0, 0.0, 0.0]);
    assert_eq!(circle.normal, [0.0, 0.0, 1.0]);
}

#[test]
fn extrude_r_decodes_the_profile_circle_radius() {
    // The §18.7 ExtrudeR quad (radius 3.125): the CALL bit-length
    // grows 16 -> 80 — the radius takes the raw BD form where the
    // landed fixture's exactly-1.0 took the two-bit short.
    let view = sweep_tail_view(&unhex(EXTRUDE_R_HEX), EXTRUDE_R_BITS)
        .expect("the ExtrudeR tail must decode");
    let call = view.profile.as_ref().expect("the profile CALL decodes");
    assert_eq!(call.kind, 18);
    assert_eq!(call.bit_len, 80);
    let circle = call.circle.as_ref().expect("the circle body decodes");
    assert_eq!(circle.radius, 3.125);
    assert_eq!(circle.center, [0.0, 0.0, 0.0]);
    assert_eq!(circle.normal, [0.0, 0.0, 1.0]);
    assert_eq!(view.draft_angle, Some(0.0));
    assert_eq!(view.scale_factor, Some(1.0));
}

#[test]
fn extrude_p_records_the_polyline_profile_call() {
    // The §18.7 ExtrudeP quad (closed LWPOLYLINE rectangle): the CALL
    // is type 77 = OBJ_LWPOLYLINE with a 544-bit window. The §18 walk
    // decodes the body through the embedded-LWPOLYLINE grammar: flag
    // 512 (closed), four raw (x, y) vertices — the 4×3 rectangle —
    // and the trailing reserved pair that closes the window.
    let view = sweep_tail_view(&unhex(EXTRUDE_P_HEX), EXTRUDE_P_BITS)
        .expect("the ExtrudeP tail must decode");
    let call = view.profile.as_ref().expect("the profile CALL decodes");
    assert_eq!(call.kind, 77);
    assert_eq!(call.bit_len, 544);
    assert!(call.circle.is_none());
    assert_eq!(view.scale_factor, Some(1.0));
    let polyline = call
        .polyline
        .as_ref()
        .expect("the kind-77 body decodes (the §18 walk)");
    assert_eq!(polyline.flag, 512);
    assert_eq!(polyline.num_points, 4);
    assert_eq!(
        polyline.points,
        vec![
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 3.0],
            [0.0, 3.0],
        ]
    );
    assert!(polyline.bulges.is_empty());
}

#[test]
fn extrude_p_polyline_edits_land_bit_locally() {
    // The §18 walk's write rule: a vertex edit through the named
    // points splice lands inside its own 128-bit vertex frames —
    // here vertex[2] (bits [626..754) of the tail) — nothing else
    // moves, the tail keeps its length, and the polyline body stays
    // exactly bit_len windowed. The edit base is a re-read op (its
    // modeled direction + tail_decode match the retained bits).
    let tail = unhex(EXTRUDE_P_HEX);
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(entity, sweep_op(&tail, EXTRUDE_P_BITS))
        .unwrap();
    // Phase 1: write once and read back so the modeled direction
    // matches the retained bits (a programmatic op carries the default
    // direction vector; the modeled direction splice is the one
    // legitimate non-verbatim re-encode for it). The polyline body
    // itself survives phase 1 untouched.
    mark_tree_captured(&mut document, entity);
    let first = DwgWriter::write_to_vec(&document).unwrap();
    let mut roundtrip = DwgReader::from_stream(Cursor::new(first)).read().unwrap();
    let (phase1_tail, phase1_points) = {
        let SolidHistoryOperation::Sweep(sweep) =
            &roundtrip.solid_history_operations(entity).unwrap()[0]
        else {
            panic!("expected a sweep operation node");
        };
        let polyline = sweep
            .tail_decode
            .as_ref()
            .and_then(|view| view.profile.as_ref())
            .and_then(|call| call.polyline.as_ref())
            .expect("the phase-1 op carries the decoded polyline");
        (sweep.shsw_raw_tail.clone(), polyline.points.clone())
    };
    assert_eq!(phase1_points.len(), 4);
    assert_eq!(phase1_points[2], [4.0, 3.0]);
    // Phase 2: edit vertex[2] through the named points field of the
    // re-read op (its modeled direction now matches its bits).
    let mut replacement = roundtrip.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Sweep(sweep) = &mut replacement else {
        panic!("expected a sweep operation node");
    };
    let polyline = sweep
        .tail_decode
        .as_mut()
        .and_then(|view| view.profile.as_mut())
        .and_then(|call| call.polyline.as_mut())
        .expect("the re-read op carries the decoded polyline");
    polyline.points[2] = [6.125, 3.0];
    roundtrip.update_solid_history_step(entity, replacement).unwrap();

    let edited = DwgWriter::write_to_vec(&roundtrip).unwrap();
    let back = DwgReader::from_stream(Cursor::new(edited)).read().unwrap();
    let SolidHistoryOperation::Sweep(sweep) =
        back.solid_history_operations(entity).unwrap()[0].clone()
    else {
        panic!("expected a sweep operation node");
    };
    assert_eq!(sweep.shsw_raw_tail.len(), phase1_tail.len());
    assert_eq!(sweep.shsw_raw_tail_bit_len, EXTRUDE_P_BITS);
    let differing: Vec<usize> = sweep
        .shsw_raw_tail
        .iter()
        .zip(phase1_tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty(), "the edit must land somewhere");
    // vertex[2].x lives at bits [626..690) -> bytes [78..86); y at
    // [690..754) -> bytes [86..94). Nothing outside vertex[2]'s
    // frames moves between phase 1 and phase 2.
    for index in &differing {
        assert!(
            (626 / 8..754 / 8).contains(index),
            "edits must stay inside vertex[2]'s frames, got {differing:?}"
        );
    }
    let view = sweep.tail_decode.as_ref().unwrap();
    let polyline = view
        .profile
        .as_ref()
        .and_then(|call| call.polyline.as_ref())
        .expect("the polyline body re-decodes");
    assert_eq!(polyline.points[2], [6.125, 3.0]);
    assert_eq!(polyline.points[0], [0.0, 0.0]);
    assert!(polyline.bulges.is_empty());
}

#[test]
fn extrude_profile_radius_edits_land_bit_locally() {
    // The profile circle fields are splice-backed: editing the
    // ExtrudeR radius (a raw span) lands inside its 64 value bits
    // only — the CALL length, the spine and the flags stay put.
    let tail = unhex(EXTRUDE_R_HEX);
    let mut document = CadDocument::with_version(DxfVersion::AC1032);
    let entity = document
        .add_entity(EntityType::Solid3D(Solid3D::new()))
        .unwrap();
    document
        .create_solid_history(
            entity,
            SolidHistoryOperation::Sweep(SolidHistorySweep {
                base: SolidHistoryNodeBase::new(1),
                shsw_raw_tail: tail.clone(),
                shsw_raw_tail_bit_len: EXTRUDE_R_BITS,
                // The tail's direction is the extrusion vector (0, 0, 2):
                // the render splices the model's direction against the
                // decode, so the test must carry the real value.
                direction: acadrust::types::Vector3::new(0.0, 0.0, 2.0),
                ..SolidHistorySweep::default()
            }),
        )
        .unwrap();
    let mut replacement = document.solid_history_operations(entity).unwrap()[0].clone();
    let SolidHistoryOperation::Sweep(sweep) = &mut replacement else {
        panic!("expected a sweep operation node");
    };
    // Populate the view from the real decode, then edit one field (the
    // render splices every difference against the re-decode).
    let mut view = sweep_tail_view(&tail, EXTRUDE_R_BITS).unwrap();
    view.profile
        .as_mut()
        .expect("the profile CALL decodes")
        .circle
        .as_mut()
        .expect("the circle body decodes")
        .radius = 2.5;
    sweep.tail_decode = Some(view);
    document.update_solid_history_step(entity, replacement).unwrap();

    mark_tree_captured(&mut document, entity);
    let bytes = DwgWriter::write_to_vec(&document).unwrap();
    let roundtrip = DwgReader::from_stream(Cursor::new(bytes)).read().unwrap();
    let SolidHistoryOperation::Sweep(sweep) =
        &roundtrip.solid_history_operations(entity).unwrap()[0]
    else {
        panic!("expected a sweep operation node");
    };
    let call = sweep.tail_decode.as_ref().unwrap().profile.as_ref().unwrap();
    assert_eq!(call.bit_len, 80, "the CALL length never moves");
    assert_eq!(call.circle.as_ref().unwrap().radius, 2.5);
    assert_eq!(sweep.shsw_raw_tail.len(), tail.len());
    let differing: Vec<usize> = sweep
        .shsw_raw_tail
        .iter()
        .zip(tail.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(index, _)| index)
        .collect();
    assert!(!differing.is_empty());
    // The radius raw value bits: [198..262) -> bytes 24..=32.
    assert!(
        differing.iter().all(|index| (24..33).contains(index)),
        "radius edits must stay inside the radius span, got {differing:?}"
    );
}
