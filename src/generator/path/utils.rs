use std::collections::{BTreeMap, HashMap};

use oas3::{
    spec::{MediaType, ObjectOrReference, ObjectSchema, RequestBody, Response},
    Spec,
};
use reqwest::StatusCode;
use tracing::{error, trace};

use crate::{
    generator::{
        component::{
            object_definition::{
                get_object_or_ref_struct_name, get_or_create_object, is_object_empty,
            },
            type_definition::get_type_from_schema,
        },
        types::{
            ContentTypeValue, ModuleInfo, ObjectDatabase, ObjectDefinition, RequestEntity,
            ResponseEntities, ResponseEntity, StructDefinition, TransferMediaType, TypeDefinition,
        },
    },
    utils::{config::Config, name_mapping::NameMapping},
    GeneratorError,
};

pub fn is_path_parameter(path_component: &str) -> bool {
    path_component.starts_with("{") && path_component.ends_with("}")
}

fn parse_json_data(
    spec: &Spec,
    definition_path: Vec<String>,
    name_mapping: &NameMapping,
    new_object_name: &str,
    object_database: &ObjectDatabase,
    json_schema_object_or_ref: &ObjectOrReference<ObjectSchema>,
    config: &Config,
) -> Result<Option<TypeDefinition>, GeneratorError> {
    let is_json_object_empty = match json_schema_object_or_ref.resolve(spec) {
        Ok(schema_object) => is_object_empty(&schema_object),
        Err(err) => {
            return Err(GeneratorError::ResolveError(format!(
                "Failed to resolve json response {}",
                err.to_string()
            )));
        }
    };

    if is_json_object_empty {
        return Ok(None);
    }

    let json_object_definition_opt = match json_schema_object_or_ref {
        ObjectOrReference::Ref { ref_path: _ } => match get_object_or_ref_struct_name(
            spec,
            &definition_path,
            name_mapping,
            &json_schema_object_or_ref,
        ) {
            Ok((_, object_name, description, example)) => Some(TypeDefinition {
                module: Some(ModuleInfo {
                    path: format!(
                        "crate::objects::{}",
                        name_mapping.name_to_module_name(&object_name)
                    ),
                    name: object_name.clone(),
                }),
                name: object_name.clone(),
                description,
                example,
            }),
            Err(err) => return Err(err),
        },
        ObjectOrReference::Object(object_schema) => match get_type_from_schema(
            spec,
            object_database,
            definition_path.clone(),
            &object_schema,
            Some(new_object_name),
            name_mapping,
            config,
        ) {
            Ok(type_definition) => Some(type_definition),
            Err(err) => return Err(err),
        },
    };

    match json_object_definition_opt {
        Some(json_object_definition) => Ok(Some(json_object_definition)),
        None => {
            return Err(GeneratorError::ParameterError(
                "JsonObjectName not found".to_owned(),
                String::new(),
            ))
        }
    }
}

fn generate_json_content(
    spec: &Spec,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    object_database: &ObjectDatabase,
    json_media_type: &MediaType,
    content_object_name: &str,
    config: &Config,
) -> Result<TransferMediaType, GeneratorError> {
    let json_schema_object_or_ref = match json_media_type.schema {
        Some(ref schema) => schema,
        None => {
            return Err(GeneratorError::ParseError(
                "Failed to parse response json data".to_owned(),
            ))
        }
    };

    let json_object = parse_json_data(
        spec,
        definition_path.clone(),
        name_mapping,
        &name_mapping.name_to_struct_name(&definition_path, content_object_name),
        object_database,
        json_schema_object_or_ref,
        config,
    )?;

    let json_object_type_definition = match json_object {
        Some(json_object) => json_object,
        None => {
            trace!(
                "{} empty json request body object skipped",
                content_object_name
            );
            return Ok(TransferMediaType::ApplicationJson(None));
        }
    };

    Ok(TransferMediaType::ApplicationJson(Some(
        json_object_type_definition,
    )))
}

fn generate_content_type(
    spec: &Spec,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    object_database: &ObjectDatabase,
    content_type: &str,
    media_type: &MediaType,
    content_object_name: &str,
    config: &Config,
) -> Result<TransferMediaType, GeneratorError> {
    match content_type {
        "text/plain" => Ok(TransferMediaType::TextPlain),
        "application/json" => generate_json_content(
            spec,
            definition_path,
            name_mapping,
            object_database,
            media_type,
            &format!("{}Json", content_object_name),
            config,
        ),
        _ => Err(GeneratorError::UnsupportedError(format!(
            "Content-Type {}",
            content_type
        ))),
    }
}

