//! TODO B2 (2026-10-01): the ASSOCEDGEACTIONPARAM subcurve kinds.
//!
//! The modeled wire forms assert BIT-EXACT against the authored
//! specimens' pinned record bodies (the prefix + region bits the DWG
//! reader captures per handle): the ARC (11) twelve-BD form with the
//! R2013+ frames' two-bit `10` trailing form, the ELLIPSE (17)
//! thirteen-BD form, the LINESEG3D (23) six-BD form, and the
//! capture+replay path for the untyped kinds (NURB3D 42 here) that
//! re-emits the verbatim region on a same-version write. Provenance
//! and the dissection method: the B2 session record in
//! `tests/gold_harness/IMPLEMENTATION.md` (the H8h-ext-4 BD-walk,
//! the accoreconsole-authored quads and the 2004/Surface.dwg corpus
//! cross-source; gold's own spec switch is dead code behind
//! HANDLE_UNKNOWN_BITS, so none of this is gold-attested).

use acadrust::objects::{
    AssocActionParam, AssocArcSubcurve, AssocEdgeActionParam, AssocEllipseSubcurve,
    AssocLineSegment3dSubcurve, AssocNurb3dSubcurve, AssocSingleDependencyActionParam,
    AssocSubcurve, AssocSubcurveKind, AssociativeData, AssociativeObject, ObjectType,
};
use acadrust::types::{DxfVersion, Vector3};
use acadrust::{CadDocument, DwgReader, DwgWriter};
use std::io::Cursor;

// Her ExtrudeM_2018 record 0x742: the 27-bit typed prefix (is_r2013
// BS 1, versions BL 0, has_action true, action_type BL 11) + the
// 90-bit region (twelve BDs + the R2013+ `10` trailing form).
const ARC_R2013_BODY_BITS: u32 = 117;
const ARC_R2013_BODY_HEX: &str = "406AA17552D30305A88A9F64232810";

// Her ExtrudeM_2007 record 0x742: the 17-bit prefix (is_r2013 BS 0,
// no aap_version BL on the pre-R2013 grammar) + the 88-bit region
// (twelve BDs only — the trailing form is R2013+).
const ARC_R2007_BODY_BITS: u32 = 105;
const ARC_R2007_BODY_HEX: &str = "AA85D54B4C0C16A22A7D908CA00";

// Her ExtrudeEllipse_2018 record 0x739: prefix + the 220-bit
// thirteen-BD region (center, major/minor axis units, radii, angles
// + the R2013+ trailing form).
const ELLIPSE_R2013_BODY_BITS: u32 = 247;
const ELLIPSE_R2013_BODY_HEX: &str =
    "406AA235353000000000000010800000000000007C1FC0C16A22A7D908CA04";

// Her ExtrudeLine_2018 record 0x739: prefix + the 76-bit six-BD
// region (start/end points — no trailing form on any frame).
const LINESEG3D_R2013_BODY_BITS: u32 = 103;
const LINESEG3D_R2013_BODY_HEX: &str = "406AA2F5000000000000020814";

// Her ExtrudePline_2018 record 0x739: the gold-unknown composite (47)
// region, 610 bits — the verbatim replay payload (the capture+replay
// net's remaining user now that 42 parses typed, TODO A8 2026-10-02).
const PLINE47_REGION_BITS: u32 = 610;
const PLINE47_REGION_HEX: &str = concat!(
    "41117A80000000000001040A45C0000000000001040A8000000000000084091700000000",
    "0000041000000000000000840800000000000010C0A45E00000000000002102800000000",
    "000008C080"
);

fn unhex(value: &str) -> Vec<u8> {
    let cleaned: String = value.chars().filter(|c| *c != '"').collect();
    let padded = if cleaned.len() % 2 == 0 {
        cleaned
    } else {
        format!("{cleaned}0")
    };
    (0..padded.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&padded[index..index + 2], 16).unwrap())
        .collect()
}

