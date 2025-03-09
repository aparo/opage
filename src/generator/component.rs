use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use oas3::Spec;
use object_definition::{
    generate_object, get_components_base_path, get_object_name, modules_to_string,
    types::{ObjectDatabase, ObjectDefinition},
};
use tracing::{error, info, trace};

use crate::utils::{
    config::Config,
    name_mapping::{extract_rust_name, fix_rust_description},
};

use super::templates::rust::RustTypeTemplate;
use askama::Template;

pub mod object_definition;
pub mod type_definition;

pub fn generate_components(
    spec: &Spec,
    config: &Config,
    mut object_database: ObjectDatabase,
) -> Result<ObjectDatabase, String> {
    let components = match spec.components {
        Some(ref components) => components,
        None => return Ok(object_database),
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

        let component_name = validate_component_name(&component_name);
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
            &mut object_database,
            definition_path,
            &object_name,
            &resolved_object,
            &config.name_mapping,
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
                object_database.insert(&object_name.clone(), object_definition);
            }
        }
    }

    Ok(object_database)
}

pub fn write_object_database(
    output_dir: &PathBuf,
    object_database: &ObjectDatabase,
    config: &Config,
) -> Result<(), String> {
    let name_mapping = &config.name_mapping;
    let target_dir = if config.use_scope {
        output_dir.join("src")
    } else {
        output_dir.join("src").join("objects")
    };

    fs::create_dir_all(&target_dir).expect("Creating objects dir failed");

    for (_, object_definition) in object_database.iter() {
        let object_name = get_object_name(object_definition);

        let module_name = name_mapping.name_to_module_name(&object_name);

        let target_file = target_dir.join(format!(
            "{}.rs",
            module_name.replace(".", "/").replace("::", "/")
        ));
        fs::create_dir_all(&target_file.parent().unwrap()).expect("Creating objects dir failed");

        let mut object_file = match File::create(target_file) {
            Ok(file) => file,
            Err(err) => {
                error!(
                    "Unable to create file {}.rs {}",
                    module_name,
                    err.to_string()
                );
                continue;
            }
        };

        match object_definition {
            ObjectDefinition::Struct(struct_definition) => {
                object_file
                    .write(modules_to_string(&struct_definition.get_required_modules()).as_bytes())
                    .expect("Failed to write imports");
                object_file.write("\n".as_bytes()).unwrap();

                object_file
                    .write(struct_definition.to_string(true, config).as_bytes())
                    .unwrap();
            }
            ObjectDefinition::Enum(enum_definition) => {
                object_file
                    .write(modules_to_string(&enum_definition.get_required_modules()).as_bytes())
                    .expect("Failed to write imports");
                object_file.write("\n".as_bytes()).unwrap();

                object_file
                    .write(enum_definition.to_string(true).as_bytes())
                    .unwrap();
            }
            ObjectDefinition::Primitive(primitive_definition) => {
                object_file
                    .write(
                        modules_to_string(
                            &primitive_definition
                                .primitive_type
                                .module
                                .as_ref()
                                .map_or(vec![], |module| vec![module]),
                        )
                        .as_bytes(),
                    )
                    .expect("Failed to write imports");
                object_file.write("\n".as_bytes()).unwrap();

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

                object_file.write(template.as_bytes()).unwrap();
            }
        }
    }

    let target_mod = target_dir.join("mod.rs");

    let mut object_mod_file = match File::create(&target_mod) {
        Ok(file) => file,
        Err(err) => {
            return Err(format!(
                "Unable to create file {} {}",
                target_mod.as_os_str().to_string_lossy(),
                err.to_string()
            ))
        }
    };

    for (struct_name, _) in object_database.iter() {
        match object_mod_file.write(
            format!(
                "pub mod {};\n",
                name_mapping.name_to_module_name(struct_name)
            )
            .to_string()
            .as_bytes(),
        ) {
            Ok(_) => (),
            Err(err) => return Err(format!("Failed to write to mod {}", err.to_string())),
        }
    }
    Ok(())
}

fn validate_component_name(component_name: &str) -> String {
    let mut result = component_name.replace("___", ".").replace(".", "::");
    if result.starts_with("_") {
        result = result.trim_start_matches("_").to_owned();
        return result;
    }
    if !result.contains("::") {
        result = format!("common::{}", result);
    }
    result
}
