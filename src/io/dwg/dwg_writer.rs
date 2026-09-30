//! Top-level DWG file writer
//!
//! Orchestrates all section writers to produce a complete DWG binary file.
//! Supports three file formats:
//!
//! - **AC15** (R13/R14/R2000): Linear format with sequential sections
//! - **AC18** (R2004/R2010+): Page-based format with LZ77 compression
//! - **AC21** (R2007): RS-encoded pages with LZ77 AC21 compression and CRC-64
//!
//! ## Usage
//!
//! ```no_run
//! use acadrust::document::CadDocument;
//! use acadrust::io::dwg::DwgWriter;
//!
//! let doc = CadDocument::new();
//! DwgWriter::write_to_file("output.dwg", &doc).unwrap();
//! ```
//!
//! Based on the reference `DwgWriter` class.

use std::fs::File;
use std::io::{BufWriter, Cursor, Seek, Write};
use std::path::Path;

use crate::document::{CadDocument, HeaderVariables};
use crate::error::{DxfError, Result};
use crate::types::{DxfVersion, Handle};

use super::dwg_stream_writers::{
    app_info_writer, aux_header_writer, classes_writer, handle_writer, header_writer,
    DwgObjectWriter,
};
use super::file_headers::{
    section_names, DwgFileHeaderWriterAC15, DwgFileHeaderWriterAC18, DwgFileHeaderWriterAC21,
};

// ════════════════════════════════════════════════════════════════════════════
//  Public API
// ════════════════════════════════════════════════════════════════════════════

/// DWG binary file writer.
///
/// Produces a complete DWG file from a [`CadDocument`].
/// The output version is determined by [`CadDocument::version`].
pub struct DwgWriter;

impl DwgWriter {
    /// Write a DWG file to the given path.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The version is `Unknown`
    /// - An I/O error occurs
    /// - The document contains invalid data
    pub fn write_to_file<P: AsRef<Path>>(path: P, document: &CadDocument) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Self::write_to_writer(writer, document)
    }

    /// Write a DWG file to any `Write + Seek` output.
    pub fn write_to_writer<W: Write + Seek>(mut output: W, document: &CadDocument) -> Result<()> {
        // ── §19 H8g: the whole-file echo — every container family ──
        // The H8e-2/H8f/H8g doctrine unified: when the document came
        // from a same-version DWG read, her whole on-disk file is
        // retained, and the document-state hash holds (§19 H8g: the
        // full semantic content, so ANY edit — including in-place
        // field edits — declines), the rewrite re-emits her bytes
        // verbatim. The gate runs BEFORE the prepare pipeline (the
        // writer's own fixups — table-key resync, database-reference
        // repair, surface-class preparation — are not user edits) and
        // before any emission work. Edited documents and conversions
        // never reach this arm; the mirrored and conventional paths
        // stay for every other case, and the read axis (0/0
        // corpus-wide) independently verifies the model the echo
        // bypasses.
        // The pre-prepare state verdict (computed once): the AC21
        // objects echo and the whole-file echo below both consume it —
        // the impls run after the prepare pipeline, which may repair
        // the document, and those repairs are not user edits.
        // `DWG_NO_ECHO` (§19 H8h diagnostics): force every echo arm
        // to decline so the conventional compressed emission runs —
        // the record-level study path (the α-era measurement form).
        let state_matches = document.dwg_state_fingerprint != 0
            && super::document_state_fingerprint(document)
                == document.dwg_state_fingerprint
            && std::env::var_os("DWG_NO_ECHO").is_none();
        if document.dwg_source_version == Some(document.version) {
            if let Some(tail) = document.raw_ac21_tail.as_deref() {
                if state_matches {
                    if std::env::var_os("AC21_MIRROR_DEBUG").is_some() {
                        eprintln!(
                            "[dwg-echo] full echo ENGAGED — her file {} bytes",
                            tail.len()
                        );
                    }
                    output.seek(std::io::SeekFrom::Start(0))?;
                    output.write_all(tail)?;
                    output.seek(std::io::SeekFrom::End(0))?;
                    return Ok(());
                }
                if std::env::var_os("AC21_MIRROR_DEBUG").is_some() {
                    eprintln!("[dwg-echo] full echo DECLINED — document state changed");
                }
            }
        }
        let mut prepared = crate::io::loft_parameters::prepared(document);
        prepare_surface_classes(&mut prepared);
        prepare_database_references(&mut prepared);
        prepare_table_keys(&mut prepared);
        let document = prepared.as_ref();
        let perf = std::env::var_os("PERF").is_some();
        let started = web_time::Instant::now();
        validate_version(document.version)?;
        let version = document.version;

        // Programmatically added table entries may carry NULL handles
        // (Layer::new + layers.add never assigns one). DWG table records
        // must each carry a real handle - a record written under handle 0
        // is dropped by the handle map and disappears from the re-opened
        // drawing. Assign fresh handles on a cloned document up front so
        // the controls, entries and handle map all agree (issue #51/#64
        // class of bug).
        let mut owned;
        let document: &CadDocument = if document.has_null_table_entries()
            || document.version < DxfVersion::AC1027
        {
            owned = document.clone();
            if owned.has_null_table_entries() {
                owned.assign_table_entry_handles();
            }
            if owned.version < DxfVersion::AC1027 {
                // A same-version DWG→DWG round trip must keep the source
                // file's class table VERBATIM: document.classes was read
                // from that very file and every class-indirected object
                // (ACSH_* shells, evaluation graphs, render entries,
                // dynamic-block evaluation nodes, …) resolves by its
                // ORIGINAL class number. The legacy-table prune below
                // renumbers survivors via add_or_update and leaves the
                // pruned classes' records falling back to type 500
                // (ACDBDICTIONARYWDFLT), re-typing every such object in
                // the re-read file (harness class: DYB→WDFLT counterfeits,
                // 186 records on ATMOS-DC22S alone).
                let same_version_roundtrip =
                    owned.dwg_source_version == Some(owned.version);
                if same_version_roundtrip {
                    // The file's own dictionaries are all legal in this
                    // version; nothing here may be stripped or renumbered.
                } else {
                let required: Vec<_> = owned
                    .entities()
                    .filter_map(|entity| {
                        let name = match entity {
                            crate::entities::EntityType::Surface(surface) => {
                                surface.kind.dxf_name()
                            }
                            crate::entities::EntityType::Extended(entity) => entity.class_name(),
                            crate::entities::EntityType::Underlay(entity) => entity.entity_name(),
                            _ => entity.as_entity().entity_type(),
                        };
                        owned.classes.get_by_name(name).cloned()
                    })
                    .collect();
                // Native class-registered objects (e.g. ACDBSECTIONVIEWSTYLE,
                // ACDBDETAILVIEWSTYLE) are not part of the legacy class table,
                // but if they have live instances they must remain so the writer
                // can emit the correct type code instead of falling back to 500.
                let required_object_classes: Vec<_> = owned
                    .objects
                    .values()
                    .filter_map(|obj| {
                        if let crate::objects::ObjectType::ClassObject(co) = obj {
                            let name = co.dxf_name();
                            if !name.is_empty() {
                                return owned.classes.get_by_name(name).cloned();
                            }
                        }
                        None
                    })
                    .collect();
                owned.classes.retain_legacy_dwg_classes();
                for mut class in required
                    .into_iter()
                    .chain(required_object_classes.into_iter())
                {
                    if !owned.classes.contains(&class.dxf_name) {
                        class.class_number = 0;
                        owned.classes.add_or_update(class);
                    }
                }
                prepare_legacy_document(&mut owned);
                }
            }
            &owned
        } else {
            document
        };

        let result = if uses_ac21_format(version) {
            write_ac21(&mut output, document, version, state_matches)
        } else if uses_paged_format(version) {
            write_ac18(&mut output, document, version)
        } else {
            write_ac15(&mut output, document, version)
        };
        if perf {
            eprintln!(
                "[perf] dwg-write total={:.1}ms version={:?} entities={} objects={}",
                started.elapsed().as_secs_f64() * 1000.0,
                version,
                document.entities().count(),
                document.objects.len(),
            );
        }
        result
    }

    /// Write an AC21 DWG file **without LZ77 compression** (diagnostic).
    ///
    /// Pages are still RS-encoded but LZ77 is bypassed, storing raw data.
    /// Useful for isolating whether a read error is caused by compression
    /// or by the object data itself.
    pub fn write_to_file_no_lz77<P: AsRef<Path>>(path: P, document: &CadDocument) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Self::write_to_writer_no_lz77(writer, document)
    }

    /// Write an AC21 DWG without LZ77 to any `Write + Seek` output.
    pub fn write_to_writer_no_lz77<W: Write + Seek>(
        mut output: W,
        document: &CadDocument,
    ) -> Result<()> {
        validate_version(document.version)?;
        let mut prepared = crate::io::loft_parameters::prepared(document);
        prepare_surface_classes(&mut prepared);
        prepare_database_references(&mut prepared);
        prepare_table_keys(&mut prepared);
        write_ac21_impl(&mut output, prepared.as_ref(), document.version, true, false)
    }

    /// Write a DWG file to a byte vector (useful for testing).
    pub fn write_to_vec(document: &CadDocument) -> Result<Vec<u8>> {
        let mut buffer = Cursor::new(Vec::new());
        Self::write_to_writer(&mut buffer, document)?;
        Ok(buffer.into_inner())
    }
}

/// Repair relationships that are stored in both directions in a DWG database.
/// The caller's document stays unchanged; corrections exist only in the output
/// copy.
pub(crate) fn prepare_database_references(document: &mut std::borrow::Cow<'_, CadDocument>) {
    use crate::entities::EntityType;
    use crate::objects::ObjectType;
    use std::collections::{HashMap, HashSet};

    let mut missing_hatch_reactors = Vec::new();
    let mut invalid_hatches = Vec::new();
    let mut table_repairs = Vec::new();
    let mut table_style_repairs = Vec::new();
    let mut mleader_style_repairs = Vec::new();
    let mut mtext_attachment_repairs = Vec::new();
    let mut mline_repairs = Vec::new();
    let mut underlay_reactors = Vec::new();
    for entity in document.entities() {
        match entity {
            EntityType::Hatch(hatch) => {
                if !hatch.is_associative {
                    continue;
                }
                let boundaries: Vec<Handle> = hatch
                    .paths
                    .iter()
                    .flat_map(|path| path.boundary_handles.iter().copied())
                    .collect();
                if boundaries.is_empty()
                    || boundaries
                        .iter()
                        .any(|handle| document.get_entity(*handle).is_none())
                {
                    invalid_hatches.push(hatch.common.handle);
                    continue;
                }
                for boundary_handle in boundaries {
                    if document
                        .get_entity(boundary_handle)
                        .is_some_and(|boundary| {
                            !boundary.common().reactors.contains(&hatch.common.handle)
                        })
                    {
                        missing_hatch_reactors.push((boundary_handle, hatch.common.handle));
                    }
                }
            }
            EntityType::Table(table) => {
                if table
                    .table_style_handle
                    .is_none_or(|handle| handle.is_null())
                {
                    let style = document
                        .objects
                        .get(&document.header.named_objects_dict_handle)
                        .and_then(|object| match object {
                            ObjectType::Dictionary(root) => root.get("ACAD_TABLESTYLE"),
                            _ => None,
                        })
                        .and_then(|handle| document.objects.get(&handle))
                        .and_then(|object| match object {
                            ObjectType::Dictionary(styles) => styles
                                .get(&document.header.current_table_style_name)
                                .or_else(|| styles.get("Standard")),
                            _ => None,
                        })
                        .filter(|handle| {
                            matches!(
                                document.objects.get(handle),
                                Some(ObjectType::TableStyle(_))
                            )
                        });
                    if let Some(style) = style {
                        table_style_repairs.push((table.common.handle, style));
                    }
                }
                let resolved = table
                    .block_record_handle
                    .filter(|handle| !handle.is_null())
                    .and_then(|handle| {
                        document
                            .block_records
                            .iter()
                            .find(|record| record.handle == handle)
                    })
                    .or_else(|| document.block_records.get(&table.block_name));
                if resolved.is_none_or(|record| {
                    table.block_record_handle != Some(record.handle)
                        || table.block_name != record.name
                        || record.handle.is_null()
                        || record.block_entity_handle.is_null()
                        || record.block_end_handle.is_null()
                }) {
                    table_repairs.push((
                        table.common.handle,
                        resolved
                            .map(|record| record.name.clone())
                            .unwrap_or_else(|| table.block_name.clone()),
                    ));
                }
            }
            EntityType::MultiLeader(mleader)
                if mleader.style_handle.is_none_or(|handle| handle.is_null()) =>
            {
                // A native AcDbMLeader references a resolvable MLEADERSTYLE
                // — a null pointer is an invalid authored state that both
                // loaders' audits repair (the gen_all canonical's verdict:
                // "LeaderStyle Id is Null", 3 fixed at every open). Resolve
                // the document's current/Standard style through the
                // ACAD_MLEADERSTYLE dictionary (the MLine/Table repair
                // pattern).
                let style = document
                    .objects
                    .get(&document.header.named_objects_dict_handle)
                    .and_then(|object| match object {
                        ObjectType::Dictionary(root) => root.get("ACAD_MLEADERSTYLE"),
                        _ => None,
                    })
                    .and_then(|handle| document.objects.get(&handle))
                    .and_then(|object| match object {
                        ObjectType::Dictionary(styles) => styles
                            .get(&document.header.current_mleader_style_name)
                            .or_else(|| styles.get("Standard")),
                        _ => None,
                    })
                    .filter(|handle| {
                        matches!(
                            document.objects.get(handle),
                            Some(ObjectType::MultiLeaderStyle(_))
                        )
                    });
                if let Some(style) = style {
                    mleader_style_repairs.push((mleader.common.handle, style));
                }
            }
            EntityType::MText(mtext) if mtext.ignore_attachment == 0 => {
                // The R2018+ redundant-block header BL repeats the
                // ABSOLUTE ATTACHMENT POINT (despite gold's misleading
                // `ignore_attachment` name — the census measured the
                // authored genus: TopLeft→1, MiddleCenter→5). A
                // DWG-read captures the raw value verbatim; a
                // CONSTRUCTED MText carries the default 0 — a corrupt
                // repetition AutoCAD's audit repairs at every open
                // (2026-09-30: "AcDbMText(3B)/(40) was repaired / 2
                // fixed"; the audit-repaired staged copy carried
                // ignore_attachment=1 = TopLeft, the true repeat).
                // Normalize only the degenerate zero — non-zero
                // captures write verbatim (the corpus record identity
                // holds; the era census re-verification is the gate).
                let repeated = mtext.attachment_point as i32;
                if repeated != 0 {
                    mtext_attachment_repairs.push((mtext.common.handle, repeated));
                }
            }
            EntityType::MLine(mline)
                if mline.style_handle.is_none_or(|handle| handle.is_null()) =>
            {
                let style = document
                    .objects
                    .iter()
                    .find_map(|(handle, object)| match object {
                        ObjectType::MLineStyle(style)
                            if style.name.eq_ignore_ascii_case(&mline.style_name) =>
                        {
                            Some(*handle)
                        }
                        _ => None,
                    })
                    .unwrap_or(document.header.current_multiline_style_handle);
                if !style.is_null() {
                    mline_repairs.push((mline.common.handle, style));
                }
            }
            EntityType::Underlay(underlay) => {
                if let Some(ObjectType::UnderlayDefinition(definition)) =
                    document.objects.get(&underlay.definition_handle)
                {
                    if !definition.reactors.contains(&underlay.common.handle) {
                        underlay_reactors
                            .push((underlay.definition_handle, underlay.common.handle));
                    }
                }
            }
            _ => {}
        }
    }

    let root_handle = document.header.named_objects_dict_handle;
    let layout_dictionary = document
        .objects
        .get(&root_handle)
        .and_then(|object| match object {
            ObjectType::Dictionary(dictionary) => dictionary.get("ACAD_LAYOUT"),
            _ => None,
        })
        .unwrap_or(document.header.acad_layout_dict_handle);
    let layout_names: HashMap<Handle, String> = document
        .objects
        .iter()
        .filter_map(|(handle, object)| match object {
            ObjectType::Layout(layout) => Some((*handle, layout.name.clone())),
            _ => None,
        })
        .collect();
    let layout_dictionary_needs_repair = document
        .objects
        .get(&layout_dictionary)
        .and_then(|object| match object {
            ObjectType::Dictionary(dictionary) => Some(dictionary),
            _ => None,
        })
        .is_some_and(|dictionary| {
            let targets: HashSet<Handle> = dictionary
                .entries
                .iter()
                .filter_map(|(key, handle)| {
                    layout_names
                        .get(handle)
                        .filter(|name| *name == key)
                        .map(|_| *handle)
                })
                .collect();
            dictionary.entries.len() != targets.len() || targets.len() != layout_names.len()
        });

    if missing_hatch_reactors.is_empty()
        && invalid_hatches.is_empty()
        && table_repairs.is_empty()
        && table_style_repairs.is_empty()
        && mleader_style_repairs.is_empty()
        && mtext_attachment_repairs.is_empty()
        && mline_repairs.is_empty()
        && underlay_reactors.is_empty()
        && !layout_dictionary_needs_repair
    {
        return;
    }

    let output = document.to_mut();
    for (handle, repeated) in mtext_attachment_repairs {
        if let Some(EntityType::MText(mtext)) = output.get_entity_mut(handle) {
            mtext.ignore_attachment = repeated;
        }
    }
    for (handle, style) in mleader_style_repairs {
        if let Some(EntityType::MultiLeader(mleader)) = output.get_entity_mut(handle) {
            mleader.style_handle = Some(style);
        }
    }
    for (handle, style) in table_style_repairs {
        if let Some(EntityType::Table(table)) = output.get_entity_mut(handle) {
            table.table_style_handle = Some(style);
        }
    }
    for (definition, reactor) in underlay_reactors {
        if let Some(ObjectType::UnderlayDefinition(definition)) =
            output.objects.get_mut(&definition)
        {
            if !definition.reactors.contains(&reactor) {
                definition.reactors.push(reactor);
            }
        }
    }
    for (handle, style) in mline_repairs {
        if let Some(EntityType::MLine(mline)) = output.get_entity_mut(handle) {
            mline.style_handle = Some(style);
        }
    }
    if !table_repairs.is_empty() {
        output.synchronize_handle_allocator();
        for (table_handle, requested_name) in table_repairs {
            let existing = if !requested_name.is_empty() {
                output.block_records.get(&requested_name).cloned()
            } else {
                None
            };
            let (block_record_handle, block_name) = if let Some(mut record) = existing {
                if record.name.starts_with("*T") {
                    record.flags.anonymous = true;
                }
                if record.handle.is_null() {
                    record.handle = output.allocate_handle();
                }
                if record.block_entity_handle.is_null() {
                    record.block_entity_handle = output.allocate_handle();
                }
                if record.block_end_handle.is_null() {
                    record.block_end_handle = output.allocate_handle();
                }
                let result = (record.handle, record.name.clone());
                output.block_records.add_or_replace(record);
                result
            } else {
                let name = if requested_name.is_empty() {
                    let mut index = 1;
                    while output.block_records.get(&format!("*T{index}")).is_some() {
                        index += 1;
                    }
                    format!("*T{index}")
                } else {
                    requested_name
                };
                let mut record = crate::tables::BlockRecord::new(&name);
                record.handle = output.allocate_handle();
                record.block_entity_handle = output.allocate_handle();
                record.block_end_handle = output.allocate_handle();
                record.flags.anonymous = name.starts_with('*');
                let handle = record.handle;
                output
                    .block_records
                    .add(record)
                    .expect("new table block name is unique");
                (handle, name)
            };
            if let Some(EntityType::Table(table)) = output.get_entity_mut(table_handle) {
                table.block_record_handle = Some(block_record_handle);
                table.block_name = block_name;
            }
        }
    }
    for (boundary_handle, hatch_handle) in missing_hatch_reactors {
        if let Some(boundary) = output.get_entity_mut(boundary_handle) {
            if !boundary.common().reactors.contains(&hatch_handle) {
                boundary.common_mut().reactors.push(hatch_handle);
            }
        }
    }
    for hatch_handle in invalid_hatches {
        if let Some(EntityType::Hatch(hatch)) = output.get_entity_mut(hatch_handle) {
            hatch.is_associative = false;
            for path in &mut hatch.paths {
                path.boundary_handles.clear();
            }
        }
    }

    if layout_dictionary_needs_repair {
        if let Some(ObjectType::Dictionary(dictionary)) = output.objects.get_mut(&layout_dictionary)
        {
            dictionary.entries.clear();
            let mut layouts: Vec<_> = layout_names.into_iter().collect();
            layouts.sort_by_key(|(handle, _)| handle.value());
            let mut names = HashSet::new();
            for (handle, name) in layouts {
                if names.insert(name.clone()) {
                    dictionary.entries.push((name, handle));
                }
            }
        }
        for object in output.objects.values_mut() {
            if let ObjectType::Layout(layout) = object {
                layout.owner = layout_dictionary;
            }
        }
    }
}

