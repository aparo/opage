use convert_case::Casing;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;

use crate::{generator::templates::rust::RUST_PRIMITIVE_TYPES, GeneratorError};

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
    // Use scope for module names: propagated from config
    #[serde(default)]
    pub use_scope: bool,
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
            use_scope: false,
        }
    }

    pub fn set_scope(&mut self, use_scope: bool) {
        self.use_scope = use_scope;
    }

    pub fn name_to_struct_name(&self, path: &Vec<String>, name: &str) -> String {
        for primitive_type in RUST_PRIMITIVE_TYPES.iter() {
            if name.eq_ignore_ascii_case(primitive_type) {
                return primitive_type.to_string();
            }
        }
        let name = fix_struct_names(name, self.use_scope);
        let converted_name = convert_name(&name);
        let path_str = path_to_string(path, &converted_name);

        // trace!("name_to_struct_name {}", path_str);
        match self.struct_mapping.get(&path_str) {
            Some(name) => name.clone(),
            None => name.replace(".", "::").replace("::_common::", "::"),
        }
    }

    pub fn extract_struct_name(&self, full_name: &str) -> String {
        let parts = split_on_special_chars(full_name);
        let last_part = parts.last().unwrap();
        last_part.to_case(convert_case::Case::Pascal)
    }

    pub fn extract_function_name(&self, full_name: &str) -> String {
        let parts = split_on_special_chars(full_name);
        let last_part = parts.last().unwrap();
        last_part.to_case(convert_case::Case::Snake)
    }

    pub fn extract_package_name(&self, full_name: &str) -> String {
        let parts: Vec<&str> = full_name.split("::").collect();
        let mut package = String::new();
        for pos in 0..parts.len() - 1 {
            let part = parts[pos];
            if pos > 0 {
                package.push_str("::");
            }
            package.push_str(part);
        }
        package
    }

    pub fn name_to_property_name(&self, path: &Vec<String>, name: &str) -> String {
        let converted_name = name.to_case(convert_case::Case::Snake);
        let path_str = path_to_string(path, &converted_name);
        // trace!("name_to_property_name {}", path_str);
        match self.property_mapping.get(&path_str) {
            Some(name) => name.clone(),
            None => converted_name,
        }
    }

    pub fn type_to_property_type(&self, name: &str, original_type: &str) -> String {
        let converted_name = name.to_case(convert_case::Case::Snake);

        // trace!("type_to_property_type {} {}", converted_name, original_type);
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
        let mut name = name;
        for pos in 0..9 {
            if name.ends_with(format!(".{}", pos).as_str()) {
                name = &name[..name.len() - 2];
                break;
            }
        }
        let converted_name = name.to_case(convert_case::Case::Snake);

        match self.module_mapping.get(&converted_name) {
            Some(name) => name.clone(),
            None => {
                if self.use_scope {
                    if converted_name.contains(".") || converted_name.contains("::") {
                        converted_name
                    } else {
                        format!("common.{}", converted_name)
                    }
                } else {
                    converted_name
                }
            }
        }
    }

    pub fn status_code_to_canonical_name(
        &self,
        status_code: StatusCode,
    ) -> Result<String, GeneratorError> {
        if let Some(canonical_name) = self.status_code_mapping.get(status_code.as_str()) {
            return Ok(canonical_name.clone());
        }

        match status_code.canonical_reason() {
            Some(canonical_status_code) => Ok(canonical_status_code.to_owned()),
            None => {
                return Err(GeneratorError::StatusCodeError(
                    status_code.to_string(),
                    "Failed to get canonical status code".to_owned(),
                ))
            }
        }
    }

    pub fn validate_object_name_path(&self, name: &str, path: &str) -> (String, String) {
        if !name.contains(".") && !path.contains(".") {
            return (name.to_owned(), path.to_owned());
        }
        if name.contains(".") {
            let name_parts: Vec<&str> = name.split('.').collect();
            let last_part = name_parts.last().unwrap();
            if last_part.chars().next().unwrap().is_uppercase() {
                return (last_part.to_string(), path.to_owned());
            }
            let (prefix, remainer) = split_on_first_upper(last_part);

            return (
                remainer,
                path.replace(&format!("{}_", prefix), &format!("{}.", prefix)),
            );
        }
        (name.to_owned(), path.to_owned())
    }
}

fn split_on_first_upper(name: &str) -> (String, String) {
    let mut prefix = String::new();
    let mut remainer = String::new();
    let mut in_reminear = false;
    for c in name.chars() {
        if c.is_uppercase() {
            remainer.push(c);
            in_reminear = true;
            continue;
        }
        if in_reminear {
            remainer.push(c);
        } else {
            prefix.push(c);
        }
    }
    (prefix, remainer)
}

pub fn split_on_special_chars(name: &str) -> Vec<String> {
    let mut parts = vec![];
    let mut part = String::new();
    for c in name.chars() {
        if c == '.' || c == ':' {
            if !part.is_empty() {
                parts.push(part);
                part = String::new();
            }
            continue;
        }
        part.push(c);
    }
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

pub fn convert_name(name: &str) -> String {
    let tokens = split_on_special_chars(name);
    if tokens.len() == 1 {
        return name.to_case(convert_case::Case::Pascal);
    }
    let mut converted_name = String::new();
    for pos in 0..tokens.len() {
        let token = &tokens[pos];
        if pos > 0 {
            converted_name.push_str("::");
        }
        if pos == tokens.len() - 1 {
            converted_name.push_str(&token.to_case(convert_case::Case::Pascal));
            continue;
        } else {
            converted_name.push_str(&token);
        }
    }
    converted_name
}

pub fn fix_struct_names(name: &str, use_scope: bool) -> String {
    let mut name = name.replace(".", "::");
    if name.contains("___") {
        let parts: Vec<&str> = name.split("___").collect();
        let mut fixed_name = String::new();
        for pos in 0..parts.len() {
            let part = parts[pos];
            if pos > 0 {
                fixed_name.push_str("::");
            }
            fixed_name.push_str(&part.trim_start_matches("_"));
        }
        return fixed_name;
    }
    for pos in 0..9 {
        if name.ends_with(format!("::{}", pos).as_str()) {
            name = name[..name.len() - 2].to_owned();
            break;
        }
    }
    if !name.contains("::") {
        if use_scope {
            name = format!("common::{}", name.clone());
        } else {
            name = format!("models::{}", name.clone());
        }
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_object_name_path() {
        let name_mapping = NameMapping::new();
        let (name, path) = name_mapping.validate_object_name_path(
            "Common.aggregationsFieldDateMath",
            "common.aggregations_field_date_math",
        );
        assert_eq!(name, "FieldDateMath");
        assert_eq!(path, "common.aggregations.field_date_math");
    }

    #[test]
    fn test_fix_struct_names() {
        let name = "_common___Metadata";
        let fixed_name = fix_struct_names(name, true);
        assert_eq!(fixed_name, "common::Metadata");
    }

    #[test]
    fn test_split_on_special_chars() {
        let name = "common.aggregations::field_date_math";
        let parts = split_on_special_chars(name);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "common");
        assert_eq!(parts[1], "aggregations");
        assert_eq!(parts[2], "field_date_math");
    }
}
