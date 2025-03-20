use crate::generator::component::object_definition::get_object_name;
use crate::generator::types::{
    ModuleInfo, ObjectDatabase, ObjectDefinition, PathDatabase, PropertyDefinition, TypeDefinition,
};
use crate::utils::config::Config;
use crate::utils::file::write_filename;
use crate::utils::name_mapping::convert_name;
use crate::GeneratorError;
use askama::Template;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
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

#[derive(Template)]
#[template(path = "rust/client_init.j2", escape = "none")]
pub struct RustClientInitTemplate<'a> {
    pub name: &'a str,
    pub client_name: &'a str,
    pub server_url: &'a str,
    pub user_agent: &'a str,
    pub version: &'a str,
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
            typ: config.project_metadata.client_name.clone(),
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

pub fn generate_clients(
    output_dir: &PathBuf,
    path_database: &PathDatabase,
    config: &Config,
    object_database: &ObjectDatabase,
) -> Result<(), GeneratorError> {
    // Write all registered API calls in a client
    let target_dir = output_dir.join("src");
    let chunks = path_database.iter().chunk_by(|f| f.value().package.clone());

    let mut grouped_paths: Vec<_> = chunks.into_iter().collect();

    grouped_paths.sort_by(|a, b| a.0.cmp(&b.0));

    for (namespace, group) in grouped_paths {
        let items = group.map(|f| f.clone()).collect::<Vec<_>>();
        let (client_code, builders) = generate_rust_client_code(items, config, object_database);
        let mut path = namespace.replace(".", "/").replace("::", "/");
        if path.is_empty() {
            path = "lib".to_owned();
        }

        let full_path = target_dir.join(format!("{}.rs", path));
        println!(
            "Writing to {} \n{}",
            full_path.to_str().unwrap(),
            &client_code
        );
        write_filename(&full_path, &client_code)?;

        // we create builder files
        let mut imports = vec![];
        let mut builder_code = String::new();
        for builder in builders {
            for import in builder.imports {
                let use_def = import.to_use();
                if imports.contains(&use_def) {
                    continue;
                }
                imports.push(import.to_use());
            }
            builder_code.push_str(&builder.code);
            builder_code.push_str("\n");
        }
        let mut full_builder = String::new();
        full_builder.push_str("use crate::Client;\n");
        full_builder.push_str("use crate::client::ResponseValue;\n");
        full_builder.push_str("use crate::client::Request;\n");
        full_builder.push_str("use reqwest::Method;\n");
        full_builder.push_str("use derive_builder::Builder;\n");
        imports.sort();
        for import in imports {
            full_builder.push_str(&import);
            full_builder.push_str("\n");
        }
        full_builder.push_str("\n");
        full_builder.push_str(&builder_code);

        let builder_path = target_dir.join("builders.rs");
        println!(
            "Writing to {} \n{}",
            builder_path.to_str().unwrap(),
            &full_builder
        );
        write_filename(&builder_path, &full_builder)?;
    }

    Ok(())
}

