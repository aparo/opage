use std::{
    collections::HashMap,
    fs::{self},
    path::PathBuf,
};

use crate::generator::types::{ObjectDatabase, ObjectDefinition};
use crate::{
    utils::file::write_filename,
    utils::{
        config::Config,
        name_mapping::{extract_rust_name, extract_rust_namespace, fix_rust_description},
    },
    GeneratorError,
};
use oas3::Spec;
use object_definition::{
    generate_object, get_components_base_path, get_object_name, modules_to_string,
};
use tracing::{error, info, trace};

use super::templates::rust::RustTypeTemplate;
use askama::Template;

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

    fs::create_dir_all(&target_dir).expect("Creating objects dir failed");

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
                result.push_str(&struct_definition.to_string(true, config));
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
                result.push_str(&enum_definition.to_string(true));
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
