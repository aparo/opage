use convert_case::Casing;
use log::trace;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct NameMapping {
    #[serde(default)]
    pub struct_mapping: HashMap<String, String>,
    #[serde(default)]
    pub property_mapping: HashMap<String, String>,
    #[serde(default)]
    pub property_type_mapping: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub module_mapping: HashMap<String, String>,
    #[serde(default)]
    pub status_code_mapping: HashMap<String, String>,
    #[serde(default)]
    pub i32_to_u32: bool,
}

fn path_to_string(path: &Vec<String>, token_name: &str) -> String {
    let path_str = path.join("/");
    match path_str.len() {
        0 => format!("/{}", token_name),
        _ => format!("/{}/{}", path_str, token_name),
    }
    .replace("//", "/")
}

impl NameMapping {
    pub fn new() -> Self {
        NameMapping {
            module_mapping: HashMap::new(),
            property_mapping: HashMap::new(),
            property_type_mapping: HashMap::new(),
            struct_mapping: HashMap::new(),
            status_code_mapping: HashMap::new(),
            i32_to_u32: false,
        }
    }

    pub fn name_to_struct_name(&self, path: &Vec<String>, name: &str) -> String {
        let converted_name = name.to_case(convert_case::Case::Pascal);
        let path_str = path_to_string(path, &converted_name);

        trace!("name_to_struct_name {}", path_str);
        match self.struct_mapping.get(&path_str) {
            Some(name) => name.clone(),
            None => converted_name,
        }
    }

    pub fn name_to_property_name(&self, path: &Vec<String>, name: &str) -> String {
        let converted_name = name.to_case(convert_case::Case::Snake);
        let path_str = path_to_string(path, &converted_name);
        trace!("name_to_property_name {}", path_str);
        match self.property_mapping.get(&path_str) {
            Some(name) => name.clone(),
            None => converted_name,
        }
    }

    pub fn type_to_property_type(&self, name: &str, original_type: &str) -> String {
        let converted_name = name.to_case(convert_case::Case::Snake);

        trace!("type_to_property_type {} {}", converted_name, original_type);
        match self.property_type_mapping.get(&converted_name) {
            Some(name_types) => match name_types.get(original_type) {
                Some(final_type) => final_type.to_owned(),
                None => {
                    if self.i32_to_u32 && original_type.eq_ignore_ascii_case("i32") {
                        "u32".to_owned()
                    } else {
                        original_type.to_owned()
                    }
                }
            },
            None => {
                if self.i32_to_u32 && original_type.eq_ignore_ascii_case("i32") {
                    "u32".to_owned()
                } else {
                    original_type.to_owned()
                }
            }
        }
    }

    pub fn name_to_module_name(&self, name: &str) -> String {
        let converted_name = name.to_case(convert_case::Case::Snake);

        match self.module_mapping.get(&converted_name) {
            Some(name) => name.clone(),
            None => converted_name,
        }
    }

    pub fn status_code_to_canonical_name(&self, status_code: StatusCode) -> Result<String, String> {
        if let Some(canonical_name) = self.status_code_mapping.get(status_code.as_str()) {
            return Ok(canonical_name.clone());
        }

        match status_code.canonical_reason() {
            Some(canonical_status_code) => Ok(canonical_status_code.to_owned()),
            None => {
                return Err(format!(
                    "Failed to get canonical status code {}",
                    status_code
                ))
            }
        }
    }
}
