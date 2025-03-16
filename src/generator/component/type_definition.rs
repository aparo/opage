use oas3::{
    spec::{ObjectSchema, SchemaTypeSet},
    Spec,
};
use tracing::trace;

use crate::{
    generator::types::{ModuleInfo, TypeDefinition},
    utils::{config::Config, name_mapping::NameMapping},
    GeneratorError,
};

use super::{
    object_definition::{get_object_name, get_object_or_ref_struct_name, get_or_create_object},
    ObjectDatabase,
};

pub fn get_type_from_schema(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: Vec<String>,
    object_schema: &ObjectSchema,
    object_variable_fallback_name: Option<&str>,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<TypeDefinition, GeneratorError> {
    if let Some(ref schema_type) = object_schema.schema_type {
        return get_type_from_schema_type(
            spec,
            object_database,
            definition_path,
            schema_type,
            object_schema,
            object_variable_fallback_name,
            name_mapping,
            config,
        );
    }

    if object_schema.any_of.len() > 0 {
        return get_type_from_any_type(
            spec,
            object_database,
            definition_path,
            object_schema,
            object_variable_fallback_name,
            name_mapping,
            config,
        );
    }

    if object_schema.one_of.len() > 0 {
        return get_type_from_any_type(
            spec,
            object_database,
            definition_path,
            object_schema,
            object_variable_fallback_name,
            name_mapping,
            config,
        );
    }

    // Fallback to string if no type is set
    get_type_from_schema_type(
        spec,
        object_database,
        definition_path,
        &SchemaTypeSet::Single(oas3::spec::SchemaType::String),
        object_schema,
        object_variable_fallback_name,
        name_mapping,
        config,
    )
}

pub fn get_type_from_any_type(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: Vec<String>,
    object_schema: &ObjectSchema,
    object_variable_fallback_name: Option<&str>,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<TypeDefinition, GeneratorError> {
    let object_variable_name = match object_schema.title {
        Some(ref title) => &name_mapping.name_to_struct_name(&definition_path, &title),
        None => match object_variable_fallback_name {
            Some(title_fallback) => title_fallback,
            None => {
                return Err(GeneratorError::ResolveError(
                    "Cannot fetch type because no title or title_fallback was given".to_string(),
                ))
            }
        },
    };

    trace!("Generating any_type {}", object_variable_name);

    let object_definition = get_or_create_object(
        spec,
        object_database,
        definition_path,
        &object_variable_name,
        &object_schema,
        name_mapping,
        config,
    )?;

    let object_name = get_object_name(&object_definition);
    let object_path = name_mapping.name_to_module_name(&object_name);

    let (object_name, object_path) =
        name_mapping.validate_object_name_path(&object_name, &object_path);

    Ok(TypeDefinition {
        name: object_name.clone(),
        module: Some(ModuleInfo::new(
            &format!("crate::{}", object_path.replace(".", "::")),
            &object_name,
        )),
        description: object_schema.description.clone(),
    })
}

pub fn get_type_from_schema_type(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: Vec<String>,
    schema_type: &SchemaTypeSet,
    object_schema: &ObjectSchema,
    object_variable_fallback_name: Option<&str>,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<TypeDefinition, GeneratorError> {
    let single_type = match schema_type {
        oas3::spec::SchemaTypeSet::Single(single_type) => single_type,
        _ => return Err(GeneratorError::UnsupportedError("MultiType".to_owned())),
    };

    let object_variable_name = match object_schema.title {
        Some(ref title) => title,
        None => match object_variable_fallback_name {
            Some(title_fallback) => title_fallback,
            None => {
                return Err(GeneratorError::ResolveError(format!(
                    "Cannot fetch type because no title or title_fallback was given {:#?}",
                    object_schema
                )))
            }
        },
    };

    match single_type {
        oas3::spec::SchemaType::Boolean => Ok(TypeDefinition {
            name: "bool".to_owned(),
            module: None,
            description: object_schema.description.clone(),
        }),
        oas3::spec::SchemaType::String => Ok(TypeDefinition {
            name: "String".to_owned(),
            module: None,
            description: object_schema.description.clone(),
        }),
        oas3::spec::SchemaType::Number => Ok(TypeDefinition {
            name: "f64".to_owned(),
            module: None,
            description: object_schema.description.clone(),
        }),
        oas3::spec::SchemaType::Integer => Ok(TypeDefinition {
            name: "i32".to_owned(),
            module: None,
            description: object_schema.description.clone(),
        }),
        oas3::spec::SchemaType::Array => {
            let item_object_ref = match object_schema.items {
                Some(ref item_object) => item_object,
                None => {
                    return Err(GeneratorError::UnsupportedError(
                        "Array has no item type".to_string(),
                    ))
                }
            };

            let (item_type_definition_path, item_type_name, _) = get_object_or_ref_struct_name(
                spec,
                &definition_path,
                name_mapping,
                &item_object_ref,
            )?;

            let item_object = match item_object_ref.resolve(spec) {
                Ok(item_object) => item_object,
                Err(err) => {
                    return Err(GeneratorError::ResolveError(format!(
                        "Failed to resolve ArrayItem\n{:#?}\n{}",
                        item_object_ref,
                        err.to_string()
                    )))
                }
            };

            match get_type_from_schema(
                spec,
                object_database,
                item_type_definition_path,
                &item_object,
                Some(&item_type_name),
                name_mapping,
                config,
            ) {
                Ok(mut type_definition) => {
                    type_definition.name = format!("Vec<{}>", type_definition.name);
                    return Ok(type_definition);
                }
                Err(err) => Err(err),
            }
        }
        oas3::spec::SchemaType::Object => {
            let object_definition = get_or_create_object(
                spec,
                object_database,
                definition_path,
                &object_variable_name,
                &object_schema,
                name_mapping,
                config,
            )?;

            let object_name = get_object_name(&object_definition);
            if object_name.eq("object") || object_name.eq("dict") {
                return Ok(TypeDefinition {
                    name: "serde_json::Value".to_owned(),
                    module: None,
                    description: object_schema.description.clone(),
                });
            }

            let object_path = name_mapping.name_to_module_name(&object_name);

            let (object_name, object_path) =
                name_mapping.validate_object_name_path(&object_name, &object_path);

            Ok(TypeDefinition {
                name: object_name.clone(),
                module: Some(ModuleInfo::new(
                    &format!("crate::{}", object_path.replace(".", "::")),
                    &object_name,
                )),
                description: object_schema.description.clone(),
            })
        }
        _ => Err(GeneratorError::UnsupportedError(format!(
            "Type {:?}",
            single_type
        ))),
    }
}
