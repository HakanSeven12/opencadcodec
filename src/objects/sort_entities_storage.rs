use super::*;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredSortEntry {
    entity_handle: Handle,
    sort_value: u64,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredSortEntitiesTable {
    handle: Handle,
    owner_handle: Handle,
    block_owner_handle: Handle,
    entries: Vec<StoredSortEntry>,
}
impl From<&SortEntitiesTable> for StoredSortEntitiesTable {
    fn from(value: &SortEntitiesTable) -> Self {
        Self {
            handle: value.handle,
            owner_handle: value.owner_handle,
            block_owner_handle: value.block_owner_handle,
            entries: value
                .entries
                .iter()
                .map(|e| StoredSortEntry {
                    entity_handle: e.entity_handle,
                    sort_value: e.sort_handle.value(),
                })
                .collect(),
        }
    }
}
impl From<StoredSortEntitiesTable> for SortEntitiesTable {
    fn from(value: StoredSortEntitiesTable) -> Self {
        let entries: Vec<_> = value
            .entries
            .into_iter()
            .map(|e| SortEntsEntry::new(e.entity_handle, Handle::new(e.sort_value)))
            .collect();
        let entry_map = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.entity_handle.value(), i))
            .collect();
        Self {
            handle: value.handle,
            owner_handle: value.owner_handle,
            block_owner_handle: value.block_owner_handle,
            entries,
            entry_map,
            raw_dxf_codes: None,
            raw_dxf_version: None,
        }
    }
}
