//! DWG binary file format support.
//!
//! Read and write AutoCAD's native binary format.  DWG files use
//! bit-granularity encoding, version-specific data layouts, and LZ77
//! compression (R2004+).
//!
//! # Reading
//!
//! ```rust,ignore
//! use opencadcodec::DwgReader;
//!
//! let doc = DwgReader::from_file("drawing.dwg")?.read()?;
//! ```
//!
//! # Writing
//!
//! ```rust,ignore
//! use opencadcodec::DwgWriter;
//!
//! DwgWriter::write_to_file("output.dwg", &doc)?;
//! ```
//!
//! ## Supported versions
//!
//! | DWG Version | AutoCAD | File format  |
//! |-------------|---------|-------------|
//! | AC1012      | R13     | Linear      |
//! | AC1014      | R14     | Linear      |
//! | AC1015      | R2000   | Linear      |
//! | AC1018      | R2004   | Paged + LZ77 |
//! | AC1021      | R2007   | Paged + LZ77 |
//! | AC1024      | R2010   | Paged + LZ77 |
//! | AC1027      | R2013   | Paged + LZ77 |
//! | AC1032      | R2018   | Paged + LZ77 |

pub mod annotative_eed;
pub(crate) mod typeface_eed;
pub mod acds;
pub mod checksum;
pub mod compression;
pub mod compressor_ac21;
pub mod crc;
pub mod decompressor_ac18;
pub mod decompressor_ac21;
pub mod dwg21_metadata;
pub mod dwg_document_builder;
pub mod dwg_reader;
pub mod dwg_reference_type;
pub mod dwg_stream_readers;
pub mod dwg_stream_writers;
pub mod dwg_version;
pub mod dwg_writer;
pub mod eed_codec;
pub(crate) mod embedded_entity;
pub mod file_headers;
mod legacy_viewport;
mod parallel;
pub mod preview;
pub mod reed_solomon;
pub mod sh_tail_decode;

pub use dwg_reader::DwgReadOptions;
pub use dwg_reader::DwgReader;
pub use dwg_reference_type::DwgReferenceType;
pub use dwg_version::DwgVersion;
pub use dwg_writer::DwgWriter;

pub(crate) fn sab_fingerprint<'a>(
    entries: impl IntoIterator<Item = (crate::Handle, &'a [u8])>,
) -> Vec<(u64, usize, u64)> {
    use std::hash::{Hash, Hasher};

    let mut fingerprint = Vec::new();
    for (handle, bytes) in entries {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        fingerprint.push((handle.value(), bytes.len(), hasher.finish()));
    }
    fingerprint.sort_unstable_by_key(|entry| entry.0);
    fingerprint
}

/// Â§19 H8g: the document-state hash guarding EVERY whole-file echo
/// (the objects-stream echo, the AC21 compressed-page echo, the R2000
/// and AC18-family whole-file echoes) â€” the sorted per-part hash of
/// the semantic inventory's visit (the header variables, the ten
/// tables' records, the classes, the entities with their ownership /
/// extension-dictionary / reactor relationships, the objects, the
/// summary info, the preview, the EED side channel) plus the ten
/// table control handles and the retained metadata models (aux
/// header, template, file-dep list, rev history). Computed
/// identically at read time (the end-of-read capture, after every
/// section has loaded) and at the write gates: ANY document edit â€”
/// including the in-place field edits the handle-set fingerprints
/// cannot see (the issue-80 layer rename) â€” changes a part's Debug
/// projection and declines the echo. Per-part hashes are sorted
/// before combining, so the HashMap iteration orders of the
/// document's side channels do not perturb the result; reader-skipped
/// orphans (handles in the source map the builder never materialized)
/// are outside the visited parts on both sides, so an unedited
/// document engages despite them. Residual limitation: non-public
/// side-channel state outside the inventory and the metadata models
/// is outside the hash (a false decline is always safe â€” the echo
/// falls back to our own emission).
pub(crate) fn document_state_fingerprint(document: &crate::document::CadDocument) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut part_hashes: Vec<u64> = Vec::new();
    document
        .semantic_inventory_v1()
        .visit(|part| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", part).hash(&mut hasher);
            part_hashes.push(hasher.finish());
        });
    part_hashes.sort_unstable();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    part_hashes.len().hash(&mut hasher);
    for part_hash in part_hashes {
        part_hash.hash(&mut hasher);
    }
    // The ten table control handles (the inventory visits records
    // only), in a fixed order.
    for handle in [
        document.layers.handle(),
        document.line_types.handle(),
        document.text_styles.handle(),
        document.block_records.handle(),
        document.dim_styles.handle(),
        document.app_ids.handle(),
        document.views.handle(),
        document.vports.handle(),
        document.ucss.handle(),
        document.vx_table.handle(),
    ] {
        handle.value().hash(&mut hasher);
    }
    // The retained metadata models â€” plain summary structures, so
    // their Debug projections are order-stable.
    format!("{:?}", document.dwg_aux_header).hash(&mut hasher);
    format!("{:?}", document.dwg_template).hash(&mut hasher);
    format!("{:?}", document.dwg_file_dep_list).hash(&mut hasher);
    format!("{:?}", document.dwg_rev_history).hash(&mut hasher);
    hasher.finish()
}

