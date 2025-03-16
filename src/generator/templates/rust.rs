use crate::utils::config::Config;
use crate::utils::file::write_filename;
use crate::GeneratorError;
use askama::Template;
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
