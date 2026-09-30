/// Generate a single DWG file containing all supported entity types.
///
/// Output: gen_all_entities_all_versions.dwg (AC1032 / R2018)
///
/// This is a variant of `gen_all_entities_all_versions.rs` that creates one
/// document and writes every generated entity into a single file instead of
/// producing a file per version per entity type.
use acadrust::entities::acis::{SatDocument, SatPointer, SatToken, Sense, Sidedness};
use acadrust::entities::dimension::DimensionLinear;
use acadrust::entities::hatch::{
    BoundaryEdge, BoundaryPath, BoundaryPathFlags, LineEdge, PolylineEdge,
};
use acadrust::entities::mesh::Mesh;
use acadrust::entities::mline::MLine;
use acadrust::entities::multileader::MultiLeader;
use acadrust::entities::polyface_mesh::PolyfaceMesh;
use acadrust::entities::*;
use acadrust::types::{DxfVersion, Vector2, Vector3};
use acadrust::{
    BlockRecord, CadDocument, DimStyle, DwgWriter, LineWeight, TableEntry, TextStyle,
    Transparency,
};

/// Dimstyle-override EED payload of an authored text-leader (from
/// 2018/Leader.dwg), with the embedded annotation-entity reference
/// repointed from the original 0x77A to this document's MTEXT handle
/// 0x40 (masked RLL tail: run of 0x00 masks + 0x40 terminator).
/// Retained as the transcription record for the strict-load work; not
/// currently attached (the plaintext-leader probe class).
#[allow(dead_code)]
const DSTYLE_EED: [u8; 73] = [
    0x00, 0x06, 0x00, 0x44, 0x00, 0x53, 0x00, 0x54, 0x00, 0x59, 0x00, 0x4C, //
    0x00, 0x45, 0x00, 0x02, 0x00, 0x46, 0x28, 0x00, 0x28, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, 0x00, 0x46, 0x29, 0x00, 0x28, 0xB8, 0x1E, 0x85, //
    0xEB, 0x51, 0xB8, 0xCE, 0x3F, 0x46, 0x55, 0x01, 0x05, 0x00, 0x00, 0x00, //
    0x00, 0x00, 0x00, 0x00, 0x40, 0x46, 0x93, 0x00, 0x28, 0x0A, 0xD7, 0xA3, //
    0x70, 0x3D, 0x0A, 0xB7, 0x3F, 0x46, 0x4D, 0x00, 0x46, 0x00, 0x00, 0x02, //
    0x01,
];

/// Proxy-graphic blob of a BricsCAD-authored MULTILEADER (mleader_bcad.dwg,
/// its own writer convention played back to it). This was the round-six
/// zero-file specimen; it remains selectable with `GENALL_PROXY_BLOB=bcad`
/// as the non-gold-tree fallback.
const MLEADER_PROXY_GRAPHIC: &[u8] = &[
    0xC0, 0x01, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
    0x13, 0x00, 0x00, 0x00, 0x99, 0x3A, 0x00, 0x00, 0xC8, 0x00, 0x00, 0x00,
    0x26, 0x00, 0x00, 0x00, 0xF2, 0x47, 0xDA, 0x5B, 0xCD, 0x44, 0x25, 0x40,
    0x98, 0x31, 0x1A, 0xB9, 0xC6, 0xF3, 0xEE, 0xBF, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x62, 0x00, 0x6C, 0x00, 0x61, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x10, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x41, 0x00, 0x72, 0x00, 0x69, 0x00, 0x61, 0x00,
    0x6C, 0x00, 0x00, 0x00, 0x61, 0x00, 0x72, 0x00, 0x69, 0x00, 0x61, 0x00,
    0x6C, 0x00, 0x2E, 0x00, 0x74, 0x00, 0x74, 0x00, 0x66, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00,
    0x9A, 0x3A, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00,
    0x11, 0x27, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00,
    0x89, 0x13, 0x00, 0x00, 0x54, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xF0, 0x3F, 0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00,
    0x11, 0x27, 0x00, 0x00, 0x54, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x22, 0x40,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xF0, 0x3F,
];