/// Â§19 H7 CLASSES row: the state hash guarding the verbatim classes
/// re-emission â€” the ordered class identity tuple plus the document's
/// per-class object census (entities + objects resolved through the
/// class table, mirroring the writer's required-classes walk). Computed
/// identically at read time (the capture) and at write time (the gate):
/// any class-table edit or object-set change that touches a class's
/// instance census forces the sane re-encode.
pub(crate) fn classes_state_fingerprint(document: &crate::document::CadDocument) -> u64 {
    use std::hash::{Hash, Hasher};

    let census = document_class_census(document);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for class in document.classes.iter() {
        class.class_number.hash(&mut hasher);
        class.dxf_name.hash(&mut hasher);
        class.cpp_class_name.hash(&mut hasher);
        class.application_name.hash(&mut hasher);
        class.proxy_flags.0.hash(&mut hasher);
        class.was_zombie.hash(&mut hasher);
        class.is_an_entity.hash(&mut hasher);
        class.item_class_id.hash(&mut hasher);
        class.dwg_version.hash(&mut hasher);
        class.maintenance_version.hash(&mut hasher);
        class.unknown1.hash(&mut hasher);
        class.unknown2.hash(&mut hasher);
        census
            .get(&class.class_number)
            .copied()
            .unwrap_or(0)
            .hash(&mut hasher);
    }
    hasher.finish()
}