pub fn write_object_database(
    output_dir: &PathBuf,
    object_database: &ObjectDatabase,
    config: &Config,
) -> Result<(), GeneratorError> {
    let name_mapping = &config.name_mapping;
    let target_dir = if config.name_mapping.use_scope {
        output_dir.join("src")
    } else {
        output_dir.join("src").join("models")
    };
    let mut type_map: HashMap<String, (Vec<String>, Vec<String>)> =
        std::collections::HashMap::new();

    let mut mods_map: HashMap<String, Vec<String>> = HashMap::new();

    std::fs::create_dir_all(&target_dir).expect("Creating objects dir failed");

    for item in object_database.iter() {
        let object_definition = item.value();
        let object_name = get_object_name(object_definition);

        let module_name = name_mapping.name_to_module_name(&object_name);

        let target_file = target_dir.join(format!(
            "{}.rs",
            module_name.replace(".", "/").replace("::", "/")
        ));
        let namespace = extract_rust_namespace(&module_name);

        match object_definition {
            ObjectDefinition::Struct(struct_definition) => {
                let mut result = modules_to_string(&struct_definition.get_required_modules());
                result.push_str("\n");
                result.push_str(&struct_definition.to_string(true, config)?);
                write_filename(&target_file, &result).unwrap();
                let mut mods = vec![];
                if mods_map.contains_key(&namespace) {
                    mods = mods_map.get(&namespace).unwrap().clone();
                }
                mods.push(format!(
                    "pub mod {};",
                    &target_file.file_stem().unwrap().to_str().unwrap()
                ));
                mods_map.insert(namespace, mods);
            }
            ObjectDefinition::Enum(enum_definition) => {
                let mut result = modules_to_string(&enum_definition.get_required_modules());
                result.push_str("\n");
                result.push_str(&enum_definition.to_string(true, config)?);
                write_filename(&target_file, &result).unwrap();
                // we update the mods list
                let mut mods = vec![];
                if mods_map.contains_key(&namespace) {
                    mods = mods_map.get(&namespace).unwrap().clone();
                }
                mods.push(format!(
                    "pub mod {};",
                    &target_file.file_stem().unwrap().to_str().unwrap()
                ));
                mods_map.insert(namespace, mods);
            }
            ObjectDefinition::Primitive(primitive_definition) => {
                let mut imports = vec![];
                let mut codes = vec![];
                if type_map.contains_key(&namespace) {
                    let (import, code) = type_map.get(&namespace).unwrap();
                    imports = import.clone();
                    codes = code.clone();
                }

                if let Some(module) = &primitive_definition.primitive_type.module {
                    imports.push(module.to_use());
                }

                let description = fix_rust_description(
                    "",
                    &primitive_definition
                        .description
                        .as_ref()
                        .map_or("", |d| d.as_str()),
                );

                let template = RustTypeTemplate {
                    name: extract_rust_name(&primitive_definition.name).as_str(),
                    description: description.as_str(),
                    value: extract_rust_name(&primitive_definition.primitive_type.name).as_str(),
                }
                .render()
                .unwrap();

                codes.push(template);
                type_map.insert(namespace, (imports, codes));
            }
        }
    }
    let mut created_modules = vec![];

    for (module_name, mods) in mods_map.iter() {
        let mut mods = mods.clone();
        let target_file = target_dir.join(format!("{}/mod.rs", module_name.replace("::", "/")));
        mods.sort();
        let mut result = mods.join("\n");

        if type_map.contains_key(module_name) {
            let (imports, codes) = type_map.get(module_name).unwrap();
            let mut imports = imports.clone();
            imports.sort();
            result.push_str("\n");
            result.push_str(&imports.join("\n"));
            result.push_str("\n");
            result.push_str(&codes.join("\n"));
        }

        write_filename(&target_file, &result).unwrap();
        created_modules.push(module_name);
    }

    for (module_name, (imports, codes)) in type_map.iter() {
        if created_modules.contains(&module_name) {
            continue;
        }
        let target_file = target_dir.join(format!("{}/mod.rs", module_name.replace("::", "/")));
        let mut imports = imports.clone();
        imports.sort();
        let mut result = imports.join("\n");
        result.push_str("\n");
        result.push_str(&codes.join("\n"));
        write_filename(&target_file, &result).unwrap();
        created_modules.push(module_name);
    }

    let target_mod = target_dir.join("mod.rs");
    let mut mods = vec![];

    for struct_name in object_database.iter().map(|x| x.key().clone()) {
        mods.push(
            format!(
                "pub mod {};\n",
                name_mapping.name_to_module_name(&struct_name)
            )
            .to_string(),
        )
    }

    mods.sort();
    let result = mods.join("\n");
    write_filename(&target_mod, &result)?;

    Ok(())
}

pub fn extract_rust_name(name: &str) -> String {
    let parts = name.split("::").collect::<Vec<&str>>();
    fix_private_name(parts[parts.len() - 1])
}

pub fn extract_rust_namespace(name: &str) -> String {
    let parts = name.split("::").collect::<Vec<&str>>();
    let mut namespace = String::new();
    for pos in 0..parts.len() - 1 {
        let part = parts[pos];
        if pos > 0 {
            namespace.push_str("::");
        }
        namespace.push_str(part);
    }
    namespace
}

fn fix_private_name(name: &str) -> String {
    if name.eq_ignore_ascii_case("type") {
        "r#type".to_string()
    } else {
        name.to_string()
    }
}