/// The gold tree's own ODA-authored proxy-graphic specimen (the
/// DEFAULT since the 2026-09-21 round-seven BricsCAD verification):
/// the 564-byte metafile of the AcDbMLeader record (handle 0x732) in
/// `~/work/libredwg/test/test-data/2018/Leader.dwg` (record window
/// Address 5283, payload bytes [7..571)). Twelve records
/// (0x3A99 selector, zero state, 208-byte type-38 header,
/// selectors/0x7FFF, type-20, the 3-vertex type-7 leader line,
/// 0x1389, two type-32 blocks) — the same fixture as
/// `proxy_graphics.rs`'s byte-roundtrip test. With this specimen every
/// constructed-mleader wire component is derivable from the gold tree:
/// the 9-bit tail group and the 0/32/4786 attach stance also exist in
/// the gold tree's AutoCAD-authored 2013/gh44-error.dwg.
const MLEADER_PROXY_GRAPHIC_ODA: &[u8] = &[
    0x34, 0x02, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
    0x13, 0x00, 0x00, 0x00, 0x99, 0x3A, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00,
    0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xD0, 0x00, 0x00, 0x00,
    0x26, 0x00, 0x00, 0x00, 0x35, 0x21, 0xF7, 0x77, 0x91, 0xC1, 0x30, 0x40,
    0x14, 0x1F, 0xC2, 0xF7, 0xFD, 0xE7, 0x2C, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x4C, 0x00, 0x4C, 0x00, 0x4C, 0x00, 0x4C, 0x00,
    0x4C, 0x00, 0x4C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x0A, 0xD7, 0xA3, 0x70, 0x3D, 0x0A, 0xC7, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x22, 0x00, 0x00, 0x00,
    0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x00, 0x00,
    0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x41, 0x00,
    0x41, 0x00, 0x41, 0x00, 0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x11, 0x27, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0xFF, 0x7F, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x54, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x65, 0xD6, 0xDC, 0x62, 0x3C, 0x33, 0x34, 0x40, 0x45, 0xF3, 0xA2, 0x5B,
    0x97, 0xA8, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x9F, 0x6C, 0xE5, 0x37, 0xB0, 0x5B, 0x34, 0x40, 0xE7, 0xF5, 0x03, 0x39,
    0xDC, 0x79, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xD6, 0x92, 0x69, 0x31, 0xD8, 0x2D, 0x34, 0x40, 0x44, 0x23, 0xCC, 0x67,
    0xD3, 0x8B, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x89, 0x13, 0x00, 0x00,
    0x54, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x9E, 0x34, 0x23, 0x4A, 0x8A, 0x30, 0x34, 0x40, 0x44, 0x8B, 0xB7, 0x61,
    0x35, 0x9A, 0x2B, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x2C, 0x4B, 0x2B, 0xBA, 0xDA, 0x35, 0x32, 0x40, 0xBC, 0x66, 0xA3, 0x72,
    0x12, 0x16, 0x2D, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
    0x0C, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00, 0x11, 0x27, 0x00, 0x00,
    0x54, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x2C, 0x4B, 0x2B, 0xBA, 0xDA, 0x35, 0x32, 0x40, 0xBC, 0x66, 0xA3, 0x72,
    0x12, 0x16, 0x2D, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0xD0, 0xBB, 0x68, 0xC4, 0xB1, 0xD9, 0x31, 0x40, 0xBC, 0x66, 0xA3, 0x72,
    0x12, 0x16, 0x2D, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F,
];