/// The document's per-class object census for the classes gate above:
/// every entity/object that resolves through the class table counts
/// under its class number. This is the document-state census (not the
/// object writer's write-time census, which cannot see the
/// raw-passthrough records) â€” self-consistency between the read-time
/// capture and the write-time gate is what matters.
///
/// The census must mirror the writer's full class-resolution landscape:
/// every `class_type_code` call in the object writer corresponds to a
/// class instance the raw classes passthrough claims a count for. The
/// 2026-10-02 AutoCAD manual-test round found the gap: only three entity
/// kinds were counted, so adding a helix, view border, section symbol,
/// wipeout, image, table, light or multileader to a same-version read
/// document left the classes fingerprint unchanged â€” the raw class bytes
/// re-emitted with the source file's zero-instance zombie state while
/// real records referenced those classes â€” and AutoCAD's strict
/// open-time validation refused the whole file (BricsCAD tolerated it).
/// Missing arms resolved to type code 0 or their fixed fallbacks wrote
/// the same, but the stale table is what a strict loader cannot accept:
/// count every class-resolved kind, over-counting being harmless (any
/// census change merely forces the sane re-encode).
fn document_class_census(
    document: &crate::document::CadDocument,
) -> std::collections::HashMap<i16, i32> {
    let mut counts: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
    let bump = |name: &str, counts: &mut std::collections::HashMap<i16, i32>| {
        if let Some(class) = document.classes.get_by_name(name) {
            *counts.entry(class.class_number).or_default() += 1;
        }
    };
    use crate::entities::EntityType;
    for entity in document.entities() {
        match entity {
            // â”€â”€ class-resolved entities, mirroring the writer dispatch â”€â”€
            EntityType::Surface(surface) => bump(surface.kind.dxf_name(), &mut counts),
            EntityType::Extended(entity) => bump(entity.class_name(), &mut counts),
            EntityType::Underlay(entity) => bump(entity.entity_name(), &mut counts),
            EntityType::SectionSymbol(_) => bump("SECTIONLINE", &mut counts),
            EntityType::ViewBorder(_) => bump("DRAWINGVIEW", &mut counts),
            EntityType::Helix(_) => bump("HELIX", &mut counts),
            EntityType::Mesh(_) => bump("MESH", &mut counts),
            EntityType::Table(_) => bump("ACAD_TABLE", &mut counts),
            EntityType::RasterImage(_) => bump("IMAGE", &mut counts),
            EntityType::Wipeout(_) => bump("WIPEOUT", &mut counts),
            EntityType::Light(_) => bump("LIGHT", &mut counts),
            EntityType::MultiLeader(_) => bump("MULTILEADER", &mut counts),
            EntityType::Ole2Frame(_) => bump("OLE2FRAME", &mut counts),
            // An mpolygon is a hatch whose record the writer routes through
            // the MPOLYGON class; plain hatches stay fixed-type.
            EntityType::Hatch(entity) => {
                if entity.is_mpolygon {
                    bump("MPOLYGON", &mut counts);
                }
            }
            // A view-rep insert is the one INSERT form that resolves through
            // its class; plain and MINSERT forms are fixed-type.
            EntityType::Insert(insert) => {
                if insert.view_rep_handle.is_some() {
                    bump("ACDBVIEWREPBLOCKREFERENCE", &mut counts);
                }
            }
            // The arc and large-radial dimension subtypes are class-resolved;
            // the rest of the dimension family is fixed-type.
            EntityType::Dimension(dimension) => match dimension {
                crate::entities::dimension::Dimension::Arc(_) => {
                    bump("ARC_DIMENSION", &mut counts)
                }
                crate::entities::dimension::Dimension::LargeRadial(_) => {
                    bump("LARGE_RADIAL_DIMENSION", &mut counts)
                }
                _ => {}
            },
            _ => {}
        }
    }
    for object in document.objects.values() {
        match object {
            // â”€â”€ class-resolved objects, mirroring the writer dispatch â”€â”€
            crate::objects::ObjectType::ClassObject(class_object) => {
                let name = class_object.dxf_name();
                if !name.is_empty() {
                    bump(name, &mut counts);
                }
            }
            crate::objects::ObjectType::UnderlayDefinition(def) => {
                bump(def.entity_name(), &mut counts)
            }
            crate::objects::ObjectType::ImageDefinition(_) => bump("IMAGEDEF", &mut counts),
            crate::objects::ObjectType::ImageDefinitionReactor(_) => {
                bump("IMAGEDEF_REACTOR", &mut counts)
            }
            crate::objects::ObjectType::MultiLeaderStyle(_) => {
                bump("MLEADERSTYLE", &mut counts)
            }
            crate::objects::ObjectType::TableContent(_) => bump("TABLECONTENT", &mut counts),
            crate::objects::ObjectType::SortEntitiesTable(_) => {
                bump("SORTENTSTABLE", &mut counts)
            }
            crate::objects::ObjectType::BlockVisibilityParameter(_) => {
                bump("BLOCKVISIBILITYPARAMETER", &mut counts)
            }
            crate::objects::ObjectType::ObjectContextData(data) => {
                bump(data.class_name(), &mut counts)
            }
            crate::objects::ObjectType::DgnLineStyle(object) => {
                bump(object.dxf_name(), &mut counts)
            }
            // Fixed-family classes: the records resolve through the class
            // table too (class_type_code with a fixed fallback), so their
            // instances count the same way.
            crate::objects::ObjectType::PlotSettings(_) => bump("PLOTSETTINGS", &mut counts),
            crate::objects::ObjectType::Scale(_) => bump("SCALE", &mut counts),
            crate::objects::ObjectType::DictionaryVariable(_) => {
                bump("DICTIONARYVAR", &mut counts)
            }
            crate::objects::ObjectType::DictionaryWithDefault(_) => {
                bump("ACDBDICTIONARYWDFLT", &mut counts)
            }
            crate::objects::ObjectType::XRecord(_) => bump("XRECORD", &mut counts),
            _ => {}
        }
    }
    counts
}
pub use file_headers::{DwgFileHeaderWriterAC15, DwgFileHeaderWriterAC18};
