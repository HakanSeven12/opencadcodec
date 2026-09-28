//! The §20 constructed-fixture family generator (genus_gates.py's input).
//!
//! Emits one small DWG per constructed-content family — the shapes the
//! genus gates decode and assert against the authored-specimen genus
//! (IMPLEMENTATION.md §20.3): one solid per SAB surface family, one
//! `create_solid_history` tree, one region. All files are AC1032 (R2018)
//! so the modeler data rides the AcDs data-store SAB blobs gold cannot
//! read — silver's decode is the genus reference there (§20.1).
//!
//! The family is CONSTRUCTED content by definition: every document is
//! authored from scratch through the programmatic API (the primitives
//! builders + `CadDocument::create_solid_history`), never read from a
//! file. The history-tree fixture exercises the constructed-tree elide
//! contract (2646f05): the factory tree is not a genus any strict loader
//! accepts, so its ACSH_ records elide at save and the decode carries
//! the solid with a NULL history soft-pointer — the gates rank that
//! elide as the observed state of the tree genus.
//!
//! Usage: `genus_constructed <outdir>` — writes Box.dwg, Sphere.dwg,
//! Cylinder.dwg, Cone.dwg, Torus.dwg, Region.dwg, HistoryTree.dwg.

use acadrust::entities::acis::primitives::{
    build_box, build_cone, build_cylinder, build_sphere, build_torus,
};
use acadrust::entities::acis::{SatDocument, SatPointer, SatToken, Sense, Sidedness};
use acadrust::entities::{Body, Region, Solid3D};
use acadrust::objects::{SolidHistoryCylinder, SolidHistoryOperation};
use acadrust::types::DxfVersion;
use acadrust::{CadDocument, DwgWriter, EntityType, Handle};
use std::path::Path;