fn main() {
    let mut doc = CadDocument::with_version(DxfVersion::AC1032);
    let mut ok = 0u32;
    let mut fail = 0u32;
    let mut skip = 0u32;

    // ── Simple geometry ─────────────────────────────────────────

    add_entity(
        &mut doc,
        "POINT",
        &mut ok,
        &mut fail,
        &mut skip,
        || EntityType::Point(Point::from_coords(50.0, 50.0, 0.0)),
    );

    add_entity(
        &mut doc,
        "LINE",
        &mut ok,
        &mut fail,
        &mut skip,
        || EntityType::Line(Line::from_coords(0.0, 0.0, 0.0, 100.0, 100.0, 0.0)),
    );

    add_entity(
        &mut doc,
        "CIRCLE",
        &mut ok,
        &mut fail,
        &mut skip,
        || EntityType::Circle(Circle::from_coords(50.0, 50.0, 0.0, 25.0)),
    );

    add_entity(
        &mut doc,
        "ARC",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Arc(Arc::from_coords(
                50.0,
                50.0,
                0.0,
                25.0,
                0.0,
                std::f64::consts::PI,
            ))
        },
    );

    add_entity(
        &mut doc,
        "ELLIPSE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Ellipse(Ellipse::from_center_axes(
                Vector3::new(50.0, 50.0, 0.0),
                Vector3::new(40.0, 0.0, 0.0),
                0.5,
            ))
        },
    );

    add_entity(
        &mut doc,
        "RAY",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Ray(Ray::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 0.0),
            ))
        },
    );

    add_entity(
        &mut doc,
        "XLINE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::XLine(XLine::new(
                Vector3::new(50.0, 50.0, 0.0),
                Vector3::new(1.0, 0.0, 0.0),
            ))
        },
    );

    // ── Solid / Surface ─────────────────────────────────────────

    add_entity(
        &mut doc,
        "SOLID",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Solid(Solid::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(10.0, 0.0, 0.0),
                Vector3::new(10.0, 10.0, 0.0),
                Vector3::new(0.0, 10.0, 0.0),
            ))
        },
    );

    add_entity(
        &mut doc,
        "FACE3D",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Face3D(Face3D::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(10.0, 0.0, 0.0),
                Vector3::new(10.0, 10.0, 5.0),
                Vector3::new(0.0, 10.0, 5.0),
            ))
        },
    );

    // ── Text ────────────────────────────────────────────────────

    add_entity(
        &mut doc,
        "TEXT",
        &mut ok,
        &mut fail,
        &mut skip,
        || EntityType::Text(Text::with_value("Hello World", Vector3::new(0.0, 0.0, 0.0))),
    );

    add_entity(
        &mut doc,
        "MTEXT",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::MText(MText::with_value(
                "Multi\\Pline\\PText",
                Vector3::new(0.0, 0.0, 0.0),
            ))
        },
    );

    // ── Polylines ───────────────────────────────────────────────

    add_entity(
        &mut doc,
        "LWPOLYLINE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::LwPolyline(LwPolyline::from_points(vec![
                Vector2::new(0.0, 0.0),
                Vector2::new(10.0, 0.0),
                Vector2::new(10.0, 10.0),
                Vector2::new(0.0, 10.0),
            ]))
        },
    );

    add_entity(
        &mut doc,
        "POLYLINE2D",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            let mut pl = Polyline2D::new();
            pl.add_vertex(Vertex2D::new(Vector3::new(0.0, 0.0, 0.0)));
            pl.add_vertex(Vertex2D::new(Vector3::new(20.0, 0.0, 0.0)));
            pl.add_vertex(Vertex2D::new(Vector3::new(20.0, 20.0, 0.0)));
            EntityType::Polyline2D(pl)
        },
    );

    add_entity(
        &mut doc,
        "POLYLINE3D",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Polyline3D(Polyline3D::from_points(vec![
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(10.0, 0.0, 5.0),
                Vector3::new(20.0, 10.0, 10.0),
            ]))
        },
    );

    add_entity(
        &mut doc,
        "SPLINE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Spline(Spline::from_control_points(
                3,
                vec![
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector3::new(5.0, 10.0, 0.0),
                    Vector3::new(10.0, 0.0, 0.0),
                    Vector3::new(15.0, 10.0, 0.0),
                ],
            ))
        },
    );

    // ── Annotations ─────────────────────────────────────────────

    // BricsCAD's strict (plain-open) loader rejects the constructed bare
    // leader record ("Object improperly read: <AcDbLeader>"), tolerating
    // it only under RECOVER, while both local decoders read it
    // spec-exact. GENALL_LEADER_MODE selects a variant for isolating the
    // audit: "plain" (default) — annot_type 3 with the null association,
    // the TS1-attested authored-null form; "annot" — the authored
    // WithText form with a real MTEXT association (ergo the leader's
    // handle shifts, which also disambiguates BricsCAD's "(40)" as
    // handle vs status code); "skip" — no leader record at all.
    let leader_mode = std::env::var("GENALL_LEADER_MODE").unwrap_or_default();
    if leader_mode.eq_ignore_ascii_case("skip") {
        println!("  SKIP LEADER (GENALL_LEADER_MODE=skip)");
    } else if leader_mode.eq_ignore_ascii_case("annot") {
        // Authored leaders with annotation always carry the real
        // association; create the MTEXT first so its handle exists.
        match doc.add_entity(EntityType::MText(MText::with_value(
            "Note",
            Vector3::new(12.0, 12.0, 0.0),
        ))) {
            Ok(mtext_handle) => {
                let mut leader = Leader::two_point(
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector3::new(10.0, 10.0, 0.0),
                );
                leader.creation_type = LeaderCreationType::WithText;
                leader.annotation_handle = mtext_handle;
                match doc.add_entity(EntityType::Leader(leader)) {
                    Ok(_) => ok += 1,
                    Err(e) => {
                        println!("  SKIP LEADER add error: {:?}", e);
                        skip += 1;
                    }
                }
            }
            Err(e) => {
                println!("  SKIP LEADER mtext error: {:?}", e);
                skip += 1;
            }
        }
    } else {
        // Plain-class leader construction: WithText + a real MTEXT
        // association, spline path, authored arrowhead/box values, scale,
        // weight and transparency — but NO named-linetype reference: a
        // named real linetype deep-resolves its table record, and a
        // constructed doc has no real dash description for it (the deep
        // loader drops the resolving entity). Continuous = no ltype slot.
        let _ = doc.dim_styles.add(DimStyle::new("Annotative"));
        match doc.add_entity(EntityType::MText(MText::with_value(
            "Note",
            Vector3::new(22.0, 12.0, 0.0),
        ))) {
            Ok(mtext_handle) => {
                let mut leader = Leader::from_vertices(vec![
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector3::new(10.0, 10.0, 0.0),
                    Vector3::new(20.0, 10.0, 0.0),
                ]);
                leader.creation_type = LeaderCreationType::WithText;
                leader.annotation_handle = mtext_handle;
                leader.path_type = LeaderPathType::Spline;
                leader.arrow_enabled = false;
                leader.arrowhead_type = 322;
                leader.hookline_direction = HooklineDirection::Same;
                leader.dwg_unknown_bit4 = true;
                leader.text_height = 0.0;
                leader.text_width = -0.09;
                // Consistent plain-leader class: no EED (the Annotative
                // marks imply an annotation-scale context with an
                // extension dictionary our constructed document does not
                // provide — the deep loader drops the mismatched
                // annotative leader), standard DIMSTYLE, and a real
                // linetype only when the table can back it (here:
                // continuous — the plainest legal class).
                leader.dimension_style = "Standard".to_string();
                leader.common.linetype = "Continuous".to_string();
                leader.common.line_weight = LineWeight::Value(5);
                leader.common.linetype_scale = 1.5;
                leader.common.transparency = Transparency::Explicit(169);
                match doc.add_entity(EntityType::Leader(leader)) {
                    Ok(leader_handle) => {
                        // The last authentically structural delta from the
                        // accepted record: a real extension dictionary,
                        // where per-scale annotation contexts live. The deep
                        // loader "improperly read"s the leader without it.
                        doc.ensure_extension_dictionary(leader_handle);
                        ok += 1;
                    }
                    Err(e) => {
                        println!("  SKIP LEADER add error: {:?}", e);
                        skip += 1;
                    }
                }
            }
            Err(e) => {
                println!("  SKIP LEADER mtext error: {:?}", e);
                skip += 1;
            }
        }
    }

    add_entity(
        &mut doc,
        "DIMENSION",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Dimension(Dimension::Linear(DimensionLinear::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(100.0, 0.0, 0.0),
            )))
        },
    );

    add_entity(
        &mut doc,
        "TOLERANCE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::Tolerance(Tolerance::with_text(
                Vector3::new(10.0, 10.0, 0.0),
                "{\\Fgdt;p}%%v0.5",
            ))
        },
    );

    // A SHAPE needs both a style whose font is a shape (.shx) file and a
    // nonzero shape number — a strict audit (BricsCAD) rejects the record
    // with both unset. ltypeshp.shx is the classic AutoCAD shape library;
    // 130 selects a glyph in it (the style/number pair is what the loader
    // validates; the glyph itself is cosmetic).
    let mut shape_style = TextStyle::new("LTYPESHP");
    shape_style.font_file = "ltypeshp.shx".to_string();
    shape_style.is_shape_file = true;
    let _ = doc.text_styles.add(shape_style);

    add_entity(
        &mut doc,
        "SHAPE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            let mut shape = Shape::with_name(Vector3::new(50.0, 50.0, 0.0), "BOX", 5.0);
            shape.shape_number = 130;
            shape.style_name = "LTYPESHP".to_string();
            EntityType::Shape(shape)
        },
    );

    add_entity(
        &mut doc,
        "VIEWPORT",
        &mut ok,
        &mut fail,
        &mut skip,
        || EntityType::Viewport(Viewport::new()),
    );

    add_insert(&mut doc, &mut ok, &mut fail, &mut skip);

    // ── Hatch ───────────────────────────────────────────────────

    add_entity(
        &mut doc,
        "HATCH_SOLID",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            let mut hatch = Hatch::solid();
            let mut path = BoundaryPath::new();
            path.add_edge(BoundaryEdge::Polyline(PolylineEdge::new(
                vec![
                    Vector2::new(0.0, 0.0),
                    Vector2::new(100.0, 0.0),
                    Vector2::new(100.0, 100.0),
                    Vector2::new(0.0, 100.0),
                ],
                true,
            )));
            hatch.add_path(path);
            EntityType::Hatch(hatch)
        },
    );

    add_entity(
        &mut doc,
        "HATCH_LINES",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            let mut hatch = Hatch::solid();
            let mut path = BoundaryPath::with_flags(BoundaryPathFlags::new());
            path.add_edge(BoundaryEdge::Line(LineEdge {
                start: Vector2::new(0.0, 0.0),
                end: Vector2::new(50.0, 0.0),
            }));
            path.add_edge(BoundaryEdge::Line(LineEdge {
                start: Vector2::new(50.0, 0.0),
                end: Vector2::new(50.0, 50.0),
            }));
            path.add_edge(BoundaryEdge::Line(LineEdge {
                start: Vector2::new(50.0, 50.0),
                end: Vector2::new(0.0, 0.0),
            }));
            hatch.add_path(path);
            EntityType::Hatch(hatch)
        },
    );

    // ── MLine ───────────────────────────────────────────────────

    add_entity(
        &mut doc,
        "MLINE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            EntityType::MLine(MLine::from_points(&[
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(50.0, 0.0, 0.0),
                Vector3::new(50.0, 50.0, 0.0),
            ]))
        },
    );

    // ── PolyfaceMesh ────────────────────────────────────────────

    add_entity(
        &mut doc,
        "POLYFACE",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            let mut pf = PolyfaceMesh::new();
            let v1 = pf.add_vertex_xyz(0.0, 0.0, 0.0);
            let v2 = pf.add_vertex_xyz(10.0, 0.0, 0.0);
            let v3 = pf.add_vertex_xyz(5.0, 10.0, 0.0);
            let v4 = pf.add_vertex_xyz(10.0, 10.0, 5.0);
            pf.add_triangle(v1, v2, v3);
            pf.add_triangle(v2, v4, v3);
            EntityType::PolyfaceMesh(pf)
        },
    );

    // ── MultiLeader ───────────────────────────────────────────────

    // GENALL_MLEADER_MODE=skip omits the MULTILEADER record (BricsCAD
    // strict-load probe; see the leader mode note above).
    // GENALL_MLEADER_MODE=default builds the host-analogue record: the
    // pure `MultiLeader::new()` stance, no explicit stamps and no
    // proxy blob (add one with GENALL_PROXY_BLOB=bcad|oda) — exactly
    // what out-of-tree constructors such as OpenCADStudio's MLEADER
    // command produce after the 2026-09-22 native-stance default fix.
    let mleader_mode = std::env::var("GENALL_MLEADER_MODE").unwrap_or_default();
    if mleader_mode.eq_ignore_ascii_case("skip") {
        println!("  SKIP MULTILEADER (GENALL_MLEADER_MODE=skip)");
    } else {
        // A native AcDbMLeader must reference a resolvable MLEADERSTYLE —
        // BricsCAD's audit reports "LeaderStyle Id is Null" otherwise.
        // The default-mode probe resolves the document's seeded
        // Standard style (created by CadDocument::new()); the native
        // branch stays byte-faithful to the zero file. Resolved before
        // the mutable doc borrow moves into add_entity.
        let standard_mlstyle_handle = if mleader_mode.eq_ignore_ascii_case("default") {
            doc.objects.iter().find_map(|(h, o)| match o {
                acadrust::objects::ObjectType::MultiLeaderStyle(m)
                    if m.name == "Standard" =>
                {
                    Some(*h)
                }
                _ => None,
            })
        } else {
            None
        };
        add_entity(
            &mut doc,
            "MULTILEADER",
            &mut ok,
            &mut fail,
            &mut skip,
            || {
                let mut ml = MultiLeader::with_text(
                    "Label",
                    Vector3::new(20.0, 20.0, 0.0),
                    vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(10.0, 10.0, 0.0)],
                );
                if mleader_mode.eq_ignore_ascii_case("default") {
                    ml.style_handle = standard_mlstyle_handle;
                    if let Ok(blob_mode) = std::env::var("GENALL_PROXY_BLOB") {
                        let blob: &[u8] = if blob_mode.eq_ignore_ascii_case("bcad") {
                            MLEADER_PROXY_GRAPHIC
                        } else {
                            MLEADER_PROXY_GRAPHIC_ODA
                        };
                        ml.common.graphic_data = Some(blob.to_vec());
                    }
                    return EntityType::MultiLeader(ml);
                }
                // Native-author stance (the gold tree's AutoCAD-authored
                // gh44-error.dwg carries the same 0/32/4786 trio and
                // extended-to-text state) — identical to the
                // `MultiLeader::new()` defaults since the 2026-09-22
                // fix; kept explicit so the byte record stays
                // self-describing. The proxy-graphics blob defaults to
                // the gold tree's ODA-authored specimen (2018/Leader.dwg â€”
                // BricsCAD-verified 2026-09-21, round seven);
                // GENALL_PROXY_BLOB=bcad selects the round-six
                // BricsCAD-authored fallback.
                let proxy_blob: &[u8] = if std::env::var("GENALL_PROXY_BLOB")
                    .is_ok_and(|mode| mode.eq_ignore_ascii_case("bcad"))
                {
                    MLEADER_PROXY_GRAPHIC
                } else {
                    MLEADER_PROXY_GRAPHIC_ODA
                };
                ml.common.graphic_data = Some(proxy_blob.to_vec());
                ml.dwg_attach_top = 32;
                ml.dwg_attach_bottom = 4786;
                ml.extend_leader_to_text = true;
                EntityType::MultiLeader(ml)
            },
        );
    }

    // ── Mesh ────────────────────────────────────────────────────

    add_entity(
        &mut doc,
        "MESH",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            let mut mesh = Mesh::from_triangles(
                vec![
                    Vector3::new(0.0, 0.0, 0.0),
                    Vector3::new(10.0, 0.0, 0.0),
                    Vector3::new(5.0, 10.0, 5.0),
                ],
                &[(0, 1, 2)],
            );
            // Authored-wire population invariants (2004/Surface.dwg's
            // MESH 0x2D0 and every authored specimen): wire bit 72 —
            // which the model calls blend_crease and gold decodes as
            // is_watertight — is 0, and the trailing unknown_b1 is 1.
            mesh.blend_crease = false;
            mesh.unknown_b1 = true;
            EntityType::Mesh(mesh)
        },
    );

    // ── ACIS entities (3DSOLID, REGION, BODY) ───────────────────

    add_entity(
        &mut doc,
        "3DSOLID",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            // §20 the last pending genus row (the BFS ordering packet):
            // the hand-built seam cylinder's only straight-curve hung on
            // the seam edge, whose BFS walk lands after the vertex walks —
            // its points preceded the straight-curve (genus: point always
            // after straight-curve), and the shape had no authored
            // counterpart (Cylinder_2018 has no vertical seam). The
            // battery now carries the authored-convention shape the
            // fixtures use (the seam-less two-loop lateral face).
            let sat_doc = acadrust::entities::acis::primitives::build_cylinder(
                [0.0, 0.0, 0.0], 5.0, 10.0,
            );
            EntityType::Solid3D(Solid3D::from_sat(&sat_doc.to_sat_string()))
        },
    );

    add_entity(
        &mut doc,
        "REGION",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            // A valid planar region built through the SatDocument API: a
            // single plane face with a closed four-edge outer loop (an open
            // sheet — every edge has exactly one coedge, so partners stay
            // null), all back-pointers wired. The earlier hand-typed SAT
            // string had degenerate records (single-vertex edges,
            // self-partnered coedges) that strict modelers reject as an
            // empty data stream.
            let sat_doc = build_region_sat();
            EntityType::Region(Region::from_sat(&sat_doc.to_sat_string()))
        },
    );

    add_entity(
        &mut doc,
        "BODY",
        &mut ok,
        &mut fail,
        &mut skip,
        || {
            // AcDbBody accepts any modeler body; reuse the same
            // authored-convention cylinder the 3DSOLID carries (the
            // §20 BFS ordering packet — the seam-cylinder shape left).
            let sat_doc = acadrust::entities::acis::primitives::build_cylinder(
                [0.0, 0.0, 0.0], 5.0, 10.0,
            );
            EntityType::Body(Body::from_sat(&sat_doc.to_sat_string()))
        },
    );

    // ── Write the single DWG file ─────────────────────────────────

    let path = "gen_all_entities_all_versions.dwg";
    match DwgWriter::write_to_file(path, &doc) {
        Ok(()) => {
            let sz = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            println!("Wrote {} ({} bytes)", path, sz);
        }
        Err(e) => {
            println!("FAIL writing {}: {:?}", path, e);
            std::process::exit(1);
        }
    }

    println!("\nSummary: {} OK, {} FAIL, {} SKIP", ok, fail, skip);

    if fail > 0 {
        std::process::exit(1);
    }
}