/// Surface records use class numbers, not fixed object codes. Documents created
/// from scratch or opened before a surface subtype was added may lack its class.
/// Append only missing classes on the output copy, retaining existing class
/// order, numbers, and metadata for every other object in the drawing.
fn prepare_surface_classes(document: &mut std::borrow::Cow<'_, CadDocument>) {
    use crate::entities::{EntityType, SurfaceKind};

    let mut missing = Vec::new();
    for entity in document.entities() {
        let EntityType::Surface(surface) = entity else {
            continue;
        };
        let name = surface.kind.dxf_name();
        if document.classes.contains(name) || missing.iter().any(|(dxf, _)| *dxf == name) {
            continue;
        }
        let cpp = match surface.kind {
            SurfaceKind::Generic => "AcDbSurface",
            SurfaceKind::Plane => "AcDbPlaneSurface",
            SurfaceKind::Extruded => "AcDbExtrudedSurface",
            SurfaceKind::Lofted => "AcDbLoftedSurface",
            SurfaceKind::Revolved => "AcDbRevolvedSurface",
            SurfaceKind::Swept => "AcDbSweptSurface",
            SurfaceKind::Nurb => "AcDbNurbSurface",
        };
        missing.push((name, cpp));
    }
    if missing.is_empty() {
        return;
    }
    let output = document.to_mut();
    for (name, cpp) in missing {
        output
            .classes
            .add_or_update(crate::classes::DxfClass::new_entity(name, cpp));
    }
}

/// Re-key symbol-table entries that were renamed in place.
///
/// Tables key entries by the name captured at insertion, so a caller that
/// assigns `layer.name` through `layers.iter_mut()` leaves the entry reachable
/// only under its old name. The entity writer resolves an entity's layer by
/// name, so every entity on that layer would be written with a NULL layer hard
/// pointer — a required reference — and produces an invalid drawing (issue
/// #80). Repair the output copy only; the caller's document is untouched.
///
/// Shared with the DXF writer: the stale key desyncs the same name lookups
/// there, leaving entities pointing at a layer name the LAYER table no longer
/// defines.
pub(crate) fn prepare_table_keys(document: &mut std::borrow::Cow<'_, CadDocument>) {
    if !document.has_stale_table_keys() {
        return;
    }
    document.to_mut().resync_table_keys();
}

/// Remove style dictionaries only before the versions introducing their
/// objects: TABLESTYLE in R2004 and MLEADERSTYLE in R2007.
fn prepare_legacy_document(document: &mut CadDocument) {
    use crate::objects::ObjectType;
    if document.version <= DxfVersion::AC1014 {
        let missing: Vec<_> = document
            .entities()
            .filter_map(|entity| match entity {
                crate::entities::EntityType::Viewport(viewport)
                    if !document
                        .vx_table
                        .iter()
                        .any(|record| record.viewport == viewport.common.handle) =>
                {
                    Some((viewport.common.handle, viewport.status.is_on))
                }
                _ => None,
            })
            .collect();
        if !missing.is_empty() && document.vx_table.is_empty() {
            let mut reserved = crate::tables::VxTableRecord::new("");
            reserved.handle = document.allocate_handle();
            reserved.is_xref_reference = true;
            document.vx_table.add_allow_duplicate(reserved);
        }
        for (viewport, is_on) in missing {
            let first = document.header.current_vx_handle.is_null();
            let mut record = crate::tables::VxTableRecord::new(if first { "1" } else { "" });
            record.handle = document.allocate_handle();
            record.viewport = viewport;
            record.is_on = is_on;
            record.is_xref_reference = true;
            if first {
                document.header.current_vx_handle = record.handle;
            }
            document.vx_table.add_allow_duplicate(record);
        }
    }

    let root_handle = document.header.named_objects_dict_handle;
    let mut obsolete = Vec::new();
    if let Some(ObjectType::Dictionary(root)) = document.objects.get_mut(&root_handle) {
        root.entries.retain(|(name, handle)| {
            let remove = (name == "ACAD_MLEADERSTYLE" && document.version < DxfVersion::AC1021)
                || (name == "ACAD_TABLESTYLE" && document.version < DxfVersion::AC1018);
            if remove {
                obsolete.push(*handle);
            }
            !remove
        });
    }

    for handle in obsolete {
        if let Some(ObjectType::Dictionary(dictionary)) = document.objects.remove(&handle) {
            for (_, child) in dictionary.entries {
                document.objects.remove(&child);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  Validation
// ════════════════════════════════════════════════════════════════════════════

/// Validate that the document version is supported for DWG writing.
fn validate_version(version: DxfVersion) -> Result<()> {
    match version {
        DxfVersion::Unknown => Err(DxfError::UnsupportedVersion("Unknown version".to_string())),
        _ => Ok(()),
    }
}

/// Build class records from the objects that were actually encoded.
///
/// A zero-version class with no emitted instances is an unloaded declaration,
/// not a live runtime class. Describing it as live leaves class dictionary
/// slots that a database audit has to replace with dummy entries. Fixed object
/// classes remain registered even when their records use fixed type codes.
fn reconciled_classes(
    document: &CadDocument,
    instance_counts: &std::collections::HashMap<i16, i32>,
    counts_complete: bool,
) -> Vec<crate::classes::DxfClass> {
    const FIXED_CLASSES: &[&str] = &[
        "ACDBDICTIONARYWDFLT",
        "DICTIONARYVAR",
        "LAYOUT",
        "ACDBPLACEHOLDER",
        "PLOTSETTINGS",
        "SCALE",
    ];

    document
        .classes
        .iter()
        .cloned()
        .map(|mut class| {
            if let Some(&count) = instance_counts.get(&class.class_number) {
                class.instance_count = count;
                class.was_zombie = false;
            } else if counts_complete {
                class.instance_count = 0;
                if class.dwg_version == 0
                    && class.maintenance_version == 0
                    && !FIXED_CLASSES
                        .iter()
                        .any(|name| class.dxf_name.eq_ignore_ascii_case(name))
                {
                    class.was_zombie = true;
                }
            }
            class
        })
        .collect()
}

fn acds_data<'a>(
    document: &'a CadDocument,
    version: DxfVersion,
    sab_entries: &[(Handle, Vec<u8>)],
) -> std::borrow::Cow<'a, [u8]> {
    let fingerprint = super::sab_fingerprint(
        sab_entries
            .iter()
            .map(|(handle, bytes)| (*handle, bytes.as_slice())),
    );
    if document.dwg_source_version == Some(version) && fingerprint == document.raw_acds_fingerprint
    {
        if let Some(raw) = document.raw_acds_data.as_deref() {
            return std::borrow::Cow::Borrowed(raw.as_slice());
        }
    }
    // The thumbnail record's owner: the Model Layout object (her row 0's
    // handle IS her layout object's — measured 2026-09-30, Box_2018 +
    // example_2018 both at 0x22). The 2026-09-30 ACAD re-probe evidence:
    // AutoCAD's open-time validation prompts RECOVER at the empty
    // schema-0 search block — the block is empty only because the
    // container carries no thumbnail row.
    let thumbnail = document
        .objects
        .iter()
        .find_map(|(handle, object)| match object {
            crate::objects::ObjectType::Layout(layout) if layout.name == "Model" => {
                Some(handle.value() as u32)
            }
            _ => None,
        });
    std::borrow::Cow::Owned(build_acds_prototype(sab_entries, version, thumbnail))
}

/// §19 H7 CLASSES row: the section bytes — verbatim re-emission of the
/// source file's class table when the same-version roundtrip left the
/// class table and the per-class object census unchanged (the state
/// hash matches `raw_classes_fingerprint`). The authored desync bytes
/// reproduce gold's walk exactly — any re-encoding desyncs it
/// differently — and the bytes carry the author's `num_instances`/
/// zombie flags for classes whose instances re-emit through the
/// raw-object passthrough (outside the write census). Falls back to
/// the sane encoding on any change or version conversion.
fn classes_section_data<'a>(
    document: &'a CadDocument,
    version: DxfVersion,
    classes: &[crate::classes::DxfClass],
    maint: u8,
    encoding: &'static encoding_rs::Encoding,
) -> std::borrow::Cow<'a, [u8]> {
    if document.dwg_source_version == Some(version) {
        if let Some(raw) = document.raw_classes_data.as_deref() {
            if super::classes_state_fingerprint(document) == document.raw_classes_fingerprint {
                return std::borrow::Cow::Borrowed(raw.as_slice());
            }
        }
    }
    std::borrow::Cow::Owned(classes_writer::write_classes_with_encoding(
        version, classes, maint, encoding,
    ))
}

/// Whether the version uses the AC21 (R2007) file format.
///
/// AC1021 uses RS-encoded pages, LZ77 AC21 compression, and CRC-64
/// checksums — distinct from both the AC18 and AC15 formats.
fn uses_ac21_format(version: DxfVersion) -> bool {
    version == DxfVersion::AC1021
}

/// Whether the version uses the AC18 page-based file format.
fn uses_paged_format(version: DxfVersion) -> bool {
    version >= DxfVersion::AC1018
}

/// Prepare the header for writing by synchronizing all handle references
/// from the actual document objects and correcting the handle seed.
///
/// This is critical because after a DWG read-roundtrip, header handle
/// references may be NULL (the header reader for R2007+ doesn't correctly
/// read handles from the three-stream merged format). Without syncing,
/// the header would write NULL handles for table controls and the root
/// dictionary, causing IntelliCAD (and other CAD apps) to report
/// "null object id" for every object.
///
/// Also updates EXTMIN/EXTMAX from the computed model-space extents so that
/// "Zoom Extents" works correctly when the file is first opened.
fn prepare_header(
    document: &CadDocument,
    handle_map: &[(u64, u32)],
    extents: &Option<crate::types::BoundingBox3D>,
) -> HeaderVariables {
    let mut h = document.header.clone();
    let emitted = |handle: Handle| {
        !handle.is_null() && handle_map.iter().any(|(value, _)| *value == handle.value())
    };

    // ── Sync table control handles from actual table objects ──
    // The tables always have valid handles from initialize_defaults(),
    // but the header might have NULL handles after a DWG read.
    h.block_control_handle = document.block_records.handle();
    h.layer_control_handle = document.layers.handle();
    h.style_control_handle = document.text_styles.handle();
    h.linetype_control_handle = document.line_types.handle();
    h.view_control_handle = document.views.handle();
    h.ucs_control_handle = document.ucss.handle();
    h.vport_control_handle = document.vports.handle();
    h.appid_control_handle = document.app_ids.handle();
    h.dimstyle_control_handle = document.dim_styles.handle();

    // ── Sync root dictionary handle ──
    // Find the root dictionary by scanning document.objects for a
    // Dictionary with owner == NULL. Prefer non-0x0C handles (file's
    // root dict) over the initialize_defaults() one.
    if h.named_objects_dict_handle.is_null() {
        h.named_objects_dict_handle = find_root_dict_handle(&document.objects);
    }
    // Verify the root dict handle actually exists in objects
    if !h.named_objects_dict_handle.is_null()
        && !document.objects.contains_key(&h.named_objects_dict_handle)
    {
        // Handle points to nonexistent object — try to find the real root dict
        h.named_objects_dict_handle = find_root_dict_handle(&document.objects);
    }

    // ── Sync child dictionary handles from root dict entries ──
    // Always overwrite — reader may produce garbage handles.
    // If root dict doesn't have an entry, set handle to NULL.
    let root_dict_entries = match document.objects.get(&h.named_objects_dict_handle) {
        Some(crate::objects::ObjectType::Dictionary(root_dict)) => Some(root_dict),
        _ => None,
    };
    h.acad_group_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_GROUP"))
        .unwrap_or(Handle::NULL);
    h.acad_mlinestyle_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_MLINESTYLE"))
        .unwrap_or(Handle::NULL);
    h.acad_layout_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_LAYOUT"))
        .unwrap_or(Handle::NULL);
    h.acad_plotsettings_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_PLOTSETTINGS"))
        .unwrap_or(Handle::NULL);
    h.acad_plotstylename_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_PLOTSTYLENAME"))
        .unwrap_or(Handle::NULL);
    h.acad_material_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_MATERIAL"))
        .unwrap_or(Handle::NULL);
    h.acad_color_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_COLOR"))
        .unwrap_or(Handle::NULL);
    h.acad_visualstyle_dict_handle = root_dict_entries
        .and_then(|d| d.get("ACAD_VISUALSTYLE"))
        .unwrap_or(Handle::NULL);

    // ── Sync linetype handles by name ──
    if let Some(lt) = document.line_types.get("ByLayer") {
        h.bylayer_linetype_handle = lt.handle;
    }
    if let Some(lt) = document.line_types.get("ByBlock") {
        h.byblock_linetype_handle = lt.handle;
    }
    if let Some(lt) = document.line_types.get("Continuous") {
        h.continuous_linetype_handle = lt.handle;
    }

    // ── Sync model/paper space block handles ──
    if let Some(br) = document.block_records.get("*Model_Space") {
        h.model_space_block_handle = br.handle;
    }
    if let Some(br) = document.block_records.get("*Paper_Space") {
        h.paper_space_block_handle = br.handle;
    }

    // ── Sync current style handles (validate against actual objects) ──
    // CLAYER: must point to an actual layer; fall back to "0" if invalid
    {
        let clayer_valid = !h.current_layer_handle.is_null()
            && document
                .layers
                .iter()
                .any(|l| l.handle == h.current_layer_handle);
        if !clayer_valid {
            if let Some(layer) = document.layers.get("0") {
                h.current_layer_handle = layer.handle;
            }
        }
    }

    // The header reader can produce garbage (multi-byte) handle values
    // when the bit stream is misaligned.  Unconditionally sync every
    // "current style" handle so that garbage values are overwritten
    // with valid handles from the document model.
    {
        let text_valid = !h.current_text_style_handle.is_null()
            && document
                .text_styles
                .iter()
                .any(|s| s.handle == h.current_text_style_handle);
        if !text_valid {
            h.current_text_style_handle = document
                .text_styles
                .get("Standard")
                .map(|s| s.handle)
                .unwrap_or(Handle::NULL);
        }
    }
    {
        let ds_valid = !h.current_dimstyle_handle.is_null()
            && document
                .dim_styles
                .iter()
                .any(|ds| ds.handle == h.current_dimstyle_handle);
        if !ds_valid {
            h.current_dimstyle_handle = document
                .dim_styles
                .get("Standard")
                .map(|ds| ds.handle)
                .unwrap_or(Handle::NULL);
        }
    }
    {
        let lt_valid = !h.current_linetype_handle.is_null()
            && document
                .line_types
                .iter()
                .any(|lt| lt.handle == h.current_linetype_handle);
        if !lt_valid {
            h.current_linetype_handle = h.bylayer_linetype_handle;
        }
    }
    {
        let mls_valid = !h.current_multiline_style_handle.is_null()
            && document.objects.iter().any(|(_, obj)| {
                if let crate::objects::ObjectType::MLineStyle(mls) = obj {
                    mls.handle == h.current_multiline_style_handle
                } else {
                    false
                }
            });
        if !mls_valid {
            let dictionary_standard = document
                .objects
                .get(&h.acad_mlinestyle_dict_handle)
                .and_then(|object| match object {
                    crate::objects::ObjectType::Dictionary(dictionary) => {
                        dictionary.get("Standard")
                    }
                    _ => None,
                })
                .filter(|handle| {
                    matches!(
                        document.objects.get(handle),
                        Some(crate::objects::ObjectType::MLineStyle(style))
                            if style.handle == *handle
                    )
                });

            h.current_multiline_style_handle = dictionary_standard
                .or_else(|| {
                    document
                        .objects
                        .values()
                        .filter_map(|object| match object {
                            crate::objects::ObjectType::MLineStyle(style)
                                if style.name.eq_ignore_ascii_case("Standard")
                                    && !style.handle.is_null() =>
                            {
                                Some(style.handle)
                            }
                            _ => None,
                        })
                        .min_by_key(|handle| handle.value())
                })
                .unwrap_or(Handle::NULL);
        }
    }

    // R2007+: current_material_handle — validate against emitted objects.
    // Unsupported raw materials may be dropped during a version conversion.
    {
        if !emitted(h.current_material_handle) {
            h.current_material_handle = Handle::NULL;
        }
    }

    // dim_text_style_handle — validate against text styles
    {
        let dts_valid = !h.dim_text_style_handle.is_null()
            && document
                .text_styles
                .iter()
                .any(|s| s.handle == h.dim_text_style_handle);
        if !dts_valid {
            h.dim_text_style_handle = document
                .text_styles
                .get("Standard")
                .map(|s| s.handle)
                .unwrap_or(Handle::NULL);
        }
    }

    // UCS ortho ref handles — validate against UCS table
    {
        let ucs_valid = |handle: Handle| -> bool {
            handle.is_null() || document.ucss.iter().any(|u| u.handle == handle)
        };
        if !ucs_valid(h.paper_ucs_ortho_ref) {
            h.paper_ucs_ortho_ref = Handle::NULL;
        }
        if !ucs_valid(h.ucs_ortho_ref) {
            h.ucs_ortho_ref = Handle::NULL;
        }
    }

    // ── Validate dim linetype handles against actual linetypes ──
    // These can become corrupt during header read/write due to stream alignment.
    {
        let valid_lt = |h: Handle| -> bool {
            h.is_null() || document.line_types.iter().any(|lt| lt.handle == h)
        };
        if !valid_lt(h.dim_linetype_handle) {
            h.dim_linetype_handle = Handle::NULL;
        }
        if !valid_lt(h.dim_linetype1_handle) {
            h.dim_linetype1_handle = Handle::NULL;
        }
        if !valid_lt(h.dim_linetype2_handle) {
            h.dim_linetype2_handle = Handle::NULL;
        }
    }

    // ── Correct HANDSEED ──
    // Only for programmatic documents: a read document preserves the
    // author's seed even when the file's own max handle reaches past it
    // (the 2000/PolyLine2D quirk: HANDSEED 975 < max 978 — gold writes
    // it back unchanged; §19 H7). Entity additions still grow the seed
    // through the document API's own bump.
    if document.dwg_header_raw.is_none() {
        let max_handle = handle_map.iter().map(|&(ha, _)| ha).max().unwrap_or(0);
        if h.handle_seed <= max_handle {
            h.handle_seed = max_handle + 1;
        }
    }

    // ── Update model-space extents ──
    // Only for programmatic documents: a read document carries the
    // author's saved extents in the raw mirror (§19 H7) — recomputing
    // would replace them with silver's own bounds and break the
    // write-target preservation row (EXTMIN/EXTMAX).
    if document.dwg_header_raw.is_none() {
        if let Some(ref ext) = extents {
            h.model_space_extents_min = ext.min;
            h.model_space_extents_max = ext.max;
        }
    }

    h
}

/// Find the root dictionary handle by scanning the objects map.
///
/// The root dictionary is a Dictionary with `owner == Handle::NULL`.
/// If multiple candidates exist (e.g., from `initialize_defaults` and
/// from file data), prefer the one with more entries (the file's root dict).
fn find_root_dict_handle(
    objects: &std::collections::HashMap<crate::types::Handle, crate::objects::ObjectType>,
) -> crate::types::Handle {
    use crate::objects::ObjectType;
    use crate::types::Handle;

    let mut best_handle = Handle::NULL;
    let mut best_entry_count = 0usize;

    for (handle, obj) in objects {
        if let ObjectType::Dictionary(dict) = obj {
            if dict.owner.is_null() {
                // Prefer the dictionary with more entries (richer = file's root dict);
                // on tie, prefer higher handle (likely from file, not initialize_defaults)
                if dict.entries.len() > best_entry_count
                    || (dict.entries.len() == best_entry_count
                        && handle.value() > best_handle.value())
                {
                    best_handle = *handle;
                    best_entry_count = dict.entries.len();
                }
            }
        }
    }

    best_handle
}

// ════════════════════════════════════════════════════════════════════════════
//  AC15 format (R13/R14/R2000) — linear file layout
// ════════════════════════════════════════════════════════════════════════════

