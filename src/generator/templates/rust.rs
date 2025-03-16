use crate::generator::types::{ModuleInfo, PropertyDefinition};
use crate::utils::config::Config;
use crate::utils::file::write_filename;
use crate::utils::name_mapping::convert_name;
use crate::GeneratorError;
use askama::Template;
use clap::builder::Str;
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
#[template(path = "rust/cargo.j2", escape = "none")]
pub struct CargoTemplate<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

pub fn populate_client_files(output_dir: &PathBuf, config: &Config) -> Result<(), GeneratorError> {
    let cargo_target_file = output_dir.join("cargo.toml");

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

pub fn generate_rust_client_code(
    paths: Vec<crate::generator::types::PathDefinition>,
    config: &Config,
) -> String {
    let mut imports = HashSet::new();

    let mut client_code = String::new();
    let mut function_code = String::new();

    for path in paths.iter() {
        let required_properties = path.get_required_properties();
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
                        .unwrap_or(String::from("No description available"))
                )
                .as_str(),
            );
        }

        let function = RustClientFunctionTemplate {
            name: &path.name,
            description,
            required_properties,
            builder_name,
        };
        // let operation_id = operation.operation_id.clone();
        // let operation_code = operation.generate_rust_code(config);
        // client_code.push_str(&operation_code);
        // client_code.push_str("\n");

        // let module_info = ModuleInfo {
        //     module_name: operation_id,
        //     module_path: format!("{}.rs", operation_id),
        // };
        function_code.push_str(&function.render().unwrap());
        for import in path.used_modules.iter() {
            imports.insert(import.clone());
        }
    }
    client_code.push_str(&function_code);
    client_code
}
