use serde::Deserialize;
use serde_aux::prelude::*;
use std::{fs::File, path::Path};

use super::{name_mapping::NameMapping, spec_ignore::SpecIgnore};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    pub version: String,
}

impl ProjectMetadata {
    pub fn new() -> Self {
        ProjectMetadata {
            name: String::new(),
            version: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Config {
    pub project_metadata: ProjectMetadata,
    pub name_mapping: NameMapping,
    pub ignore: SpecIgnore,
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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            project_metadata: ProjectMetadata::new(),
            name_mapping: NameMapping::new(),
            ignore: SpecIgnore::new(),
            serde_skip_empty_map: true,
            serde_skip_empty_vec: true,
            serde_skip_null: true,
            serde_serialize: true,
            serde_deserialize: true,
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
}
