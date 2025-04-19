use std::collections::{HashMap, HashSet, LinkedList};

#[derive(Debug, Default, Clone)]
pub struct CodegenResponse {
    pub headers: Vec<CodegenProperty>,
    pub response_headers: Vec<CodegenParameter>,
    pub code: Option<String>,
    pub is1xx: bool,
    pub is2xx: bool,
    pub is3xx: bool,
    pub is4xx: bool,
    pub is5xx: bool,
    pub message: Option<String>,
    pub examples: Vec<HashMap<String, String>>,
    pub data_type: Option<String>,
    pub base_type: Option<String>,
    pub container_type: Option<String>,
    pub container_type_mapped: Option<String>,
    pub has_headers: bool,
    pub is_string: bool,
    pub is_numeric: bool,
    pub is_integer: bool,
    pub is_short: bool,
    pub is_long: bool,
    pub is_unbounded_integer: bool,
    pub is_number: bool,
    pub is_float: bool,
    pub is_double: bool,
    pub is_decimal: bool,
    pub is_byte_array: bool,
    pub is_boolean: bool,
    pub is_date: bool,
    pub is_datetime: bool,
    pub is_uuid: bool,
    pub is_email: bool,
    pub is_password: bool,
    pub is_model: bool,
    pub is_free_form_object: bool,
    pub is_any_type: bool,
    pub is_default: bool,
    pub simple_type: bool,
    pub primitive_type: bool,
    pub is_map: bool,
    pub is_optional: bool,
    pub is_array: bool,
    pub is_binary: bool,
    pub is_file: bool,
    pub is_null: bool,
    pub is_void: bool,
    pub schema: Option<String>,
    pub json_schema: Option<String>,
    pub vendor_extensions: HashMap<String, String>,
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
    pub vars: Vec<CodegenProperty>,
    pub required_vars: Vec<CodegenProperty>,
    pub has_validation: bool,
    pub additional_properties_is_any_type: bool,
    pub has_vars: bool,
    pub has_required: bool,
    pub has_discriminator_with_non_empty_mapping: bool,
    pub composed_schemas: Option<CodegenComposedSchemas>,
    pub has_multiple_types: bool,
    pub content: Option<HashMap<String, CodegenMediaType>>,
    pub required_vars_map: Option<HashMap<String, CodegenProperty>>,
    pub ref_: Option<String>,
    pub return_property: Option<CodegenProperty>,
    pub schema_is_from_additional_properties: bool,
}

impl CodegenResponse {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_wildcard(&self) -> bool {
        matches!(self.code.as_deref(), Some("0") | Some("default"))
    }

    pub fn is_range(&self) -> bool {
        if let Some(code) = &self.code {
            code.len() == 3 && code[1..].eq_ignore_ascii_case("XX")
        } else {
            false
        }
    }

    pub fn set_additional_properties_is_any_type(&mut self, value: bool) {
        self.additional_properties_is_any_type = value;
    }

    pub fn get_has_vars(&self) -> bool {
        self.has_vars
    }

    pub fn set_has_vars(&mut self, value: bool) {
        self.has_vars = value;
    }

    pub fn get_has_discriminator_with_non_empty_mapping(&self) -> bool {
        self.has_discriminator_with_non_empty_mapping
    }

    pub fn set_has_discriminator_with_non_empty_mapping(&mut self, value: bool) {
        self.has_discriminator_with_non_empty_mapping = value;
    }

    pub fn get_is_string(&self) -> bool {
        self.is_string
    }

    pub fn set_is_string(&mut self, value: bool) {
        self.is_string = value;
    }

    pub fn get_is_number(&self) -> bool {
        self.is_number
    }

    pub fn set_is_number(&mut self, value: bool) {
        self.is_number = value;
    }

    pub fn get_is_any_type(&self) -> bool {
        self.is_any_type
    }

    pub fn set_is_any_type(&mut self, value: bool) {
        self.is_any_type = value;
    }

    pub fn get_is_free_form_object(&self) -> bool {
        self.is_free_form_object
    }

    pub fn set_is_free_form_object(&mut self, value: bool) {
        self.is_free_form_object = value;
    }
}

// Placeholder structs for dependencies
#[derive(Debug, Default, Clone)]
pub struct CodegenProperty {
    pub base_name: String,
    pub data_type: String,
    pub is_container: bool,
    pub items: Option<Box<CodegenProperty>>,
    pub is_self_reference: bool,
}

#[derive(Debug, Default, Clone)]
pub struct CodegenParameter;

#[derive(Debug, Default, Clone)]
pub struct CodegenComposedSchemas;

#[derive(Debug, Default, Clone)]
pub struct CodegenMediaType;
