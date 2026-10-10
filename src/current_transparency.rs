use crate::objects::{Dictionary, DictionaryVariable, ObjectType};
use crate::types::{Handle, Transparency};
use crate::CadDocument;

const VARIABLES: &str = "AcDbVariableDictionary";
const CURRENT: &str = "CETRANSPARENCY";

impl CadDocument {
    /// Current entity transparency, stored in the drawing variable dictionary.
    /// Missing or malformed values retain the inherited default.
    pub fn current_entity_transparency(&self) -> Transparency {
        let read = || {
            let ObjectType::Dictionary(root) =
                self.objects.get(&self.header.named_objects_dict_handle)?
            else {
                return None;
            };
            let ObjectType::Dictionary(variables) = self.objects.get(&root.get(VARIABLES)?)? else {
                return None;
            };
            let ObjectType::DictionaryVariable(value) =
                self.objects.get(&variables.get(CURRENT)?)?
            else {
                return None;
            };
            Some(Transparency::from_alpha_value(
                value.value.parse::<u32>().ok()?,
            ))
        };
        read().unwrap_or(Transparency::ByLayer)
    }

    /// Store current entity transparency using the packed dictionary value.
    /// Existing entries are updated in place; conflicting object types are preserved.
    pub fn set_current_entity_transparency(&mut self, value: Transparency) -> bool {
        if matches!(value, Transparency::Explicit(amount) if amount > 230) {
            return false;
        }
        let mut root_handle = self.header.named_objects_dict_handle;
        if root_handle == Handle::NULL {
            root_handle = self.allocate_handle();
            let mut root = Dictionary::new();
            root.handle = root_handle;
            self.objects
                .insert(root_handle, ObjectType::Dictionary(root));
            self.header.named_objects_dict_handle = root_handle;
        }
        let Some(ObjectType::Dictionary(root)) = self.objects.get(&root_handle) else {
            return false;
        };
        let variables_handle = root.get(VARIABLES);
        let variables_handle = if let Some(handle) = variables_handle {
            if !matches!(self.objects.get(&handle), Some(ObjectType::Dictionary(_))) {
                return false;
            }
            handle
        } else {
            let handle = self.allocate_handle();
            let mut variables = Dictionary::new();
            variables.handle = handle;
            variables.owner = root_handle;
            self.objects
                .insert(handle, ObjectType::Dictionary(variables));
            if let Some(ObjectType::Dictionary(root)) = self.objects.get_mut(&root_handle) {
                root.add_entry(VARIABLES, handle);
            }
            handle
        };
        let Some(ObjectType::Dictionary(variables)) = self.objects.get(&variables_handle) else {
            return false;
        };
        let packed = value.to_dxf_value().to_string();
        if let Some(handle) = variables.get(CURRENT) {
            let Some(ObjectType::DictionaryVariable(variable)) = self.objects.get_mut(&handle)
            else {
                return false;
            };
            variable.value = packed;
        } else {
            let handle = self.allocate_handle();
            let mut variable = DictionaryVariable::new(CURRENT, &packed);
            variable.handle = handle;
            variable.owner_handle = variables_handle;
            self.objects
                .insert(handle, ObjectType::DictionaryVariable(variable));
            if let Some(ObjectType::Dictionary(variables)) = self.objects.get_mut(&variables_handle)
            {
                variables.add_entry(CURRENT, handle);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_transparency_roundtrips_and_rejects_unsupported_values() {
        let mut document = CadDocument::new();
        assert_eq!(
            document.current_entity_transparency(),
            Transparency::ByLayer
        );
        assert!(document.set_current_entity_transparency(Transparency::T_30));
        assert_eq!(document.current_entity_transparency(), Transparency::T_30);
        let object_count = document.objects.len();
        assert!(!document.set_current_entity_transparency(Transparency::Explicit(231)));
        assert_eq!(document.current_entity_transparency(), Transparency::T_30);
        assert_eq!(document.objects.len(), object_count);
    }
}