fn write_ac15<W: Write + Seek>(
    output: &mut W,
    document: &CadDocument,
    version: DxfVersion,
) -> Result<()> {
    let mut fhw = DwgFileHeaderWriterAC15::new(version);
    // New documents do not carry source maintenance metadata. AC15 files use
    // the established R2000-era default rather than zero, which strict
    // readers reject in the file header.
    fhw.set_maintenance_version(if document.maintenance_version == 0 {
        15
    } else {
        document.maintenance_version
    });
    fhw.set_code_page(crate::io::dxf::code_page::dwg_code_page_index(
        &document.header.code_page,
    ));
    // §19 H7f: a same-version roundtrip re-emits the author's
    // dwg_version/maint_version pair at 0x11/0x12 ("of app which stored
    // it / the actual dwg version" — per-release bytes; the corpus R2000
    // authors wrote 0x17..0x21 × 0..0x1D) instead of the fixed pair.
    if document.dwg_source_version == Some(version) {
        if let Some(fh) = document.dwg_file_header.as_ref() {
            fhw.set_source_version_pair(fh.dwg_version, fh.maint_version);
        }
    }

    // §19 H8f's whole-file echo is HOISTED to `write_to_writer` (it
    // fires before the prepare pipeline on the pre-prepare state
    // verdict); reaching here means it did not engage — the
    // conventional flat-container emission below serves every other
    // case.

    // ── Phase 1: Compute objects FIRST to get handle map ──
    let objects_started = web_time::Instant::now();
    let obj_writer = DwgObjectWriter::new(document)?;
    let (
        obj_data,
        handle_map_u32,
        extents,
        _sab_entries,
        class_instance_counts,
        class_counts_complete,
    ) = obj_writer.write_with_class_metadata();
    if std::env::var_os("PERF").is_some() {
        eprintln!(
            "[perf] dwg-write objects={:.1}ms bytes={} handles={} format=ac15",
            objects_started.elapsed().as_secs_f64() * 1000.0,
            obj_data.len(),
            handle_map_u32.len(),
        );
    }

    // ── Phase 2: Prepare header (sync handles + correct HANDSEED) ──
    let corrected_header = prepare_header(document, &handle_map_u32, &extents);

    // ── Section: Header (uses synced + corrected header) ──
    let maint = document.maintenance_version;
    let header_encoding =
        crate::io::dxf::code_page::encoding_from_code_page(&document.header.code_page)
            .unwrap_or(encoding_rs::WINDOWS_1252);
    let header_data = header_writer::write_header_with_encoding_opt(
        version,
        &corrected_header,
        maint,
        header_encoding,
        document.dwg_header_raw.as_ref(),
    );
    fhw.add_section(section_names::HEADER, header_data);

    // ── Section: Classes ──
    let classes = reconciled_classes(document, &class_instance_counts, class_counts_complete);
    let classes_data =
        classes_section_data(document, version, &classes, maint, header_encoding);
    fhw.add_section(section_names::CLASSES, classes_data.into_owned());

    // ── Section: AcDbObjects (pre-computed) ──
    fhw.add_section(section_names::ACDB_OBJECTS, obj_data);

    // ── Section: ObjFreeSpace ──
    // §19 H7e: gold reads the R2000 section only at the position
    // directly after the handles map (the AC15 writer's record order
    // pins the placement), so the content must be the author's own —
    // verbatim on a same-version roundtrip, a NUL locator record
    // (seeker 0 — the author's own absent form) when the source
    // carried no section, and the historical rebuild only for
    // programmatic documents and version conversions.
    if document.dwg_source_version == Some(version) {
        if let Some(raw) = document.raw_obj_free_space_data.clone() {
            fhw.add_section(section_names::OBJ_FREE_SPACE, (*raw).clone());
        }
    } else {
        let obj_free_space = build_obj_free_space(version, document, handle_map_u32.len());
        fhw.add_section(section_names::OBJ_FREE_SPACE, obj_free_space);
    }

    // ── Section: Template ──
    let template = build_template(&[], document.header.measurement)?;
    fhw.add_section(section_names::TEMPLATE, template);

    // ── Section: AuxHeader (uses corrected HANDSEED) ──
    let aux_data = aux_header_writer::write_aux_header(version, &corrected_header);
    fhw.add_section(section_names::AUX_HEADER, aux_data);

    // ── Section: Handles (must be last — needs objects offset) ──
    let section_offset = fhw.handle_section_offset() as i32;
    let handle_map_i64: Vec<(u64, i64)> =
        handle_map_u32.iter().map(|&(h, o)| (h, o as i64)).collect();
    let handles_data = handle_writer::write_handles(&handle_map_i64, section_offset);
    fhw.add_section(section_names::HANDLES, handles_data);

    // ── Section: Preview ──
    // Preview is the last section, so its file offset is known now; the
    // container's image `start` fields are absolute file offsets relative to it.
    let preview_base = fhw.pending_section_offset() as u64;
    let preview_data =
        crate::io::dwg::preview::build_preview(document.preview.as_ref(), preview_base);
    fhw.add_section(section_names::PREVIEW, preview_data);

    // ── Write final file ──
    fhw.write_file(output)?;

    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
//  AC18 format (R2004/R2010/R2013/R2018) — page-based with LZ77
// ════════════════════════════════════════════════════════════════════════════

fn write_ac18<W: Write + Seek>(
    output: &mut W,
    document: &CadDocument,
    version: DxfVersion,
) -> Result<()> {
    // §19 H8g's whole-file echo is HOISTED to `write_to_writer` (it
    // fires before the prepare pipeline on the pre-prepare state
    // verdict — the maintainer's AC18 decision, 2026-09-27); reaching
    // here means it did not engage — the H7g mirror and the
    // conventional paged emission below serve every other case.

    // The maintenance-release version must match the value the AuxHeader
    // writes AND the file-header metadata byte, because readers gate the
    // R2010+ per-section "extra RL" (which locates the header/classes string
    // stream) on `maintenance_version > 3`. A fresh document defaults to 0,
    // which for R2013 (AC1027) omits that RL and makes the header string
    // stream unreadable in AutoCAD/TrueView. Use the canonical per-version
    // value so R2013 files are always well-formed.
    //
    // §19 H7f: a same-version roundtrip re-emits the AUTHOR's maint byte
    // instead of the canonical — it is file identity (gold prints it as
    // FILEHEADER.maint_version) and the layout gate is layout-stable for
    // every corpus class (R2004 never reads the extra RL — its header
    // vars go through dwg_decode_header_variables which reads no
    // bitsize_hi; the R2010/R2013 authors' bytes are all > 3 like the
    // canonical; R2018's gate has the `|| >= R2018` arm). The five
    // FILEHEADER identity bytes (maint_rel_version 0x0B, dwg_version
    // 0x11, maint_version 0x12, app pair 0x16/0x17) mirror with it.
    let source_fh = if document.dwg_source_version == Some(version) {
        document.dwg_file_header.as_ref()
    } else {
        None
    };
    let maint = source_fh.map_or(
        aux_header_writer::dwg_maintenance_version(version) as u8,
        |fh| fh.maint_version,
    );

    // AC18 writer reserves 0x100 bytes at file start for metadata
    let mut fhw = DwgFileHeaderWriterAC18::new(version, maint, output)?;
    if let Some(fh) = source_fh {
        fhw.set_source_header_bytes(
            fh.maint_rel_version,
            fh.dwg_version,
            fh.maint_version,
            fh.unknown_0,
            fh.app_dwg_version,
            fh.app_maint_version,
        );
    }
    fhw.set_code_page(crate::io::dxf::code_page::dwg_code_page_index(
        &document.header.code_page,
    ));

    // R2004+ default page size for most sections
    const PAGE_SIZE: usize = 0x7400;
    // Smaller page for metadata-style sections
    const SMALL_PAGE: usize = 0x80;

    // ── Phase 1: Compute objects FIRST to get handle map ──
    let objects_started = web_time::Instant::now();
    let obj_writer = DwgObjectWriter::new(document)?;
    let (
        obj_data,
        handle_map_u32,
        extents,
        sab_entries,
        class_instance_counts,
        class_counts_complete,
    ) = obj_writer.write_with_class_metadata();
    if std::env::var_os("PERF").is_some() {
        eprintln!(
            "[perf] dwg-write objects={:.1}ms bytes={} handles={} format=ac18",
            objects_started.elapsed().as_secs_f64() * 1000.0,
            obj_data.len(),
            handle_map_u32.len(),
        );
    }

    // ── Phase 2: Prepare header (sync handles + correct HANDSEED) ──
    let corrected_header = prepare_header(document, &handle_map_u32, &extents);

    // ── Section: Header (uses synced + corrected header) ──
    let header_encoding =
        crate::io::dxf::code_page::encoding_from_code_page(&document.header.code_page)
            .unwrap_or(encoding_rs::WINDOWS_1252);
    let header_data = header_writer::write_header_with_encoding_opt(
        version,
        &corrected_header,
        maint,
        header_encoding,
        document.dwg_header_raw.as_ref(),
    );

    // ── Build the remaining section buffers up front (§19 H7g) ──
    // The container-shape mirror gates on the content lengths and may
    // emit in the author's physical order, so every section is
    // constructed before anything is written — the builders are pure,
    // only `add_section` moves the output stream, so the conventional
    // path below writes byte-identical output to the historical
    // interleaved emission. The preview content is the one lazy
    // section: its container embeds its own page-data address, so it
    // is built at emission time.
    let same_origin = document.dwg_source_version == Some(version);

    // ── Section: Classes ──
    let classes = reconciled_classes(document, &class_instance_counts, class_counts_complete);
    let classes_data =
        classes_section_data(document, version, &classes, maint, header_encoding).into_owned();

    // ── Section: SummaryInfo ──
    // The presence-coupling gate (§19 H7 review): gold emits SummaryInfo
    // only when the original's summaryinfo_address was set (out_json.c:2663)
    // — a document read from a file whose author carried no section keeps
    // its zeroed model and the writer must not materialize a section out
    // of nothing (gh209_1: address 0, no section, no gold key). The
    // escape hatch: a programmatically modified summary (≠ the default)
    // writes the section — user intent wins over roundtrip presence.
    let summary_orig_present = document
        .dwg_file_header
        .as_ref()
        .map(|fh| fh.summaryinfo_address != 0)
        .unwrap_or(true);
    let summary_section =
        if summary_orig_present || document.summary_info != crate::document::SummaryInfo::default()
        {
            Some(build_summary_info(version, &document.summary_info))
        } else {
            None
        };

    // ── Section: Preview ──
    // The image `start` fields are absolute file offsets, so the preview page's
    // data position must be known BEFORE building the container (the ODA page
    // checksum covers the final bytes — patching offsets afterward would break
    // it). `add_section` aligns via `write_magic_number` (pads by `pos % 0x20`)
    // then writes a 0x20 page header; the header's preview seeker points at
    // `page + 0x20`, which is where the container lands. The content itself is
    // built at emission time (see below); only its gate length is fixed here.
    let preview_gate_len = match document.preview.as_ref() {
        Some(p) if !p.raw.is_empty() => p.raw.len(),
        p => crate::io::dwg::preview::build_preview(p, 0).len(),
    };

    // ── Section: AppInfo ── (§19 H7: verbatim from the source when the
    // same-version roundtrip carried one; SKIPPED when the source had
    // none — gold prints the section unconditionally (zeroed when
    // absent), so materializing the boilerplate there would diverge.
    // Programmatic documents and version conversions keep the
    // historical boilerplate.)
    let app_info_section: Option<Vec<u8>> = if same_origin {
        document.raw_app_info_data.as_deref().map(|raw| raw.to_vec())
    } else {
        Some(app_info_writer::write_app_info(version))
    };

    // ── Section: AppInfoHistory ── (§19 H7: never written before this
    // row — same verbatim/skip rule; a source without the section keeps
    // gold's zeroed print on both sides.)
    let app_info_history_section: Option<Vec<u8>> = if same_origin {
        document
            .raw_app_info_history_data
            .as_deref()
            .map(|raw| raw.to_vec())
    } else {
        None
    };

    // ── Section: FileDepList ──
    let file_dep_data = build_file_dep_list();

    // ── Section: RevHistory ──
    let rev_history_data = build_rev_history();

    // ── Section: AuxHeader (uses corrected HANDSEED) ──
    let aux_data = aux_header_writer::write_aux_header(version, &corrected_header);

    // ── Section: AcDsPrototype_1b (AC1027+ ACIS SAB storage) ──
    let acds_section: Option<Vec<u8>> =
        if !sab_entries.is_empty() || (same_origin && document.raw_acds_data.is_some()) {
            Some(acds_data(document, version, &sab_entries).into_owned())
        } else {
            None
        };

    // ── Section: ObjFreeSpace ──
    // §19 H7e: the content is authored file state (the author's
    // numhandles pattern words, TDUPDATE, the max constants) — verbatim
    // on a same-version roundtrip; a source without the section gets
    // none materialized (gold prints the section unconditionally on
    // R2004+, zeroed when absent — both sides match then); programmatic
    // documents and version conversions keep the historical rebuild.
    let obj_free_space_section: Option<Vec<u8>> = if same_origin {
        document
            .raw_obj_free_space_data
            .as_deref()
            .map(|raw| raw.to_vec())
    } else {
        Some(build_obj_free_space(version, document, handle_map_u32.len()))
    };

    // ── Section: XrefManifest ──
    // §19 H7g: the R2013+ external-reference table — raw verbatim on a
    // same-version roundtrip (authored state, unmodeled, unprinted by
    // gold); never materialized otherwise. The mirror-only emission
    // keeps the conventional path byte-identical to the historical
    // writer.
    let xref_manifest_section: Option<Vec<u8>> = if same_origin {
        document
            .raw_xref_manifest_data
            .as_deref()
            .map(|raw| raw.to_vec())
    } else {
        None
    };

    // ── Section: Template ──
    let template = build_template(&[], document.header.measurement)?;

    // ── Section: Handles (needs objects data) ──
    let section_offset = fhw.handle_section_offset() as i32;
    let handle_map_i64: Vec<(u64, i64)> =
        handle_map_u32.iter().map(|&(h, o)| (h as u64, o as i64)).collect();
    let handles_data = handle_writer::write_handles(&handle_map_i64, section_offset);

    let sections = Ac18Sections {
        header: &header_data,
        classes: &classes_data,
        summary: summary_section.as_deref(),
        app_info: app_info_section.as_deref(),
        app_info_history: app_info_history_section.as_deref(),
        file_dep: &file_dep_data,
        rev_history: &rev_history_data,
        aux_header: &aux_data,
        objects: &obj_data,
        acds: acds_section.as_deref(),
        obj_free_space: obj_free_space_section.as_deref(),
        xref_manifest: xref_manifest_section.as_deref(),
        template: &template,
        handles: &handles_data,
    };

    // ── The §19 H7g container-shape mirror gate and emission ──
    // One root closes three census families at once when it holds:
    // `numsections` IS the page-map entry count and the id fields
    // (@0x28 `last_section_id`, @0x50 `section_map_id`, @0x5C
    // `section_info_id`, @0x60 `section_array_size`) follow the page
    // space, so the author's per-descriptor page splitting + id
    // pattern reproduce the R2004_Header counts exactly; the summary
    // and preview pages are the FIRST TWO pages in every corpus
    // author's layout, so an order-faithful prefix also reproduces
    // `summaryinfo_address`/`thumbnail_address` (the FILEHEADER
    // addresses are page-data positions, seeker+0x20) and the preview
    // chain's absolute image offsets (the THUMBNAILIMAGE identity).
    // The gate falls back to the conventional sequential layout on
    // ANY divergence — an author section our content does not fit, a
    // presence divergence, gap entries, interleaved pages: the
    // rewrite stays valid everywhere, the residue rows stay open on
    // the files it declines.
    let mirror_plan = match (
        document.dwg_ac18_shape.as_ref(),
        document.dwg_r2004_header.as_ref(),
    ) {
        (Some(shape), Some(sys)) => {
            if same_origin {
                ac18_mirror_plan(shape, sys, &sections, preview_gate_len)
            } else {
                None
            }
        }
        _ => None,
    };

    if let Some(plan) = mirror_plan {
        let shape = document
            .dwg_ac18_shape
            .as_ref()
            .expect("the mirror plan implies the shape");
        fhw.set_mirror_ids(
            shape.section_info_id as i32,
            shape.section_map_id as i32,
            shape.section_array_size,
        );
        for sec_index in plan.order {
            let sec = &shape.sections[sec_index];
            if sec.name != section_names::PREVIEW {
                // The gate verified the content parity; 0-page sections
                // carry no bytes (our writer skipped them — the author's
                // shape drives the presence).
                let data: &[u8] = sections.get(&sec.name).unwrap_or(&[]);
                fhw.add_section_shaped(
                    output,
                    &sec.name,
                    &sec.raw_name,
                    data,
                    sec.compressed_code == 2,
                    sec.max_decomp as usize,
                    &sec.pages,
                )?;
                continue;
            }
            // The preview page lands at the author's `thumbnail_address`
            // exactly when the pages before it (the byte-faithful
            // summary-prefix) kept the author's on-disk sizes; then the
            // retained raw container re-emits verbatim — its embedded
            // offsets are the author's, and they are still the correct
            // absolute addresses in our file. Otherwise the container
            // is rebuilt around OUR actual address (honest bytes; the
            // census row keeps its diff on that file).
            let addr = fhw.next_page_data_address(output)?;
            let thumbnail_addr = document
                .dwg_file_header
                .as_ref()
                .map_or(0, |fh| fh.thumbnail_address);
            let preview_bytes = match document.preview.as_ref() {
                Some(p)
                    if !p.raw.is_empty()
                        && thumbnail_addr == addr as i32
                        && p.raw.len() <= sec.max_decomp as usize =>
                {
                    p.raw.clone()
                }
                p => crate::io::dwg::preview::build_preview(p, addr),
            };
            fhw.add_section_shaped(
                output,
                &sec.name,
                &sec.raw_name,
                &preview_bytes,
                sec.compressed_code == 2,
                sec.max_decomp as usize,
                &sec.pages,
            )?;
        }
    } else {
        // The conventional sequential layout (byte-identical to the
        // historical emission for every non-mirrored file).
        fhw.add_section(output, section_names::HEADER, &header_data, true, PAGE_SIZE)?;
        fhw.add_section(output, section_names::CLASSES, &classes_data, true, PAGE_SIZE)?;
        if let Some(summary_data) = &summary_section {
            fhw.add_section(
                output,
                section_names::SUMMARY_INFO,
                summary_data,
                false,
                0x100,
            )?;
        }

        let cur = output.seek(std::io::SeekFrom::Current(0))? as u64;
        let preview_base = (cur + cur % 0x20) + 0x20;
        let preview_data =
            crate::io::dwg::preview::build_preview(document.preview.as_ref(), preview_base);
        // Keep the whole preview in one contiguous page (a split would scatter the
        // container across page headers): a decompressed size ≥ its length, rounded
        // up to a 0x20 multiple so the uncompressed page needs no compression pad.
        let preview_page = ((preview_data.len() + 0x1F) & !0x1F).max(0x20);
        fhw.add_section(
            output,
            section_names::PREVIEW,
            &preview_data,
            false,
            preview_page,
        )?;

        if let Some(app_info_data) = &app_info_section {
            fhw.add_section(output, section_names::APP_INFO, app_info_data, false, SMALL_PAGE)?;
        }
        if let Some(app_info_history_data) = &app_info_history_section {
            fhw.add_section(
                output,
                section_names::APP_INFO_HISTORY,
                app_info_history_data,
                false,
                SMALL_PAGE,
            )?;
        }
        fhw.add_section(
            output,
            section_names::FILE_DEP_LIST,
            &file_dep_data,
            false,
            SMALL_PAGE,
        )?;
        fhw.add_section(
            output,
            section_names::REV_HISTORY,
            &rev_history_data,
            true,
            PAGE_SIZE,
        )?;
        fhw.add_section(
            output,
            section_names::AUX_HEADER,
            &aux_data,
            true,
            PAGE_SIZE,
        )?;
        fhw.add_section(
            output,
            section_names::ACDB_OBJECTS,
            &obj_data,
            true,
            PAGE_SIZE,
        )?;
        if let Some(acds_data) = &acds_section {
            fhw.add_section(
                output,
                section_names::ACDS_PROTOTYPE,
                acds_data,
                true,
                PAGE_SIZE,
            )?;
        }
        if let Some(obj_free_space) = &obj_free_space_section {
            fhw.add_section(
                output,
                section_names::OBJ_FREE_SPACE,
                obj_free_space,
                true,
                PAGE_SIZE,
            )?;
        }
        fhw.add_section(output, section_names::TEMPLATE, &template, true, PAGE_SIZE)?;
        fhw.add_section(
            output,
            section_names::HANDLES,
            &handles_data,
            true,
            PAGE_SIZE,
        )?;
    }

    // ── Write file header, section map, and page map ──
    fhw.write_file(output)?;

    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
//  AC18 container-shape mirror (§19 H7g)
// ════════════════════════════════════════════════════════════════════════════

/// The pre-built AC18 section buffers (§19 H7g), with the writer's own
/// presence gates applied: `None` on an `Option` field = the section
/// is skipped (the verbatim/skip arms); the preview section is
/// excluded — its container embeds its own page-data address, so it is
/// built at emission time.
struct Ac18Sections<'a> {
    header: &'a [u8],
    classes: &'a [u8],
    summary: Option<&'a [u8]>,
    app_info: Option<&'a [u8]>,
    app_info_history: Option<&'a [u8]>,
    file_dep: &'a [u8],
    rev_history: &'a [u8],
    aux_header: &'a [u8],
    objects: &'a [u8],
    acds: Option<&'a [u8]>,
    obj_free_space: Option<&'a [u8]>,
    xref_manifest: Option<&'a [u8]>,
    template: &'a [u8],
    handles: &'a [u8],
}

impl Ac18Sections<'_> {
    /// The buffer for a canonical section name; `None` = our writer
    /// skips the section.
    fn get(&self, name: &str) -> Option<&[u8]> {
        Some(match name {
            section_names::HEADER => self.header,
            section_names::CLASSES => self.classes,
            section_names::SUMMARY_INFO => self.summary?,
            section_names::APP_INFO => self.app_info?,
            section_names::APP_INFO_HISTORY => self.app_info_history?,
            section_names::FILE_DEP_LIST => self.file_dep,
            section_names::REV_HISTORY => self.rev_history,
            section_names::AUX_HEADER => self.aux_header,
            section_names::ACDB_OBJECTS => self.objects,
            section_names::ACDS_PROTOTYPE => self.acds?,
            section_names::OBJ_FREE_SPACE => self.obj_free_space?,
            section_names::XREF_MANIFEST => self.xref_manifest?,
            section_names::TEMPLATE => self.template,
            section_names::HANDLES => self.handles,
            _ => return None,
        })
    }
}

/// The §19 H7g mirror emission plan: the shape-section indices to
/// emit, in the author's physical order (0-page descriptor-only
/// sections appended).
struct Ac18MirrorPlan {
    order: Vec<usize>,
}

