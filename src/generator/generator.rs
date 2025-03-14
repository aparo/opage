use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use oas3::{spec::Operation, Spec};
use tracing::{error, info};

use crate::{
    generator::path::{default_request, websocket_request},
    utils::config::Config,
};

use super::{
    component::{
        generate_components, object_definition::types::ObjectDatabase, write_object_database,
    },
    templates::rust::populate_client_files,
};

pub struct Generator {
    config: Config,
    output_dir: PathBuf,
    specs: Vec<PathBuf>,
    object_database: ObjectDatabase,
}

impl Generator {
    pub fn new(config: Config, output_dir: PathBuf, specs: Vec<PathBuf>) -> Self {
        Self {
            config,
            output_dir,
            specs,
            object_database: ObjectDatabase::new(),
        }
    }

    pub fn generate_paths(&self) -> u32 {
        let mut generated_paths = 0;
        for spec_file_path in self.specs.iter() {
            let spec = oas3::from_path(spec_file_path).expect("Failed to read spec");
            // 3. Generate Code
            // 3.1 Components and database for type referencing
            generate_components(&spec, &self.config, &self.object_database).unwrap();
            // object_database = odb;
            // 3.2 Generate paths requests
            generated_paths += self
                .generate_inner_paths(&spec)
                .expect("Failed to generated paths");
        }
        generated_paths
        // generate_paths(&self.config, &self.output_dir, &self.specs);
    }

    pub fn generate_inner_paths(&self, spec: &Spec) -> Result<u32, String> {
        let mut generated_path_count = 0;

        let paths = match spec.paths {
            Some(ref paths) => paths,
            None => return Ok(generated_path_count),
        };
        let target_dir = self.output_dir.join("src");

        fs::create_dir_all(&target_dir).expect("Creating objects dir failed");

        let mut mods_to_create = vec![];

        for (name, path_item) in paths {
            if self.config.ignore.path_ignored(&name) {
                info!("{} ignored", name);
                continue;
            }

            info!("{}", name);

            let mut operations = vec![];
            if let Some(ref operation) = path_item.get {
                operations.push((reqwest::Method::GET, operation));
            }
            if let Some(ref operation) = path_item.post {
                operations.push((reqwest::Method::POST, operation));
            }
            if let Some(ref operation) = path_item.delete {
                operations.push((reqwest::Method::DELETE, operation));
            }
            if let Some(ref operation) = path_item.put {
                operations.push((reqwest::Method::PUT, operation));
            }
            if let Some(ref operation) = path_item.patch {
                operations.push((reqwest::Method::PATCH, operation));
            }

            for operation in operations {
                match self.write_operation_to_file(
                    spec,
                    &operation.0,
                    &name,
                    operation.1,
                    &target_dir,
                ) {
                    Ok(operation_id) => {
                        mods_to_create.push(operation_id.clone());
                        ()
                    }
                    Err(err) => {
                        error!("{}", err);
                    }
                }
                generated_path_count += 1;
            }
        }

        let target_file = target_dir.join("mod.rs");

        let mut mod_file = match File::create(target_file) {
            Ok(file) => file,
            Err(err) => {
                return Err(format!("Unable to create file mod.rs {}", err.to_string()));
            }
        };

        for mod_name in mods_to_create {
            mod_file
                .write(format!("pub mod {};\n", mod_name).as_bytes())
                .expect("Failed to write to mod.rs");
        }

        Ok(generated_path_count)
    }

    fn write_operation_to_file(
        &self,
        spec: &Spec,
        method: &reqwest::Method,
        path: &str,
        operation: &Operation,
        output_path: &PathBuf,
    ) -> Result<String, String> {
        let operation_id = match operation.operation_id {
            Some(ref operation_id) => &self.config.name_mapping.name_to_module_name(operation_id),
            None => {
                return Err(format!("{} {} has no id", path, method.as_str()));
            }
        };

        let generate_websocket = match operation.extensions.get("serverstream") {
            Some(extension_value) => match extension_value {
                serde_json::Value::Bool(generate_websocket) => generate_websocket,
                _ => return Err("Invalid x-serverstream value".to_owned()),
            },
            None => &false,
        };

        let request_code = match generate_websocket {
            true => match websocket_request::generate_operation(
                spec,
                &self.config.name_mapping,
                &path,
                &operation,
                &self.object_database,
                &self.config,
            ) {
                Ok(request_code) => request_code,
                Err(err) => return Err(format!("Failed to generated websocket code {}", err)),
            },
            _ => match default_request::generate_operation(
                spec,
                &self.config.name_mapping,
                method,
                &path,
                &operation,
                &self.object_database,
                &self.config,
            ) {
                Ok(request_code) => request_code,
                Err(err) => {
                    return Err(format!("Failed to generate code {}", err));
                }
            },
        };

        let mut full_path = output_path.join("src");
        let parts = operation_id.split('.').collect::<Vec<&str>>();
        for pos in 0..parts.len() {
            let part = parts[pos];
            if pos == parts.len() - 1 {
                full_path = full_path.join(format!("{}.rs", part));
                break;
            }

            full_path = full_path.join(part);
        }

        fs::create_dir_all(&full_path.parent().unwrap()).expect("Creating objects dir failed");

        let mut path_file = match File::create(&full_path) {
            Ok(file) => file,
            Err(err) => {
                return Err(format!(
                    "Unable to create file {}.rs {}",
                    operation_id,
                    err.to_string()
                ));
            }
        };

        println!(
            "Writing to {} \n{}",
            full_path.to_str().unwrap(),
            &request_code
        );
        path_file.write(request_code.as_bytes()).unwrap();
        Ok(operation_id.clone())
    }

    pub fn generate_objects(&self) {
        // 3.3 Write all registered objects to individual type definitions
        write_object_database(&self.output_dir, &self.object_database, &self.config)
            .expect("Write objects failed");
    }

    pub fn populate_client_files(&self) {
        populate_client_files(&self.output_dir, &self.config)
            .expect("Failed to populate client files");
    }
}
