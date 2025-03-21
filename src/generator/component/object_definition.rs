use std::collections::HashMap;

use crate::generator::types::{
    EnumDefinition, EnumValue, ModuleInfo, ObjectDefinition, PrimitiveDefinition,
    PropertyDefinition, StructDefinition,
};
use oas3::{
    spec::{ObjectOrReference, ObjectSchema, SchemaTypeSet},
    Spec,
};
use tracing::{error, info, trace};

use crate::{
    utils::{config::Config, name_mapping::NameMapping},
    GeneratorError,
};

use super::{type_definition::get_type_from_schema, ObjectDatabase};

pub fn get_components_base_path() -> Vec<String> {
    vec![
        String::from("#"),
        String::from("components"),
        String::from("schemas"),
    ]
}

pub fn get_object_name(object_definition: &ObjectDefinition) -> String {
    match object_definition {
        ObjectDefinition::Struct(struct_definition) => struct_definition.id(),
        ObjectDefinition::Enum(enum_definition) => enum_definition.name.clone(),
        ObjectDefinition::Primitive(type_definition) => type_definition.name.clone(),
    }
}

pub fn is_object_empty(object_schema: &ObjectSchema) -> bool {
    return object_schema.schema_type.is_none()
        && object_schema.const_value.is_none()
        && object_schema.any_of.is_empty()
        && object_schema.all_of.is_empty()
        && object_schema.one_of.is_empty();
}