/// The §19 H7g content-parity gate. Every check is a fall-back-to-
/// conventional trigger: the author's page space is mirrored only when
/// our re-encoded content demonstrably fits it and the author's layout
/// is one our writer can reproduce. Set `AC18_MIRROR_DEBUG` to trace
/// the decline reason per file.
fn ac18_mirror_plan(
    shape: &crate::document::DwgAc18ContainerShape,
    sys: &crate::document::DwgR2004SystemHeader,
    sections: &Ac18Sections<'_>,
    preview_len: usize,
) -> Option<Ac18MirrorPlan> {
    use std::collections::{HashMap, HashSet};

    macro_rules! decline {
        ($why:expr) => {{
            if std::env::var_os("AC18_MIRROR_DEBUG").is_some() {
                eprintln!("[ac18-mirror] declined: {}", $why);
            }
            return None;
        }};
    }

    // Gap entries (negative map records) are not reproducible by our
    // writer; the author's numgaps must count none.
    if sys.numgaps != 0 {
        decline!(format!("numgaps {} (gap map entries)", sys.numgaps));
    }
    // The retained identity must be self-coherent: gold validates
    // max id == section_array_size, and last_section_id names the last
    // allocated id (the page-map box on every corpus file).
    let page_map_id = shape.section_map_id as i32;
    if sys.section_map_id != shape.section_map_id
        || sys.section_info_id != shape.section_info_id as i32
        || sys.section_array_size != shape.section_array_size as i32
        || sys.last_section_id != page_map_id
        || sys.section_array_size < page_map_id
    {
        decline!("incoherent retained ids");
    }
    // The core sections: a shape without any of them is not a
    // mirrorable container — rewriting must never drop one.
    for must in [
        section_names::HEADER,
        section_names::CLASSES,
        section_names::ACDB_OBJECTS,
        section_names::HANDLES,
    ] {
        if !shape.sections.iter().any(|sec| sec.name == must) {
            decline!(format!("shape lacks core section {must}"));
        }
    }

    // Per-section content parity: our re-encoded content must cover
    // the author's last page offset within the author's max-decomp
    // capacity, and the per-page offsets must ascend (the reassembly
    // is offset-driven).
    for sec in &shape.sections {
        if sec.compressed_code != 1 && sec.compressed_code != 2 {
            decline!(format!(
                "section {} compressed_code {}",
                sec.name, sec.compressed_code
            ));
        }
        let len: u64 = if sec.name == section_names::PREVIEW {
            if sec.pages.len() != 1 {
                decline!(format!(
                    "preview has {} pages (single-page expected)",
                    sec.pages.len()
                ));
            }
            preview_len as u64
        } else {
            match sections.get(&sec.name) {
                Some(data) => data.len() as u64,
                None => {
                    // Our writer skips this section: only a 0-page
                    // descriptor can mirror then.
                    if !sec.pages.is_empty() {
                        decline!(format!(
                            "our writer skips {} but the author has {} pages",
                            sec.name,
                            sec.pages.len()
                        ));
                    }
                    0
                }
            }
        };
        if sec.pages.is_empty() {
            continue;
        }
        for window in sec.pages.windows(2) {
            if window[0].1 >= window[1].1 {
                decline!(format!("section {} page offsets not ascending", sec.name));
            }
        }
        let last_offset = sec.pages[sec.pages.len() - 1].1;
        if len <= last_offset {
            decline!(format!(
                "section {} content len {len} does not reach the author's last page offset {last_offset}",
                sec.name
            ));
        }
        if len - last_offset > sec.max_decomp as u64 {
            decline!(format!(
                "section {} overflows the author's page space (len {len}, last offset {last_offset}, capacity {})",
                sec.name, sec.max_decomp
            ));
        }
    }

    // The entry count: every declared data page plus the two boxes
    // must match the author's map and her numsections.
    let data_pages: usize = shape.sections.iter().map(|sec| sec.pages.len()).sum();
    if data_pages + 2 != shape.map_order.len() {
        decline!(format!(
            "map entries {} vs declared pages {data_pages} + 2",
            shape.map_order.len()
        ));
    }
    if (data_pages + 2) as u32 != sys.numsections {
        decline!(format!(
            "numsections {} vs computed entry count {}",
            sys.numsections,
            data_pages + 2
        ));
    }

    // The physical order: the map's entries minus the trailing box
    // entries must walk the sections' pages contiguously (the boxes
    // sit last in the author's layout; our writer appends them there
    // too).
    let n = shape.map_order.len();
    if n < 2
        || shape.map_order[n - 1].id != page_map_id
        || shape.map_order[n - 2].id != shape.section_info_id as i32
    {
        decline!("the page map does not end with the two box pages");
    }
    let mut id_owner: HashMap<i32, usize> = HashMap::new();
    for (index, sec) in shape.sections.iter().enumerate() {
        for (id, _) in &sec.pages {
            if id_owner.insert(*id, index).is_some() {
                decline!(format!("duplicate page id {id}"));
            }
        }
    }
    let mut order: Vec<usize> = Vec::new();
    for entry in &shape.map_order[..n - 2] {
        let owner = match id_owner.get(&entry.id) {
            Some(owner) => *owner,
            None => decline!(format!("map id {} owns no descriptor page", entry.id)),
        };
        if order.last() != Some(&owner) {
            if order.contains(&owner) {
                // Interleaved section pages: not reproducible without
                // scattering a section's pages across the emission.
                decline!(format!(
                    "interleaved pages around map id {}",
                    entry.id
                ));
            }
            order.push(owner);
        }
    }
    // Every paged section must appear in the physical walk.
    let paged: HashSet<usize> = shape
        .sections
        .iter()
        .enumerate()
        .filter(|(_, sec)| !sec.pages.is_empty())
        .map(|(index, _)| index)
        .collect();
    if !order.iter().all(|index| paged.contains(index)) || order.len() != paged.len() {
        decline!("a paged section is missing from the physical page walk");
    }
    // The 0-page descriptors (the unnamed AcDs) emit descriptor-table
    // entries only — appended after the data sections.
    for (index, sec) in shape.sections.iter().enumerate() {
        if sec.pages.is_empty() {
            order.push(index);
        }
    }

    if std::env::var_os("AC18_MIRROR_DEBUG").is_some() {
        eprintln!(
            "[ac18-mirror] engaged: {} data pages, order {:?}",
            data_pages, order
        );
    }
    Some(Ac18MirrorPlan { order })
}

// ════════════════════════════════════════════════════════════════════════════
//  AC21 format (R2007) — RS-encoded pages with LZ77 AC21 compression
// ════════════════════════════════════════════════════════════════════════════

fn write_ac21<W: Write + Seek>(
    output: &mut W,
    document: &CadDocument,
    version: DxfVersion,
    state_matches: bool,
) -> Result<()> {
    write_ac21_impl(output, document, version, false, state_matches)
}

fn write_ac21_impl<W: Write + Seek>(
    output: &mut W,
    document: &CadDocument,
    version: DxfVersion,
    skip_lz77: bool,
    state_matches: bool,
) -> Result<()> {
    // AC21 writer reserves 0x480 bytes at file start (0x80 metadata + 0x400 file header)
    let mut fhw = DwgFileHeaderWriterAC21::new(version, output)?;
    fhw.skip_lz77 = skip_lz77;
    // §19 H7f: a same-version roundtrip re-emits the author's FILEHEADER
    // identity bytes (the R2007 metadata hardcodes 0x19/0x1B/0x19/30
    // before this row; the corpus R2007 authors wrote e.g. 50/33/255/30
    // per build — Box_2007: maint_rel 50, maint 255, app pair 33/255).
    // R2007's section layout has no maint-gated arms (its reader path
    // never reads the extra RL), so the mirror is layout-neutral.
    if document.dwg_source_version == Some(version) {
        if let Some(fh) = document.dwg_file_header.as_ref() {
            fhw.set_source_header_bytes(
                fh.maint_rel_version,
                fh.dwg_version,
                fh.maint_version,
                fh.codepage,
                fh.unknown_0,
                fh.app_dwg_version,
                fh.app_maint_version,
            );
        }
        // §19 H7g: the author's `random_seed` — the AC21 CRC encoder's
        // seed — on the same-version roundtrip (decode-inert; the
        // derived crc-seed fields follow the author's RNG sequence).
        // Gated on the author's `crc_seed` matching our fixed 0 (the
        // spec constant), so the draws stay the deterministic author
        // sequence.
        if let Some(sys) = document.dwg_r2007_header.as_ref() {
            if sys.crc_seed == 0 {
                fhw.set_source_random_seed(sys.random_seed);
            }
        }
    }

    // ── Phase 1: Compute objects FIRST to get handle map ──
    let objects_started = web_time::Instant::now();
    let obj_writer = DwgObjectWriter::new(document)?;
    let (
        obj_data,
        handle_map_u32,
        extents,
        sab_entries,
        class_instance_counts,
        class_counts_complete,
    ) = obj_writer.write_with_class_metadata();
    if std::env::var_os("PERF").is_some() {
        eprintln!(
            "[perf] dwg-write objects={:.1}ms bytes={} handles={} format=ac21",
            objects_started.elapsed().as_secs_f64() * 1000.0,
            obj_data.len(),
            handle_map_u32.len(),
        );
    }
    if std::env::var_os("DWG_RECORD_TRACE").is_some() {
        // §19 H8h diagnostics: our record map — (handle, offset into
        // the compact conventional objects stream) — paired with the
        // reader trace for record-level autopsy.
        for &(h, o) in &handle_map_u32 {
            eprintln!("[record-trace our] {h:X} {o}");
        }
    }

    // ── Phase 2: Prepare header (sync handles + correct HANDSEED) ──
    let corrected_header = prepare_header(document, &handle_map_u32, &extents);

    // ── The section buffers, all built up front (§19 H8b) ──
    // The AC21 container-shape mirror gates on the content lengths and
    // emits in the author's physical order, so every section is
    // constructed before anything is written — the builders are pure,
    // only `add_section` moves the output stream, so the conventional
    // path below writes byte-identical output to the historical
    // interleaved emission. The preview content is the one lazy
    // section: its container embeds its own absolute data address, so
    // it is built at emission time (her landing slot under the mirror,
    // the current stream position on the conventional path).
    let same_origin = document.dwg_source_version == Some(version);

    // SummaryInfo — the same presence-coupling gate as the AC18 writer
    // (§19 H7 review): skip the section when the original read carried
    // summaryinfo_address 0 and the model still holds the default
    // (a modified summary writes the section — user intent wins).
    let summary_orig_present = document
        .dwg_file_header
        .as_ref()
        .map(|fh| fh.summaryinfo_address != 0)
        .unwrap_or(true);
    let summary_section: Option<Vec<u8>> =
        if summary_orig_present || document.summary_info == crate::document::SummaryInfo::default()
        {
            Some(build_summary_info(version, &document.summary_info))
        } else {
            None
        };

    // AppInfo (§19 H7: verbatim from the source on a same-version
    // roundtrip; SKIPPED when the source had none — gold prints the
    // section unconditionally, so the boilerplate would diverge there.
    // Programmatic documents and version conversions keep the
    // historical boilerplate.)
    let app_info_section: Option<Vec<u8>> = if same_origin {
        document.raw_app_info_data.as_deref().map(|raw| raw.to_vec())
    } else {
        Some(app_info_writer::write_app_info(version))
    };

    // AppInfoHistory (§19 H7: never written before this row — verbatim
    // when the same-version source carried one, skipped otherwise.)
    let app_info_history_section: Option<Vec<u8>> = if same_origin {
        document
            .raw_app_info_history_data
            .as_deref()
            .map(|raw| raw.to_vec())
    } else {
        None
    };

    // FileDepList
    let file_dep_data = build_file_dep_list();

    // RevHistory
    let rev_history_data = build_rev_history();

    // AcDsPrototype_1b (AC1027+ ACIS SAB storage)
    let acds_section: Option<Vec<u8>> = if !sab_entries.is_empty()
        || (same_origin && document.raw_acds_data.is_some())
    {
        Some(acds_data(document, version, &sab_entries).into_owned())
    } else {
        None
    };

    // ObjFreeSpace
    // §19 H7e: verbatim from the same-version source (the author's
    // pattern words / TDUPDATE / max constants); SKIPPED when the
    // source had none (gold's zeroed print then matches on both
    // sides); programmatic documents and conversions keep the rebuild.
    let obj_free_space_section: Option<Vec<u8>> = if same_origin {
        document
            .raw_obj_free_space_data
            .as_deref()
            .map(|raw| raw.to_vec())
    } else {
        Some(build_obj_free_space(version, document, handle_map_u32.len()))
    };

    // Template
    let template = build_template(&[], document.header.measurement)?;

    // Handles (needs objects data for offsets)
    let section_offset = fhw.handle_section_offset() as i32;
    let handle_map_i64: Vec<(u64, i64)> =
        handle_map_u32.iter().map(|&(h, o)| (h as u64, o as i64)).collect();
    let handles_data = handle_writer::write_handles(&handle_map_i64, section_offset);

    // Classes
    let classes = reconciled_classes(document, &class_instance_counts, class_counts_complete);
    let maint = document.maintenance_version;
    let header_encoding =
        crate::io::dxf::code_page::encoding_from_code_page(&document.header.code_page)
            .unwrap_or(encoding_rs::WINDOWS_1252);
    let classes_data =
        classes_section_data(document, version, &classes, maint, header_encoding);

    // AuxHeader (uses corrected HANDSEED)
    let aux_data = aux_header_writer::write_aux_header(version, &corrected_header);

    // Header (uses corrected HANDSEED)
    let header_data = header_writer::write_header_with_encoding_opt(
        version,
        &corrected_header,
        maint,
        header_encoding,
        document.dwg_header_raw.as_ref(),
    );

    // ── The §19 H8b container-shape mirror gate and emission ──
    // One root closes three census families at once when it holds:
    // the author's pages-map (size, id) pairs re-emitted in her
    // physical order reproduce her tiling exactly (a running sum from
    // 0x480), so the R2007_Header pages-map family, the FILEHEADER
    // 0x80-block addresses and the THUMBNAILIMAGE identity land hers
    // wherever our re-encoded content demonstrably fits her page
    // slots. The gate falls back to the conventional layout on ANY
    // divergence — a section our content does not fit, our-extra
    // content with no page space, a terminator pair inside her map
    // bytes, an over-slot RS form: the rewrite stays valid everywhere
    // and the residue rows stay open on the files it declines.
    //
    // §19 H8d: the objects-stream raw echo — her reconstructed section
    // re-emits verbatim in her slots when the document's object
    // identity is unchanged (the classes-verbatim fingerprint
    // doctrine). Her physical layout is her editor's incremental-save
    // allocation history — unmodelable by rule — and the mirror's own
    // doctrine for unmodelable authored state is echo. The echoed
    // section needs her record addresses, so the handle map re-emits
    // from the captured pairs (her offsets into her stream), not our
    // compact emission's.
    let objects_echo: Option<(&[u8], Vec<u8>)> = match (
        document.raw_acdb_objects_data.as_deref(),
        document.raw_acdb_objects_handles.as_deref(),
    ) {
        (Some(her_raw), Some(her_handles)) if state_matches => {
            let her_handles_data = handle_writer::write_handles(her_handles, 0);
            if std::env::var_os("AC21_MIRROR_DEBUG").is_some() {
                eprintln!(
                    "[ac21-mirror] objects echo ENGAGED — her raw {} bytes, {} handles",
                    her_raw.len(),
                    her_handles.len()
                );
            }
            Some((&her_raw[..], her_handles_data))
        }
        _ => {
            if std::env::var_os("AC21_MIRROR_DEBUG").is_some()
                && document.raw_acdb_objects_data.is_some()
            {
                // §19 H8g: the state-hash gate — the decline means the
                // document changed between read and write (any edit,
                // including in-place field edits).
                eprintln!(
                    "[ac21-mirror] objects echo DECLINED — document state changed"
                );
            }
            None
        }
    };
    let (objects_buffer, handles_buffer): (&[u8], &[u8]) = match &objects_echo {
        Some((raw, her_handles_data)) => (&raw[..], &her_handles_data[..]),
        None => (&obj_data[..], &handles_data[..]),
    };
    // §19 H8e-2's full-file echo is HOISTED to `write_to_writer` (it
    // fires before the prepare pipeline, on the pre-prepare state
    // verdict); reaching here means it did not engage — the mirror
    // fit-gate path below serves the files the whole-file echo
    // cannot (a missing tail retention, or a same-origin document
    // whose state still holds but whose tail is absent).
    let mirror_plan = match (
        document.dwg_ac21_shape.as_ref(),
        document.dwg_r2007_header.as_ref(),
    ) {
        (Some(shape), Some(sys)) if same_origin => {
            // The preview bytes: the retained raw container re-emits
            // verbatim at her address (its embedded image offsets are
            // hers and stay the valid absolute addresses in the
            // mirrored layout); a rebuild addresses her landing slot.
            let her_preview_addr = shape
                .sections
                .iter()
                .find(|sec| sec.name == section_names::PREVIEW)
                .and_then(|sec| sec.pages.first())
                .and_then(|page| {
                    crate::io::dwg::file_headers::file_header_ac21::ac21_author_page_address(
                        shape, page.id,
                    )
                })
                .unwrap_or(0);
            let preview_bytes: std::borrow::Cow<'_, [u8]> = match &document.preview {
                Some(p) if !p.raw.is_empty() => std::borrow::Cow::from(&p.raw[..]),
                p => std::borrow::Cow::Owned(crate::io::dwg::preview::build_preview(
                    p.as_ref(),
                    her_preview_addr,
                )),
            };
            let buffers =
                crate::io::dwg::file_headers::file_header_ac21::Ac21MirrorBuffers {
                    summary: summary_section.as_deref(),
                    preview: &preview_bytes,
                    app_info: app_info_section.as_deref(),
                    app_info_history: app_info_history_section.as_deref(),
                    file_dep: Some(&file_dep_data),
                    rev_history: &rev_history_data,
                    objects: objects_buffer,
                    acds: acds_section.as_deref(),
                    obj_free_space: obj_free_space_section.as_deref(),
                    xref_manifest: document
                        .raw_xref_manifest_data
                        .as_deref()
                        .map(|raw| &raw[..]),
                    template: &template,
                    handles: handles_buffer,
                    classes: &classes_data,
                    aux_header: &aux_data,
                    header: &header_data,
                };
            fhw.mirror_plan(shape, sys, &buffers)
        }
        _ => None,
    };

    if let Some(plan) = mirror_plan {
        fhw.write_mirrored_pages(output, &plan)?;
        fhw.write_file_mirrored(output)?;
        // The §19 H8b debug oracle: the derived 0x80-block addresses
        // must equal the retained author values under the mirrored
        // layout (they land naturally — nothing is forced but the
        // layout itself; AC21_MIRROR_DEBUG carries the trace).
        if std::env::var_os("AC21_MIRROR_DEBUG").is_some() {
            if let Some(fh) = document.dwg_file_header.as_ref() {
                eprintln!(
                    "[ac21-mirror] 0x80 addresses — summaryinfo derived {} retained {}",
                    fhw.section_page_address(section_names::SUMMARY_INFO),
                    fh.summaryinfo_address
                );
                eprintln!(
                    "[ac21-mirror] 0x80 addresses — thumbnail derived {} retained {}",
                    fhw.section_page_address(section_names::PREVIEW),
                    fh.thumbnail_address
                );
            }
        }
        return Ok(());
    }

    // ── Sections in spec §5.1 stream order (the conventional
    // sequential layout — byte-identical to the historical emission
    // for every non-mirrored file) ──
    if let Some(summary_data) = &summary_section {
        fhw.add_section(output, section_names::SUMMARY_INFO, summary_data)?;
    }

    // Preview
    // AC21 encoding=1 stores contiguous data followed by RS parity. The preview
    // stays in one page, so its image offsets address the data at this position.
    // Compute them before encoding, since the page CRC covers the final bytes.
    let preview_base = output.seek(std::io::SeekFrom::Current(0))? as u64;
    let preview_data =
        crate::io::dwg::preview::build_preview(document.preview.as_ref(), preview_base);
    fhw.add_section(output, section_names::PREVIEW, &preview_data)?;

    if let Some(app_info_data) = &app_info_section {
        fhw.add_section(output, section_names::APP_INFO, app_info_data)?;
    }
    if let Some(app_info_history_data) = &app_info_history_section {
        fhw.add_section(output, section_names::APP_INFO_HISTORY, app_info_history_data)?;
    }
    fhw.add_section(output, section_names::FILE_DEP_LIST, &file_dep_data)?;
    fhw.add_section(output, section_names::REV_HISTORY, &rev_history_data)?;
    fhw.add_section(output, section_names::ACDB_OBJECTS, &obj_data)?;
    if let Some(data) = &acds_section {
        fhw.add_section(output, section_names::ACDS_PROTOTYPE, data)?;
    }
    if let Some(data) = &obj_free_space_section {
        fhw.add_section(output, section_names::OBJ_FREE_SPACE, data)?;
    }
    fhw.add_section(output, section_names::TEMPLATE, &template)?;
    fhw.add_section(output, section_names::HANDLES, &handles_data)?;
    fhw.add_section(output, section_names::CLASSES, &classes_data)?;
    fhw.add_section(output, section_names::AUX_HEADER, &aux_data)?;
    fhw.add_section(output, section_names::HEADER, &header_data)?;

    // ── Finalize: section map, page map, file header, metadata ──
    fhw.write_file(output)?;

    Ok(())
}