/// A planar region sheet: one plane face with a closed four-edge outer
/// loop, every coedge partner null (an open sheet), all back-pointers
/// wired — the same construction as the generator example's region.
fn build_region_sat() -> SatDocument {
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    let p0 = sat.add_point(0.0, 0.0, 0.0);
    let p1 = sat.add_point(10.0, 0.0, 0.0);
    let p2 = sat.add_point(10.0, 10.0, 0.0);
    let p3 = sat.add_point(0.0, 10.0, 0.0);

    let surf = sat.add_plane_surface([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]);

    let c0 = sat.add_straight_curve([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    let c1 = sat.add_straight_curve([10.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
    let c2 = sat.add_straight_curve([10.0, 10.0, 0.0], [-1.0, 0.0, 0.0]);
    let c3 = sat.add_straight_curve([0.0, 10.0, 0.0], [0.0, -1.0, 0.0]);

    let v0 = sat.add_vertex(SatPointer::NULL, ptr(p0));
    let v1 = sat.add_vertex(SatPointer::NULL, ptr(p1));
    let v2 = sat.add_vertex(SatPointer::NULL, ptr(p2));
    let v3 = sat.add_vertex(SatPointer::NULL, ptr(p3));

    let e0 = sat.add_edge(ptr(v0), 0.0, ptr(v1), 10.0, SatPointer::NULL, ptr(c0), Sense::Forward);
    let e1 = sat.add_edge(ptr(v1), 0.0, ptr(v2), 10.0, SatPointer::NULL, ptr(c1), Sense::Forward);
    let e2 = sat.add_edge(ptr(v2), 0.0, ptr(v3), 10.0, SatPointer::NULL, ptr(c2), Sense::Forward);
    let e3 = sat.add_edge(ptr(v3), 0.0, ptr(v0), 10.0, SatPointer::NULL, ptr(c3), Sense::Forward);

    let base = sat.records.len() as i32;
    let co = |i: i32| base + i;
    let loop_idx = base + 4;
    let face_idx = base + 5;
    let shell_idx = base + 6;
    let lump_idx = base + 7;

    let edges = [e0, e1, e2, e3];
    for i in 0..4i32 {
        sat.add_coedge(
            ptr(co((i + 1) % 4)),
            ptr(co((i + 3) % 4)),
            SatPointer::NULL,
            ptr(edges[i as usize]),
            Sense::Forward,
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
        Sidedness::Single,
    );
    sat.add_shell(ptr(face_idx), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    let coedges = [co(0), co(1), co(2), co(3)];
    for i in 0..4usize {
        if let Some(r) = sat.record_mut(edges[i] as usize) {
            r.tokens[5] = SatToken::Pointer(ptr(coedges[i]));
        }
    }
    let verts = [v0, v1, v2, v3];
    for i in 0..4usize {
        if let Some(r) = sat.record_mut(verts[i] as usize) {
            r.tokens[1] = SatToken::Pointer(ptr(edges[i]));
        }
    }

    sat
}

fn new_document() -> CadDocument {
    CadDocument::with_version(DxfVersion::AC1032)
}

fn write_fixture(doc: CadDocument, name: &str, outdir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = outdir.join(format!("{name}.dwg"));
    DwgWriter::write_to_file(&path, &doc)?;
    let size = std::fs::metadata(&path)?.len();
    println!("wrote {} ({} bytes)", path.display(), size);
    Ok(())
}

fn solid_fixture(sat: SatDocument) -> CadDocument {
    let mut doc = new_document();
    let entity = EntityType::Solid3D(Solid3D::from_sat(&sat.to_sat_string()));
    doc.add_entity(entity)
        .expect("add_entity(Solid3D) on a fresh document cannot fail");
    doc
}

fn main() {
    let outdir = match std::env::args().nth(1) {
        Some(dir) => std::path::PathBuf::from(dir),
        None => {
            eprintln!("usage: genus_constructed <outdir>");
            std::process::exit(2);
        }
    };
    std::fs::create_dir_all(&outdir).expect("create outdir");

    // One solid per SAB surface family (the primitive builders are the
    // programmatic-API constructors; the surface class each family pins
    // is the G-A width target).
    write_fixture(solid_fixture(build_box([0.0, 0.0, 0.0], 10.0, 10.0, 10.0)), "Box", &outdir)
        .expect("write Box.dwg");
    write_fixture(solid_fixture(build_sphere([0.0, 0.0, 0.0], 5.0)), "Sphere", &outdir)
        .expect("write Sphere.dwg");
    write_fixture(solid_fixture(build_cylinder([0.0, 0.0, 0.0], 5.0, 10.0)), "Cylinder", &outdir)
        .expect("write Cylinder.dwg");
    write_fixture(solid_fixture(build_cone([0.0, 0.0, 0.0], 5.0, 10.0)), "Cone", &outdir)
        .expect("write Cone.dwg");
    write_fixture(solid_fixture(build_torus([0.0, 0.0, 0.0], 10.0, 3.0)), "Torus", &outdir)
        .expect("write Torus.dwg");

    // One region: the planar sheet (plane-surface + straight-curve).
    let mut region_doc = new_document();
    region_doc
        .add_entity(EntityType::Region(Region::from_sat(
            &build_region_sat().to_sat_string(),
        )))
        .expect("add_entity(Region) on a fresh document cannot fail");
    write_fixture(region_doc, "Region", &outdir).expect("write Region.dwg");

    // One create_solid_history tree: a cylinder solid plus the factory
    // history tree. At save the constructed ACSH_ records elide (the
    // 2646f05 verdict) — the decode shows the solid with a NULL history
    // soft-pointer, the state the G-B gates rank.
    let mut tree_doc = new_document();
    let handle: Handle = tree_doc
        .add_entity(EntityType::Solid3D(Solid3D::from_sat(
            &build_cylinder([0.0, 0.0, 0.0], 5.0, 10.0).to_sat_string(),
        )))
        .expect("add_entity(Solid3D) on a fresh document cannot fail");
    let operation = SolidHistoryOperation::Cylinder(SolidHistoryCylinder {
        height: 10.0,
        major_radius: 5.0,
        minor_radius: 5.0,
        x_radius: 5.0,
        ..SolidHistoryCylinder::default()
    });
    let graph = tree_doc.create_solid_history(handle, operation);
    assert!(graph.is_some(), "create_solid_history on a fresh solid must build a tree");
    write_fixture(tree_doc, "HistoryTree", &outdir).expect("write HistoryTree.dwg");

    // A BODY reusing the cylinder model (the third ACIS entity class).
    let mut body_doc = new_document();
    body_doc
        .add_entity(EntityType::Body(Body::from_sat(
            &build_cylinder([0.0, 0.0, 0.0], 5.0, 10.0).to_sat_string(),
        )))
        .expect("add_entity(Body) on a fresh document cannot fail");
    write_fixture(body_doc, "Body", &outdir).expect("write Body.dwg");
}