/// The first `bits` bits of a capture hex, as a bit vector.
fn capture_bits(hex: &str, bits: u32) -> Vec<bool> {
    let bytes = unhex(hex.trim_matches('"').replace('"', "").as_str());
    let mut out = Vec::with_capacity(bits as usize);
    'outer: for byte in bytes {
        for shift in (0..8).rev() {
            out.push((byte >> shift) & 1 == 1);
            if out.len() as u32 == bits {
                break 'outer;
            }
        }
    }
    out
}

fn document_with_edge_param(
    version: DxfVersion,
    action_type: i32,
    subcurve: Option<AssocSubcurve>,
    wire: Option<(Vec<u8>, u32, Option<DxfVersion>)>,
) -> (CadDocument, acadrust::types::Handle) {
    let is_r2013 = if version >= DxfVersion::AC1027 { 1 } else { 0 };
    let mut document = CadDocument::with_version(version);
    let handle = document.allocate_handle();
    let owner = document.header.named_objects_dict_handle;
    // The class entry a real drawing carries (the authored fixtures'
    // class tables), so the writer emits the record under its own
    // class number instead of the 500 fallback (which collides with
    // ACDBDICTIONARYWDFLT).
    document.classes.add_or_update(acadrust::classes::DxfClass {
        dxf_name: "ACDBASSOCEDGEACTIONPARAM".to_string(),
        cpp_class_name: "AcDbAssocEdgeActionParam".to_string(),
        application_name: "ObjectDBX Classes".to_string(),
        proxy_flags: acadrust::classes::ProxyFlags(
            acadrust::classes::ProxyFlags::ERASE_ALLOWED.0
                | acadrust::classes::ProxyFlags::CLONING_ALLOWED.0
                | acadrust::classes::ProxyFlags::DISABLES_PROXY_WARNING_DIALOG.0,
        ),
        instance_count: 0,
        was_zombie: false,
        is_an_entity: false,
        class_number: 0,
        item_class_id: 0x1F3,
        dwg_version: 0,
        maintenance_version: 0,
        unknown1: 0,
        unknown2: 0,
        gold_shadow: None,
    });
    let (wire, wire_bits, wire_version) = match wire {
        Some((bytes, bits, ver)) => (Some(bytes), bits, ver),
        None => (None, 0, None),
    };
    document.objects.insert(
        handle,
        ObjectType::Associative(AssociativeObject {
            handle,
            owner,
            dxf_name: "ACDBASSOCEDGEACTIONPARAM".to_string(),
            cpp_class_name: "AcDbAssocEdgeActionParam".to_string(),
            data: AssociativeData::EdgeActionParam(AssocEdgeActionParam {
                single_dependency: AssocSingleDependencyActionParam {
                    action_param: AssocActionParam {
                        is_r2013,
                        version: 0,
                        name: String::new(),
                    },
                    dependency_class_version: 0,
                    dependency: acadrust::types::Handle::from(0u64),
                    class_version: 0,
                },
                parameter: acadrust::types::Handle::from(0u64),
                has_action: true,
                action_type,
                subcurve_kind: match action_type {
                    11 => AssocSubcurveKind::Arc,
                    17 => AssocSubcurveKind::Ellipse,
                    23 => AssocSubcurveKind::LineSegment3d,
                    42 => AssocSubcurveKind::Nurb3d,
                    _ => AssocSubcurveKind::None,
                },
                subcurve,
                subcurve_wire: wire,
                subcurve_wire_bit_len: wire_bits,
                subcurve_wire_dxf_version: wire_version,
            }),
            ..Default::default()
        }),
    );
    (document, handle)
}

