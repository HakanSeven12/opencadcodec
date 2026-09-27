//! DWG binary file format support.
//!
//! Read and write AutoCAD's native binary format.  DWG files use
//! bit-granularity encoding, version-specific data layouts, and LZ77
//! compression (R2004+).
//!
//! # Reading
//!
//! ```rust,ignore
//! use acadrust::DwgReader;
//!
//! let doc = DwgReader::from_file("drawing.dwg")?.read()?;
//! ```
//!
//! # Writing
//!
//! ```rust,ignore
//! use acadrust::DwgWriter;
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

/// §19 H8g: the document-state hash guarding EVERY whole-file echo
/// (the objects-stream echo, the AC21 compressed-page echo, the R2000
/// and AC18-family whole-file echoes) — the sorted per-part hash of
/// the semantic inventory's visit (the header variables, the ten
/// tables' records, the classes, the entities with their ownership /
/// extension-dictionary / reactor relationships, the objects, the
/// summary info, the preview, the EED side channel) plus the ten
/// table control handles and the retained metadata models (aux
/// header, template, file-dep list, rev history). Computed
/// identically at read time (the end-of-read capture, after every
/// section has loaded) and at the write gates: ANY document edit —
/// including the in-place field edits the handle-set fingerprints
/// cannot see (the issue-80 layer rename) — changes a part's Debug
/// projection and declines the echo. Per-part hashes are sorted
/// before combining, so the HashMap iteration orders of the
/// document's side channels do not perturb the result; reader-skipped
/// orphans (handles in the source map the builder never materialized)
/// are outside the visited parts on both sides, so an unedited
/// document engages despite them. Residual limitation: non-public
/// side-channel state outside the inventory and the metadata models
/// is outside the hash (a false decline is always safe — the echo
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
    // The retained metadata models — plain summary structures, so
    // their Debug projections are order-stable.
    format!("{:?}", document.dwg_aux_header).hash(&mut hasher);
    format!("{:?}", document.dwg_template).hash(&mut hasher);
    format!("{:?}", document.dwg_file_dep_list).hash(&mut hasher);
    format!("{:?}", document.dwg_rev_history).hash(&mut hasher);
    hasher.finish()
}

/// §19 H7 CLASSES row: the state hash guarding the verbatim classes
/// re-emission — the ordered class identity tuple plus the document's
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
/// raw-passthrough records) — self-consistency between the read-time
/// capture and the write-time gate is what matters.
fn document_class_census(
    document: &crate::document::CadDocument,
) -> std::collections::HashMap<i16, i32> {
    let mut counts: std::collections::HashMap<i16, i32> = std::collections::HashMap::new();
    let bump = |name: &str, counts: &mut std::collections::HashMap<i16, i32>| {
        if let Some(class) = document.classes.get_by_name(name) {
            *counts.entry(class.class_number).or_default() += 1;
        }
    };
    for entity in document.entities() {
        match entity {
            crate::entities::EntityType::Surface(surface) => {
                bump(surface.kind.dxf_name(), &mut counts)
            }
            crate::entities::EntityType::Extended(entity) => {
                bump(entity.class_name(), &mut counts)
            }
            crate::entities::EntityType::Underlay(entity) => {
                bump(entity.entity_name(), &mut counts)
            }
            _ => {}
        }
    }
    for object in document.objects.values() {
        if let crate::objects::ObjectType::ClassObject(class_object) = object {
            let name = class_object.dxf_name();
            if !name.is_empty() {
                bump(name, &mut counts);
            }
        }
    }
    counts
}
pub use file_headers::{DwgFileHeaderWriterAC15, DwgFileHeaderWriterAC18};
