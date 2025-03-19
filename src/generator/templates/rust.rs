use crate::generator::types::{ModuleInfo, ObjectDatabase, PropertyDefinition, TypeDefinition};
use crate::utils::config::Config;
use crate::utils::file::write_filename;
use crate::utils::name_mapping::convert_name;
use crate::GeneratorError;
use askama::Template;
use std::collections::HashSet;
use std::path::PathBuf;

// list of primitive types of Rust language
pub const RUST_PRIMITIVE_TYPES: [&str; 13] = [
    "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "String",
];

#[derive(Template)]
#[template(path = "rust/enum.j2", escape = "none")]
pub struct RustEnumTemplate<'a> {
    pub imports: Vec<String>,
    pub derivations: Vec<&'a str>,
    pub description: &'a str,
    pub name: &'a str,
    pub variants: Vec<String>,
}

#[derive(Template)]
#[template(path = "rust/type.j2", escape = "none")]
pub struct RustTypeTemplate<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub description: &'a str,
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone)]
pub struct Field {
    pub annotations: Vec<String>,
    pub description: String,
    pub modifier: String,
    pub name: String,
    pub typ: String,
}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}
#[derive(Template)]
#[template(path = "rust/struct.j2", escape = "none")]
pub struct RustStructTemplate<'a> {
    pub imports: Vec<String>,
    pub derivations: Vec<&'a str>,
    pub description: &'a str,
    pub name: &'a str,
    pub fields: Vec<Field>,
}

#[derive(Template)]
#[template(path = "rust/builder_struct.j2", escape = "none")]
pub struct RustBuilderStructTemplate<'a> {
    pub imports: Vec<ModuleInfo>,
    pub derivations: Vec<&'a str>,
    pub description: &'a str,
    pub name: &'a str,
    pub response_type: &'a str,
    pub builder_name: &'a str,
    pub fields: Vec<Field>,
    pub method: &'a str,
    pub path: &'a str,
    pub path_fields: Vec<Field>,
    pub query_fields: Vec<Field>,
    pub body_fields: Vec<Field>,
    pub body_request: Option<TypeDefinition>,
}

#[derive(Template)]
#[template(path = "rust/cargo.j2", escape = "none")]
pub struct CargoTemplate<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

pub fn populate_client_files(output_dir: &PathBuf, config: &Config) -> Result<(), GeneratorError> {
    let cargo_target_file = output_dir.join("Cargo.toml");

    let template = CargoTemplate {
        name: config.project_metadata.name.as_str(),
        version: config.project_metadata.version.as_str(),
    }
    .render()
    .unwrap();

    write_filename(&cargo_target_file, &template)?;

    let files = vec![
        (
            embed_file::embed_string!("embedded/rust/auth_middleware.rs"),
            "src/auth_middleware.rs",
        ),
        (
            embed_file::embed_string!("embedded/rust/credentials.rs"),
            "src/credentials.rs",
        ),
        (
            embed_file::embed_string!("embedded/rust/client.rs"),
            "src/client.rs",
        ),
    ];

    for (content, file_name) in files {
        let target_file = output_dir.join(file_name);
        write_filename(&target_file, &content)?;
    }

    Ok(())
}

#[derive(Template)]
#[template(path = "rust/client_function.j2", escape = "none")]
pub struct RustClientFunctionTemplate<'a> {
    pub name: &'a str,
    pub description: String,
    pub required_properties: Vec<PropertyDefinition>,
    pub builder_name: String,
}

#[derive(Clone, Debug)]
pub struct BuilderInfo {
    pub name: String,
    pub code: String,
    pub imports: Vec<ModuleInfo>,
}