/// Write + read back, returning the record's captured body hex and
/// the decoded record. The capture is keyed by the DECODED record's
/// own handle — the AC1021 writer canonicalizes handles, so the
/// document-time allocation may not survive the write.
fn roundtrip(document: CadDocument) -> (String, AssocEdgeActionParam) {
    let bytes = DwgWriter::write_to_vec(&document).expect("write DWG");
    let decoded = DwgReader::from_stream(Cursor::new(bytes))
        .read()
        .expect("read DWG");
    let (record_handle, record) = decoded
        .objects
        .iter()
        .find_map(|(h, object)| match object {
            ObjectType::Associative(AssociativeObject {
                data: AssociativeData::EdgeActionParam(value),
                ..
            }) => Some((*h, value.clone())),
            _ => None,
        })
        .expect("edge action param should round-trip");
    assert_ne!(record_handle, acadrust::types::Handle::from(0u64));
    let captured = decoded
        .unknown_bits_by_handle
        .get(&record_handle)
        .cloned()
        .expect("the record carries a captured body");
    (captured, record)
}

fn assert_bits_match(captured_hex: &str, pinned_hex: &str, pinned_bits: u32, what: &str) {
    let got = capture_bits(captured_hex, pinned_bits);
    let want = capture_bits(pinned_hex, pinned_bits);
    assert_eq!(got, want, "{what} diverged from the captured specimen");
}

#[test]
fn dwg_subcurve_arc_typed_emission_r2013() {
    let subcurve = AssocSubcurve::Arc(AssocArcSubcurve {
        center: Vector3::new(0.0, 0.0, 0.0),
        normal: Vector3::new(0.0, 0.0, 1.0),
        x_axis: Vector3::new(1.0, 0.0, 0.0),
        radius: 1.0,
        start_angle: 0.0,
        end_angle: 6.283185307179586,
    });
    let (document, _) =
        document_with_edge_param(DxfVersion::AC1032, 11, Some(subcurve), None);
    let (captured, record) = roundtrip(document);
    assert_bits_match(
        &captured,
        ARC_R2013_BODY_HEX,
        ARC_R2013_BODY_BITS,
        "the ARC region at R2013+ (twelve BDs + the trailing form)",
    );
    assert_eq!(record.subcurve_kind, AssocSubcurveKind::Arc);
    assert!(matches!(record.subcurve, Some(AssocSubcurve::Arc(_))));
    assert!(record.subcurve_wire.is_none());
}

#[test]
fn dwg_subcurve_arc_typed_emission_r2007_has_no_trailing_form() {
    let subcurve = AssocSubcurve::Arc(AssocArcSubcurve {
        center: Vector3::new(0.0, 0.0, 0.0),
        normal: Vector3::new(0.0, 0.0, 1.0),
        x_axis: Vector3::new(1.0, 0.0, 0.0),
        radius: 1.0,
        start_angle: 0.0,
        end_angle: 6.283185307179586,
    });
    let (document, _) =
        document_with_edge_param(DxfVersion::AC1021, 11, Some(subcurve), None);
    let (captured, _record) = roundtrip(document);
    assert_bits_match(
        &captured,
        ARC_R2007_BODY_HEX,
        ARC_R2007_BODY_BITS,
        "the ARC region at R2007 (twelve BDs, no trailing form)",
    );
}

#[test]
fn dwg_subcurve_ellipse_typed_emission_r2013() {
    let subcurve = AssocSubcurve::Ellipse(AssocEllipseSubcurve {
        center: Vector3::new(0.0, 0.0, 0.0),
        major_axis: Vector3::new(1.0, 0.0, 0.0),
        minor_axis: Vector3::new(0.0, 1.0, 0.0),
        major_radius: 3.0,
        minor_radius: 1.5,
        start_angle: 0.0,
        end_angle: 6.283185307179586,
    });
    let (document, _) =
        document_with_edge_param(DxfVersion::AC1032, 17, Some(subcurve), None);
    let (captured, record) = roundtrip(document);
    assert_bits_match(
        &captured,
        ELLIPSE_R2013_BODY_HEX,
        ELLIPSE_R2013_BODY_BITS,
        "the ELLIPSE region at R2013+ (thirteen BDs + the trailing form)",
    );
    assert_eq!(record.subcurve_kind, AssocSubcurveKind::Ellipse);
    assert!(matches!(record.subcurve, Some(AssocSubcurve::Ellipse(_))));
    assert!(record.subcurve_wire.is_none());
}

