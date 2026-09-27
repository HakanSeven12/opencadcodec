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

/// §19 H8d/H8e: the identity hash guarding the objects-section raw
/// echo — the sorted handle set of the DOCUMENT'S object universe
/// (`document_object_handles` below), computed identically at read
/// time (the post-build capture) and at write time (the gate): adds,
/// deletes and renumberings change the set and fall the echo back to
/// our own emission. Reader-skipped orphans — handles in the source's
/// handles map that the builder never materialized, the author's
/// edit-history residue (the ACIS-solids files' single orphan each,
/// ATMOS's 84) — are outside the universe on BOTH sides, so an
/// unedited document engages despite them. In-place field edits on an
/// unchanged handle set are outside this identity fingerprint (the
/// same limitation class as the classes fingerprint).
pub(crate) fn objects_handle_set_fingerprint(handles: impl Iterator<Item = u64>) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut sorted: Vec<u64> = handles.collect();
    sorted.sort_unstable();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    sorted.len().hash(&mut hasher);
    for handle in sorted {
        handle.hash(&mut hasher);
    }
    hasher.finish()
}

/// §19 H8e: the document's object universe — every handle the object
/// writer's walk can emit: the entities, the objects map, and the ten
/// tables' control + record handles. The objects-echo identity gate
/// hashes this set on both sides (read-time capture post-build,
/// write-time gate), so emission-side drops (orphan records the
/// ownership walk never reaches) do not falsely decline an unedited
/// document the way the emission-vs-source-map comparison did.
pub(crate) fn document_object_handles(document: &crate::document::CadDocument) -> Vec<u64> {
    let mut handles =
        Vec::with_capacity(document.entities.len() + document.objects.len() + 64);
    for entity in &document.entities {
        handles.push(entity.common().handle.value());
    }
    for handle in document.objects.keys() {
        handles.push(handle.value());
    }
    macro_rules! table_handles {
        ($table:expr) => {{
            handles.push($table.handle().value());
            for record in $table.iter() {
                handles.push(record.handle.value());
            }
        }};
    }
    table_handles!(document.layers);
    table_handles!(document.line_types);
    table_handles!(document.text_styles);
    table_handles!(document.block_records);
    table_handles!(document.dim_styles);
    table_handles!(document.app_ids);
    table_handles!(document.views);
    table_handles!(document.vports);
    table_handles!(document.ucss);
    table_handles!(document.vx_table);
    handles
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
