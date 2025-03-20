use std::{
    collections::HashMap,
    fs::{self},
    path::PathBuf,
};

use crate::generator::types::ObjectDatabase;
use crate::{utils::config::Config, GeneratorError};
use oas3::Spec;
use object_definition::{generate_object, get_components_base_path, get_object_name};
use tracing::{error, info, trace};

pub mod object_definition;
pub mod type_definition;

pub fn generate_components(
    spec: &Spec,
    config: &Config,
    object_database: &ObjectDatabase,
) -> Result<(), GeneratorError> {
    let components = match spec.components {
        Some(ref components) => components,
        None => return Ok(()),
    };

    for (component_name, object_ref) in &components.schemas {
        // fix for broken names
        let component_name = component_name
            .replace("._common___", ".")
            .replace("._common___", ".");
        if config.ignore.component_ignored(&component_name) {
            info!("\"{}\" ignored", component_name);
            continue;
        }

        info!("Generating component \"{}\"", component_name);

        let resolved_object = match object_ref.resolve(spec) {
            Ok(object) => object,
            Err(err) => {
                error!(
                    "Unable to parse component {} {}",
                    component_name,
                    err.to_string()
                );
                continue;
            }
        };

        let component_name =
            validate_component_name(&component_name, config.name_mapping.use_scope);
        let definition_path = get_components_base_path();
        let object_name = match resolved_object.title {
            Some(ref title) => config
                .name_mapping
                .name_to_struct_name(&definition_path, &title),
            None => config
                .name_mapping
                .name_to_struct_name(&definition_path, &component_name),
        };

        if object_database.contains_key(&object_name) {
            info!(
                "Component \"{}\" already found in database and will be skipped",
                object_name
            );
            continue;
        }

        let object_definition = match generate_object(
            spec,
            &object_database,
            definition_path,
            &object_name,
            &resolved_object,
            &config.name_mapping,
            config,
        ) {
            Ok(object_definition) => object_definition,
            Err(err) => {
                error!("{} {}\n", component_name, err);
                continue;
            }
        };

        // if let ObjectDefinition::Primitive(type_definition) = object_definition {
        //     trace!(
        //         "Primitive object {} will not be added to database",
        //         type_definition.name
        //     );
        //     continue;
        // }

        let object_name = get_object_name(&object_definition);

        match object_database.contains_key(&object_name) {
            true => {
                error!("ObjectDatabase already contains an object {}", object_name);
                continue;
            }
            _ => {
                trace!("Adding component/struct {} to database", object_name);
                object_database.insert(object_name.clone(), object_definition);
            }
        }
    }

    Ok(())
}

fn validate_component_name(component_name: &str, use_scope: bool) -> String {
    let mut result = component_name.replace("___", ".").replace(".", "::");
    if result.starts_with("_") {
        result = result.trim_start_matches("_").to_owned();
        return result;
    }
    if !result.contains("::") {
        if use_scope {
            result = format!("common::{}", result);
        } else {
            result = format!("models::{}", result);
        }
    }
    result
}