// ════════════════════════════════════════════════════════════════════════════
//  Section data builders for simple/metadata sections
// ════════════════════════════════════════════════════════════════════════════

/// Build ObjFreeSpace section data.
///
/// Contains approximate object count and a fixed data template.
/// Matches the reference `writeObjFreeSpace`.
fn build_obj_free_space(
    version: DxfVersion,
    document: &CadDocument,
    handle_count: usize,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(64);

    // Int32: 0
    data.extend_from_slice(&0i32.to_le_bytes());
    // UInt32: approximate number of objects (handles)
    data.extend_from_slice(&(handle_count as u32).to_le_bytes());

    // Julian datetime (8 bytes)
    // For simplicity, write zeros (ODA-compatible)
    if version >= DxfVersion::AC1015 {
        let _ = &document.header; // future: use TDUPDATE
    }
    data.extend_from_slice(&0i32.to_le_bytes()); // jdate
    data.extend_from_slice(&0i32.to_le_bytes()); // milli

    // UInt32: offset of objects section (0 for paged format)
    data.extend_from_slice(&0u32.to_le_bytes());

    // UInt8: number of 64-bit values that follow (ODA writes 4)
    data.push(4);
    // 4 × (u32 + u32) = 4 × 8 bytes of fixed ODA values
    data.extend_from_slice(&0x00000032u32.to_le_bytes());
    data.extend_from_slice(&0x00000000u32.to_le_bytes());
    data.extend_from_slice(&0x00000064u32.to_le_bytes());
    data.extend_from_slice(&0x00000000u32.to_le_bytes());
    data.extend_from_slice(&0x00000200u32.to_le_bytes());
    data.extend_from_slice(&0x00000000u32.to_le_bytes());
    data.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes());
    data.extend_from_slice(&0x00000000u32.to_le_bytes());

    data
}

/// Build Template section data.
///
/// Contains a two-byte description length, the encoded description, and the
/// two-byte MEASUREMENT value.
fn build_template(description: &[u8], measurement: i16) -> Result<Vec<u8>> {
    let description_len = i16::try_from(description.len())
        .map_err(|_| DxfError::InvalidFormat("Template description is too long".into()))?;
    if !matches!(measurement, 0 | 1) {
        return Err(DxfError::InvalidFormat(format!(
            "Invalid MEASUREMENT value: {measurement}"
        )));
    }

    let mut data = Vec::with_capacity(description.len() + 4);
    data.extend_from_slice(&description_len.to_le_bytes());
    data.extend_from_slice(description);
    data.extend_from_slice(&measurement.to_le_bytes());
    Ok(data)
}

/// Build SummaryInfo section data (AC18+ only).
///
/// Writes the document's summary info (§19 H7: the values the H4 read
/// retained — a default document emits the historical all-empty block).
///
/// **AC1018 (R2004)**: Windows-1252 (ANSI) strings.
///   Format: UInt16(byte_count_incl_null) + bytes + null.
///   Empty → UInt16(1) + 0x00 = 3 bytes.
///
/// **AC1021 (R2007)**: UTF-16LE strings.
///   Format: UInt16(char_count_incl_null) + UTF-16LE chars.
///   Empty → UInt16(1) + 0x00 0x00 = 4 bytes.
fn build_summary_info(version: DxfVersion, si: &crate::document::SummaryInfo) -> Vec<u8> {
    let is_utf16 = version >= DxfVersion::AC1021;

    let mut data = Vec::with_capacity(128);
    let push_string = |data: &mut Vec<u8>, s: &str| {
        if !is_utf16 {
            let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(s);
            let n = bytes.len() as u16 + 1; // byte count including null
            data.extend_from_slice(&n.to_le_bytes());
            data.extend_from_slice(&bytes);
            data.push(0);
        } else {
            let units: Vec<u16> = s.encode_utf16().collect();
            let n = units.len() as u16 + 1; // char count including null
            data.extend_from_slice(&n.to_le_bytes());
            for u in units {
                data.extend_from_slice(&u.to_le_bytes());
            }
            data.extend_from_slice(&0u16.to_le_bytes());
        }
    };

    // 8 fixed strings:
    // Title, Subject, Author, Keywords, Comments, LastSavedBy, RevisionNumber, HyperlinkBase
    for s in [
        &si.title,
        &si.subject,
        &si.author,
        &si.keywords,
        &si.comments,
        &si.last_saved_by,
        &si.revision_number,
        &si.hyperlink_base,
    ] {
        push_string(&mut data, s);
    }

    // Total editing time: 2 × u32 (days, ms)
    data.extend_from_slice(&si.tdindwg[0].to_le_bytes());
    data.extend_from_slice(&si.tdindwg[1].to_le_bytes());

    // Created date: 2 × u32
    data.extend_from_slice(&si.tdcreate[0].to_le_bytes());
    data.extend_from_slice(&si.tdcreate[1].to_le_bytes());

    // Modified date: 2 × u32
    data.extend_from_slice(&si.tdupdate[0].to_le_bytes());
    data.extend_from_slice(&si.tdupdate[1].to_le_bytes());

    // Property count: u16, then the (tag, value) pairs
    data.extend_from_slice(&(si.custom_properties.len() as u16).to_le_bytes());
    for (tag, val) in &si.custom_properties {
        push_string(&mut data, tag);
        push_string(&mut data, val);
    }

    // 2 × u32 trailing (gold's unknown1/unknown2)
    data.extend_from_slice(&si.unknown1.to_le_bytes());
    data.extend_from_slice(&si.unknown2.to_le_bytes());

    data
}

/// Build FileDepList section data (AC18+ only).
///
/// Empty dependency list (0 features, 0 files).
fn build_file_dep_list() -> Vec<u8> {
    let mut data = Vec::with_capacity(8);
    // Int32: feature count (0)
    data.extend_from_slice(&0u32.to_le_bytes());
    // Int32: file count (0)
    data.extend_from_slice(&0u32.to_le_bytes());
    data
}

/// Build RevHistory section data (AC18+ only).
///
/// Empty revision history (3 × Int32 zeros).
fn build_rev_history() -> Vec<u8> {
    let mut data = Vec::with_capacity(16);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes()); // revision counter
    data.extend_from_slice(&0u32.to_le_bytes());
    data
}

/// Build AcDsPrototype_1b section data (AC1027+ only).
///
/// This section stores SAB binary ACIS data for 3DSOLID, REGION, and BODY
/// entities in R2013+ DWG files.  The entity stream writes `acis_empty=true`
/// and the actual SAB data lives here, linked by entity handle.
///
/// ## Binary format (reverse-engineered from IntelliCAD-saved reference files)
///
/// The section consists of a 128-byte "jard" header followed by 7 segments:
///
/// | Segment   | ID | Description                                   |
/// |-----------|----|-----------------------------------------------|
/// | `_data_`  | 2  | SAB binary data records (one per entity)      |
/// | `_data_`  | 3  | Thumbnail data table (empty, boilerplate)     |
/// | `datidx`  | 4  | Data index                                    |
/// | `schdat`  | 5  | Schema column definitions                     |
/// | `schidx`  | 6  | Schema index + schema names                   |
/// | `search`  | 7  | Handle-based search/lookup index              |
/// | `segidx`  | 1  | Segment index (offsets of all other segments)  |
///
/// Each segment has a 48-byte header:
///   `marker[8] + id[4] + pad[4] + size[8] + records[8] + meta[8] + fill[8]`
///
/// Segments are padded with 0x70 bytes to 16-byte alignment.
/// One container segment kind in the AcDs data store.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AcDsSegKind {
    Segidx,
    Datidx,
    Data,
    SchdatA,
    SchdatB,
    Schidx,
    Prvsav,
    Search,
    Freesp,
}

/// The AcDs container genus per era (§20.2 G-C): the authored-specimen
/// invariants measured live from the sh_history fixtures on 2026-09-28
/// — ds_version, the fixed 91/97-row scales, the slot allocations (the
/// tail cluster plus the early schdat and the fixed schidx/schdat
/// slots), the named pointers, and the physical emission order. The
/// genus is the reader-visible pattern; segment interior sizes beyond
/// the fixed templates are not genus (the gate measures the row view).
struct AcDsEraProfile {
    ds_version: u32,
    num_segidx: usize,
    slot_schdat_a: usize,
    slot_segidx: usize,
    slot_datidx: usize,
    slot_data: usize,
    slot_prvsav: usize,
    slot_schidx: usize,
    slot_schdat_b: usize,
    slot_search: usize,
    slot_freesp: Option<usize>,
    /// Per-segment ds_version — the 2026-09-30 wrapper-arm round-2
    /// measurement (Box_2013/Box_2018/Cone/Sphere-2018, the
    /// MODELED Form-A specimens): every authored container's
    /// SEGMENTS agree with its file-level ds_version — the tail
    /// (segidx, datidx, _data_, prvsav, search, freesp) carries
    /// the save-era value (17 for R2013, 16 for R2018), while the
    /// schdat-A slot keeps 1 and the schema pair (schidx +
    /// schdat-B) carry 16 in BOTH eras — those two live inside the
    /// captured template bytes natively. The constructed container
    /// had hardcoded 1 in EVERY segment against the 16/17 file
    /// header — an incoherence neither authored form carries,
    /// invisible to the G-C gate set (the per-segment fields were
    /// never pinned) and a modeler-reject candidate ("Data stream
    /// is empty").
    seg_ds_tail: u32,
    /// Physical emission order after the segidx segment.
    order: &'static [AcDsSegKind],
}

/// R2013 (AC1027): ds_version=17, 97 rows; the schema pair at the
/// fixed slots 88/89; the operational tail segidx..search at 91..95
/// plus freesp; prvsav physically after schidx.
const ACDS_ERA_2013: AcDsEraProfile = AcDsEraProfile {
    ds_version: 17,
    num_segidx: 97,
    slot_schdat_a: 5,
    slot_segidx: 91,
    slot_datidx: 92,
    slot_data: 93,
    slot_prvsav: 94,
    slot_schidx: 88,
    slot_schdat_b: 89,
    slot_search: 95,
    slot_freesp: Some(96),
    seg_ds_tail: 17,
    order: &[
        AcDsSegKind::Datidx,
        AcDsSegKind::SchdatB,
        AcDsSegKind::SchdatA,
        AcDsSegKind::Data,
        AcDsSegKind::Schidx,
        AcDsSegKind::Prvsav,
        AcDsSegKind::Search,
        AcDsSegKind::Freesp,
    ],
};

/// R2018 (AC1032): ds_version=16, 91 rows; the schema pair at the fixed
/// slots 88/89 plus the early schdat at 5; the operational tail
/// segidx..search at 84..90; prvsav physically right after datidx.
const ACDS_ERA_2018: AcDsEraProfile = AcDsEraProfile {
    ds_version: 16,
    num_segidx: 91,
    slot_schdat_a: 5,
    slot_segidx: 84,
    slot_datidx: 85,
    slot_data: 86,
    slot_prvsav: 87,
    slot_schidx: 88,
    slot_schdat_b: 89,
    slot_search: 90,
    slot_freesp: None,
    seg_ds_tail: 16,
    order: &[
        AcDsSegKind::Datidx,
        AcDsSegKind::Prvsav,
        AcDsSegKind::SchdatB,
        AcDsSegKind::SchdatA,
        AcDsSegKind::Data,
        AcDsSegKind::Schidx,
        AcDsSegKind::Search,
    ],
};

fn acds_era_profile(dxf_version: DxfVersion) -> &'static AcDsEraProfile {
    if dxf_version == DxfVersion::AC1027 {
        &ACDS_ERA_2013
    } else {
        &ACDS_ERA_2018
    }
}

fn build_acds_prototype(
    sab_entries: &[(Handle, Vec<u8>)],
    dxf_version: DxfVersion,
    thumbnail: Option<u32>,
) -> Vec<u8> {
    if sab_entries.is_empty() {
        return Vec::new();
    }
    let profile = acds_era_profile(dxf_version);

    // ── Content segments (the fixed datastore schema + the SAB records) ──
    // The 2026-09-30 wrapper-arm round-2 measured set: the segment-id
    // fields carry the SEGIDX SLOT numbers (the authored containers'
    // 84-90 / 91-96 ids — the old builders wrote a 1-9 convention
    // the modelers' resid lookup would not resolve), the per-segment
    // ds_version must agree with the file-level value (16/17), and
    // the schema pair is the authored Form-A bytes verbatim —
    // schdat-B defines the EIGHT-column ASM_Data schema the _data_
    // record rows reference (the old 448-byte IntelliCAD-era capture
    // defined seven columns: a ds-16 modeler resolving a record
    // against the wrong schema walks away empty-handed — "Data
    // stream is empty").
    //
    // The 2026-09-30 thumbnail-row packet: her `_data_` row 0 is
    // ALWAYS the Model Layout's preview record (the PNG chunk,
    // handle = the layout object's) and the ASM records follow at
    // rows 1..n — the entry list below prepends the thumbnail so the
    // generic row/locator logic shifts every ASM row +1 exactly as
    // her containers carry them, and the datidx/search builders emit
    // the row-0 record (schidx 0 / the schema-0 search entry).
    let png = acds_thumbnail_png();
    let data_entries: Vec<(Handle, Vec<u8>)> = thumbnail
        .map(|layout_handle| {
            let mut v = Vec::with_capacity(sab_entries.len() + 1);
            v.push((Handle::new(layout_handle as u64), png.clone()));
            v.extend_from_slice(sab_entries);
            v
        })
        .unwrap_or_else(|| sab_entries.to_vec());
    let data = build_acds_data2_segment(
        &data_entries,
        profile.slot_data as u32,
        profile.seg_ds_tail,
    );
    let datidx = build_acds_datidx(
        sab_entries.len(),
        profile.slot_datidx as u32,
        profile.seg_ds_tail,
        profile.slot_data as u32,
        thumbnail.is_some(),
    );
    // schdat-A keeps its native id 5 / ds 1 / align 14 (both eras,
    // identical bytes); schdat-B and schidx carry their native slot
    // ids (89/88) and ds 16 — the captured bytes need no patching.
    let schdat_a = ACDS_SCHDAT_A_TEMPLATE.to_vec();
    let schdat_b = ACDS_SCHDAT_B_TEMPLATE.to_vec();
    let schidx = ACDS_SCHIDX_TEMPLATE.to_vec();
    let prvsav = build_acds_empty_segment(
        b"prvsav",
        profile.slot_prvsav as u32,
        256,
        profile.seg_ds_tail,
    );
    let search = {
        let handles: Vec<u32> = sab_entries.iter().map(|(h, _)| h.value() as u32).collect();
        build_acds_search_segment(
            &handles,
            profile.slot_search as u32,
            profile.seg_ds_tail,
            thumbnail,
        )
    };
    let freesp = profile
        .slot_freesp
        .map(|slot| build_acds_empty_segment(b"freesp", slot as u32, 128, profile.seg_ds_tail));

    // ── Layout: the jard header (128) then the segidx FIRST (the
    // authored segidx-first genus), then the physical order above ──
    let segidx_offset = 0x80usize;
    let segidx_size = align16(48 + profile.num_segidx * 12);
    let mut segidx = {
        let mut seg = vec![0u8; segidx_size];
        seg[0..8].copy_from_slice(&[0xAC, 0xD5, 0x73, 0x65, 0x67, 0x69, 0x64, 0x78]); // "segidx"
        seg[8..12].copy_from_slice(&(profile.slot_segidx as u32).to_le_bytes()); // the slot id
        seg[12..16].copy_from_slice(&0u32.to_le_bytes()); // pad
        seg[16..24].copy_from_slice(&(segidx_size as u64).to_le_bytes()); // segment size
        seg[24..32].copy_from_slice(&(profile.seg_ds_tail as u64).to_le_bytes()); // ds_version (era-coherent, measured)
        seg[32..40].copy_from_slice(&0u64.to_le_bytes()); // meta
        seg[40..48].copy_from_slice(&[0x55; 8]); // fill
        seg
    };

    let mut off = segidx_offset + segidx_size;
    let mut placed: Vec<(AcDsSegKind, usize, usize)> = Vec::new();
    for kind in profile.order {
        let bytes: &[u8] = match kind {
            AcDsSegKind::Datidx => &datidx,
            AcDsSegKind::Prvsav => &prvsav,
            AcDsSegKind::SchdatB => &schdat_b,
            AcDsSegKind::SchdatA => &schdat_a,
            AcDsSegKind::Data => &data,
            AcDsSegKind::Schidx => &schidx,
            AcDsSegKind::Search => &search,
            AcDsSegKind::Freesp => freesp.as_deref().expect("freesp only ordered for 2013"),
            AcDsSegKind::Segidx => unreachable!("segidx is placed by the layout, not the order"),
        };
        placed.push((*kind, off, bytes.len()));
        off += bytes.len();
    }
    let total_size = off;

    // ── The 91/97-row index table: zeroed rows except the allocated
    // slots (the unused rows are offset 0 — the reader's empty slots) ──
    let slot_for = |kind: AcDsSegKind| -> usize {
        match kind {
            AcDsSegKind::SchdatA => profile.slot_schdat_a,
            AcDsSegKind::Segidx => profile.slot_segidx,
            AcDsSegKind::Datidx => profile.slot_datidx,
            AcDsSegKind::Data => profile.slot_data,
            AcDsSegKind::Prvsav => profile.slot_prvsav,
            AcDsSegKind::Schidx => profile.slot_schidx,
            AcDsSegKind::SchdatB => profile.slot_schdat_b,
            AcDsSegKind::Search => profile.slot_search,
            AcDsSegKind::Freesp => profile.slot_freesp.expect("freed slot only ordered for 2013"),
        }
    };
    let mut rows: Vec<(usize, u32, u32)> = vec![(0, 0, 0); profile.num_segidx];
    rows[slot_for(AcDsSegKind::Segidx)] = (slot_for(AcDsSegKind::Segidx), segidx_offset as u32, segidx_size as u32);
    for (kind, offset, size) in &placed {
        rows[slot_for(*kind)] = (slot_for(*kind), *offset as u32, *size as u32);
    }
    let mut pos = 48;
    for (_slot, offset, size) in &rows {
        write_segidx_entry(&mut segidx, pos, *offset, *size);
        pos += 12;
    }

    // ── Jard header ──────────────────────────────────────────────
    let header = build_acds_jard_header(profile, total_size as u32);

    // ── Assemble ─────────────────────────────────────────────────
    let mut result = Vec::with_capacity(total_size);
    result.extend_from_slice(&header);
    debug_assert_eq!(result.len(), segidx_offset);
    result.extend_from_slice(&segidx);
    for (kind, offset, size) in &placed {
        let bytes: &[u8] = match kind {
            AcDsSegKind::Datidx => &datidx,
            AcDsSegKind::Prvsav => &prvsav,
            AcDsSegKind::SchdatB => &schdat_b,
            AcDsSegKind::SchdatA => &schdat_a,
            AcDsSegKind::Data => &data,
            AcDsSegKind::Schidx => &schidx,
            AcDsSegKind::Search => &search,
            AcDsSegKind::Freesp => freesp.as_deref().expect("freesp only ordered for 2013"),
            AcDsSegKind::Segidx => unreachable!(),
        };
        debug_assert_eq!(result.len(), *offset);
        debug_assert_eq!(bytes.len(), *size);
        result.extend_from_slice(bytes);
    }
    debug_assert_eq!(result.len(), total_size);
    result
}

/// An empty content segment of a fixed total size: the 48-byte seg
/// header (the reader only maps names to kinds) plus a zero body —
/// honest semantics for the sections a fresh file carries no data in
/// (prvsav: no previous save; freesp: an empty free-space list).
fn build_acds_empty_segment(
    name: &[u8; 6],
    segment_idx: u32,
    total_size: usize,
    ds_version: u32,
) -> Vec<u8> {
    let mut seg = vec![0u8; total_size];
    seg[0..2].copy_from_slice(&[0xAC, 0xD5]);
    seg[2..8].copy_from_slice(name);
    seg[8..12].copy_from_slice(&segment_idx.to_le_bytes());
    seg[12..16].copy_from_slice(&0u32.to_le_bytes()); // is_blob01
    seg[16..24].copy_from_slice(&(total_size as u64).to_le_bytes()); // segment size
    seg[24..28].copy_from_slice(&ds_version.to_le_bytes()); // ds_version (era-coherent)
    seg[28..32].copy_from_slice(&0u32.to_le_bytes()); // unknown_3
    seg[32..40].copy_from_slice(&0u64.to_le_bytes()); // align offsets
    seg[40..48].copy_from_slice(&[0x55; 8]); // fill
    seg
}

