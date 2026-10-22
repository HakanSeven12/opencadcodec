//! Explicit root projection; no reader repair or document initialization.
use super::*;
use crate::storage::records::{RecordBody, RecordTables, StorageDocumentContext, TableRecords};

fn pairs<T: Clone>(map: &HashMap<Handle, T>) -> Vec<(Handle, T)> {
    let mut values: Vec<_> = map
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect();
    values.sort_by_key(|(key, _)| *key);
    values
}

fn table<T: TableEntry + Clone>(source: &Table<T>) -> TableRecords<T> {
    TableRecords {
        control_handle: source.handle(),
        entries: source.iter().cloned().collect(),
    }
}

impl CadDocument {
    pub(crate) fn storage_records(&self) -> RecordBody {
        let Self {
            version,
            maintenance_version,
            header,
            layers,
            line_types,
            text_styles,
            block_records,
            dim_styles,
            app_ids,
            views,
            vports,
            ucss,
            vx_table,
            vx_control_entries,
            classes,
            notifications: _,
            entities,
            entity_index: _,
            objects,
            block_visibility_params,
            context_scales,
            block_representations,
            fields,
            summary_info,
            source_path,
            dgn_ls_definitions,
            dgn_ls_components,
            eed_by_handle: _,
            object_xdata,
            xdic_by_handle,
            reactors_by_handle,
            block_entity_handles: _,
            dwg_source_version,
            preview,
            acis_sab_handles: _,
            raw_acds_data: _,
            r12_header_raw: _,
            raw_records: _,
            original_objects: _,
            raw_record_owners: _,
            raw_acds_fingerprint: _,
            dwg_data_store_handles: _,
            section_view_style,
            view_rep_refs,
            section_view_reps,
            next_handle,
        } = self;
        RecordBody {
            extended_data_evidence: Vec::new(),
            entities: entities
                .iter()
                .map(|value| value.as_ref().clone())
                .collect(),
            objects: pairs(objects),
            classes: classes.iter().cloned().collect(),
            tables: RecordTables {
                layers: table(layers),
                line_types: table(line_types),
                text_styles: table(text_styles),
                block_records: table(block_records),
                dim_styles: table(dim_styles),
                app_ids: table(app_ids),
                views: table(views),
                vports: table(vports),
                ucss: table(ucss),
                vx_table: table(vx_table),
            },
            context: StorageDocumentContext {
                version: *version,
                maintenance_version: *maintenance_version,
                header: header.clone(),
                allocation_floor: (*next_handle).max(header.handle_seed),
                vx_control_entries: vx_control_entries.clone(),
                block_visibility_params: pairs(block_visibility_params),
                context_scales: pairs(context_scales),
                block_representations: pairs(block_representations),
                fields: pairs(fields),
                summary_info: summary_info.clone(),
                source_path: source_path.clone(),
                dgn_ls_definitions: pairs(dgn_ls_definitions),
                dgn_ls_components: pairs(dgn_ls_components),
                object_xdata: pairs(object_xdata),
                xdictionary_links: pairs(xdic_by_handle),
                reactor_links: pairs(reactors_by_handle),
                dwg_source_version: *dwg_source_version,
                preview: preview.clone(),
                section_view_style: section_view_style.clone(),
                view_rep_refs: pairs(view_rep_refs),
                section_view_reps: section_view_reps.clone(),
            },
        }
    }

    pub(crate) fn storage_source_eed(&self) -> &HashMap<Handle, Vec<(u64, Vec<u8>)>> {
        &self.eed_by_handle
    }

    pub(crate) fn storage_root_exclusions(&self) -> Vec<crate::storage::CaptureEvidence> {
        use crate::storage::{CaptureEvidence, ContentCoverage};
        let mut entries = Vec::new();
        for (field, present) in [
            ("raw_acds_data", self.raw_acds_data.is_some()),
            (
                "raw_acds_fingerprint",
                !self.raw_acds_fingerprint.is_empty(),
            ),
            (
                "dwg_data_store_handles",
                !self.dwg_data_store_handles.is_empty(),
            ),
            ("r12_header_raw", self.r12_header_raw.is_some()),
            ("raw_records", !self.raw_records.is_empty()),
            ("original_objects", !self.original_objects.is_empty()),
            ("raw_record_owners", !self.raw_record_owners.is_empty()),
        ] {
            if present {
                entries.push(CaptureEvidence { location:format!("document/{field}"),
                coverage:ContentCoverage::PartialTyped,code:"storage.source_storage_excluded".into(),
                message:"Original document storage is excluded; available typed records are captured separately.".into() });
            }
        }
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{CapturedXDataValue, StorageNamespace, TypedRecordContainer};
    #[test]
    fn root_projection_excludes_private_source_storage_and_keeps_decoded_global_eed() {
        let mut source = CadDocument::new();
        source.dwg_source_version = Some(DxfVersion::AC1032);
        source.raw_acds_data = Some(Arc::new(b"EXCLUDED_PRIVATE_ACDS_BYTES".to_vec()));
        source.r12_header_raw = Some(b"EXCLUDED_PRIVATE_R12_HEADER".to_vec());
        source.raw_records.insert(
            999,
            (
                1,
                Arc::new(crate::entities::RawRecord {
                    data: b"EXCLUDED_ORIGINAL_RECORD_BYTES".to_vec(),
                    handle_bits: 8,
                    version: DxfVersion::AC1032,
                }),
            ),
        );
        source.raw_acds_fingerprint.push((999, 123, 456));
        source.dwg_data_store_handles.insert(Handle::new(999));
        source.preview = Some(Preview {
            format: PreviewFormat::Png,
            data: b"ALLOWED_PREVIEW_BYTES".to_vec(),
        });
        let layer = source.layers.iter().next().unwrap().handle;
        let app = source.app_ids.iter().next().unwrap().handle;
        source
            .eed_by_handle
            .insert(layer, vec![(app.value(), vec![70, 19, 0])]);
        let captured = TypedRecordContainer::capture_document(
            &source,
            StorageNamespace::new("root-test".into()).unwrap(),
        )
        .unwrap();
        let bytes = captured.container.to_bytes().unwrap();
        for sentinel in [
            b"EXCLUDED_PRIVATE_ACDS_BYTES".as_slice(),
            b"EXCLUDED_PRIVATE_R12_HEADER",
            b"EXCLUDED_ORIGINAL_RECORD_BYTES",
        ] {
            assert!(!bytes.windows(sentinel.len()).any(|value| value == sentinel));
        }
        let decoded = TypedRecordContainer::from_bytes(&bytes).unwrap();
        let views = decoded.records();
        assert_eq!(
            views.document_context().preview.as_ref().unwrap().data,
            b"ALLOWED_PREVIEW_BYTES"
        );
        assert_eq!(views.extended_data_evidence().len(), 1);
        assert_eq!(views.extended_data_evidence()[0].source_handle, layer);
        assert_eq!(
            views.extended_data_evidence()[0].values,
            vec![CapturedXDataValue::Value(XDataValue::Integer16(19))]
        );
        assert!(decoded
            .capture_report()
            .entries
            .iter()
            .any(|e| e.location == "document/raw_acds_data"));
        assert!(source.raw_acds_data.is_some());
        assert_eq!(source.eed_by_handle[&layer].len(), 1);
    }
}
