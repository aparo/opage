use crate::generator::component::object_definition::get_object_name;
use crate::generator::types::{
    ModuleInfo, ObjectDatabase, ObjectDefinition, PathDatabase, PathDefinition, PropertyDefinition,
    TransferMediaType, TypeDefinition,
};
use crate::utils::config::Config;
use crate::utils::file::write_filename;
use crate::utils::name_mapping::convert_name;
use crate::utils::string::capitalize;
use crate::GeneratorError;
use askama::Template;
use convert_case::Casing;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// list of primitive types of Rust language
pub const RUST_PRIMITIVE_TYPES: [&str; 13] = [
    "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "String",
];

#[derive(Template)]
#[template(path = "rust/partial_header.j2", escape = "none")]
pub struct PartialHeaderTemplateContext {
    pub app_name: String,
    pub app_description: Option<String>,
    pub version: String,
    pub info_email: Option<String>,
}

#[derive(Template)]
#[template(path = "rust/configuration.j2", escape = "none")]
pub struct ConfigurationTemplateContext {
    pub base_url: String,
    pub user_agent: String,
    pub support_middleware: bool,
    pub support_token_source: bool,
    pub with_aws_v4_signature: bool,
}

#[derive(Clone, Debug, Default)]
struct VendorExtensions {
    pub x_group_parameters: bool,
}

#[derive(Clone, Debug)]
struct Response {
    pub code: String,
    pub data_type: String,
    pub is_default: bool,
    pub is2xx: bool,
    pub is3xx: bool,
    pub is4xx: bool,
    pub is5xx: bool,
}

#[derive(Clone, Debug)]
struct AuthMethod {
    pub is_api_key: bool,
    pub is_key_in_query: bool,
    pub is_key_in_header: bool,
    pub key_param_name: String,
    pub is_oauth: bool,
    pub support_token_source: bool,
    pub is_basic: bool,
    pub is_basic_basic: bool,
    pub is_basic_bearer: bool,
}
#[derive(Clone, Debug)]
struct Operation {
    pub operation_id: String,
    pub operation_id_camel_case: String,
    pub path: String,
    pub http_method: String,
    pub description: String,
    pub notes: Option<String>,
    pub method: String,
    pub support_multiple_responses: bool,
    pub all_parameters: Vec<Field>,
    pub path_parameters: Vec<Field>,
    pub query_parameters: Vec<Field>,
    pub body_parameters: Vec<Field>,
    pub header_parameters: Vec<Field>,
    pub form_parameters: Vec<Field>,
    pub is_multipart: bool,
    pub response_type: TypeDefinition,
    pub return_type: String,
    pub vendor_extensions: VendorExtensions,
    pub responses: Vec<Response>,
    pub auth_methods: Vec<AuthMethod>,
    // configuration
    pub use_bon_builder: bool,
    pub group_parameters: bool,
    pub with_aws_v4_signature: bool,
}

impl Operation {
    pub fn has_auth_methods(&self) -> bool {
        self.auth_methods.len() > 0
    }

    pub fn has_path_parameters(&self) -> bool {
        self.path_parameters.len() > 0
    }
    pub fn has_query_parameters(&self) -> bool {
        self.query_parameters.len() > 0
    }
    pub fn has_body_parameters(&self) -> bool {
        self.body_parameters.len() > 0
    }
    pub fn has_header_parameters(&self) -> bool {
        self.header_parameters.len() > 0
    }
    pub fn has_form_parameters(&self) -> bool {
        self.form_parameters.len() > 0
    }
    pub fn has_response(&self) -> bool {
        self.responses.len() > 0
    }
}

#[derive(Template)]
#[template(path = "rust/api.j2", escape = "none")]
pub struct ApiTemplateContext {
    pub classname: String,
    pub mockall: bool,
    pub operations: Vec<Operation>,
    pub support_multiple_responses: bool,
    pub with_aws_v4_signature: bool,
}

#[derive(Template)]
#[template(path = "rust/gitignore.j2", escape = "none")]
pub struct RustGitIgnoreTemplate {}

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
    pub base_name: String,
    pub name: String,
    pub data_type: String,
    pub required: bool,
    pub is_nullable: bool,
    pub is_array: bool,
    pub is_file: bool,
}

impl Ord for Field {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Field {
    pub fn collection_format(&self) -> String {
        if self.is_array {
            "multi".to_owned()
        } else {
            "single".to_owned()
        }
    }