pub fn render_struct_definition(
    struct_definition: &crate::generator::types::StructDefinition,
    serializable: bool,
    config: &Config,
) -> String {
    let description = fix_rust_description(
        "",
        &struct_definition
            .description
            .as_ref()
            .map_or("", |d| d.as_str()),
    );
    let mut derivations = vec!["Debug", "Clone", "PartialEq"];
    if serializable {
        derivations.push("Serialize");
        derivations.push("Deserialize");
    }
    let has_default = struct_definition.all_properties_default();
    if has_default {
        derivations.push("Default");
    }
    let mut fields: Vec<Field> = vec![];
    for (_, property) in &struct_definition.properties {
        let mut annotations = vec![];
        let mut serde_parts = vec![];
        if serializable
            && (property.name != property.real_name || is_private_name(&property.real_name))
        {
            serde_parts.push(format!("alias = \"{}\"", property.real_name));
        }
        let field_description = fix_rust_description(
            "  ",
            &property.description.as_ref().map_or("", |d| d.as_str()),
        );

        if property.type_name.starts_with("Vec<") {
            serde_parts.push("default".to_string());
            serde_parts.push("skip_serializing_if = \"Vec::is_empty\"".to_string());
        } else if property.type_name.starts_with("Map<") {
            serde_parts.push("default".to_string());
            serde_parts.push("skip_serializing_if = \"Map::is_empty\"".to_string());
        } else if !property.required && serializable {
            if config.serde_skip_null {
                serde_parts.push("default".to_string());
                serde_parts.push("skip_serializing_if = \"Option::is_none\"".to_string());
            } else {
                serde_parts.push("default".to_string());
            }
        }
        if has_default {
            if serde_parts.contains(&"default".to_string()) {
                serde_parts.push("default".to_string());
            }
        }

        if property.required
            || property.type_name.starts_with("Vec<")
            || property.type_name.starts_with("Map<")
        {
            if !serde_parts.is_empty() {
                annotations.push(format!("#[serde({})]", serde_parts.join(", ")));
            }
            fields.push(Field {
                annotations,
                description: field_description,
                modifier: "pub".to_string(),
                name: extract_rust_name(&property.name),
                typ: property.type_name.clone(),
            });
        } else {
            if serializable {
                annotations.push(format!("#[serde({})]", serde_parts.join(", ")));
            }
            let name = extract_rust_name(&property.name);
            fields.push(Field {
                annotations,
                description: field_description,
                modifier: "pub".to_string(),
                name,
                typ: format!("Option<{}>", extract_rust_name(&property.type_name)),
            });
        }
    }
    fields.sort();
    let template = RustStructTemplate {
        name: extract_rust_name(&struct_definition.name).as_str(),
        description: description.as_str(),
        derivations,
        fields,
        imports: struct_definition
            .get_required_modules()
            .iter()
            .map(|module| module.to_use())
            .collect(),
    }
    .render()
    .unwrap();
    template
}

fn is_private_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("type") || name.starts_with("r#")
}

pub fn render_enum_definition(
    enum_definition: &crate::generator::types::EnumDefinition,
    serializable: bool,
) -> String {
    // let mut definition_str = String::new();
    let description = fix_rust_description(
        "",
        &enum_definition
            .description
            .as_ref()
            .map_or("", |d| d.as_str()),
    );
    let variants = enum_definition
        .values
        .iter()
        .map(|(_, enum_value)| {
            format!(
                "{}({})",
                extract_rust_name(&enum_value.name),
                extract_rust_name(&enum_value.value_type.name)
            )
        })
        .collect();

    let mut derivations = vec!["Debug", "Clone", "PartialEq"];
    if serializable {
        derivations.push("Serialize");
        derivations.push("Deserialize");
    }

    let template = RustEnumTemplate {
        name: extract_rust_name(&enum_definition.name).as_str(),
        description: description.as_str(),
        derivations,
        variants: variants,
        imports: enum_definition
            .get_required_modules()
            .iter()
            .map(|module| module.to_use())
            .collect(),
    }
    .render()
    .unwrap();
    template
}

pub fn modules_to_string(modules: &Vec<&ModuleInfo>) -> String {
    let mut module_import_string = String::new();
    let mut unique_modules: Vec<&ModuleInfo> = vec![];
    for module in modules {
        if unique_modules.contains(&module) {
            continue;
        }
        unique_modules.push(&module);
        module_import_string += format!("use {}::{};\n", module.path, module.name).as_str();
    }
    module_import_string
}