#[test]
fn dwg_subcurve_linesegment3d_typed_emission_r2013() {
    let subcurve = AssocSubcurve::LineSegment3d(AssocLineSegment3dSubcurve {
        start_point: Vector3::new(0.0, 0.0, 0.0),
        end_point: Vector3::new(4.0, 0.0, 0.0),
    });
    let (document, _) =
        document_with_edge_param(DxfVersion::AC1032, 23, Some(subcurve), None);
    let (captured, record) = roundtrip(document);
    assert_bits_match(
        &captured,
        LINESEG3D_R2013_BODY_HEX,
        LINESEG3D_R2013_BODY_BITS,
        "the LINESEG3D region (six BDs, no trailing form)",
    );
    assert_eq!(record.subcurve_kind, AssocSubcurveKind::LineSegment3d);
    assert!(matches!(record.subcurve, Some(AssocSubcurve::LineSegment3d(_))));
    assert!(record.subcurve_wire.is_none());
}

#[test]
fn dwg_subcurve_untyped_kind_replays_captured_wire_same_version() {
    // the gold-unknown composite (47) — the capture+replay net's
    // remaining user since the NURB3D (42) moved to the typed ladder
    // (TODO A8, 2026-10-02).
    let wire = unhex(PLINE47_REGION_HEX.replace('"', "").as_str());
    let wire_bits = PLINE47_REGION_BITS;
    let (document, _) = document_with_edge_param(
        DxfVersion::AC1032,
        47,
        None,
        Some((wire, wire_bits, Some(DxfVersion::AC1032))),
    );
    let (captured, record) = roundtrip(document);
    // the replay lands at the read-back capture offset 27 (after the
    // typed prefix): capture[27 .. 27+610) == the pinned region bits.
    let got = capture_bits(&captured, 27 + wire_bits);
    let want = capture_bits(PLINE47_REGION_HEX, wire_bits);
    assert_eq!(
        got[27..27 + wire_bits as usize],
        want[..wire_bits as usize],
        "the kind-47 region replay diverged from the captured specimen"
    );
    assert_eq!(record.subcurve_kind, AssocSubcurveKind::None);
    assert!(record.subcurve.is_none());
    assert_eq!(record.subcurve_wire_bit_len, wire_bits);
    assert!(record.subcurve_wire.is_some());
}

#[test]
fn dwg_subcurve_wire_replay_is_version_gated() {
    let wire = unhex(PLINE47_REGION_HEX.replace('"', "").as_str());
    let (document, _) = document_with_edge_param(
        DxfVersion::AC1032,
        47,
        None,
        // capture claims AC1021 while the write targets AC1032: the
        // replay must NOT fire (era forms differ across versions).
        Some((wire, PLINE47_REGION_BITS, Some(DxfVersion::AC1021))),
    );
    let (captured, record) = roundtrip(document);
    // the region is absent: the whole captured record (typed body +
    // framing tail) must be far shorter than one carrying the
    // 610-bit region after its 27-bit prefix.
    assert!(
        record.subcurve_wire.is_none(),
        "a mismatched-capture record must not replay the region"
    );
    let captured_bits = unhex(&captured).len() as u32 * 8;
    assert!(
        captured_bits < 27 + PLINE47_REGION_BITS,
        "the region should not have been emitted (capture: {captured_bits} bits)"
    );
}