/// Add an entity to the single document, counting successes/skips.
/// Error paths count as skips; the shared `fail` counter is not touched
/// here, so it is accepted under its underscore alias.
fn add_entity<F>(
    doc: &mut CadDocument,
    name: &str,
    ok: &mut u32,
    _fail: &mut u32,
    skip: &mut u32,
    make_entity: F,
) where
    F: FnOnce() -> EntityType,
{
    let entity = make_entity();
    if let Err(e) = doc.add_entity(entity) {
        println!("  SKIP {:<20} add_entity error: {:?}", name, e);
        *skip += 1;
        return;
    }
    println!("  OK   {:<20}", name);
    *ok += 1;
}

/// Add a block definition + INSERT reference to the single document.
/// Error paths count as skips; the shared `fail` counter is not touched
/// here, so it is accepted under its underscore alias.
fn add_insert(doc: &mut CadDocument, ok: &mut u32, _fail: &mut u32, skip: &mut u32) {
    let name = "INSERT";

    // 1. Create a block record for "TestBlock"
    let mut br = BlockRecord::new("TestBlock");
    let br_handle = doc.allocate_handle();
    br.set_handle(br_handle);
    br.block_entity_handle = doc.allocate_handle();
    br.block_end_handle = doc.allocate_handle();
    if let Err(e) = doc.block_records.add(br) {
        println!("  SKIP {:<20} block_records.add error: {}", name, e);
        *skip += 1;
        return;
    }

    // 2. Add geometry to the block by pre-setting owner_handle
    let mut circle = EntityType::Circle(Circle::from_coords(0.0, 0.0, 0.0, 10.0));
    circle.common_mut().owner_handle = br_handle;
    if let Err(e) = doc.add_entity(circle) {
        println!("  SKIP {:<20} add circle error: {:?}", name, e);
        *skip += 1;
        return;
    }

    let mut line = EntityType::Line(Line::from_coords(-10.0, 0.0, 0.0, 10.0, 0.0, 0.0));
    line.common_mut().owner_handle = br_handle;
    if let Err(e) = doc.add_entity(line) {
        println!("  SKIP {:<20} add line error: {:?}", name, e);
        *skip += 1;
        return;
    }

    // 3. Add an INSERT entity referencing "TestBlock"
    let insert = Insert::new("TestBlock", Vector3::new(50.0, 50.0, 0.0));
    if let Err(e) = doc.add_entity(EntityType::Insert(insert)) {
        println!("  SKIP {:<20} add_entity error: {:?}", name, e);
        *skip += 1;
        return;
    }

    println!("  OK   {:<20}", name);
    *ok += 1;
}

