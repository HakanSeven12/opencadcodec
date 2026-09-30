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
    // The full raw-stream mirror of the authored plain R2018 region
    // (2026-09-30, the chimera blame-split's SAB arm — the candidate-6
    // sheet mirror was REFUTED by the restore_gap_diffs travel audit:
    // the authored loop walks CCW around its effective normal
    // (+25.8M dot) because her CANONICAL edge ring is wound CW — each
    // curve's origin at its edge's start vertex, params 0→length —
    // and her four REVERSED coedges then walk it CCW; the candidate-6
    // "reversed ring" over CCW canonicals doubly inverted the
    // travel (−200). Her cell, mirrored verbatim:
    //   v14=top-left   v15=top-right   v18=bottom-left   v20=bottom-right
    //   e10 top L→R    e12 left B→T    e17 bottom R→L   e13 right T→B
    //   chain (by next): co(top)→co(left)→co(bottom)→co(right), ffff
    // plus the sheet flags: sideness DOUBLE (her f token — her solid
    // faces carry single, the sheets double) and the face's ninth
    // containment token completing at the SAB boundary
    // (complete_class_width ("face", 8)); her vertex edge-backptrs
    // mirrored (TL/TR→top, BL→bottom, BR→right) with the vertex int 2.
    let mut sat = SatDocument::new_body();
    let body_idx = SatPointer::new(0);
    let ptr = |i: i32| SatPointer::new(i);

    // Corners of a 10x10 square in the XY plane.
    let p_bl = sat.add_point(0.0, 0.0, 0.0);
    let p_br = sat.add_point(10.0, 0.0, 0.0);
    let p_tl = sat.add_point(0.0, 10.0, 0.0);
    let p_tr = sat.add_point(10.0, 10.0, 0.0);

    // The plane origin at the rectangle's centre — her surface origin
    // (−5234,1969) is her rectangle's centre, not a corner.
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

    // Her canonical ring, CW as a cycle (T→R→B→L): the four edges.
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
            SatPointer::NULL,
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
    // The authored SHEET face's ninth token — her plain R2018 region's
    // face carries a third bool after sense+sidedness that the SOLID
    // faces do not (the authored Box 2007/2010/2013/2018 census: the
    // solid faces are 8-token; her region face `... 0b 0a 0b` is 9) —
    // a face-attrib-form distinction the fixed-width SAB class reader
    // reads positionally, so a region assembled without it desyncs a
    // strict restorer. Added here on the region, never class-wide.
    if let Some(r) = sat.record_mut(face_idx as usize) {
        r.tokens.push(SatToken::True);
    }
    sat.add_shell(ptr(face_idx), ptr(lump_idx));
    sat.add_lump(ptr(shell_idx), body_idx);

    if let Some(body_rec) = sat.record_mut(0) {
        body_rec.tokens[1] = SatToken::Pointer(ptr(lump_idx));
    }

    // Back-pointers: edge → its chain coedge; vertex → its edge after
    // HER mapping, and the authored sheet vertex int 2 (the sheet
    // census; the solid census's 0/1 roles do not apply to sheets).
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