    pub fn is_deep_object(&self) -> bool {
        self.data_type == "serde_json::Value"
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

#[derive(Template, Default)]
#[template(path = "rust/Cargo.j2", escape = "none")]
pub struct CargoTemplate<'a> {
    pub package_name: &'a str,
    pub package_version: Option<&'a str>,
    pub lambda_version: bool,
    pub info_email: Option<String>,
    pub app_description: Option<String>,
    pub license_info: Option<String>,
    pub publish_rust_registry: Option<String>,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub home_page_url: Option<String>,
    pub serde_with: bool,
    pub has_uuids: bool,
    pub hyper: bool,
    pub hyper0x: bool,
    pub with_aws_v4_signature: bool,
    pub reqwest: bool,
    pub support_async: bool,
    pub support_middleware: bool,
    pub support_token_source: bool,
    pub reqwest_trait: bool,
    pub mockall: bool,
    pub use_bon_builder: bool,
}
// impl<'a> Default in CargoTemplate<'a> {
//     fn default() -> Self {
//         Self {
//             package_name: "my_project",
//             package_version: Some("0.1.0".to_string()),
//             lambda_version: false,
//             info_email: None,
//             app_description: None,
//             license_info: None,
//             publish_rust_registry: None,
//             repository_url: None,
//             documentation_url: None,
//             home_page_url: None,
//             serde_with: false,
//             has_uuids: false,
//             hyper: false,
//             hyper0x: false,
//             with_aws_v4_signature: false,
//             reqwest: true,
//             support_async: true,
//             support_middleware: true,
//             support_token_source: false,
//             reqwest_trait: true,
//             mockall: false,
//             use_bon_builder: true,
//         }
//     }
// }

pub fn populate_client_files(output_dir: &PathBuf, config: &Config) -> Result<(), GeneratorError> {
    let header = &render_partial_header(config);
    // producing Cargo.toml
    let cargo_target_file = output_dir.join("Cargo.toml");

    let template = CargoTemplate {
        package_name: config.project_metadata.name.as_str(),
        package_version: Some(&config.project_metadata.version),
        support_async: true,
        support_middleware: true,
        reqwest_trait: true,
        use_bon_builder: true,
        ..Default::default()
    }
    .render()
    .unwrap();

    write_filename(&cargo_target_file, &template)?;

    // producing .gitignore
    let git_ignore_file = output_dir.join(".gitignore");
    let template = RustGitIgnoreTemplate {}.render().unwrap();
    write_filename(&git_ignore_file, &template)?;

    // producing src/api/configuration.rs
    let configuration_file = output_dir.join("src").join("api").join("configuration.rs");
    let mut configuration_content = String::new();
    configuration_content.push_str(header);
    let configuration_template = ConfigurationTemplateContext {
        base_url: config.project_metadata.server_url.clone(),
        user_agent: config.project_metadata.user_agent.clone(),
        support_middleware: true,
        support_token_source: false,
        with_aws_v4_signature: false,
    };
    configuration_content.push_str(&configuration_template.render().unwrap());
    write_filename(&configuration_file, &configuration_content)?;

    // producing other files
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

pub fn render_partial_header(config: &Config) -> String {
    let context = PartialHeaderTemplateContext {
        app_name: config.project_metadata.name.clone(),
        app_description: config.project_metadata.description.clone(),
        version: config.project_metadata.version.clone(),
        info_email: config.project_metadata.info_email.clone(),
    };
    context.render().unwrap()
}

pub fn generate_rust_client_code(
    paths: Vec<crate::generator::types::PathDefinition>,
    config: &Config,
    _object_database: &ObjectDatabase,
) -> (String, Vec<BuilderInfo>) {
    let mut imports = HashSet::new();

    let mut client_code = String::new();
    let mut function_code = String::new();

    let mut builders: Vec<BuilderInfo> = vec![];

    for path in paths.iter() {
        let required_properties = path.get_required_properties();
        let response_type = extract_default_rust_response_type(path.extract_response_type());
        let _scope: Vec<String> = vec![];
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
            base_name: "client".to_string(),
            data_type: config.project_metadata.client_name.clone(),
            required: true,
            is_nullable: false,
            is_array: false,
            is_file: false,
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
                    base_name: property.real_name.clone(),
                    data_type: fix_type_name_property(&property.type_name),
                    required: property.required,
                    is_nullable: !property.required,
                    is_array: property.is_array(),
                    is_file: property.is_file(),
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
        base_name: property.real_name.clone(),
        data_type: fix_type_name_property(&property.type_name),
        required: property.required,
        is_nullable: !property.required,
        is_array: property.is_array(),
        is_file: property.is_file(),
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

pub fn build_responses(path: &PathDefinition) -> Vec<Response> {
    path.response_entities
        .iter()
        .map(|(status_code, entity)| {
            let code_number = status_code.parse::<u16>().unwrap_or(0);
            let data_type: String = if entity.content.len() > 0 {
                let first_key = entity.content.keys().next().unwrap().to_string();
                match entity.content.get(&first_key).unwrap() {
                    TransferMediaType::ApplicationJson(object) => match object {
                        Some(typ) => {
                            if typ.name.starts_with("crate::") {
                                typ.name.clone()
                            } else {
                                format!("crate::{}", typ.name)
                            }
                        }
                        _ => "serde_json::Value".to_string(),
                    },
                    _ => "String".to_string(),
                }
            } else {
                "serde_json::Value".to_string()
            };
            Response {
                code: status_code.to_string(),
                data_type,
                is_default: code_number >= 200 && code_number < 300,
                is2xx: code_number >= 200 && code_number < 300,
                is3xx: code_number >= 300 && code_number < 400,
                is4xx: code_number >= 400 && code_number < 500,
                is5xx: code_number >= 500 && code_number < 600,
            }
        })
        .collect()
}

pub fn generate_clients(
    output_dir: &PathBuf,
    path_database: &PathDatabase,
    config: &Config,
    _object_database: &ObjectDatabase,
) -> Result<(), GeneratorError> {
    // Write all registered API calls in a client
    let target_dir = output_dir.join("src");
    for item in path_database.iter() {
        println!("Path: {}", item.key());
    }
    let header = &render_partial_header(config);

    let chunks = path_database.iter().chunk_by(|f| f.value().package.clone());

    let mut grouped_paths: Vec<_> = chunks.into_iter().collect();

    grouped_paths.sort_by(|a, b| a.0.cmp(&b.0));
    // let group_number = grouped_paths.len();

    for (namespace, group) in grouped_paths {
        let mut namespace = namespace;
        if namespace.is_empty() {
            namespace = "api".to_owned();
        }
        tracing::debug!("Processing Namespace: {}", namespace);
        let items = group
            .map(|f| (f.key().clone(), f.clone()))
            .collect::<Vec<_>>();
        let mut operations: Vec<Operation> = vec![];
        for (id, path) in items {
            // we populate an operation from a PathDefinition
            let _required_properties = path.get_required_properties();
            let _response_type = extract_default_rust_response_type(path.extract_response_type());
            let _scope: Vec<String> = vec![];
            let path_parameters: Vec<Field> = path
                .path_parameters
                .parameters_struct
                .properties
                .iter()
                .map(|f| property_definition_to_field(f.1))
                .collect();
            let query_parameters: Vec<Field> = path
                .query_parameters
                .query_struct
                .properties
                .iter()
                .map(|f| property_definition_to_field(f.1))
                .collect();
            let body_parameters: Vec<Field> = path
                .extract_body_properties()
                .iter()
                .map(|f| property_definition_to_field(&f.1))
                .collect();
            let mut all_parameters = vec![];
            all_parameters.extend(path_parameters.clone());
            all_parameters.extend(query_parameters.clone());
            all_parameters.extend(body_parameters.clone());

            let operation = Operation {
                operation_id: id.to_owned(),
                operation_id_camel_case: capitalize(&id.to_case(convert_case::Case::Camel)),
                path: path.url.clone(),
                http_method: path.method.to_string(),
                description: path.description.clone(),
                notes: None,               //TODO: propagate notes
                auth_methods: vec![],      //TODO: propagate notes
                header_parameters: vec![], //TODO: propagate headers that are missing in PathDefinition
                form_parameters: vec![], //TODO: propagate forms that are missing in PathDefinition
                is_multipart: false,     //TODO: propagate forms that are missing in PathDefinition
                method: path.method.to_string(),
                support_multiple_responses: path.response_entities.len() > 0,
                all_parameters,
                path_parameters,
                query_parameters,
                body_parameters,
                response_type: path.get_request_type().unwrap(),
                return_type: path.response_name.clone(),
                vendor_extensions: VendorExtensions::default(),
                responses: build_responses(&path),
                use_bon_builder: config.rust.use_bon_builder,
                group_parameters: config.rust.group_parameters,
                with_aws_v4_signature: config.auth.with_aws_v4_signature,
            };
            operations.push(operation);
        }

        let mut final_client_code = String::new();
        // we add headers
        final_client_code.push_str(header);
        final_client_code.push_str("\n");

        // we add the client code
        let api_template = ApiTemplateContext {
            classname: config.project_metadata.client_name.clone(),
            mockall: config.rust.mockall,
            operations,
            support_multiple_responses: false,
            with_aws_v4_signature: config.auth.with_aws_v4_signature,
        };
        let mut path = namespace.replace(".", "/").replace("::", "/");
        if path.is_empty() {
            path = "lib".to_owned();
        }

        final_client_code.push_str(&api_template.render().unwrap());
        final_client_code.push_str("\n");

        let full_path = target_dir.join(format!("{}.rs", path));
        println!(
            "Writing to {} \n{}",
            full_path.to_str().unwrap(),
            &final_client_code
        );
        write_filename(&full_path, &final_client_code)?;
    }

    Ok(())
}

// old implementation that generates all the single files
pub fn generate_clients_old(
    output_dir: &PathBuf,
    path_database: &PathDatabase,
    config: &Config,
    object_database: &ObjectDatabase,
) -> Result<(), GeneratorError> {
    // Write all registered API calls in a client
    let target_dir = output_dir.join("src");
    for item in path_database.iter() {
        println!("Path: {}", item.key());
    }

    let chunks = path_database.iter().chunk_by(|f| f.value().package.clone());

    let mut grouped_paths: Vec<_> = chunks.into_iter().collect();

    grouped_paths.sort_by(|a, b| a.0.cmp(&b.0));
    // let group_number = grouped_paths.len();

    for (namespace, group) in grouped_paths {
        let mut namespace = namespace;
        if namespace.is_empty() {
            namespace = "api".to_owned();
        }
        tracing::debug!("Processing Namespace: {}", namespace);
        let items = group.map(|f| f.clone()).collect::<Vec<_>>();
        let (client_code, builders) = generate_rust_client_code(items, config, object_database);
        let mut path = namespace.replace(".", "/").replace("::", "/");
        if path.is_empty() {
            path = "lib".to_owned();
        }
        let mut final_client_code = String::new();
        // we add the client_init_code
        let client_init_template = RustClientInitTemplate {
            name: config.project_metadata.name.as_str(),
            client_name: config.project_metadata.client_name.as_str(),
            server_url: config.project_metadata.server_url.as_str(),
            user_agent: config.project_metadata.user_agent.as_str(),
            version: config.project_metadata.version.as_str(),
        };
        final_client_code.push_str(&client_init_template.render().unwrap());
        final_client_code.push_str("\n");
        final_client_code.push_str(&client_code);
        final_client_code.push_str("}\n");

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

// extract scoped name from the full name
fn extract_base_name(name: &str) -> String {
    let parts = name.split("::").collect::<Vec<&str>>();
    parts.iter().take(parts.len() - 1).join("::")
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
        output_dir.join("src")
    };

    // for item in object_database.iter() {
    //     println!("Object: {}", item.key());
    // }

    std::fs::create_dir_all(&target_dir).expect("Creating objects dir failed");

    let chunks = object_database
        .iter()
        .chunk_by(|f| extract_base_name(&f.key()));

    let mut grouped_objects: Vec<_> = chunks.into_iter().collect();

    grouped_objects.sort_by(|a, b| a.0.cmp(&b.0));

    for (namespace, group) in grouped_objects {
        let mut type_map: HashMap<String, (Vec<String>, Vec<String>)> =
            std::collections::HashMap::new();
        let mut mods_map: HashMap<String, Vec<String>> = HashMap::new();

        let mut items = group.map(|f| f.clone()).collect::<Vec<_>>();
        items.sort_by(|a, b| a.name().cmp(&b.name()));

        let target_file = target_dir.join(format!(
            "{}.rs",
            namespace.replace(".", "/").replace("::", "/")
        ));
        let mut struct_codes = String::new();
        let mut all_imports = HashSet::new();
        for object_definition in items.iter() {
            let object_name = get_object_name(object_definition);

            let module_name = name_mapping.name_to_module_name(&object_name);

            let namespace = extract_rust_namespace(&module_name);

            match object_definition {
                ObjectDefinition::Struct(struct_definition) => {
                    for module in struct_definition.get_required_modules() {
                        all_imports.insert(module.to_use());
                    }

                    let mut result = String::new();
                    result.push_str("\n");
                    result.push_str(&struct_definition.to_string(true, config)?);
                    struct_codes.push_str(&result);
                    // write_filename(&target_file, &result).unwrap();
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
                    for module in enum_definition.get_required_modules() {
                        all_imports.insert(module.to_use());
                    }

                    let mut result = String::new();
                    result.push_str("\n");
                    result.push_str(&enum_definition.to_string(true, config)?);
                    struct_codes.push_str(&result);
                    // write_filename(&target_file, &result).unwrap();
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
                        value: extract_rust_name(&primitive_definition.primitive_type.name)
                            .as_str(),
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

        let mut types = String::new();
        for (module_name, (imports, codes)) in type_map.iter() {
            if created_modules.contains(&module_name) {
                continue;
            }
            for import in imports {
                all_imports.insert(import.clone());
            }
            // let target_file = target_dir.join(format!("{}/mod.rs", module_name.replace("::", "/")));
            types.push_str(&codes.join("\n"));
            created_modules.push(module_name);
        }
        let mut imports = all_imports.iter().cloned().collect::<Vec<String>>();
        imports.sort();
        let mut result = imports.join("\n");
        result.push_str("\n");
        result.push_str(&types);
        result.push_str(&struct_codes);
        write_filename(&target_file, &result).unwrap();
        // println!("Writing to {} \n{}", target_file.to_str().unwrap(), &result);
    }

    // let target_mod = target_dir.join("mod.rs");
    // let mut mods = vec![];

    // for struct_name in object_database.iter().map(|x| x.key().clone()) {
    //     mods.push(
    //         format!(
    //             "pub mod {};\n",
    //             name_mapping.name_to_module_name(&struct_name)
    //         )
    //         .to_string(),
    //     )
    // }

    // mods.sort();
    // let result = mods.join("\n");
    // write_filename(&target_mod, &result)?;

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
        let mut serde_parts = HashSet::new();
        if serializable
            && (property.name != property.real_name || is_private_name(&property.real_name))
        {
            serde_parts.insert(format!("alias = \"{}\"", property.real_name));
        }
        let field_description = fix_rust_description(
            "  ",
            &property.description.as_ref().map_or("", |d| d.as_str()),
        );

        if property.type_name.starts_with("Vec<") {
            serde_parts.insert("default".to_string());
            serde_parts.insert("skip_serializing_if = \"Vec::is_empty\"".to_string());
        } else if property.type_name.starts_with("Map<") {
            serde_parts.insert("default".to_string());
            serde_parts.insert("skip_serializing_if = \"Map::is_empty\"".to_string());
        } else if !property.required && serializable {
            if config.rust.serde_skip_null {
                serde_parts.insert("default".to_string());
                serde_parts.insert("skip_serializing_if = \"Option::is_none\"".to_string());
            } else {
                serde_parts.insert("default".to_string());
            }
        }
        if has_default {
            if serde_parts.contains(&"default".to_string()) {
                serde_parts.insert("default".to_string());
            }
        }

        if property.required
            || property.type_name.starts_with("Vec<")
            || property.type_name.starts_with("Map<")
        {
            if !serde_parts.is_empty() {
                let mut serds: Vec<String> = serde_parts.iter().cloned().collect();
                serds.sort();
                annotations.push(format!("#[serde({})]", serds.join(", ")));
            }
            fields.push(Field {
                annotations,
                description: field_description,
                modifier: "pub".to_string(),
                name: extract_rust_name(&property.name),
                base_name: property.real_name.clone(),
                data_type: property.type_name.clone(),
                required: property.required,
                is_nullable: !property.required,
                is_array: true,
                is_file: property.is_file(),
            });
        } else {
            if serializable {
                let mut serds: Vec<String> = serde_parts.iter().cloned().collect();
                serds.sort();
                annotations.push(format!("#[serde({})]", serds.join(", ")));
            }
            let name = extract_rust_name(&property.name);
            fields.push(Field {
                annotations,
                description: field_description,
                modifier: "pub".to_string(),
                name,
                base_name: property.real_name.clone(),
                data_type: format!("Option<{}>", extract_rust_name(&property.type_name)),
                required: property.required,
                is_nullable: !property.required,
                is_array: false,
                is_file: property.is_file(),
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