fn generated_content_types_from_content_map(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    content: &BTreeMap<String, MediaType>,
    content_object_name: &str,
    config: &Config,
) -> HashMap<ContentTypeValue, TransferMediaType> {
    let mut content_map = HashMap::new();

    for (content_type, media_type) in content {
        match generate_content_type(
            spec,
            definition_path,
            name_mapping,
            object_database,
            content_type,
            media_type,
            content_object_name,
            config,
        ) {
            Ok(transfer_media_type) => {
                if content_map.contains_key(content_type) {
                    error!("Content-Type {} is already in content map", content_type);
                    continue;
                }
                content_map.insert(content_type.clone(), transfer_media_type);
            }
            Err(err) => error!("{} failed: {}", content_type, err),
        }
    }

    content_map
}

pub fn generate_request_body(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    request_body: &ObjectOrReference<RequestBody>,
    function_name: &str,
    config: &Config,
) -> Result<ObjectDefinition, GeneratorError> {
    let request = match request_body.resolve(spec) {
        Ok(request) => request,
        Err(err) => {
            return Err(GeneratorError::ResolveError(format!(
                "Failed to resolve request body {}",
                err.to_string()
            )))
        }
    };
    for (_, media_type) in &request.content {
        // we skipping content type for now
        match media_type.schema {
            Some(ref schema) => match schema.resolve(spec) {
                Ok(schema) => {
                    return get_or_create_object(
                        spec,
                        object_database,
                        definition_path.clone(),
                        function_name,
                        &schema,
                        name_mapping,
                        config,
                    )
                }
                Err(err) => {
                    error!("Failed to resolve request body schema: {}", err);
                    return Err(GeneratorError::ResolveError(format!(
                        "Failed to resolve request body {}",
                        err.to_string()
                    )));
                }
            },
            None => {
                error!("Failed to parse request body content type");
                return Err(GeneratorError::ResolveError(format!(
                    "Missing schema for {}",
                    function_name.to_string()
                )));
            }
        }
    }
    Err(GeneratorError::ResolveError(format!(
        "Failed to resolve request body {}",
        function_name.to_string()
    )))
}

pub fn generate_request_body_entity(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    request_body: &ObjectOrReference<RequestBody>,
    function_name: &str,
    config: &Config,
) -> Result<RequestEntity, GeneratorError> {
    let request = match request_body.resolve(spec) {
        Ok(request) => request,
        Err(err) => {
            return Err(GeneratorError::ResolveError(format!(
                "Failed to resolve request body {}",
                err.to_string()
            )))
        }
    };

    Ok(RequestEntity {
        content: generated_content_types_from_content_map(
            spec,
            object_database,
            definition_path,
            name_mapping,
            &request.content,
            &format!("{}RequestBody", function_name),
            config,
        ),
    })
}

pub fn generate_responses(
    spec: &Spec,
    object_database: &ObjectDatabase,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    responses: &BTreeMap<String, Response>,
    function_name: &str,
    config: &Config,
) -> Result<ResponseEntities, GeneratorError> {
    let mut response_entities = ResponseEntities::new();
    for (response_key, response) in responses {
        trace!("Generate response {}", response_key);
        if response_key == "default" {
            continue;
        }

        let canonical_status_code = match StatusCode::from_bytes(response_key.as_bytes()) {
            Ok(status_code) => match name_mapping.status_code_to_canonical_name(status_code) {
                Ok(canonical_status_code) => canonical_status_code,
                Err(err) => return Err(err),
            },
            Err(err) => {
                return Err(GeneratorError::StatusCodeError(
                    response_key.to_string(),
                    err.to_string(),
                ))
            }
        };

        response_entities.insert(
            response_key.clone(),
            ResponseEntity {
                canonical_status_code: canonical_status_code.to_owned(),
                content: generated_content_types_from_content_map(
                    spec,
                    object_database,
                    definition_path,
                    name_mapping,
                    &response.content,
                    &format!("{}{}", &function_name, &canonical_status_code),
                    config,
                ),
            },
        );
    }
    Ok(response_entities)
}
