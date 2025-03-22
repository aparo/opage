use super::CodegenModel;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
/// Represents the OpenAPI discriminator construct.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CodegenDiscriminator {
    /// The name of the property in the payload that will hold the discriminator value.
    pub property_name: Option<String>,
    pub property_base_name: Option<String>,
    pub property_getter: Option<String>,
    pub property_type: Option<String>,
    pub mapping: HashMap<String, String>,
    pub mapped_models: BTreeSet<MappedModel>,
    pub vendor_extensions: HashMap<String, serde_json::Value>,
    pub is_enum: bool,
}

impl CodegenDiscriminator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_is_enum(&self) -> bool {
        self.is_enum
    }

    pub fn set_is_enum(&mut self, is_enum: bool) {
        self.is_enum = is_enum;
    }
}

/// Represents a mapped model in the discriminator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedModel {
    /// The value of the discriminator property in the payload.
    pub mapping_name: Option<String>,
    /// The OAS schema name.
    pub model_name: Option<String>,
    pub model: Option<CodegenModel>,
    pub explicit_mapping: bool,
}

impl MappedModel {
    pub fn new(
        mapping_name: Option<String>,
        model_name: Option<String>,
        explicit_mapping: bool,
    ) -> Self {
        Self {
            mapping_name,
            model_name,
            model: None,
            explicit_mapping,
        }
    }

    pub fn new_with_defaults(mapping_name: Option<String>, model_name: Option<String>) -> Self {
        Self::new(mapping_name, model_name, false)
    }
}

impl Ord for MappedModel {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.mapping_name, &other.mapping_name) {
            (None, None) => Ordering::Equal,
            (None, _) => Ordering::Greater,
            (_, None) => Ordering::Less,
            (Some(self_name), Some(other_name)) => {
                if self.explicit_mapping != other.explicit_mapping {
                    if self.explicit_mapping {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    }
                } else {
                    self_name.cmp(other_name)
                }
            }
        }
    }
}

impl PartialOrd for MappedModel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for MappedModel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mapping_name.hash(state);
        self.model_name.hash(state);
    }
}