/// Build the AcDsPrototype_1b file header ("jard", 128 bytes).
///
/// Fourteen little-endian `RL` fields per the ODA datastore layout (field
/// names follow libredwg's `acds.spec`), then zero padding to 128 bytes. A
/// strict reader validates these; a wrong `ds_version` in particular made
/// the whole section read as "invalid data". The values are the
/// authored-specimen genus per era (§20.2 G-C): file_header_size 65664,
/// unknown_1 8, ds_version 16/17, segidx FIRST at offset 128, the era's
/// row scale, and the named pointers into the era's slot allocation.
fn build_acds_jard_header(profile: &AcDsEraProfile, file_size: u32) -> Vec<u8> {
    let mut h = vec![0u8; 128];
    h[0..4].copy_from_slice(b"jard"); // file_signature
    h[4..8].copy_from_slice(&65664u32.to_le_bytes()); // file_header_size
    h[8..12].copy_from_slice(&8u32.to_le_bytes()); // unknown_1
    h[12..16].copy_from_slice(&2u32.to_le_bytes()); // version
    h[16..20].copy_from_slice(&0u32.to_le_bytes()); // unknown_2
    h[20..24].copy_from_slice(&profile.ds_version.to_le_bytes()); // ds_version
    h[24..28].copy_from_slice(&0x80u32.to_le_bytes()); // segidx_offset — segidx-first
    h[28..32].copy_from_slice(&0u32.to_le_bytes()); // segidx_unknown
    h[32..36].copy_from_slice(&(profile.num_segidx as u32).to_le_bytes()); // num_segidx
    h[36..40].copy_from_slice(&(profile.slot_schidx as u32).to_le_bytes()); // schidx_segidx
    h[40..44].copy_from_slice(&(profile.slot_datidx as u32).to_le_bytes()); // datidx_segidx
    h[44..48].copy_from_slice(&(profile.slot_search as u32).to_le_bytes()); // search_segidx
    h[48..52].copy_from_slice(&(profile.slot_prvsav as u32).to_le_bytes()); // prvsav_segidx
    h[52..56].copy_from_slice(&file_size.to_le_bytes()); // file_size
                                                          // Remaining bytes are zero (padding to 128).
    h
}

/// A minimal valid PNG (1×1, white, 8-bit RGB) — the datastore's
/// layout-preview record content (the 2026-09-30 thumbnail-row packet).
///
/// Her `_data_` row 0 is ALWAYS the Model Layout's preview record (a
/// PNG chunk, handle = the layout object's — measured: Box_2018's
/// 896-byte PNG, example_2018's 2017-byte one), and the 2026-09-30
/// ACAD re-probe proved AutoCAD's open-time validation rejects its
/// absence (the RECOVER prompt at the empty schema-0 search block).
/// The preview is document sugar, not author identity — the writer
/// does not render, so the record carries a structurally valid
/// placeholder image (signature + IHDR + a stored-deflate IDAT +
/// IEND, with the crc32/adler32 checksums computed in-place).
fn acds_thumbnail_png() -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
    fn adler32(data: &[u8]) -> u32 {
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &byte in data {
            a = (a + byte as u32) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
    fn push_chunk(kind: &[u8; 4], data: &[u8], out: &mut Vec<u8>) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let mut crc_input = Vec::with_capacity(4 + data.len());
        crc_input.extend_from_slice(kind);
        crc_input.extend_from_slice(data);
        out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    }

    let mut png = Vec::with_capacity(68);
    png.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    // IHDR: 1×1 pixels, 8-bit truecolor RGB, no interlace.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    push_chunk(b"IHDR", &ihdr, &mut png);
    // IDAT: a zlib stream over the scanline (filter byte 0 + one white
    // RGB pixel) using a single final stored-deflate block — no
    // compressor dependency, still a structurally valid stream.
    let raw = [0x00u8, 0xFF, 0xFF, 0xFF];
    let mut idat = Vec::with_capacity(15);
    idat.extend_from_slice(&[0x78, 0x01]); // zlib header (CMF/FLG, check = 0x7801 % 31 == 0)
    idat.push(0x01); // BFINAL=1, BTYPE=00 (stored), padded to the byte
    idat.extend_from_slice(&(raw.len() as u16).to_le_bytes());
    idat.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
    idat.extend_from_slice(&raw);
    idat.extend_from_slice(&adler32(&raw).to_be_bytes());
    push_chunk(b"IDAT", &idat, &mut png);
    push_chunk(b"IEND", &[], &mut png);
    png
}

/// Build `_data_` segment id=2 containing one SAB record per ACIS entity.
///
/// **HER record-row form (the 2026-09-30 row-locator fix — measured on
/// example_2018 [4 rows: LOCs 0 / 0x7e5 / 0xe96 / 0x30f8] and Box_2018
/// [LOCs 0 / 0x384] against the chunk chain)**: a contiguous 20-byte
/// record table —
///
/// ```text
/// (col0 = 0x14 = the row's own size, 1, the owner handle, 0,
///  LOC = the row's chunk offset within the aligned blob area)
/// ```
///
/// — then 0x62 alignment fill to the blob area (her own filler byte),
/// then the length-prefixed chunks `[len u32][blob]` in row order at
/// `blob_base + LOC_i`. LOC_0 is 0 and each LOC is the cumulative sum
/// of the previous chunks' total sizes (4-byte prefix + blob), so row
/// i's chunk sits exactly where its locator points. The 4th u32 stays
/// ZERO (the datidx-fix measurement), and the datidx's row offsets
/// (i*20) name the row positions exactly.
///
/// The multi-record writer defect this row form closes: our old table
/// wrote 16-byte rows with NO locator, so the modeler walking the
/// author's 20-byte stride landed its row reads misaligned — the
/// record resolution collapsed to near-sequence-zero garbage (the
/// A-2 audit: 37D rendered row 0's blob, 176/2E1 "Data stream is
/// empty").
fn build_acds_data2_segment(
    entries: &[(Handle, Vec<u8>)],
    segment_idx: u32,
    ds_version: u32,
) -> Vec<u8> {
    let table_size = entries.len() * 20;
    let blob_base = 48 + table_size;
    let align_pad = (16 - blob_base % 16) % 16;
    let data_start = blob_base + align_pad;
    let records_size: usize = entries.iter().map(|(_, sab)| 4 + sab.len()).sum();
    let raw_size = data_start + records_size;
    let seg_size = align16(raw_size);
    let padding = seg_size - raw_size;

    let mut seg = Vec::with_capacity(seg_size);

    // Segment header (48 bytes)
    seg.extend_from_slice(&[0xAC, 0xD5, 0x5F, 0x64, 0x61, 0x74, 0x61, 0x5F]); // "_data_"
    seg.extend_from_slice(&segment_idx.to_le_bytes()); // the segidx slot id
    seg.extend_from_slice(&0u32.to_le_bytes()); // is_blob01
    seg.extend_from_slice(&(seg_size as u64).to_le_bytes()); // segment size
    seg.extend_from_slice(&ds_version.to_le_bytes()); // ds_version (era-coherent, NOT the record count)
    seg.extend_from_slice(&0u32.to_le_bytes()); // unknown_3
    seg.extend_from_slice(&0u32.to_le_bytes()); // meta field1 = 0
    seg.extend_from_slice(&((data_start / 16) as u32).to_le_bytes()); // objdata_algn_offset
    seg.extend_from_slice(&[0x55; 8]); // fill "UUUUUUUU"

    // The 20-byte record rows (five u32 words) with the per-row chunk
    // locators — the handle is a plain u32 word (her rows: the value's
    // high word is always zero).
    let mut loc: u32 = 0;
    for (handle, sab_data) in entries {
        seg.extend_from_slice(&0x14u32.to_le_bytes()); // col0 = the row size (20)
        seg.extend_from_slice(&1u32.to_le_bytes()); // schema revision, not record index
        seg.extend_from_slice(&(handle.value() as u32).to_le_bytes());
        seg.extend_from_slice(&0u32.to_le_bytes()); // the 4th u32: ZERO (the datidx-fix measurement)
        seg.extend_from_slice(&loc.to_le_bytes()); // the chunk locator (cumulative offsets)
        loc += 4 + sab_data.len() as u32;
    }
    debug_assert_eq!(seg.len(), blob_base);
    // Her alignment filler to the 16-byte-aligned blob area, then the
    // chunks in row order.
    seg.extend(std::iter::repeat(0x62u8).take(align_pad));
    for (_, sab_data) in entries {
        seg.extend_from_slice(&(sab_data.len() as u32).to_le_bytes()); // SAB blob size
        seg.extend_from_slice(sab_data);
    }

    // Padding with 0x70 to 16-byte alignment
    seg.extend(std::iter::repeat(0x70u8).take(padding));

    debug_assert_eq!(seg.len(), seg_size);
    seg
}

/// Write the 48-byte AcDs segment header (signature + 6-char name + the fixed
/// field block: segment_idx, is_blob01=0, segsize, unknown_2=0, the era's
/// ds_version, unknown_3=0, align offsets=0, 8× 0x55 fill).
fn acds_segment_header(seg: &mut [u8], name: &[u8; 6], segment_idx: u32, ds_version: u32) {
    let seg_len = seg.len() as u64;
    seg[0..2].copy_from_slice(&[0xAC, 0xD5]);
    seg[2..8].copy_from_slice(name);
    seg[8..12].copy_from_slice(&segment_idx.to_le_bytes());
    seg[12..16].copy_from_slice(&0u32.to_le_bytes()); // is_blob01
    seg[16..24].copy_from_slice(&seg_len.to_le_bytes()); // segsize + unknown_2
    seg[24..28].copy_from_slice(&ds_version.to_le_bytes()); // ds_version (era-coherent)
    seg[28..32].copy_from_slice(&0u32.to_le_bytes()); // unknown_3
    seg[32..40].copy_from_slice(&0u64.to_le_bytes()); // data/objdata align offsets
    seg[40..48].copy_from_slice(&[0x55; 8]); // fill
}

/// Build the `datidx` segment — one index entry per ACIS record.
///
/// Layout (the authored Form-A genus, measured 2026-09-30 on
/// Box_2018/Box_2013 — the datidx audit): the 48-byte header, then
/// `num_entries` (RL), `di_unknown` (RL = 0), then one 12-byte entry
/// per record — `(segidx = THE _data_ SEGIDX SLOT (86 in the 2018
/// profile / 93 in the 2013 profile — the old builder wrote the
/// 1-9-convention id 2, an EMPTY row in the 91/97-slot table: a
/// loader following our datidx resolved to NOTHING — the modeler's
/// "Data stream is empty" and the blank screen the maintainer saw),
/// offset = i*20 (the record row), schidx = 5 (the
/// AcDb3DSolid_ASM_Data schema — slot 5 in the era-shared schidx
/// template's schema list; her thumbnail row uses 0)`.
fn build_acds_datidx(
    num_records: usize,
    segment_idx: u32,
    ds_version: u32,
    data_slot: u32,
    thumbnail: bool,
) -> Vec<u8> {
    // With the thumbnail record (the 2026-09-30 packet): row 0 is the
    // Model Layout's preview (schidx 0 — the layout/thumbnail schema)
    // and the ASM records follow at rows 1..n (her Box_2018: rows
    // (86, 0, 0) + (86, 20, 5)).
    let num = num_records + usize::from(thumbnail);
    let num = num.max(1);
    let raw = 48 + 8 + num * 12;
    let seg_size = align16(raw).max(128);
    let mut seg = vec![0x70u8; seg_size];
    acds_segment_header(&mut seg, b"datidx", segment_idx, ds_version);

    seg[48..52].copy_from_slice(&(num as u32).to_le_bytes()); // num_entries
    seg[52..56].copy_from_slice(&0u32.to_le_bytes()); // di_unknown
    let mut pos = 56;
    // With the thumbnail record the ASM rows sit at _data_ rows 1..n
    // (her Box_2018: the solid's datidx offset is 20, its row-1
    // position in the record table).
    let row_base = usize::from(thumbnail);
    if thumbnail {
        seg[pos..pos + 4].copy_from_slice(&data_slot.to_le_bytes()); // segidx (→ the real _data_ slot)
        seg[pos + 4..pos + 8].copy_from_slice(&0u32.to_le_bytes()); // offset (row 0)
        seg[pos + 8..pos + 12].copy_from_slice(&0u32.to_le_bytes()); // schidx 0 (the layout schema)
        pos += 12;
    }
    for i in 0..num_records {
        seg[pos..pos + 4].copy_from_slice(&data_slot.to_le_bytes()); // segidx (→ the real _data_ slot)
        seg[pos + 4..pos + 8]
            .copy_from_slice(&(((i + row_base) as u32) * 20).to_le_bytes()); // offset (the record row)
        seg[pos + 8..pos + 12].copy_from_slice(&ACDS_ASM_SCHEMA_IDX.to_le_bytes()); // schidx (the ASM_Data schema)
        pos += 12;
    }
    seg
}

/// Build `search` segment id=7 — the datastore's per-schema lookup indexes,
/// HER authored layout (the 2026-09-30 search-format packet).
///
/// Empirically decoded from the authored corpus (97 specimens extracted;
/// the anchors: Box_2018/Box_2013 Form-A [thumbnail 0x22 + solid 0x2EA],
/// example_2018 Form-B [0x176/0x2E1/0x37D], the test-data files). The
/// header carries `num_search (RL)`, then one block per datastore schema:
///
/// ```text
/// RL  schema_namidx   — the schidx slot the block indexes (0 = the
///                       AcDb_Thumbnail/layout schema, 5 = the ASM_Data)
/// RL  num_sortedidx   — the record-row key count
/// RLL sortedidx[num]  — (record row) << 32, the per-schema row keys
/// RL  num_ididxs = 0  — her constant
/// RL  unknown = 1     — her constant in EVERY block (all schemas, all 97)
/// RL  zero = 0
/// RL  num_handles     — the handle-entry count
/// (RLL handle, RLL 1, RLL row) × num_handles — in row order, the i-th
///                       entry's row = the i-th key's row (the positional
///                       pairing the modeler resolves handle→record
///                       through)
/// ```
///
/// The multi-record writer defect this fixes: the modeler parses the
/// search with HER grammar (schema order + the per-block key/handle
/// interior) — our old form (namidx=1 first + handle-sorted triples +
/// a 24-byte tail) landed the modeler on garbage, resolving handle
/// 0x37D to record 0 in the A-2 audit (entity A rendered entity B's
/// B-rep in every edited multi-solid document).
///
/// Our containers carry the thumbnail record (the 2026-09-30 packet —
/// the ACAD re-probe's RECOVER evidence: AutoCAD's open-time validation
/// rejects the empty layout-schema block) plus the ASM records, so the
/// schema-0 block indexes the Model Layout's preview at row 0 and the
/// ASM block carries the SAB rows at 1..n — the same rows the datidx
/// declares (segidx slot, offset (i+1)*20, schidx 5). Without a
/// resolvable Model Layout the schema-0 block stays empty (her Arc
/// empty-block form) and the ASM rows keep their 0-based numbering.
fn build_acds_search_segment(
    handles: &[u32],
    segment_idx: u32,
    ds_version: u32,
    thumbnail: Option<u32>,
) -> Vec<u8> {
    let n = handles.len();
    let mut content: Vec<u8> = Vec::new();
    content.extend_from_slice(&2u32.to_le_bytes()); // num_search (her ds-SAB genus)

    // Block 1 — schema 0 (the thumbnail/layout schema): the Model
    // Layout's preview record at row 0 (her Box_2018 form).
    content.extend_from_slice(&0u32.to_le_bytes()); // schema_namidx
    match thumbnail {
        Some(layout_handle) => {
            content.extend_from_slice(&1u32.to_le_bytes()); // num_sortedidx
            content.extend_from_slice(&0u64.to_le_bytes()); // key: row 0 << 32
            content.extend_from_slice(&0u32.to_le_bytes()); // num_ididxs
            content.extend_from_slice(&1u32.to_le_bytes()); // unknown (her constant)
            content.extend_from_slice(&0u32.to_le_bytes()); // zero
            content.extend_from_slice(&1u32.to_le_bytes()); // num_handles
            content.extend_from_slice(&(layout_handle as u64).to_le_bytes());
            content.extend_from_slice(&1u64.to_le_bytes());
            content.extend_from_slice(&0u64.to_le_bytes()); // record row 0
        }
        None => {
            content.extend_from_slice(&0u32.to_le_bytes()); // num_sortedidx
            content.extend_from_slice(&0u32.to_le_bytes()); // num_ididxs
            content.extend_from_slice(&1u32.to_le_bytes()); // unknown (her constant)
            content.extend_from_slice(&0u32.to_le_bytes()); // zero
            content.extend_from_slice(&0u32.to_le_bytes()); // num_handles
        }
    }

    // Block 2 — schema 5 (AcDb3DSolid_ASM_Data, the datidx rows' schidx):
    // the record-row keys + the handle triples, in row order — shifted
    // past the thumbnail record when it is present.
    let row_base = u64::from(thumbnail.is_some());
    content.extend_from_slice(&ACDS_ASM_SCHEMA_IDX.to_le_bytes()); // schema_namidx
    content.extend_from_slice(&(n as u32).to_le_bytes()); // num_sortedidx
    for i in 0..n {
        content.extend_from_slice(&(((i as u64) + row_base) << 32).to_le_bytes()); // row key
    }
    content.extend_from_slice(&0u32.to_le_bytes()); // num_ididxs
    content.extend_from_slice(&1u32.to_le_bytes()); // unknown (her constant)
    content.extend_from_slice(&0u32.to_le_bytes()); // zero
    content.extend_from_slice(&(n as u32).to_le_bytes()); // num_handles
    for (i, &handle) in handles.iter().enumerate() {
        content.extend_from_slice(&(handle as u64).to_le_bytes()); // the owner handle
        content.extend_from_slice(&1u64.to_le_bytes()); // her constant
        content.extend_from_slice(&((i as u64) + row_base).to_le_bytes()); // the record row
    }

    // The authored search allocation is FIXED: the Form-A specimens
    // (Box_2018/Cone_2018/Sphere_2018, one handle each; Box_2013)
    // all carry a 256-byte search segment — the size is the slot
    // allocation, not content-computed (the 2026-09-30 size-gate
    // finding: our 192/208 forms ranked against the authored 256).
    let raw = 48 + content.len();
    let seg_size = align16(raw).max(256);
    let mut seg = vec![0x70u8; seg_size];
    acds_segment_header(&mut seg, b"search", segment_idx, ds_version);
    seg[48..48 + content.len()].copy_from_slice(&content);
    seg
}

/// Write one segidx entry: (offset u32, pad u32 = 0, size u32). Unused
/// rows stay all-zero — the reader's empty-slot convention (offset 0).
fn write_segidx_entry(buf: &mut [u8], pos: usize, offset: u32, size: u32) {
    buf[pos..pos + 4].copy_from_slice(&offset.to_le_bytes());
    buf[pos + 4..pos + 8].copy_from_slice(&0u32.to_le_bytes());
    buf[pos + 8..pos + 12].copy_from_slice(&size.to_le_bytes());
}

/// Round up to next 16-byte boundary.
fn align16(n: usize) -> usize {
    (n + 15) & !15
}

/// The `AcDb3DSolid_ASM_Data` schema's slot in the era-shared schidx
/// template — the schema the `_data_` record rows belong to (her
/// datidx entries point schidx = this slot; her search's ASM block
/// carries it as its `schema_namidx`).
const ACDS_ASM_SCHEMA_IDX: u32 = 5;

/// Schema data A (the early slot, 384 bytes, ds_version 1) - the authored
/// Form-A container's verbatim bytes (Box_2018/Box_2013 slot 5, the
/// schema which covers the Thumbnail schema's column set; era-shared:
/// IDENTICAL across the R2013 and R2018 specimens, measured 2026-09-30).
#[rustfmt::skip]
const ACDS_SCHDAT_A_TEMPLATE: &[u8] = &[
    0xAC, 0xD5, 0x73, 0x63, 0x68, 0x64, 0x61, 0x74, 0x05, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
    0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0A, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00,
    0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x73, 0x73, 0x73, 0x06, 0x00, 0x00, 0x00,
    0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A, 0x49, 0x44, 0x00, 0x54,
    0x68, 0x75, 0x6D, 0x62, 0x6E, 0x61, 0x69, 0x6C, 0x5F, 0x44, 0x61, 0x74,
    0x61, 0x00, 0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A, 0x54, 0x72,
    0x65, 0x61, 0x74, 0x65, 0x64, 0x41, 0x73, 0x4F, 0x62, 0x6A, 0x65, 0x63,
    0x74, 0x44, 0x61, 0x74, 0x61, 0x00, 0x41, 0x63, 0x44, 0x62, 0x44, 0x73,
    0x3A, 0x3A, 0x4C, 0x65, 0x67, 0x61, 0x63, 0x79, 0x00, 0x41, 0x63, 0x44,
    0x73, 0x3A, 0x49, 0x6E, 0x64, 0x65, 0x78, 0x61, 0x62, 0x6C, 0x65, 0x00,
    0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A, 0x48, 0x61, 0x6E, 0x64,
    0x6C, 0x65, 0x41, 0x74, 0x74, 0x72, 0x69, 0x62, 0x75, 0x74, 0x65, 0x00,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
];

/// Schema data B (the schema-pair slot, 256 bytes, ds_version 16) - the
/// authored Form-A container's verbatim bytes (Box_2018/Box_2013 slot
/// 89): the EIGHT-column ASM_Data schema the _data_ record rows
/// reference. The old 448-byte IntelliCAD-era capture defined a
/// SEVEN-column variant - one suspect for the modeler's
/// "Data stream is empty" SAB lookup failure (a ds-16 modeler
/// resolving the record's columns against the wrong schema).
/// Era-shared: IDENTICAL across both specimens.
#[rustfmt::skip]
const ACDS_SCHDAT_B_TEMPLATE: &[u8] = &[
    0xAC, 0xD5, 0x73, 0x63, 0x68, 0x64, 0x61, 0x74, 0x59, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
    0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x06, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0A, 0x00, 0x00, 0x00, 0x02, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x00, 0x00, 0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A,
    0x49, 0x44, 0x00, 0x41, 0x53, 0x4D, 0x5F, 0x44, 0x61, 0x74, 0x61, 0x00,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70,
];

