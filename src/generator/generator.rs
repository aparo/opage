use std::path::PathBuf;

use crate::Language;
use oas3::{spec::Operation, Spec};
use tracing::{error, info};

use crate::{
    generator::{
        path::{default_request, websocket_request},
        types::{Method, ObjectDatabase, PathDatabase},
    },
    utils::config::Config,
    GeneratorError,
};

use super::{component::generate_components, templates::rust};

pub struct Generator {
    config: Config,
    output_dir: PathBuf,
    specs: Vec<PathBuf>,
    object_database: ObjectDatabase,
    path_database: PathDatabase,
}

impl Generator {
    pub fn new(config: Config, output_dir: PathBuf, specs: Vec<PathBuf>) -> Self {
        Self {
            config,
            output_dir,
            specs,
            object_database: ObjectDatabase::new(),
            path_database: PathDatabase::new(),
        }
    }

    pub fn generate_paths(&self) -> Result<u32, GeneratorError> {
        let mut generated_paths = 0;
        for spec_file_path in self.specs.iter() {
            let spec = oas3::from_path(spec_file_path).expect("Failed to read spec");
            // Components and database for type referencing
            generate_components(&spec, &self.config, &self.object_database).unwrap();
            // Generate paths requests
            generated_paths += self
                .generate_inner_paths(&spec)
                .expect("Failed to generated paths");
        }
        Ok(generated_paths)
    }

    pub fn generate_inner_paths(&self, spec: &Spec) -> Result<u32, GeneratorError> {
        let mut generated_path_count = 0;

        let paths = match spec.paths {
            Some(ref paths) => paths,
            None => return Ok(generated_path_count),
        };

        for (name, path_item) in paths {
            if self.config.ignore.path_ignored(&name) {
                info!("{} ignored", name);
                continue;
            }

            info!("{}", name);

            let mut operations = vec![];
            if let Some(ref operation) = path_item.get {
                operations.push((Method::GET, operation));
            }
            if let Some(ref operation) = path_item.post {
                operations.push((Method::POST, operation));
            }
            if let Some(ref operation) = path_item.delete {
                operations.push((Method::DELETE, operation));
            }
            if let Some(ref operation) = path_item.put {
                operations.push((Method::PUT, operation));
            }
            if let Some(ref operation) = path_item.patch {
                operations.push((Method::PATCH, operation));
            }
            if let Some(ref operation) = path_item.options {
                operations.push((Method::OPTIONS, operation));
            }
            if let Some(ref operation) = path_item.trace {
                operations.push((Method::TRACE, operation));
            }
            if let Some(ref operation) = path_item.head {
                operations.push((Method::HEAD, operation));
            }

            for operation in operations {
                match self.generate_path_code(spec, operation.0, &name, operation.1) {
                    Ok(_) => (),
                    Err(err) => {
                        error!("{}", err);
                    }
                }
                generated_path_count += 1;
            }
        }

        Ok(generated_path_count)
    }

    fn generate_path_code(
        &self,
        spec: &Spec,
        method: Method,
        path: &str,
        operation: &Operation,
    ) -> Result<String, GeneratorError> {
        let operation_id = match operation.operation_id {
            Some(ref operation_id) => &self.config.name_mapping.name_to_module_name(operation_id),
            None => {
                return Err(GeneratorError::MissingIdError(
                    path.to_string(),
                    method.to_string(),
                ));
            }
        };

        let generate_websocket = match operation.extensions.get("serverstream") {
            Some(extension_value) => match extension_value {
                serde_json::Value::Bool(generate_websocket) => generate_websocket,
                _ => {
                    return Err(GeneratorError::InvalidValueError(
                        "x-serverstream".to_owned(),
                    ))
                }
            },
            None => &false,
        };

        match generate_websocket {
            true => match websocket_request::generate_operation(
                spec,
                &self.config.name_mapping,
                &path,
                &operation,
                &self.object_database,
                &self.path_database,
                &self.config,
            ) {
                Ok(request_code) => request_code,
                Err(err) => {
                    return Err(GeneratorError::CodeGenerationError(
                        "websocket".to_owned(),
                        err.to_string(),
                    ))
                }
            },
            _ => match default_request::generate_operation(
                spec,
                &self.config.name_mapping,
                method,
                &path,
                &operation,
                &self.object_database,
                &self.path_database,
                &self.config,
            ) {
                Ok(request_code) => request_code,
                Err(err) => return Err(err),
            },
        };

        // let mut full_path = output_path.join("src");
        // let parts = operation_id.split('.').collect::<Vec<&str>>();
        // for pos in 0..parts.len() {
        //     let part = parts[pos];
        //     if pos == parts.len() - 1 {
        //         full_path = full_path.join(format!("{}.rs", part));
        //         break;
        //     }

        //     full_path = full_path.join(part);
        // }

        // println!(
        //     "Writing to {} \n{}",
        //     full_path.to_str().unwrap(),
        //     &request_code
        // );
        // write_filename(&full_path, &request_code)?;

        Ok(operation_id.clone())
    }

    pub fn generate_objects(&self) -> Result<(), GeneratorError> {
        // Write all registered objects to individual type definitions
        match self.config.language {
            Language::Rust => {
                rust::write_object_database(&self.output_dir, &self.object_database, &self.config)
            }
            _ => Err(GeneratorError::UnsupportedLanguageError(
                self.config.language.to_string(),
            )),
        }
    }

    pub fn generate_clients(&self) -> Result<(), GeneratorError> {
        match self.config.language {
            Language::Rust => rust::generate_clients(
                &self.output_dir,
                &self.path_database,
                &self.config,
                &self.object_database,
            ),
            _ => Err(GeneratorError::UnsupportedLanguageError(
                self.config.language.to_string(),
            )),
        }
    }

    pub fn populate_client_files(&self) -> Result<(), GeneratorError> {
        match self.config.language {
            Language::Rust => rust::populate_client_files(&self.output_dir, &self.config),
            _ => Err(GeneratorError::UnsupportedLanguageError(
                self.config.language.to_string(),
            )),
        }
    }
}
