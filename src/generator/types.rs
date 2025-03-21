use crate::generator::templates::rust::{Field, RustEnumTemplate, RustStructTemplate};
use crate::utils::config::Config;
use crate::GeneratorError;
use askama::Template;
use dashmap::DashMap;
use std::collections::HashMap;

use super::templates::rust;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleInfo {
    pub name: String,
    pub path: String,
}

impl ModuleInfo {
    pub fn new(path: &str, name: &str) -> Self {
        let mut final_name = name.to_string();
        let mut final_path = path.to_string();
        if final_name.contains("::") {
            let parts: Vec<&str> = name.split("::").collect();
            final_name = parts[parts.len() - 1].to_owned();
            for sep in parts[..parts.len() - 1].iter() {
                if final_path.contains(format!("::{}", sep).as_str()) {
                    continue;
                }
                if !final_path.is_empty() {
                    final_path.push_str("::");
                }
                final_path.push_str(sep);
            }
        }

        ModuleInfo {
            name: final_name,
            path: final_path,
        }
    }

    pub fn to_use(&self) -> String {
        if self.path.is_empty() {
            return format!("use {};", self.name);
        }
        format!("use {}::{};", self.path, self.name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDefinition {
    pub name: String,
    pub module: Option<ModuleInfo>,
    pub description: Option<String>,
    pub example: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyDefinition {
    pub name: String,
    pub real_name: String,
    pub type_name: String,
    pub module: Option<ModuleInfo>,
    pub required: bool,
    pub description: Option<String>,
    pub example: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectDefinition {
    Struct(StructDefinition),
    Enum(EnumDefinition),
    Primitive(PrimitiveDefinition),
}

impl ObjectDefinition {
    pub fn name(&self) -> String {
        match self {
            ObjectDefinition::Struct(struct_definition) => struct_definition.name.clone(),
            ObjectDefinition::Enum(enum_definition) => enum_definition.name.clone(),
            ObjectDefinition::Primitive(primitive_definition) => primitive_definition.name.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub value_type: TypeDefinition,
}

pub type ObjectDatabase = DashMap<String, ObjectDefinition>;
pub type PathDatabase = DashMap<String, PathDefinition>;

#[derive(Clone, Debug, PartialEq)]
pub struct EnumDefinition {
    pub name: String,
    // pub namespace: String,
    pub used_modules: Vec<ModuleInfo>,
    pub values: HashMap<String, EnumValue>,
    pub description: Option<String>,
}

impl EnumDefinition {
    // pub fn id(&self) -> String {
    //     format!("{}::{}", self.namespace, self.name)
    // }

    pub fn get_required_modules(&self) -> Vec<&ModuleInfo> {
        let mut required_modules = self.used_modules.iter().collect::<Vec<&ModuleInfo>>();
        required_modules.append(
            &mut self
                .values
                .iter()
                .filter_map(|(_, enum_value)| enum_value.value_type.module.as_ref())
                .collect::<Vec<&ModuleInfo>>(),
        );
        required_modules
    }

    pub fn to_string(&self, serializable: bool, config: &Config) -> Result<String, GeneratorError> {
        match config.language {
            crate::Language::Rust => Ok(rust::render_enum_definition(&self, serializable)),
            _ => Err(GeneratorError::UnsupportedLanguageError(format!(
                "Error rendering StructDefinition {} {}",
                self.name,
                config.language.to_string()
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct StructDefinition {
    pub package: String,
    pub name: String,
    pub used_modules: Vec<ModuleInfo>,
    pub properties: HashMap<String, PropertyDefinition>,
    pub local_objects: HashMap<String, Box<ObjectDefinition>>,
    pub description: Option<String>,
}

impl StructDefinition {
    pub fn id(&self) -> String {
        format!("{}::{}", self.package, self.name)
    }

    pub fn all_properties_default(&self) -> bool {
        self.properties
            .iter()
            .all(|(_, property)| !property.required || property.type_name.starts_with("Vec<"))
    }

    pub fn get_required_modules(&self) -> Vec<&ModuleInfo> {
        let mut required_modules = self.used_modules.iter().collect::<Vec<&ModuleInfo>>();
        required_modules.append(
            &mut self
                .properties
                .iter()
                .filter_map(|(_, property)| property.module.as_ref())
                .collect::<Vec<&ModuleInfo>>(),
        );
        required_modules
    }

    pub fn to_string(&self, serializable: bool, config: &Config) -> Result<String, GeneratorError> {
        match config.language {
            crate::Language::Rust => {
                Ok(rust::render_struct_definition(&self, serializable, config))
            }
            _ => Err(GeneratorError::UnsupportedLanguageError(format!(
                "Error rendering StructDefinition {} {}",
                self.name,
                config.language.to_string()
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PrimitiveDefinition {
    pub name: String,
    pub primitive_type: TypeDefinition,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub enum TransferMediaType {
    ApplicationJson(Option<TypeDefinition>),
    TextPlain,
}

pub type ContentTypeValue = String;

#[derive(Clone, Debug)]
pub struct ResponseEntity {
    pub canonical_status_code: String,
    pub content: HashMap<ContentTypeValue, TransferMediaType>,
}

#[derive(Clone, Debug)]
pub struct RequestEntity {
    pub content: HashMap<ContentTypeValue, TransferMediaType>,
}

pub type ResponseEntities = HashMap<String, ResponseEntity>;

#[derive(Clone, Debug, Default)]
pub struct QueryParameters {
    pub query_struct: StructDefinition,
    pub query_struct_variable_name: String,
    pub unroll_query_parameters_code: String,
}

#[derive(Clone, Debug, Default)]
pub struct PathParameters {
    pub parameters_struct_variable_name: String,
    pub parameters_struct: StructDefinition,
    pub path_format_string: String,
}

#[derive(Clone, Debug)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
}

impl ToString for Method {
    fn to_string(&self) -> String {
        match self {
            Method::GET => "GET".to_string(),
            Method::POST => "POST".to_string(),
            Method::PUT => "PUT".to_string(),
            Method::DELETE => "DELETE".to_string(),
            Method::PATCH => "PATCH".to_string(),
            Method::HEAD => "HEAD".to_string(),
            Method::OPTIONS => "OPTIONS".to_string(),
            Method::TRACE => "TRACE".to_string(),
        }
    }
}

// impl std::fmt::Display for Method {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.to_string())
//     }
// }

#[derive(Clone, Debug)]
pub struct PathDefinition {
    pub package: String,
    pub name: String,
    pub method: Method,
    pub url: String,
    pub response_name: String,
    pub used_modules: Vec<ModuleInfo>,
    pub request_body: Option<ObjectDefinition>,
    pub request_entity: Option<RequestEntity>,
    pub local_objects: HashMap<String, Box<ObjectDefinition>>,
    pub description: String,
    pub response_entities: ResponseEntities,
    pub path_parameters: PathParameters,
    pub query_parameters: QueryParameters,
}

impl Default for PathDefinition {
    fn default() -> Self {
        PathDefinition {
            package: "".to_string(),
            name: "".to_string(),
            method: Method::GET,
            url: "/".to_string(),
            response_name: "".to_string(),
            used_modules: vec![],
            request_body: None,
            request_entity: None,
            local_objects: HashMap::new(),
            description: "".to_string(),
            response_entities: HashMap::new(),
            path_parameters: PathParameters::default(),
            query_parameters: QueryParameters::default(),
        }
    }
}

impl PathDefinition {
    pub fn get_request_type(&self) -> Option<TypeDefinition> {
        if let Some(object_definition) = &self.request_body {
            match object_definition {
                ObjectDefinition::Struct(struct_definition) => {
                    let object_name = struct_definition.id();
                    let object_path = struct_definition.package.clone();
                    return Some(TypeDefinition {
                        name: object_name.clone(),
                        module: Some(ModuleInfo::new(&object_path, &object_name)),
                        description: struct_definition.description.clone(),
                        example: struct_definition
                            .properties
                            .values()
                            .next()
                            .unwrap()
                            .example
                            .clone(),
                    });
                }
                // TODO manage enums
                _ => return None,
            }
        }
        None
    }
    pub fn extract_body_properties(&self) -> Vec<(String, PropertyDefinition)> {
        let mut properties = vec![];
        if let Some(object_definition) = &self.request_body {
            match object_definition {
                ObjectDefinition::Struct(struct_definition) => {
                    for (name, property) in &struct_definition.properties {
                        properties.push((name.clone(), property.clone()));
                    }
                }
                // TODO manage enums
                _ => (),
            }
        }
        properties
    }

    pub fn get_required_properties(&self) -> Vec<PropertyDefinition> {
        let mut required_properties = vec![];
        for (_, property) in &self.path_parameters.parameters_struct.properties {
            if property.required {
                required_properties.push(property.clone());
            }
        }
        for (_, property) in &self.query_parameters.query_struct.properties {
            if property.required {
                required_properties.push(property.clone());
            }
        }

        for (_, property) in self.extract_body_properties() {
            if property.required {
                required_properties.push(property.clone());
            }
        }
        required_properties
    }

    pub fn get_optional_properties(&self) -> Vec<PropertyDefinition> {
        let mut optional_properties = vec![];
        for (_, property) in &self.path_parameters.parameters_struct.properties {
            if !property.required {
                optional_properties.push(property.clone());
            }
        }
        for (_, property) in &self.query_parameters.query_struct.properties {
            if !property.required {
                optional_properties.push(property.clone());
            }
        }
        for (_, property) in self.extract_body_properties() {
            if !property.required {
                optional_properties.push(property.clone());
            }
        }

        optional_properties
    }

    pub fn extract_response_modules(&self) -> Vec<ModuleInfo> {
        let mut module_imports: Vec<ModuleInfo> = vec![];
        for (_, entity) in &self.response_entities {
            for (_, content) in &entity.content {
                match content {
                    TransferMediaType::ApplicationJson(ref type_definition) => {
                        match type_definition {
                            Some(type_definition) => match type_definition.module {
                                Some(ref module_info) => {
                                    if module_imports.contains(module_info) {
                                        continue;
                                    }
                                    module_imports.push(module_info.clone());
                                }
                                _ => (),
                            },
                            None => (),
                        }
                    }
                    TransferMediaType::TextPlain => (),
                }
            }
        }
        module_imports
    }

    pub fn extract_response_type(&self) -> Option<TypeDefinition> {
        let mut response_type = None;
        for (_, entity) in &self.response_entities {
            for (_, content) in &entity.content {
                match content {
                    TransferMediaType::ApplicationJson(ref type_definition) => {
                        match type_definition {
                            Some(type_definition) => {
                                response_type = Some(type_definition.clone());
                            }
                            None => (),
                        }
                    }
                    TransferMediaType::TextPlain => (),
                }
            }
        }
        response_type
    }
}