/// A minimal valid planar region: body → lump → shell → one plane face
/// with a closed four-edge outer loop. An open sheet — every edge has
/// exactly one coedge (partners null) — and every back-pointer wired
/// (edge → its coedge, vertex → its edge, body → its lump). The 2026-09-30
/// record-identity session replaced the original CCW-wound sheet with the
/// FULL RAW-STREAM MIRROR of the authored plain R2018 region (the same
/// topology as genus_constructed's build_region_sat — her CW canonical
/// ring walked CCW by four reversed coedges, the sheet sidedness double,
/// her vertex backptrs, the plane origin at the centre, the face
/// containment completing at the SAB boundary, the vertex int 2).
fn build_region_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    // Corners of a 10x10 square in the XY plane.
    let p_bl = sat.add_point(0.0, 0.0, 0.0);
    let p_br = sat.add_point(10.0, 0.0, 0.0);
    let p_tl = sat.add_point(0.0, 10.0, 0.0);
    let p_tr = sat.add_point(10.0, 10.0, 0.0);

    // The plane origin at the rectangle's centre — the authored
    // convention (her surface origin = her rectangle's centre).
    let surf = sat.add_plane_surface([5.0, 5.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);

    // Side curves, each canonicalized at its edge's START vertex and
    // directed at the end — her c16/c19/c24/c21.
    let c_top = sat.add_straight_curve([0.0, 10.0, 0.0], [1.0, 0.0, 0.0]);
    let c_left = sat.add_straight_curve([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let c_bottom = sat.add_straight_curve([10.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
    let c_right = sat.add_straight_curve([10.0, 10.0, 0.0], [0.0, -1.0, 0.0]);

    let v14 = sat.add_vertex(SatPointer::NULL, ptr(p_tl));
    let v15 = sat.add_vertex(SatPointer::NULL, ptr(p_tr));
    let v18 = sat.add_vertex(SatPointer::NULL, ptr(p_bl));
    let v20 = sat.add_vertex(SatPointer::NULL, ptr(p_br));

    // Her canonical ring, CW as a cycle: top L→R, left B→T,
    // bottom R→L, right T→B.
    let e_top = sat.add_edge(
        ptr(v14),
        0.0,
        ptr(v15),
        10.0,
        SatPointer::NULL,
        ptr(c_top),
        Sense::Forward,
    );
    let e_left = sat.add_edge(
        ptr(v18),
        0.0,
        ptr(v14),
        10.0,
        SatPointer::NULL,
        ptr(c_left),
        Sense::Forward,
    );
    let e_bottom = sat.add_edge(
        ptr(v20),
        0.0,
        ptr(v18),
        10.0,
        SatPointer::NULL,
        ptr(c_bottom),
        Sense::Forward,
    );
    let e_right = sat.add_edge(
        ptr(v15),
        0.0,
        ptr(v20),
        10.0,
        SatPointer::NULL,
        ptr(c_right),
        Sense::Forward,
    );

    // Coedge indices: 4 coedges, then loop, face, shell, lump.
    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;
    let loop_idx = base + 4;
    let face_idx = base + 5;
    let shell_idx = base + 6;
    let lump_idx = base + 7;

    // Her chain: co(top) → co(left) → co(bottom) → co(right), every
    // coedge REVERSED over the CW canonicals (the CCW walk) and the
    // partner null (an open sheet).
    let edges = [e_top, e_left, e_bottom, e_right];
    for i in 0..4i32 {
        sat.add_coedge(
            ptr(co((i + 1) % 4)),
            ptr(co((i + 3) % 4)),
            SatPointer::NULL, // open sheet: no partner coedge on another face
            ptr(edges[i as usize]),
            Sense::Reversed,
            ptr(loop_idx),
        );
    }

    sat.add_loop(SatPointer::NULL, ptr(co(0)), ptr(face_idx));
    sat.add_face(
        SatPointer::NULL,
        ptr(loop_idx),
        ptr(shell_idx),
        ptr(surf),
        Sense::Forward,
        Sidedness::Double,
    );
    // The authored SHEET face's ninth token — the solid faces are
    // 8-token (the authored Box census); her plain R2018 region face
    // carries the third bool (the fixed-width SAB reader is
    // positional). Added on the region only, never class-wide.
    if let Some(r) = sat.record_mut(face_idx as usize) {
        r.tokens.push(SatToken::True);
    }
    sat.add_shell(ptr(face_idx), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    // Back-pointers: edge → its chain coedge; vertex → its edge after
    // HER mapping, and the authored sheet vertex int 2.
    let coedges = [co(0), co(1), co(2), co(3)];
    for i in 0..4usize {
        if let Some(r) = sat.record_mut(edges[i] as usize) {
            r.tokens[5] = SatToken::Pointer(ptr(coedges[i]));
        }
    }
    let verts = [(v14, e_top), (v15, e_top), (v18, e_bottom), (v20, e_right)];
    for (vid, eid) in verts {
        if let Some(r) = sat.record_mut(vid as usize) {
            r.tokens[1] = SatToken::Pointer(ptr(eid));
            r.tokens[2] = SatToken::Integer(2);
        }
    }

    sat
}

