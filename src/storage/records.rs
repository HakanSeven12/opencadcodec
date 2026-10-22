use crate::xdata::XDataValue;
use crate::{
    classes::DxfClass,
    document::{FieldDef, HeaderVariables, Preview, SummaryInfo},
    entities::{EntityType, SectionViewStyle},
    objects::{BlockVisibilityParameter, DgnLsComponent, DgnLsDefinition, ObjectType},
    tables::*,
    types::{DxfVersion, Handle},
    xdata::ExtendedData,
};
use serde::{Deserialize, Serialize};

/// Decoded values whose application or layer context cannot safely be invented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapturedXDataValue {
    Value(XDataValue),
    UnresolvedLayer(Handle),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedExtendedData {
    pub location: String,
    pub source_handle: Handle,
    pub application_handle: Handle,
    #[serde(deserialize_with = "required_option")]
    pub application_name: Option<String>,
    pub values: Vec<CapturedXDataValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TableRecords<T> {
    pub control_handle: Handle,
    pub entries: Vec<T>,
}

macro_rules! tables {
    ($($field:ident: $ty:ty),* $(,)?) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub(crate) struct RecordTables { $(pub $field: TableRecords<$ty>,)* }
        impl RecordViews<'_> { $(pub fn $field(&self) -> &[$ty] { &self.body.tables.$field.entries })* }
    }
}
tables! { layers: Layer, line_types: LineType, text_styles: TextStyle, block_records: BlockRecord,
dim_styles: DimStyle, app_ids: AppId, views: View, vports: VPort, ucss: Ucs, vx_table: VxTableRecord }

/// Captured typed document context. Source paths are data, never opened by storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageDocumentContext {
    pub version: DxfVersion,
    pub maintenance_version: u8,
    pub header: HeaderVariables,
    pub allocation_floor: u64,
    pub vx_control_entries: Vec<Handle>,
    pub block_visibility_params: Vec<(Handle, BlockVisibilityParameter)>,
    pub context_scales: Vec<(Handle, Handle)>,
    pub block_representations: Vec<(Handle, Handle)>,
    pub fields: Vec<(Handle, FieldDef)>,
    pub summary_info: SummaryInfo,
    #[serde(deserialize_with = "required_option")]
    pub source_path: Option<String>,
    pub dgn_ls_definitions: Vec<(Handle, DgnLsDefinition)>,
    pub dgn_ls_components: Vec<(Handle, DgnLsComponent)>,
    pub object_xdata: Vec<(Handle, ExtendedData)>,
    pub xdictionary_links: Vec<(Handle, Handle)>,
    pub reactor_links: Vec<(Handle, Vec<Handle>)>,
    #[serde(deserialize_with = "required_option")]
    pub dwg_source_version: Option<DxfVersion>,
    #[serde(deserialize_with = "required_option")]
    pub preview: Option<Preview>,
    #[serde(deserialize_with = "required_option")]
    pub section_view_style: Option<SectionViewStyle>,
    pub view_rep_refs: Vec<(Handle, Vec<Handle>)>,
    pub section_view_reps: Vec<Handle>,
}

pub(crate) fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    de: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(de)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordBody {
    #[serde(with = "super::wire::entity_records")]
    pub entities: Vec<EntityType>,
    #[serde(with = "super::wire::object_records")]
    pub objects: Vec<(Handle, ObjectType)>,
    pub classes: Vec<DxfClass>,
    pub tables: RecordTables,
    pub context: StorageDocumentContext,
    pub extended_data_evidence: Vec<CapturedExtendedData>,
}

impl RecordBody {
    pub(crate) fn validate_record_keys(&self) -> Result<(), super::StorageError> {
        use super::{StorageError, StorageErrorKind};
        use std::collections::HashSet;
        let mut object_keys = HashSet::new();
        let mut identities = HashSet::new();
        let mut register = |handle: Handle, location: &str| {
            if !handle.is_null() && !identities.insert(handle) {
                Err(StorageError::new(
                    StorageErrorKind::DuplicateIdentity,
                    location,
                    format!("duplicate source handle {handle:?}"),
                ))
            } else {
                Ok(())
            }
        };
        for (index, entity) in self.entities.iter().enumerate() {
            register(entity.common().handle, &format!("entities/{index}"))?;
        }
        for (index, (key, object)) in self.objects.iter().enumerate() {
            let location = format!("objects/{index}");
            if !object_keys.insert(key) || *key != object.intrinsic_handle() {
                return Err(StorageError::new(
                    StorageErrorKind::DuplicateIdentity,
                    location,
                    "duplicate object key or key/intrinsic identity mismatch",
                ));
            }
            register(*key, &location)?;
        }
        macro_rules! table_keys { ($($name:ident),*)=> { $(
            register(self.tables.$name.control_handle,concat!("tables/",stringify!($name),"/control"))?;
            for (index,entry) in self.tables.$name.entries.iter().enumerate() {
                register(entry.handle(),&format!("tables/{}/{index}",stringify!($name)))?;
            }
        )* } }
        table_keys!(
            layers,
            line_types,
            text_styles,
            block_records,
            dim_styles,
            app_ids,
            views,
            vports,
            ucss,
            vx_table
        );
        // Each list represents a source map. Keys are unique within that view;
        // overlap with a backing record is intentional and is not repaired here.
        macro_rules! context_keys { ($($name:ident),*)=> { $(
            let mut keys=HashSet::new();
            for (index,(key,_)) in self.context.$name.iter().enumerate() {
                if !keys.insert(*key) {
                    return Err(StorageError::new(StorageErrorKind::DuplicateIdentity,
                        format!("context/{}/{index}",stringify!($name)),"duplicate context-map key"));
                }
            }
        )* } }
        context_keys!(
            block_visibility_params,
            context_scales,
            block_representations,
            fields,
            dgn_ls_definitions,
            dgn_ls_components,
            object_xdata,
            xdictionary_links,
            reactor_links,
            view_rep_refs
        );
        Ok(())
    }
}

/// Read-only views of retained typed values, with excluded storage absent.
#[derive(Clone, Copy)]
pub struct RecordViews<'a> {
    pub(crate) body: &'a RecordBody,
}
impl RecordViews<'_> {
    pub fn entities(&self) -> &[EntityType] {
        &self.body.entities
    }
    pub fn objects(&self) -> &[(Handle, ObjectType)] {
        &self.body.objects
    }
    pub fn classes(&self) -> &[DxfClass] {
        &self.body.classes
    }
    pub fn document_context(&self) -> &StorageDocumentContext {
        &self.body.context
    }
    pub fn extended_data_evidence(&self) -> &[CapturedExtendedData] {
        &self.body.extended_data_evidence
    }
}