/// Schema index (512 bytes, ds_version 16, id 88) - the authored Form-A
/// container's verbatim bytes (Box_2018/Box_2013 slot 88): the schema
/// list and per-schema column offset tables. Era-shared: IDENTICAL
/// across both specimens (verified bit-for-bit 2026-09-30).
#[rustfmt::skip]
const ACDS_SCHIDX_TEMPLATE: &[u8] = &[
    0xAC, 0xD5, 0x73, 0x63, 0x68, 0x69, 0x64, 0x78, 0x58, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0F, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
    0x06, 0x00, 0x00, 0x00, 0x55, 0x55, 0x55, 0x55, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x72, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x96, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
    0x59, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x0C, 0xF1, 0x0A, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x05, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x59, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x59, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
    0x59, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x59, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x06, 0x00, 0x00, 0x00, 0x41, 0x63, 0x44, 0x62, 0x5F, 0x54, 0x68, 0x75,
    0x6D, 0x62, 0x6E, 0x61, 0x69, 0x6C, 0x5F, 0x53, 0x63, 0x68, 0x65, 0x6D,
    0x61, 0x00, 0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A, 0x54, 0x72,
    0x65, 0x61, 0x74, 0x65, 0x64, 0x41, 0x73, 0x4F, 0x62, 0x6A, 0x65, 0x63,
    0x74, 0x44, 0x61, 0x74, 0x61, 0x53, 0x63, 0x68, 0x65, 0x6D, 0x61, 0x00,
    0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A, 0x4C, 0x65, 0x67, 0x61,
    0x63, 0x79, 0x53, 0x63, 0x68, 0x65, 0x6D, 0x61, 0x00, 0x41, 0x63, 0x44,
    0x62, 0x44, 0x73, 0x3A, 0x3A, 0x49, 0x6E, 0x64, 0x65, 0x78, 0x65, 0x64,
    0x50, 0x72, 0x6F, 0x70, 0x65, 0x72, 0x74, 0x79, 0x53, 0x63, 0x68, 0x65,
    0x6D, 0x61, 0x00, 0x41, 0x63, 0x44, 0x62, 0x44, 0x73, 0x3A, 0x3A, 0x48,
    0x61, 0x6E, 0x64, 0x6C, 0x65, 0x41, 0x74, 0x74, 0x72, 0x69, 0x62, 0x75,
    0x74, 0x65, 0x53, 0x63, 0x68, 0x65, 0x6D, 0x61, 0x00, 0x41, 0x63, 0x44,
    0x62, 0x33, 0x44, 0x53, 0x6F, 0x6C, 0x69, 0x64, 0x5F, 0x41, 0x53, 0x4D,
    0x5F, 0x44, 0x61, 0x74, 0x61, 0x00, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
    0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70, 0x70,
];