pub fn generate_object(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: Vec<String>,
    name: &str,
    object_schema: &ObjectSchema,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<ObjectDefinition, GeneratorError> {
    if is_object_empty(object_schema) {
        return Err(GeneratorError::InvalidValueError(
            "Object is empty".to_string(),
        ));
    }

    if object_schema.any_of.len() > 0 {
        return generate_enum_from_any(
            spec,
            object_database,
            definition_path,
            name,
            object_schema,
            name_mapping,
            config,
        );
    }

    if object_schema.one_of.len() > 0 {
        return generate_enum_from_one_of(
            spec,
            object_database,
            definition_path,
            name,
            object_schema,
            name_mapping,
            config,
        );
    }

    let schema_type = match object_schema.schema_type {
        Some(ref schema_type) => schema_type,
        None => &SchemaTypeSet::Single(oas3::spec::SchemaType::String),
    };

    match schema_type {
        SchemaTypeSet::Single(single_type) => match single_type {
            oas3::spec::SchemaType::Object => generate_struct(
                spec,
                object_database,
                definition_path,
                name,
                object_schema,
                name_mapping,
                config,
            ),
            _ => match get_type_from_schema(
                spec,
                object_database,
                definition_path,
                object_schema,
                Some(name),
                name_mapping,
                config,
            ) {
                Ok(type_definition) => Ok(ObjectDefinition::Primitive(PrimitiveDefinition {
                    name: name.to_owned(),
                    primitive_type: type_definition.clone(),
                    description: type_definition.description.clone(),
                })),
                Err(err) => Err(err),
            },
        },
        SchemaTypeSet::Multiple(_) => Err(GeneratorError::UnsupportedError(
            "Multiple types".to_string(),
        )),
    }
}

pub fn oas3_type_to_string(oas3_type: &oas3::spec::SchemaType) -> String {
    match oas3_type {
        oas3::spec::SchemaType::Boolean => String::from("Boolean"),
        oas3::spec::SchemaType::Integer => String::from("Integer"),
        oas3::spec::SchemaType::Number => String::from("Number"),
        oas3::spec::SchemaType::String => String::from("String"),
        oas3::spec::SchemaType::Array => String::from("Array"),
        oas3::spec::SchemaType::Object => String::from("Object"),
        oas3::spec::SchemaType::Null => String::from("Null"),
    }
}

pub fn get_object_or_ref_struct_name(
    spec: &Spec,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    object_or_reference: &ObjectOrReference<ObjectSchema>,
) -> Result<
    (
        Vec<String>,
        String,
        Option<String>,
        Option<serde_json::Value>,
    ),
    GeneratorError,
> {
    // last parameter is the description
    let object_schema = match object_or_reference {
        ObjectOrReference::Ref { ref_path } => {
            let ref_definition_path = get_base_path_to_ref(ref_path)?;

            match object_or_reference.resolve(spec) {
                Ok(object_schema) => match object_schema.title {
                    Some(ref ref_title) => {
                        return Ok((
                            ref_definition_path.clone(),
                            name_mapping.name_to_struct_name(&ref_definition_path, ref_title),
                            object_schema.description.clone(),
                            object_schema.example.clone(),
                        ));
                    }
                    None => {
                        let path_name = match ref_path.split("/").last() {
                            Some(last_name) => last_name,
                            None => {
                                return Err(GeneratorError::ResolveError(format!(
                                    "Unable to retrieve name from ref path {}",
                                    ref_path
                                )))
                            }
                        };

                        return Ok((
                            ref_definition_path.clone(),
                            name_mapping.name_to_struct_name(&ref_definition_path, path_name),
                            object_schema.description.clone(),
                            object_schema.example.clone(),
                        ));
                    }
                },

                Err(err) => {
                    return Err(GeneratorError::ResolveError(format!(
                        "Failed to resolve object {}",
                        err.to_string()
                    )))
                }
            }
        }
        ObjectOrReference::Object(object_schema) => object_schema,
    };

    if let Some(ref title) = object_schema.title {
        return Ok((
            definition_path.clone(),
            name_mapping.name_to_struct_name(definition_path, &title),
            object_schema.description.clone(),
            object_schema.example.clone(),
        ));
    }

    if let Some(ref schema_type) = object_schema.schema_type {
        let type_name = match schema_type {
            SchemaTypeSet::Single(single_type) => oas3_type_to_string(single_type),
            SchemaTypeSet::Multiple(multiple_types) => multiple_types
                .iter()
                .map(oas3_type_to_string)
                .collect::<Vec<String>>()
                .join(""),
        };

        return Ok((
            definition_path.clone(),
            name_mapping.name_to_struct_name(definition_path, &type_name),
            object_schema.description.clone(),
            object_schema.example.clone(),
        ));
    }

    Err(GeneratorError::CodeGenerationError(
        String::new(),
        format!(": Unable to determine object name"),
    ))
}

pub fn get_base_path_to_ref(ref_path: &str) -> Result<Vec<String>, GeneratorError> {
    let mut path_segments = ref_path
        .split("/")
        .map(|segment| segment.to_owned())
        .collect::<Vec<String>>();
    if path_segments.len() < 4 {
        return Err(GeneratorError::CodeGenerationError(
            String::new(),
            format!(": Expected 4 path segments in {}", ref_path),
        ));
    }
    // Remove component name
    path_segments.pop();
    Ok(path_segments)
}

pub fn generate_enum_from_any(
    spec: &Spec,
    object_database: &ObjectDatabase,
    mut definition_path: Vec<String>,
    name: &str,
    object_schema: &ObjectSchema,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<ObjectDefinition, GeneratorError> {
    trace!("Generating enum");
    let mut enum_definition = EnumDefinition {
        name: name_mapping
            .name_to_struct_name(&definition_path, name)
            .to_owned(),
        values: HashMap::new(),
        used_modules: vec![
            ModuleInfo {
                name: "Serialize".to_owned(),
                path: "serde".to_owned(),
            },
            ModuleInfo {
                name: "Deserialize".to_owned(),
                path: "serde".to_owned(),
            },
        ],
        description: object_schema.description.clone(),
    };
    definition_path.push(enum_definition.name.clone());

    for any_object_ref in &object_schema.any_of {
        trace!("Generating enum value");
        let (any_object_definition_path, any_object) = match any_object_ref {
            ObjectOrReference::Ref { ref_path } => match any_object_ref.resolve(spec) {
                Err(err) => {
                    error!("{} {}", name, err);
                    continue;
                }
                Ok(object_schema) => {
                    let ref_definition_path = match get_base_path_to_ref(ref_path) {
                        Ok(base_path) => base_path,
                        Err(err) => {
                            error!("Unable to retrieve ref path {}", err);
                            continue;
                        }
                    };
                    (ref_definition_path, object_schema)
                }
            },
            ObjectOrReference::Object(object_schema) => {
                (definition_path.clone(), object_schema.clone())
            }
        };

        let object_type_enum_name = match get_object_or_ref_struct_name(
            spec,
            &any_object_definition_path,
            name_mapping,
            any_object_ref,
        ) {
            Ok((_, object_type_struct_name, _, _)) => name_mapping.name_to_struct_name(
                &any_object_definition_path,
                &format!("{}Value", object_type_struct_name),
            ),
            Err(err) => {
                return Err(GeneratorError::InvalidValueError(format!(
                    "{} Anonymous enum value are not supported \"{}\"",
                    name, err
                )))
            }
        };

        enum_definition.values.insert(
            object_type_enum_name.clone(),
            match get_type_from_schema(
                spec,
                object_database,
                any_object_definition_path.clone(),
                &any_object,
                Some(&object_type_enum_name),
                name_mapping,
                config,
            ) {
                Ok(type_definition) => EnumValue {
                    name: object_type_enum_name,
                    value_type: type_definition,
                },
                Err(err) => {
                    info!("{} {}", name, err);
                    continue;
                }
            },
        );
    }
    Ok(ObjectDefinition::Enum(enum_definition))
}

pub fn generate_enum_from_one_of(
    spec: &Spec,
    object_database: &ObjectDatabase,
    mut definition_path: Vec<String>,
    name: &str,
    object_schema: &ObjectSchema,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<ObjectDefinition, GeneratorError> {
    trace!("Generating enum");
    let mut enum_definition = EnumDefinition {
        name: name_mapping
            .name_to_struct_name(&definition_path, name)
            .to_owned(),
        values: HashMap::new(),
        used_modules: vec![
            ModuleInfo {
                name: "Serialize".to_owned(),
                path: "serde".to_owned(),
            },
            ModuleInfo {
                name: "Deserialize".to_owned(),
                path: "serde".to_owned(),
            },
        ],
        description: object_schema.description.clone(),
    };
    definition_path.push(enum_definition.name.clone());

    for one_of_object_ref in &object_schema.one_of {
        trace!("Generating enum value");
        let (one_of_object_definition_path, one_of_object) = match one_of_object_ref {
            ObjectOrReference::Ref { ref_path } => match one_of_object_ref.resolve(spec) {
                Err(err) => {
                    error!("{} {}", name, err);
                    continue;
                }
                Ok(object_schema) => {
                    let ref_definition_path = match get_base_path_to_ref(ref_path) {
                        Ok(base_path) => base_path,
                        Err(err) => {
                            error!("Unable to retrieve ref path {}", err);
                            continue;
                        }
                    };
                    (ref_definition_path, object_schema)
                }
            },
            ObjectOrReference::Object(object_schema) => {
                (definition_path.clone(), object_schema.clone())
            }
        };

        let object_type_enum_name = match get_object_or_ref_struct_name(
            spec,
            &one_of_object_definition_path,
            name_mapping,
            one_of_object_ref,
        ) {
            Ok((_, object_type_struct_name, _, _)) => name_mapping.name_to_struct_name(
                &one_of_object_definition_path,
                &format!("{}Value", object_type_struct_name),
            ),
            Err(err) => {
                return Err(GeneratorError::UnsupportedError(format!(
                    "{} Anonymous enum value are not supported \"{}\"",
                    name, err
                )))
            }
        };

        enum_definition.values.insert(
            object_type_enum_name.clone(),
            match get_type_from_schema(
                spec,
                object_database,
                one_of_object_definition_path.clone(),
                &one_of_object,
                Some(&object_type_enum_name),
                name_mapping,
                config,
            ) {
                Ok(type_definition) => EnumValue {
                    name: object_type_enum_name,
                    value_type: type_definition,
                },
                Err(err) => {
                    info!("{} {}", name, err);
                    continue;
                }
            },
        );
    }
    Ok(ObjectDefinition::Enum(enum_definition))
}

pub fn generate_struct(
    spec: &Spec,
    object_database: &ObjectDatabase,
    mut definition_path: Vec<String>,
    name: &str,
    object_schema: &ObjectSchema,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<ObjectDefinition, GeneratorError> {
    let full_name = name_mapping.name_to_struct_name(&definition_path, name);
    trace!("Generating struct: {}", full_name);
    let struct_name = name_mapping.extract_struct_name(&full_name);
    let package_name = name_mapping.extract_package_name(&full_name);
    let mut struct_definition = StructDefinition {
        name: struct_name,
        package: package_name,
        properties: HashMap::new(),
        used_modules: vec![
            ModuleInfo {
                name: "Serialize".to_owned(),
                path: "serde".to_owned(),
            },
            ModuleInfo {
                name: "Deserialize".to_owned(),
                path: "serde".to_owned(),
            },
        ],
        local_objects: HashMap::new(),
        description: object_schema.description.clone(),
    };
    definition_path.push(struct_definition.name.clone());

    for (property_name, property_ref) in &object_schema.properties {
        let property_required = object_schema
            .required
            .iter()
            .any(|property| property == property_name);

        let property_definition = match get_or_create_property(
            spec,
            definition_path.clone(),
            property_name,
            property_ref,
            property_required,
            object_database,
            name_mapping,
            config,
        ) {
            Err(err) => {
                info!("{} {}", name, err);
                continue;
            }
            Ok(property_definition) => property_definition,
        };
        struct_definition
            .properties
            .insert(property_definition.name.clone(), property_definition);
    }

    Ok(ObjectDefinition::Struct(struct_definition))
}

fn get_or_create_property(
    spec: &Spec,
    definition_path: Vec<String>,
    property_name: &String,
    property_ref: &ObjectOrReference<ObjectSchema>,
    required: bool,
    object_database: &ObjectDatabase,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<PropertyDefinition, GeneratorError> {
    trace!("Creating property {}", property_name);
    let property = match property_ref.resolve(spec) {
        Ok(property) => property,
        Err(err) => {
            return Err(GeneratorError::ResolveError(format!(
                "Failed to resolve {} {}",
                property_name,
                err.to_string()
            )))
        }
    };

    let (property_type_definition_path, property_type_name, description, _example) =
        get_object_or_ref_struct_name(spec, &definition_path, name_mapping, property_ref)?;

    match get_type_from_schema(
        spec,
        object_database,
        property_type_definition_path,
        &property,
        Some(&property_type_name),
        name_mapping,
        config,
    ) {
        Ok(property_type_definition) => Ok(PropertyDefinition {
            type_name: name_mapping
                .type_to_property_type(property_name, &property_type_definition.name),
            module: property_type_definition.module,
            name: name_mapping.name_to_property_name(&definition_path, property_name),
            real_name: property_name.clone(),
            required,
            description,
            example: property.example.clone(),
        }),
        Err(err) => Err(err),
    }
}

pub fn get_or_create_object(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: Vec<String>,
    name: &str,
    property_ref: &ObjectSchema,
    name_mapping: &NameMapping,
    config: &Config,
) -> Result<ObjectDefinition, GeneratorError> {
    if let Some(object_in_database) =
        object_database.get(&name_mapping.name_to_struct_name(&definition_path, name))
    {
        return Ok(object_in_database.clone());
    }

    // create shallow hull which will be filled in later
    // the hull is needed to reference for cyclic dependencies where we would
    // otherwise create the same object every time we want to resolve the current one
    let struct_name = name_mapping.name_to_struct_name(&definition_path, name);
    if object_database.contains_key(&struct_name) {
        return Err(GeneratorError::ObjectDatabaseDuplicateError(struct_name));
    }

    trace!("Adding struct {} to database", struct_name);
    let package_name = name_mapping.extract_package_name(&struct_name);
    let name = name_mapping.extract_struct_name(&struct_name);

    object_database.insert(
        struct_name.clone(),
        ObjectDefinition::Struct(StructDefinition {
            package: package_name,
            used_modules: vec![],
            name: name.clone(),
            properties: HashMap::new(),
            local_objects: HashMap::new(),
            description: property_ref.description.clone(),
        }),
    );

    match generate_object(
        spec,
        object_database,
        definition_path,
        &struct_name,
        property_ref,
        name_mapping,
        config,
    ) {
        Ok(created_struct) => {
            let name = get_object_name(&created_struct);
            trace!("Updating struct {} in database", name);
            object_database.insert(struct_name.clone(), created_struct.clone());
            Ok(created_struct)
        }
        Err(err) => Err(err),
    }
}
