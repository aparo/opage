use convert_case::Casing;
use serde::Deserialize;
use serde_aux::prelude::*;
use std::{fs::File, path::Path};

use crate::Language;

use super::{name_mapping::NameMapping, spec_ignore::SpecIgnore};

#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct ProjectMetadata {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_client_name")]
    pub client_name: String,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default)]
    pub info_email: Option<String>,
}

impl ProjectMetadata {
    pub fn new() -> Self {
        ProjectMetadata {
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Self {
        let version = if self.version.is_empty() {
            "0.1.0".to_string()
        } else {
            self.version.clone()
        };
        let client_name = if self.client_name.is_empty() {
            let mut c_name = self.name.to_case(convert_case::Case::Pascal);
            if !c_name.ends_with("Client") {
                c_name.push_str("Client");
            }
            c_name
        } else {
            self.client_name.clone()
        };
        let user_agent = if self.user_agent.is_empty() {
            format!(
                "{}/{}",
                self.client_name.to_case(convert_case::Case::Kebab),
                version
            )
        } else {
            self.user_agent.clone()
        };
        ProjectMetadata {
            name: self.name.clone(),
            version,
            client_name,
            user_agent,
            server_url: self.server_url.clone(),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RustConfig {
    #[serde(default = "bool_true")]
    pub serde_skip_null: bool,
    #[serde(default = "bool_true")]
    pub serde_skip_empty_vec: bool,
    #[serde(default = "bool_true")]
    pub serde_skip_empty_map: bool,
    #[serde(default = "bool_true")]
    pub serde_serialize: bool,
    #[serde(default = "bool_true")]
    pub serde_deserialize: bool,
    #[serde(default = "bool_true")]
    pub use_bon_builder: bool,
    #[serde(default = "bool_true")]
    pub group_parameters: bool,
    #[serde(default)]
    pub mockall: bool,
}
impl Default for RustConfig {
    fn default() -> Self {
        RustConfig {
            serde_skip_null: true,
            serde_skip_empty_vec: true,
            serde_skip_empty_map: true,
            serde_serialize: true,
            serde_deserialize: true,
            use_bon_builder: true,
            group_parameters: true,
            mockall: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "bool_true")]
    pub basic: bool,
    #[serde(default = "bool_true")]
    pub basic_bearer: bool,
    #[serde(default = "bool_true")]
    pub oauth: bool,
    #[serde(default = "bool_true")]
    pub api_key: bool,
    #[serde(default)]
    pub api_key_in_query: bool,
    #[serde(default = "bool_true")]
    pub api_key_in_header: bool,
    #[serde(default)]
    pub with_aws_v4_signature: bool,
}
impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            basic: true,
            basic_bearer: true,
            oauth: true,
            api_key: true,
            api_key_in_query: false,
            api_key_in_header: true,
            with_aws_v4_signature: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub project_metadata: ProjectMetadata,
    pub name_mapping: NameMapping,
    pub ignore: SpecIgnore,
    #[serde(default)]
    pub rust: RustConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default = "default_language")]
    pub language: Language,
}

pub fn default_client_name() -> String {
    "Client".to_string()
}

pub fn default_server_url() -> String {
    "http://localhost:8080".to_string()
}

pub fn default_language() -> Language {
    Language::Rust
}

impl Default for Config {
    fn default() -> Self {
        Config {
            project_metadata: ProjectMetadata::new(),
            name_mapping: NameMapping::new(),
            ignore: SpecIgnore::new(),
            rust: RustConfig::default(),
            auth: AuthConfig::default(),
            language: default_language(),
        }
    }
}

impl Config {
    pub fn from(config_file_path: &Path) -> Result<Self, String> {
        let file = match File::open(config_file_path) {
            Ok(file) => file,
            Err(err) => return Err(err.to_string()),
        };
        match serde_json::from_reader(file) {
            Ok(config_object) => Ok(config_object),
            Err(err) => return Err(err.to_string()),
        }
    }

    pub fn new() -> Self {
        Config::default()
    }

    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    pub fn validate(&mut self) {
        self.project_metadata = self.project_metadata.validate();
    }
}