pub fn generate_rust_client_code(
    paths: Vec<crate::generator::types::PathDefinition>,
    config: &Config,
    object_database: &ObjectDatabase,
) -> (String, Vec<BuilderInfo>) {
    let mut imports = HashSet::new();

    let mut client_code = String::new();
    let mut function_code = String::new();

    let mut builders: Vec<BuilderInfo> = vec![];

    for path in paths.iter() {
        let required_properties = path.get_required_properties();
        let response_type = extract_default_rust_response_type(path.extract_response_type());
        let scope: Vec<String> = vec![];
        let builder_name = format!("{}Builder", convert_name(&path.name));

        // we build description for the function
        let mut description = path.description.clone();
        description.push_str("\n");
        description.push_str("\n");
        description.push_str(
            format!("Sends a `{:?}` request to `{}`\n\n", path.method, path.url).as_str(),
        );
        description.push_str("Arguments:\n");
        for property in required_properties.iter() {
            description.push_str(
                format!(
                    "- `{}`: {}\n",
                    property.name,
                    property
                        .description
                        .clone()
                        .unwrap_or(String::from("No description available")),
                )
                .as_str(),
            );
        }

        let function = RustClientFunctionTemplate {
            name: &path.name,
            description: fix_rust_description("", &description),
            required_properties,
            builder_name: builder_name.clone(),
        };
        function_code.push_str(&function.render().unwrap());

        let mut builder_imports = HashSet::new();

        for import in path.used_modules.iter() {
            imports.insert(import.clone());
            builder_imports.insert(import.clone());
        }

        // generating builder code
        let required_properties = path.get_required_properties();
        let optional_properties = path.get_optional_properties();
        let mut fields = vec![];
        let mut processed_builder_fields = vec![];
        let mut description = String::new();
        description.push_str(
            format!(
                "Builder used to sends a `{:?}` request to `{}`\n\n",
                path.method, path.url
            )
            .as_str(),
        );
        description.push_str("Arguments:\n");
        // we emit client code
        description.push_str("- `client`: The client used to send the request\n");
        fields.push(Field {
            annotations: vec![], //"#[builder(setter)]".to_string()
            description: fix_rust_description("", "The client used to send the request"),
            modifier: "pub".to_string(),
            name: "client".to_string(),
            typ: config.client_name.clone(),
        });

        for fields_group in [required_properties, optional_properties].iter() {
            for property in fields_group.iter() {
                let annotations = vec![];
                let name = property.name.clone();
                if processed_builder_fields.contains(&name) {
                    continue;
                }
                description.push_str(
                    format!(
                        "- `{}`: {}\n",
                        property.name,
                        property
                            .description
                            .clone()
                            .unwrap_or(String::from("No description available")),
                    )
                    .as_str(),
                );
                // if property.required {
                //     annotations.push("#[builder(setter)]".to_string());
                // }
                let field = Field {
                    annotations,
                    description: fix_rust_description(
                        "",
                        &property
                            .description
                            .clone()
                            .unwrap_or(String::from("No description available")),
                    ),
                    modifier: "pub".to_string(),
                    name: property.name.clone(),
                    typ: fix_type_name_property(&property.type_name),
                };
                fields.push(field);
                processed_builder_fields.push(property.name.clone());
            }
        }
        let builder_imports: Vec<ModuleInfo> = builder_imports.iter().cloned().collect();
        let body_fields: Vec<Field> = path
            .extract_body_properties()
            .iter()
            .map(|p| property_definition_to_field(&p.1))
            .collect();
        let body_request = path.get_request_type();

        let builder_template = RustBuilderStructTemplate {
            imports: builder_imports.clone(),
            derivations: vec!["Builder", "Debug", "Default"],
            description: &fix_rust_description("", &description),
            name: &convert_name(&path.name),
            builder_name: &builder_name,
            response_type: &response_type,
            fields,
            method: &path.method.to_string(),
            path: &path.url,
            path_fields: path
                .path_parameters
                .parameters_struct
                .properties
                .clone()
                .into_iter()
                .map(|p| property_definition_to_field(&p.1))
                .collect(),
            query_fields: path
                .query_parameters
                .query_struct
                .properties
                .clone()
                .into_iter()
                .map(|p| property_definition_to_field(&p.1))
                .collect(),
            body_fields,
            body_request,
        };
        let builder_code = builder_template.render().unwrap();
        builders.push(BuilderInfo {
            name: path.name.clone(),
            code: builder_code,
            imports: builder_imports,
        });
    }
    client_code.push_str(&function_code);
    (client_code, builders)
}

fn property_definition_to_field(property: &PropertyDefinition) -> Field {
    Field {
        annotations: vec![],
        description: fix_rust_description(
            "",
            &property
                .description
                .clone()
                .unwrap_or(String::from("No description available")),
        ),
        modifier: "pub".to_string(),
        name: property.name.clone(),
        typ: fix_type_name_property(&property.type_name),
    }
}

pub fn fix_type_name_property(property: &str) -> String {
    if property.starts_with("crate::") {
        return property.to_string();
    }
    if RUST_PRIMITIVE_TYPES.contains(&property) {
        return property.to_string();
    }
    if property.starts_with("models::") {
        return format!("crate::{}", property);
    }
    return property.to_string();
}

pub fn fix_rust_description(ident: &str, description: &str) -> String {
    if description.is_empty() {
        return "".to_string();
    }
    let result = description
        .lines()
        .map(|line| format!("{}/// {}\n", ident, line))
        .collect::<String>()
        .trim()
        .to_string();
    if result.starts_with("///") {
        return result;
    } else {
        return format!("/// {}", result);
    }
}

pub fn extract_default_rust_response_type(optional_response: Option<TypeDefinition>) -> String {
    match optional_response {
        Some(response) => {
            let name = response.name.clone();
            if !name.starts_with("crate::") {
                format!("crate::{}", name)
            } else {
                name
            }
        }
        None => "serde_json:Value".to_string(),
    }
}