// Her SweepSurfSpline_2018 record 0x746 (the 2026-10-02 surface-mode
// sweep path): the typed NURB3D region, 1224 bits — the measured
// grammar's smallest full specimen (TODO A8, 2026-10-02).
const NURB_SWEEP_REGION_BITS: u32 = 1224;
const NURB_SWEEP_REGION_HEX: &str = concat!(
    "103257589BA02CB844F90942508AA0D1BFF64DCA20290031480354801017400C5200D520",
    "0405D0031480354801017400C5200D5200405D0290841508422A3A06228D15B8F84FC692",
    "388674F29D33F172735CB69DCBC0FC87453A705E7EF83F1769FBD2EDCF3A8FCFC59842B8",
    "A8506400F16581DC40DB74FCF3805467D85EFA3F1874D3F13DD904502000000000000001",
    "000000000000001440",
);

fn sweep_path_nurb() -> AssocNurb3dSubcurve {
    AssocNurb3dSubcurve {
        flags: 0b001001,
        knot_tolerance: 1e-9,
        knots: vec![
            0.0,
            0.0,
            0.0,
            0.0,
            3.3166247903554,
            5.766114533138578,
            5.766114533138578,
            5.766114533138578,
            5.766114533138578,
        ],
        gap_b: 8,
        control_points: vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.5590010667954433, 0.29939637289549254, 1.027991465636453),
            Vector3::new(1.5308517822162189, 0.819911621392048, 2.8152049445882295),
            Vector3::new(0.45642482431266607, 1.6481555973369353, 4.348601405498671),
            Vector3::new(0.0, 2.0, 5.0),
        ],
    }
}

#[test]
fn dwg_subcurve_nurb3d_typed_emission_r2013() {
    let (document, _) = document_with_edge_param(
        DxfVersion::AC1032,
        42,
        Some(AssocSubcurve::Nurb3d(sweep_path_nurb())),
        None,
    );
    let (captured, record) = roundtrip(document);
    // the typed prefix (27 bits at R2013+) precedes the region
    let got = capture_bits(&captured, 27 + NURB_SWEEP_REGION_BITS);
    let want = capture_bits(NURB_SWEEP_REGION_HEX, NURB_SWEEP_REGION_BITS);
    assert_eq!(
        got[27..27 + NURB_SWEEP_REGION_BITS as usize],
        want[..],
        "the NURB3D region diverged from the captured specimen"
    );
    assert_eq!(record.subcurve_kind, AssocSubcurveKind::Nurb3d);
    let nurb = match &record.subcurve {
        Some(AssocSubcurve::Nurb3d(value)) => value,
        other => panic!("expected the typed NURB3D round-trip, got {other:?}"),
    };
    assert_eq!(nurb.flags, 0b001001);
    assert_eq!(nurb.knot_tolerance, 1e-9);
    assert_eq!(nurb.knots.len(), 9);
    assert_eq!(nurb.knots[4], 3.3166247903554);
    assert_eq!(nurb.gap_b, 8);
    assert_eq!(nurb.control_points.len(), 5);
    assert_eq!(nurb.control_points[4], Vector3::new(0.0, 2.0, 5.0));
    assert!(record.subcurve_wire.is_none());
}

#[test]
fn dwg_subcurve_nurb3d_emission_is_era_stable() {
    // The measured NURB3D regions are bit-identical across eras
    // (2007 vs 2018 on every specimen), so the typed emission is
    // NOT version-gated: a pre-R2013 write carries the same region
    // after its 17-bit prefix (no aap_version BL there).
    let (document, _) = document_with_edge_param(
        DxfVersion::AC1021,
        42,
        Some(AssocSubcurve::Nurb3d(sweep_path_nurb())),
        None,
    );
    let (captured, record) = roundtrip(document);
    let got = capture_bits(&captured, 17 + NURB_SWEEP_REGION_BITS);
    let want = capture_bits(NURB_SWEEP_REGION_HEX, NURB_SWEEP_REGION_BITS);
    assert_eq!(
        got[17..17 + NURB_SWEEP_REGION_BITS as usize],
        want[..],
        "the NURB3D region must be era-stable (the 2007/2018 specimens are bit-identical)"
    );
    assert!(matches!(record.subcurve, Some(AssocSubcurve::Nurb3d(_))));
}