// ════════════════════════════════════════════════════════════════════════════
//  Tests
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::CadDocument;
    use crate::types::{DxfVersion, Handle};

    #[test]
    fn acds_record_table_indexes_each_length_prefixed_blob() {
        for count in [1, 2, 7, 8, 16] {
            let entries: Vec<_> = (0..count)
                .map(|index| {
                    (
                        Handle::new(0x100 + index as u64),
                        vec![index as u8; 31 + index * 19],
                    )
                })
                .collect();
            let data = build_acds_data2_segment(&entries, 86, 16);
            let index = build_acds_datidx(count, 85, 16, 86, false);
            let read_u32 = |bytes: &[u8], offset| {
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize
            };
            let blob_base = read_u32(&data, 36) * 16;
            assert_eq!(blob_base, 48 + align16(count * 20));
            // the authored record row: (0x14, 1, handle, zero 4th, the
            // chunk locator). The 4th field stays ZERO (measured
            // Box_2018/Box_2013); the 5th word carries the row's
            // CUMULATIVE chunk offset (measured 2026-09-30 on
            // example_2018/Box_2018) so the modeler's 20-byte row
            // stride lands each handle on its own chunk exactly
            let mut blob_cursor = 0usize;
            for (i, (handle, blob)) in entries.iter().enumerate() {
                let record = 48 + read_u32(&index, 60 + i * 12);
                assert_eq!(record, 48 + i * 20);
                assert_eq!(read_u32(&data, record), 20);
                assert_eq!(read_u32(&data, record + 4), 1);
                assert_eq!(
                    u64::from_le_bytes(data[record + 8..record + 16].try_into().unwrap()),
                    handle.value()
                );
                // the 5th word = the row's chunk locator (the 2026-09-30
                // row-form fix: her rows carry the CUMULATIVE chunk offsets
                // — measured on example_2018 [LOCs 0/0x7e5/0xe96/0x30f8]
                // and Box_2018 [0/0x384]); row 0 is always 0 and every
                // locator equals the chain's row-wise cumulative position
                assert_eq!(read_u32(&data, record + 16), blob_cursor);
                assert_eq!(read_u32(&data, blob_base + blob_cursor), blob.len());
                assert_eq!(
                    &data[blob_base + blob_cursor + 4..blob_base + blob_cursor + 4 + blob.len()],
                    blob
                );
                blob_cursor += 4 + blob.len();
                // the datidx entry: the real _data_ slot, the row
                // offset, and the ASM_Data schema (measured: 86/5)
                assert_eq!(read_u32(&index, 56 + i * 12), 86);
                assert_eq!(read_u32(&index, 64 + i * 12), 5);
            }
        }
    }

    #[test]
    fn acds_thumbnail_row_and_search_entry_match_her_genus() {
        // The 2026-09-30 thumbnail-row packet: her `_data_` row 0 is
        // the Model Layout's preview record (a PNG chunk, handle = the
        // layout object's); the datidx gains row 0 (schidx 0) and the
        // search's schema-0 block carries the layout entry, with the
        // ASM rows shifted +1 (her Box_2018: rows [thumbnail, solid],
        // search blocks [schema-0 populated, schema-5 populated]).
        let entries: Vec<(Handle, Vec<u8>)> = (0..2)
            .map(|i| (Handle::new(0x100 + i as u64), vec![i as u8; 40]))
            .collect();
        let png = acds_thumbnail_png();
        let png_total = 4 + png.len();
        let mut with_thumb: Vec<(Handle, Vec<u8>)> = Vec::with_capacity(3);
        with_thumb.push((Handle::new(0x22), png));
        with_thumb.extend(entries.iter().cloned());

        let data = build_acds_data2_segment(&with_thumb, 86, 16);
        let index = build_acds_datidx(2, 85, 16, 86, true);
        let search = build_acds_search_segment(&[0x100, 0x101], 87, 16, Some(0x22));
        let read_u32 = |b: &[u8], o: usize| {
            u32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as usize
        };
        let read_u64 = |b: &[u8], o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());

        // the datidx: 3 rows — the thumbnail (row 0, the layout schema)
        // then the ASM rows at their shifted row positions.
        assert_eq!(read_u32(&index, 48), 3);
        assert_eq!(read_u32(&index, 56), 86); // row 0: the _data_ slot
        assert_eq!(read_u32(&index, 60), 0); // offset: row 0
        assert_eq!(read_u32(&index, 64), 0); // schidx: the layout schema
        assert_eq!(read_u32(&index, 68), 86);
        assert_eq!(read_u32(&index, 72), 20); // the first ASM row's position
        assert_eq!(read_u32(&index, 76), 5); // schidx: the ASM schema
        assert_eq!(read_u32(&index, 80), 86);
        assert_eq!(read_u32(&index, 84), 40);
        assert_eq!(read_u32(&index, 88), 5);

        // the _data_ rows: the thumbnail at row 0 (LOC 0), the ASM
        // rows' locators shifted past the PNG chunk.
        assert_eq!(read_u32(&data, 48), 20); // col0
        assert_eq!(read_u32(&data, 52), 1);
        assert_eq!(read_u32(&data, 56), 0x22); // the layout handle
        assert_eq!(read_u32(&data, 64), 0); // LOC 0
        assert_eq!(read_u32(&data, 68), 20); // row 1 col0
        assert_eq!(read_u32(&data, 76), 0x100);
        assert_eq!(read_u32(&data, 84), png_total); // LOC shifted past the PNG chunk
        assert_eq!(read_u32(&data, 96), 0x101);
        assert_eq!(read_u32(&data, 104), png_total + 44); // + the first SAB chunk

        // the search: the schema-0 block carries the layout entry at
        // row 0; the schema-5 block's keys/rows are shifted +1.
        let s = 48; // the content starts after the 48-byte segment header
        assert_eq!(read_u32(&search, s), 2); // num_search
        assert_eq!(read_u32(&search, s + 4), 0); // schema 0
        assert_eq!(read_u32(&search, s + 8), 1); // one key
        assert_eq!(read_u64(&search, s + 12), 0); // key: row 0
        assert_eq!(read_u32(&search, s + 20), 0); // num_ididxs
        assert_eq!(read_u32(&search, s + 24), 1); // unknown (her constant)
        assert_eq!(read_u32(&search, s + 28), 0); // zero
        assert_eq!(read_u32(&search, s + 32), 1); // one handle
        assert_eq!(read_u64(&search, s + 36), 0x22); // the layout handle
        assert_eq!(read_u64(&search, s + 44), 1);
        assert_eq!(read_u64(&search, s + 52), 0); // record row 0
        assert_eq!(read_u32(&search, s + 60), 5); // the ASM schema block
        assert_eq!(read_u32(&search, s + 64), 2); // two keys
        assert_eq!(read_u64(&search, s + 68), 1 << 32); // row 1
        assert_eq!(read_u64(&search, s + 76), 2 << 32); // row 2
        assert_eq!(read_u32(&search, s + 84), 0); // num_ididxs
        assert_eq!(read_u32(&search, s + 88), 1); // unknown
        assert_eq!(read_u32(&search, s + 92), 0); // zero
        assert_eq!(read_u32(&search, s + 96), 2); // two handles
        assert_eq!(read_u64(&search, s + 100), 0x100);
        assert_eq!(read_u64(&search, s + 108), 1);
        assert_eq!(read_u64(&search, s + 116), 1); // record row 1
        assert_eq!(read_u64(&search, s + 124), 0x101);
        assert_eq!(read_u64(&search, s + 132), 1);
        assert_eq!(read_u64(&search, s + 140), 2); // record row 2
    }

    #[test]
    fn acds_thumbnail_png_is_structurally_valid() {
        // The placeholder preview must parse as a PNG: the signature,
        // the IHDR dimensions/color type, the zlib stream's header
        // check, and the IEND terminator.
        let png = acds_thumbnail_png();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 1); // width
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 1); // height
        assert_eq!(png[24], 8); // bit depth
        assert_eq!(png[25], 2); // color type: truecolor
        assert_eq!(0x7801u32 % 31, 0); // the zlib header's check value
        assert!(png.windows(8).any(|w| w == b"\x00\x00\x00\x00IEND"));
    }

    #[test]
    fn test_validate_version_r2007_ok() {
        assert!(validate_version(DxfVersion::AC1021).is_ok());
    }

    #[test]
    fn test_validate_version_unknown_rejected() {
        let result = validate_version(DxfVersion::Unknown);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_version_r2000_ok() {
        assert!(validate_version(DxfVersion::AC1015).is_ok());
    }

    #[test]
    fn test_validate_version_r2004_ok() {
        assert!(validate_version(DxfVersion::AC1018).is_ok());
    }

    #[test]
    fn test_validate_version_r2010_ok() {
        assert!(validate_version(DxfVersion::AC1024).is_ok());
    }

    #[test]
    fn class_metadata_matches_emitted_records() {
        let document = CadDocument::with_version(DxfVersion::AC1032);
        let action_param = document
            .classes
            .get_by_name("ACDBASSOCCOMPOUNDACTIONPARAM")
            .expect("action parameter class");
        let point_ref = document
            .classes
            .get_by_name("ACDBASSOCOSNAPPOINTREFACTIONPARAM")
            .expect("point reference class");
        assert!(!action_param.was_zombie);
        assert!(!point_ref.was_zombie);

        let (_, _, _, _, counts, complete) = DwgObjectWriter::new(&document)
            .expect("object writer")
            .write_with_class_metadata();
        let classes = reconciled_classes(&document, &counts, complete);
        let by_name = |name: &str| {
            classes
                .iter()
                .find(|class| class.dxf_name.eq_ignore_ascii_case(name))
                .expect("class entry")
        };

        assert!(by_name("ACDBASSOCCOMPOUNDACTIONPARAM").was_zombie);
        assert!(by_name("ACDBASSOCOSNAPPOINTREFACTIONPARAM").was_zombie);
        assert!(by_name("ACDBDICTIONARYWDFLT").instance_count > 0);
    }

    #[test]
    fn output_copy_repairs_associative_hatch_reactors() {
        use crate::entities::{BoundaryPath, Circle, EntityType, Hatch};

        let mut document = CadDocument::with_version(DxfVersion::AC1032);
        let boundary = document
            .add_entity(EntityType::Circle(Circle::new()))
            .expect("boundary");
        let mut hatch = Hatch::solid();
        hatch.is_associative = true;
        let mut path = BoundaryPath::new();
        path.boundary_handles.push(boundary);
        hatch.paths.push(path);
        let hatch_handle = document
            .add_entity(EntityType::Hatch(hatch))
            .expect("hatch");

        let mut prepared = std::borrow::Cow::Borrowed(&document);
        prepare_database_references(&mut prepared);

        assert!(document
            .get_entity(boundary)
            .unwrap()
            .common()
            .reactors
            .is_empty());
        assert!(prepared
            .get_entity(boundary)
            .unwrap()
            .common()
            .reactors
            .contains(&hatch_handle));
    }

    #[test]
    fn output_copy_repairs_mleader_style_handle() {
        use crate::entities::{EntityType, MultiLeader};

        // CadDocument seeds the ACAD_MLEADERSTYLE dictionary with the
        // Standard MultiLeaderStyle — the audit-repair stance mirrors it:
        // a native AcDbMLeader with a null style pointer is an invalid
        // authored state (the loaders' audits report "LeaderStyle Id is
        // Null" and repair it at every open).
        let mut document = CadDocument::with_version(DxfVersion::AC1032);
        let mleader_handle = document
            .add_entity(EntityType::MultiLeader(MultiLeader::with_text(
                "Label",
                crate::types::Vector3::new(20.0, 20.0, 0.0),
                vec![
                    crate::types::Vector3::new(0.0, 0.0, 0.0),
                    crate::types::Vector3::new(10.0, 10.0, 0.0),
                ],
            )))
            .expect("mleader");

        let mut prepared = std::borrow::Cow::Borrowed(&document);
        prepare_database_references(&mut prepared);

        let prepared_mleader = match prepared.get_entity(mleader_handle).unwrap() {
            EntityType::MultiLeader(mleader) => mleader,
            other => panic!("unexpected entity {other:?}"),
        };
        let Some(style) = prepared_mleader.style_handle else {
            panic!("the style handle was not repaired");
        };
        assert!(!style.is_null());
        // The caller's document stays unchanged (the output-copy rule).
        match document.get_entity(mleader_handle).unwrap() {
            EntityType::MultiLeader(mleader) => {
                assert!(mleader.style_handle.is_none_or(|handle| handle.is_null()))
            }
            other => panic!("unexpected entity {other:?}"),
        }
    }

    #[test]
    fn output_copy_repairs_mtext_attachment_repeat() {
        use crate::entities::{EntityType, MText};

        // The R2018+ redundant block repeats the ABSOLUTE attachment
        // point; a constructed MText's default 0 is a corrupt
        // repetition (AutoCAD's audit repairs it — "AcDbMText was
        // repaired / 2 fixed" at every open of the canonical). The
        // output-copy repair fills the repeat; the caller's document
        // stays untouched.
        let mut document = CadDocument::with_version(DxfVersion::AC1032);
        let mtext_handle = document
            .add_entity(EntityType::MText(MText::with_value(
                "Note",
                crate::types::Vector3::new(1.0, 1.0, 0.0),
            )))
            .expect("mtext");
        // The corrupt case the repair guards: a struct-literal
        // construction bypassing the honest default (MText::new now
        // mirrors the attachment repeat).
        if let EntityType::MText(mtext) = document.get_entity_mut(mtext_handle).unwrap() {
            mtext.ignore_attachment = 0;
        }
        let attachment = match document.get_entity(mtext_handle).unwrap() {
            EntityType::MText(mtext) => mtext.attachment_point as i32,
            other => panic!("unexpected entity {other:?}"),
        };
        assert_ne!(attachment, 0);

        let mut prepared = std::borrow::Cow::Borrowed(&document);
        prepare_database_references(&mut prepared);

        match prepared.get_entity(mtext_handle).unwrap() {
            EntityType::MText(mtext) => {
                assert_eq!(mtext.ignore_attachment, attachment);
            }
            other => panic!("unexpected entity {other:?}"),
        }
        // The caller's document stays unchanged (the output-copy rule).
        match document.get_entity(mtext_handle).unwrap() {
            EntityType::MText(mtext) => assert_eq!(mtext.ignore_attachment, 0),
            other => panic!("unexpected entity {other:?}"),
        }
    }

    #[test]
    fn output_copy_synchronizes_layout_dictionary_names() {
        use crate::objects::ObjectType;

        let mut document = CadDocument::with_version(DxfVersion::AC1032);
        let dictionary_handle = document.header.acad_layout_dict_handle;
        let layout_handle = match document.objects.get(&dictionary_handle) {
            Some(ObjectType::Dictionary(dictionary)) => dictionary.entries[0].1,
            _ => panic!("layout dictionary"),
        };
        if let Some(ObjectType::Layout(layout)) = document.objects.get_mut(&layout_handle) {
            layout.name = "Renamed".to_string();
        }

        let mut prepared = std::borrow::Cow::Borrowed(&document);
        prepare_database_references(&mut prepared);

        let dictionary = match prepared.objects.get(&dictionary_handle) {
            Some(ObjectType::Dictionary(dictionary)) => dictionary,
            _ => panic!("layout dictionary"),
        };
        assert_eq!(dictionary.get("Renamed"), Some(layout_handle));
    }

    #[test]
    fn same_version_roundtrip_preserves_non_entity_data_store_section() {
        use crate::io::dwg::DwgReader;
        use crate::objects::ObjectType;
        use std::sync::Arc;

        let mut document = CadDocument::with_version(DxfVersion::AC1032);
        let layout_handle = document
            .objects
            .iter()
            .find_map(|(handle, object)| matches!(object, ObjectType::Layout(_)).then_some(*handle))
            .expect("default layout");
        document.dwg_source_version = Some(DxfVersion::AC1032);
        document.dwg_data_store_handles.insert(layout_handle);
        document.raw_acds_data = Some(Arc::new(build_acds_prototype(
            &[],
            DxfVersion::AC1032,
            None,
        )));

        let bytes = DwgWriter::write_to_vec(&document).expect("write drawing");
        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let roundtripped = reader.read().expect("read drawing");

        assert!(roundtripped.raw_acds_data.is_some());
        assert!(roundtripped.dwg_data_store_handles.contains(&layout_handle));
    }

    /// The §20 G-C container genus: a constructed data store must parse
    /// back with the authored-specimen invariants for its era — the
    /// jard header values, the row scale, the slot allocation, the named
    /// pointers, and (beyond the gate's row view) a surviving SAB blob.
    fn assert_constructed_acds_genus(version: DxfVersion, profile: &super::AcDsEraProfile) {
        use crate::entities::acis::primitives::build_cylinder;
        use crate::entities::{EntityType, Solid3D};
        use crate::io::dwg::DwgReader;

        let mut document = CadDocument::with_version(version);
        let sat = build_cylinder([0.0, 0.0, 0.0], 5.0, 10.0).to_sat_string();
        document
            .add_entity(EntityType::Solid3D(Solid3D::from_sat(&sat)))
            .expect("add solid");

        let bytes = DwgWriter::write_to_vec(&document).expect("write drawing");
        let roundtripped = DwgReader::from_stream(std::io::Cursor::new(bytes))
            .read()
            .expect("read drawing");
        let acds = roundtripped.dwg_acds.as_ref().expect("AcDs summary");

        assert_eq!(acds.ds_version, profile.ds_version, "ds_version");
        assert_eq!(acds.segidx_offset, 128, "segidx-first position");
        assert_eq!(acds.file_header_size, 65664, "file_header_size");
        assert_eq!(acds.unknown_1, 8, "unknown_1");
        assert_eq!(acds.version, 2, "container version");
        assert_eq!(acds.segidx.len(), profile.num_segidx, "row scale");
        assert_eq!(acds.segments.len(), profile.num_segidx, "segment slots");
        assert_eq!(acds.schidx_segidx as usize, profile.slot_schidx, "schidx pointer");
        assert_eq!(acds.datidx_segidx as usize, profile.slot_datidx, "datidx pointer");
        assert_eq!(acds.search_segidx as usize, profile.slot_search, "search pointer");
        assert_eq!(acds.prvsav_segidx as usize, profile.slot_prvsav, "prvsav pointer");

        let expected_type = |slot: usize| -> Option<u32> {
            if slot == profile.slot_segidx {
                Some(0)
            } else if slot == profile.slot_datidx {
                Some(1)
            } else if slot == profile.slot_data {
                Some(2)
            } else if slot == profile.slot_schidx {
                Some(3)
            } else if slot == profile.slot_schdat_a || slot == profile.slot_schdat_b {
                Some(4)
            } else if slot == profile.slot_search {
                Some(5)
            } else if slot == profile.slot_prvsav {
                Some(7)
            } else if profile.slot_freesp == Some(slot) {
                Some(8)
            } else {
                None
            }
        };
        for (slot, seg) in acds.segments.iter().enumerate() {
            assert_eq!(seg.type_, expected_type(slot), "slot {slot} kind");
        }

        // The modeler payload must survive the container roundtrip.
        let solid = roundtripped
            .entities()
            .find_map(|entity| match entity {
                EntityType::Solid3D(solid) => Some(solid),
                _ => None,
            })
            .expect("solid survives");
        assert!(
            !solid.acis_data.sab_data.is_empty(),
            "the SAB blob must re-attach from the constructed data store"
        );
    }

    #[test]
    fn constructed_acds_container_matches_the_authored_genus_2018() {
        assert_constructed_acds_genus(DxfVersion::AC1032, &super::ACDS_ERA_2018);
    }

    #[test]
    fn constructed_acds_container_matches_the_authored_genus_2013() {
        assert_constructed_acds_genus(DxfVersion::AC1027, &super::ACDS_ERA_2013);
    }

    #[test]
    fn test_build_template() {
        let t = build_template(&[], 1).unwrap();
        assert_eq!(t.len(), 4);
        assert_eq!(t, [0, 0, 1, 0]);
    }

    #[test]
    fn test_build_template_with_description() {
        let t = build_template(b"metric", 1).unwrap();
        assert_eq!(&t[..2], &6i16.to_le_bytes());
        assert_eq!(&t[2..8], b"metric");
        assert_eq!(&t[8..], &1i16.to_le_bytes());
        assert!(build_template(&[], 2).is_err());
    }

    #[test]
    fn test_build_obj_free_space() {
        let doc = CadDocument::new();
        let data = build_obj_free_space(DxfVersion::AC1015, &doc, 42);
        // Int32(0) + UInt32(42) + 8 date + UInt32(0) + 1 + 32
        assert_eq!(data.len(), 4 + 4 + 8 + 4 + 1 + 32);
        // Check handle count at offset 4
        let count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        assert_eq!(count, 42);
    }

    #[test]
    fn test_build_file_dep_list() {
        let d = build_file_dep_list();
        assert_eq!(d.len(), 8);
    }

    #[test]
    fn test_build_rev_history() {
        let d = build_rev_history();
        assert_eq!(d.len(), 16);
    }

    #[test]
    fn test_build_summary_info_ac18() {
        let d = build_summary_info(DxfVersion::AC1018, &crate::document::SummaryInfo::default());
        // 8 × 3 bytes (u16(1) + ANSI null) + 8 + 16 + 2 + 8 = 58
        assert_eq!(d.len(), 58);
        assert_eq!(u16::from_le_bytes([d[0], d[1]]), 1);
        assert_eq!(d[2], 0);
        // Next string starts at offset 3
        assert_eq!(u16::from_le_bytes([d[3], d[4]]), 1);
    }

    #[test]
    fn test_build_summary_info_ac21() {
        let d = build_summary_info(DxfVersion::AC1021, &crate::document::SummaryInfo::default());
        // 8 × 4 bytes (u16(1) + UTF-16LE null) + 8 + 16 + 2 + 8 = 66
        assert_eq!(d.len(), 66);
        // First string: u16(1) + 00 00
        assert_eq!(u16::from_le_bytes([d[0], d[1]]), 1);
        assert_eq!(d[2], 0);
        assert_eq!(d[3], 0);
        // Next string starts at offset 4
        assert_eq!(u16::from_le_bytes([d[4], d[5]]), 1);
    }

    #[test]
    fn test_write_to_vec_r2000() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1015;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // AC15 file header magic: "AC1015"
        assert!(bytes.len() > 100);
        let magic = std::str::from_utf8(&bytes[0..6]).unwrap_or("");
        assert_eq!(magic, "AC1015");
    }

    #[test]
    fn test_write_to_vec_r2004() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1018;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.len() > 0x100);
        // AC18 magic at offset 0
        let magic = std::str::from_utf8(&bytes[0..6]).unwrap_or("");
        assert_eq!(magic, "AC1018");
    }

    #[test]
    fn test_write_to_vec_r2010() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1024;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(bytes.len() > 0x100);
    }

    #[test]
    fn test_write_to_vec_r2013() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1027;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_to_vec_r2018() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1032;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_to_vec_r2007() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1021;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok(), "AC1021 writing should succeed");
        let bytes = result.unwrap();
        assert!(
            bytes.len() > 0x480,
            "AC1021 file should be larger than header"
        );
        let magic = std::str::from_utf8(&bytes[0..6]).unwrap_or("");
        assert_eq!(magic, "AC1021");
    }

    #[test]
    fn test_uses_ac21_format() {
        assert!(uses_ac21_format(DxfVersion::AC1021));
        assert!(!uses_ac21_format(DxfVersion::AC1018));
        assert!(!uses_ac21_format(DxfVersion::AC1024));
        assert!(!uses_ac21_format(DxfVersion::AC1015));
    }

    #[test]
    fn test_write_to_vec_r14() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1014;
        let result = DwgWriter::write_to_vec(&doc);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        let magic = std::str::from_utf8(&bytes[0..6]).unwrap_or("");
        assert_eq!(magic, "AC1014");
    }

    #[test]
    fn test_roundtrip_file_write() {
        let doc = CadDocument::new();
        let mut doc2 = doc.clone();
        doc2.version = DxfVersion::AC1015;

        let bytes = DwgWriter::write_to_vec(&doc2).unwrap();
        // Verify non-trivial output
        assert!(bytes.len() > 200, "DWG file should be non-trivial");
    }

    #[test]
    fn test_pre_r2013_write_uses_legacy_document_profile() {
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1015;

        let bytes = DwgWriter::write_to_vec(&doc).unwrap();
        let mut reader = crate::io::dwg::DwgReader::from_stream(std::io::Cursor::new(bytes));
        let decoded = reader.read().unwrap();

        // Mirrors the compact profile in validated R2000 fixtures.
        let mut expected = doc.classes.clone();
        expected.retain_legacy_dwg_classes();
        assert_eq!(decoded.classes.len(), expected.len());
        assert_eq!(decoded.objects.len(), 13);
    }

    #[test]
    fn test_prepare_header_syncs_null_handles() {
        // Simulate the bug: create a document and zero out all header handles
        // (as would happen after reading a DWG with a broken header reader).
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1015;

        // Save correct handles for later verification
        let correct_block_control = doc.block_records.handle();
        let correct_layer_control = doc.layers.handle();
        let correct_style_control = doc.text_styles.handle();
        let correct_ltype_control = doc.line_types.handle();
        let correct_view_control = doc.views.handle();
        let correct_ucs_control = doc.ucss.handle();
        let correct_vport_control = doc.vports.handle();
        let correct_appid_control = doc.app_ids.handle();
        let correct_dimstyle_control = doc.dim_styles.handle();
        let correct_root_dict = doc.header.named_objects_dict_handle;

        // Zero out all header handles (simulate the reader bug)
        doc.header.block_control_handle = Handle::NULL;
        doc.header.layer_control_handle = Handle::NULL;
        doc.header.style_control_handle = Handle::NULL;
        doc.header.linetype_control_handle = Handle::NULL;
        doc.header.view_control_handle = Handle::NULL;
        doc.header.ucs_control_handle = Handle::NULL;
        doc.header.vport_control_handle = Handle::NULL;
        doc.header.appid_control_handle = Handle::NULL;
        doc.header.dimstyle_control_handle = Handle::NULL;
        doc.header.named_objects_dict_handle = Handle::NULL;
        doc.header.acad_group_dict_handle = Handle::NULL;
        doc.header.acad_mlinestyle_dict_handle = Handle::NULL;
        doc.header.acad_layout_dict_handle = Handle::NULL;
        doc.header.bylayer_linetype_handle = Handle::NULL;
        doc.header.byblock_linetype_handle = Handle::NULL;
        doc.header.continuous_linetype_handle = Handle::NULL;
        doc.header.current_layer_handle = Handle::NULL;
        doc.header.current_text_style_handle = Handle::NULL;
        doc.header.current_dimstyle_handle = Handle::NULL;
        doc.header.current_linetype_handle = Handle::NULL;

        // prepare_header should sync all handles from document objects
        let handle_map = vec![(1u64, 0u32), (2, 100), (3, 200)]; // dummy
        let prepared = prepare_header(&doc, &handle_map, &None);

        // Table control handles must be synced from the actual table objects
        assert_eq!(
            prepared.block_control_handle, correct_block_control,
            "block_control_handle should be synced from block_records.handle()"
        );
        assert_eq!(
            prepared.layer_control_handle, correct_layer_control,
            "layer_control_handle should be synced from layers.handle()"
        );
        assert_eq!(
            prepared.style_control_handle, correct_style_control,
            "style_control_handle should be synced from text_styles.handle()"
        );
        assert_eq!(
            prepared.linetype_control_handle, correct_ltype_control,
            "linetype_control_handle should be synced from line_types.handle()"
        );
        assert_eq!(
            prepared.view_control_handle, correct_view_control,
            "view_control_handle should be synced from views.handle()"
        );
        assert_eq!(
            prepared.ucs_control_handle, correct_ucs_control,
            "ucs_control_handle should be synced from ucss.handle()"
        );
        assert_eq!(
            prepared.vport_control_handle, correct_vport_control,
            "vport_control_handle should be synced from vports.handle()"
        );
        assert_eq!(
            prepared.appid_control_handle, correct_appid_control,
            "appid_control_handle should be synced from app_ids.handle()"
        );
        assert_eq!(
            prepared.dimstyle_control_handle, correct_dimstyle_control,
            "dimstyle_control_handle should be synced from dim_styles.handle()"
        );

        // Root dictionary must be found
        assert_eq!(
            prepared.named_objects_dict_handle, correct_root_dict,
            "named_objects_dict_handle should be found by scanning objects"
        );
        assert!(
            !prepared.named_objects_dict_handle.is_null(),
            "named_objects_dict_handle must not be NULL"
        );

        // Dict handles from root dict entries must be resolved
        assert!(
            !prepared.acad_group_dict_handle.is_null(),
            "acad_group_dict_handle must be resolved from root dict"
        );
        assert!(
            !prepared.acad_mlinestyle_dict_handle.is_null(),
            "acad_mlinestyle_dict_handle must be resolved from root dict"
        );
        assert!(
            !prepared.acad_layout_dict_handle.is_null(),
            "acad_layout_dict_handle must be resolved from root dict"
        );

        // Linetype handles must be resolved
        assert!(
            !prepared.bylayer_linetype_handle.is_null(),
            "bylayer_linetype_handle must be resolved"
        );
        assert!(
            !prepared.byblock_linetype_handle.is_null(),
            "byblock_linetype_handle must be resolved"
        );
        assert!(
            !prepared.continuous_linetype_handle.is_null(),
            "continuous_linetype_handle must be resolved"
        );

        // Current style handles must be resolved
        assert!(
            !prepared.current_layer_handle.is_null(),
            "current_layer_handle must be resolved"
        );
        assert!(
            !prepared.current_text_style_handle.is_null(),
            "current_text_style_handle must be resolved"
        );
        assert!(
            !prepared.current_dimstyle_handle.is_null(),
            "current_dimstyle_handle must be resolved"
        );
        assert!(
            !prepared.current_linetype_handle.is_null(),
            "current_linetype_handle must be resolved (default to ByLayer)"
        );
    }

    #[test]
    fn test_prepare_header_prefers_dictionary_standard_mlinestyle() {
        let mut doc = CadDocument::new();
        let dictionary_standard = doc.header.current_multiline_style_handle;
        let orphan_handle = (1..dictionary_standard.value())
            .map(Handle::new)
            .find(|handle| !doc.objects.contains_key(handle))
            .expect("an unused lower handle");
        let mut orphan = crate::objects::MLineStyle::standard();
        orphan.handle = orphan_handle;
        doc.objects.insert(
            orphan_handle,
            crate::objects::ObjectType::MLineStyle(orphan),
        );
        doc.header.current_multiline_style_handle = Handle::NULL;

        let prepared = prepare_header(&doc, &[], &None);

        assert_eq!(
            prepared.current_multiline_style_handle, dictionary_standard,
            "the ACAD_MLINESTYLE dictionary entry should be authoritative"
        );
    }

    #[test]
    fn test_prepare_header_mlinestyle_recovery_is_deterministic() {
        let mut doc = CadDocument::new();
        let dictionary_handle = doc.header.acad_mlinestyle_dict_handle;
        let original_standard = doc.header.current_multiline_style_handle;
        let recovery_handle = (1..original_standard.value())
            .map(Handle::new)
            .find(|handle| !doc.objects.contains_key(handle))
            .expect("an unused lower handle");
        let mut recovery = crate::objects::MLineStyle::standard();
        recovery.handle = recovery_handle;
        doc.objects.insert(
            recovery_handle,
            crate::objects::ObjectType::MLineStyle(recovery),
        );
        let crate::objects::ObjectType::Dictionary(dictionary) = doc
            .objects
            .get_mut(&dictionary_handle)
            .expect("ACAD_MLINESTYLE dictionary")
        else {
            panic!("ACAD_MLINESTYLE handle should reference a dictionary");
        };
        dictionary
            .entries
            .retain(|(name, _)| !name.eq_ignore_ascii_case("Standard"));
        doc.header.current_multiline_style_handle = Handle::NULL;

        let prepared = prepare_header(&doc, &[], &None);

        assert_eq!(prepared.current_multiline_style_handle, recovery_handle);
    }

    #[test]
    fn test_prepare_header_null_handles_write_produces_valid_dwg() {
        // Simulate bug and verify the written DWG file is still valid
        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1015;

        // Zero out all header handles
        doc.header.block_control_handle = Handle::NULL;
        doc.header.layer_control_handle = Handle::NULL;
        doc.header.named_objects_dict_handle = Handle::NULL;

        // Writing should succeed (prepare_header syncs handles)
        let result = DwgWriter::write_to_vec(&doc);
        assert!(
            result.is_ok(),
            "Writing with NULL headers should succeed after sync"
        );
        let bytes = result.unwrap();
        assert!(bytes.len() > 200, "Output should be non-trivial");
    }

    // ── File-level DWG roundtrip tests for 3DSOLID / REGION / BODY ──

    fn make_sat_sample() -> &'static str {
        include_str!("../../../examples/entity_atlas_assets/region.sat")
    }

    /// Write a Solid3D with SAT data to DWG R2000, read back, verify SAT preserved.
    #[test]
    fn test_roundtrip_solid3d_r2000() {
        use crate::entities::solid3d::Solid3D;
        use crate::entities::EntityType;
        use crate::io::dwg::DwgReader;

        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1015;
        let solid = Solid3D::from_sat(make_sat_sample());
        let _ = doc.add_entity(EntityType::Solid3D(solid));

        let bytes = DwgWriter::write_to_vec(&doc).expect("write R2000 should succeed");
        assert!(bytes.len() > 200);

        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let doc2 = reader.read().expect("read R2000 should succeed");

        let solids: Vec<&Solid3D> = doc2
            .entities()
            .filter_map(|e| {
                if let EntityType::Solid3D(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(solids.len(), 1, "should have exactly one Solid3D");
        assert!(!solids[0].acis_data.is_binary, "R2000 should use SAT text");
        assert!(
            solids[0].acis_data.sat_data.contains("body"),
            "SAT data must contain 'body'"
        );
        assert!(
            solids[0].acis_data.sat_data.contains("plane-surface"),
            "SAT data must contain 'plane-surface'"
        );
    }

    /// Write a Solid3D with SAT data to DWG R2004, read back, verify SAT preserved.
    #[test]
    fn test_roundtrip_solid3d_r2004() {
        use crate::entities::solid3d::Solid3D;
        use crate::entities::EntityType;
        use crate::io::dwg::DwgReader;

        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1018;
        let solid = Solid3D::from_sat(make_sat_sample());
        let _ = doc.add_entity(EntityType::Solid3D(solid));

        let bytes = DwgWriter::write_to_vec(&doc).expect("write R2004 should succeed");
        assert!(bytes.len() > 200);

        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let doc2 = reader.read().expect("read R2004 should succeed");

        let solids: Vec<&Solid3D> = doc2
            .entities()
            .filter_map(|e| {
                if let EntityType::Solid3D(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(solids.len(), 1, "should have exactly one Solid3D");
        assert!(solids[0].acis_data.is_binary, "R2004 stores version-2 SAB");
        assert_eq!(solids[0].acis_data.parse().unwrap().bodies().len(), 1);
    }

    /// Write a Solid3D with SAT data to DWG R2007 (SAB binary format), read back.
    #[test]
    fn test_roundtrip_solid3d_r2007() {
        use crate::entities::solid3d::Solid3D;
        use crate::entities::EntityType;
        use crate::io::dwg::DwgReader;

        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1021;
        let solid = Solid3D::from_sat(make_sat_sample());
        let _ = doc.add_entity(EntityType::Solid3D(solid));

        let bytes = DwgWriter::write_to_vec(&doc).expect("write R2007 should succeed");
        assert!(bytes.len() > 200);

        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let doc2 = reader.read().expect("read R2007 should succeed");

        let solids: Vec<&Solid3D> = doc2
            .entities()
            .filter_map(|e| {
                if let EntityType::Solid3D(s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(solids.len(), 1, "should have exactly one Solid3D");
        // R2007 should use SAT text format since we provided SAT text —
        // the version in acis_data controls what's written, not the DWG version alone.
        assert!(solids[0].acis_data.has_data(), "should have ACIS data");
    }

    /// Write a Region with SAT data to DWG R2000, read back.
    #[test]
    fn test_roundtrip_region_r2000() {
        use crate::entities::solid3d::Region;
        use crate::entities::EntityType;
        use crate::io::dwg::DwgReader;

        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1015;
        let region = Region::from_sat(make_sat_sample());
        let _ = doc.add_entity(EntityType::Region(region));

        let bytes = DwgWriter::write_to_vec(&doc).expect("write R2000 should succeed");

        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let doc2 = reader.read().expect("read R2000 should succeed");

        let regions: Vec<&Region> = doc2
            .entities()
            .filter_map(|e| {
                if let EntityType::Region(r) = e {
                    Some(r)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(regions.len(), 1, "should have exactly one Region");
        assert!(regions[0].acis_data.sat_data.contains("body"));
    }

    /// Write a Body with SAT data to DWG R2004, read back.
    #[test]
    fn test_roundtrip_body_r2004() {
        use crate::entities::solid3d::Body;
        use crate::entities::EntityType;
        use crate::io::dwg::DwgReader;

        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1018;
        let body = Body::from_sat(make_sat_sample());
        let _ = doc.add_entity(EntityType::Body(body));

        let bytes = DwgWriter::write_to_vec(&doc).expect("write R2004 should succeed");

        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let doc2 = reader.read().expect("read R2004 should succeed");

        let bodies: Vec<&Body> = doc2
            .entities()
            .filter_map(|e| {
                if let EntityType::Body(b) = e {
                    Some(b)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(bodies.len(), 1, "should have exactly one Body");
        assert_eq!(bodies[0].acis_data.parse().unwrap().bodies().len(), 1);
    }

    /// Write multiple ACIS entities to a single DWG at R2010, read back all three.
    #[test]
    fn test_roundtrip_mixed_acis_r2010() {
        use crate::entities::solid3d::{Body, Region, Solid3D};
        use crate::entities::EntityType;
        use crate::io::dwg::DwgReader;

        let mut doc = CadDocument::new();
        doc.version = DxfVersion::AC1024;

        let _ = doc.add_entity(EntityType::Solid3D(Solid3D::from_sat(make_sat_sample())));
        let _ = doc.add_entity(EntityType::Region(Region::from_sat(make_sat_sample())));
        let _ = doc.add_entity(EntityType::Body(Body::from_sat(make_sat_sample())));

        let bytes = DwgWriter::write_to_vec(&doc).expect("write R2010 should succeed");

        let mut reader = DwgReader::from_stream(std::io::Cursor::new(bytes));
        let doc2 = reader.read().expect("read R2010 should succeed");

        let n_solid = doc2
            .entities()
            .filter(|e| matches!(e, EntityType::Solid3D(_)))
            .count();
        let n_region = doc2
            .entities()
            .filter(|e| matches!(e, EntityType::Region(_)))
            .count();
        let n_body = doc2
            .entities()
            .filter(|e| matches!(e, EntityType::Body(_)))
            .count();
        assert_eq!(n_solid, 1, "should have 1 Solid3D");
        assert_eq!(n_region, 1, "should have 1 Region");
        assert_eq!(n_body, 1, "should have 1 Body");

        // Verify data integrity on each
        for e in doc2.entities() {
            match e {
                EntityType::Solid3D(s) => {
                    assert_eq!(s.acis_data.parse().unwrap().bodies().len(), 1)
                }
                EntityType::Region(r) => assert_eq!(r.acis_data.parse().unwrap().bodies().len(), 1),
                EntityType::Body(b) => assert_eq!(b.acis_data.parse().unwrap().bodies().len(), 1),
                _ => {}
            }
        }
    }
}
