use std::collections::{HashMap, HashSet, LinkedList};

/// Represents a schema object in an OpenAPI document.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CodegenModel {
    pub parent: Option<String>,
    pub parent_schema: Option<String>,
    pub interfaces: Vec<String>,
    pub all_parents: Vec<String>,
    pub parent_model: Option<Box<CodegenModel>>,
    pub interface_models: Vec<CodegenModel>,
    pub children: Vec<CodegenModel>,
    pub any_of: HashSet<String>,
    pub one_of: HashSet<String>,
    pub all_of: HashSet<String>,
    pub permits: Vec<String>,
    pub name: Option<String>,
    pub schema_name: Option<String>,
    pub classname: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub class_var_name: Option<String>,
    pub model_json: Option<String>,
    pub data_type: Option<String>,
    pub xml_prefix: Option<String>,
    pub xml_namespace: Option<String>,
    pub xml_name: Option<String>,
    pub class_filename: Option<String>,
    pub unescaped_description: Option<String>,
    pub discriminator: Option<CodegenDiscriminator>,
    pub default_value: Option<String>,
    pub array_model_type: Option<String>,
    pub is_alias: bool,
    pub is_string: bool,
    pub is_integer: bool,
    pub is_long: bool,
    pub is_number: bool,
    pub is_numeric: bool,
    pub is_float: bool,
    pub is_double: bool,
    pub is_date: bool,
    pub is_datetime: bool,
    pub is_decimal: bool,
    pub is_short: bool,
    pub is_unbounded_integer: bool,
    pub is_primitive_type: bool,
    pub is_boolean: bool,
    pub is_free_form_object: bool,
    pub vars: Vec<CodegenProperty>,
    pub all_vars: Vec<CodegenProperty>,
    pub required_vars: Vec<CodegenProperty>,
    pub optional_vars: Vec<CodegenProperty>,
    pub read_only_vars: Vec<CodegenProperty>,
    pub read_write_vars: Vec<CodegenProperty>,
    pub parent_vars: Vec<CodegenProperty>,
    pub non_nullable_vars: Vec<CodegenProperty>,
    pub allowable_values: HashMap<String, String>,
    pub mandatory: HashSet<String>,
    pub all_mandatory: HashSet<String>,
    pub imports: HashSet<String>,
    pub empty_vars: bool,
    pub has_vars: bool,
    pub has_more_models: bool,
    pub has_enums: bool,
    pub is_enum: bool,
    pub has_validation: bool,
    pub is_nullable: bool,
    pub has_required: bool,
    pub has_optional: bool,
    pub is_array: bool,
    pub has_children: bool,
    pub is_map: bool,
    pub is_optional: bool,
    pub is_null: bool,
    pub is_void: bool,
    pub is_deprecated: bool,
    pub has_read_only: bool,
    pub has_only_read_only: bool,
    pub external_documentation: Option<ExternalDocumentation>,
    pub vendor_extensions: HashMap<String, String>,
    pub additional_properties_type: Option<String>,
    pub is_additional_properties_true: bool,
    pub max_properties: Option<u32>,
    pub min_properties: Option<u32>,
    pub unique_items: bool,
    pub unique_items_boolean: Option<bool>,
    pub max_items: Option<u32>,
    pub min_items: Option<u32>,
    pub max_length: Option<u32>,
    pub min_length: Option<u32>,
    pub exclusive_minimum: bool,
    pub exclusive_maximum: bool,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
    pub pattern: Option<String>,
    pub multiple_of: Option<f64>,
    pub items: Option<Box<CodegenProperty>>,
    pub additional_properties: Option<Box<CodegenProperty>>,
    pub is_model: bool,
    pub has_required_vars: bool,
    pub has_discriminator_with_non_empty_mapping: bool,
    pub is_any_type: bool,
    pub is_uuid: bool,
    pub is_uri: bool,
    pub required_vars_map: HashMap<String, CodegenProperty>,
    pub ref_: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct CodegenProperty {
    pub base_name: String,
    pub data_type: String,
    pub is_container: bool,
    pub items: Option<Box<CodegenProperty>>,
    pub is_self_reference: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CodegenDiscriminator {
    pub property_name: String,
    pub mapped_models: Vec<MappedModel>,
}

#[derive(Debug, Default, Clone)]
pub struct MappedModel {
    pub model_name: String,
}

#[derive(Debug, Default, Clone)]
pub struct ExternalDocumentation {
    pub description: Option<String>,
    pub url: Option<String>,
}

impl CodegenModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove_self_reference_import(&mut self) {
        for var in &mut self.all_vars {
            if var
                .data_type
                .eq_ignore_ascii_case(self.classname.as_deref().unwrap_or(""))
            {
                self.imports.remove(self.classname.as_deref().unwrap_or(""));
                var.is_self_reference = true;
            }
        }
    }

    pub fn remove_all_duplicated_property(&mut self) {
        self.vars = Self::remove_duplicated_property(&self.vars);
        self.optional_vars = Self::remove_duplicated_property(&self.optional_vars);
        self.required_vars = Self::remove_duplicated_property(&self.required_vars);
        self.parent_vars = Self::remove_duplicated_property(&self.parent_vars);
        self.all_vars = Self::remove_duplicated_property(&self.all_vars);
        self.non_nullable_vars = Self::remove_duplicated_property(&self.non_nullable_vars);
        self.read_only_vars = Self::remove_duplicated_property(&self.read_only_vars);
        self.read_write_vars = Self::remove_duplicated_property(&self.read_write_vars);
    }

    fn remove_duplicated_property(vars: &[CodegenProperty]) -> Vec<CodegenProperty> {
        let mut seen = HashSet::new();
        vars.iter()
            .filter(|var| seen.insert(var.base_name.clone()))
            .cloned()
            .collect()
    }
}
