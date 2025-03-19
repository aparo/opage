use std::collections::HashMap;

use convert_case::Casing;
use oas3::{
    spec::{Operation, ParameterIn, SchemaTypeSet},
    Spec,
};
use tracing::trace;

use crate::{
    generator::{
        component::{
            object_definition::oas3_type_to_string, type_definition::get_type_from_schema,
        },
        path::utils::generate_request_body,
        types::{
            Method, ModuleInfo, ObjectDatabase, ObjectDefinition, PathDatabase, PathDefinition,
            PathParameters, PropertyDefinition, QueryParameters, RequestEntity, StructDefinition,
            TransferMediaType,
        },
    },
    utils::{config::Config, name_mapping::NameMapping},
    GeneratorError,
};

use super::utils::{generate_request_body_entity, generate_responses, is_path_parameter};

pub fn generate_operation(
    spec: &Spec,
    name_mapping: &NameMapping,
    method: Method,
    path: &str,
    operation: &Operation,
    object_database: &ObjectDatabase,
    path_database: &PathDatabase,
    config: &Config,
) -> Result<String, GeneratorError> {
    trace!("Generating {:?} {}", method, path);
    let operation_definition_path: Vec<String> = vec![path.to_owned()];
    let description = operation
        .description
        .as_ref()
        .map_or(operation.summary.as_ref().map_or("", |f| f.as_str()), |d| {
            d.as_str()
        });

    let function_name = match operation.operation_id {
        Some(ref operation_id) => name_mapping.name_to_module_name(operation_id),
        None => {
            return Err(GeneratorError::MissingIdError(
                "operation_id".to_string(),
                path.to_owned(),
            ))
        }
    };

    let response_entities = generate_responses(
        spec,
        object_database,
        &operation_definition_path,
        name_mapping,
        &operation.responses(spec),
        &function_name,
        config,
    )?;

    // Path parameters
    let path_parameters = generate_path_parameters(
        spec,
        &operation,
        &operation_definition_path,
        name_mapping,
        &function_name,
        path,
    )?;

    // Response enum
    trace!("Generating response enum");

    let has_response_any_multi_content_type = response_entities
        .iter()
        .map(|response| response.1.content.len())
        .filter(|content_type_length| content_type_length > &1)
        .collect::<Vec<usize>>()
        .len()
        > 0;

    let response_enum_name = name_mapping.name_to_struct_name(
        &operation_definition_path,
        &format!(
            "{}ResponseType",
            &name_mapping
                .extract_struct_name(&function_name)
                .to_case(convert_case::Case::Pascal)
        ),
    );
    let mut response_enum_definition_path = operation_definition_path.clone();
    response_enum_definition_path.push(response_enum_name.clone());

    // let mut request_source_code = String::new();

    let module_imports = vec![ModuleInfo {
        name: "reqwest".to_owned(),
        path: String::new(),
    }];

    // Query params
    let query_parameter_code = generate_query_parameter_code(
        spec,
        operation,
        &operation_definition_path,
        name_mapping,
        object_database,
        &function_name,
        config,
    )?;

    // Request Body
    trace!("Generating request body");
    let request_entity = match operation.request_body {
        Some(ref request_body) => {
            match generate_request_body_entity(
                spec,
                object_database,
                &operation_definition_path,
                name_mapping,
                request_body,
                &function_name,
                config,
            ) {
                Ok(request_body) => Some(request_body),
                Err(err) => {
                    return Err(GeneratorError::CodeGenerationError(
                        "request body".to_string(),
                        err.to_string(),
                    ))
                }
            }
        }
        None => None,
    };
    let request_body: Option<ObjectDefinition> = match operation.request_body {
        Some(ref request_body) => {
            match generate_request_body(
                spec,
                object_database,
                &operation_definition_path,
                name_mapping,
                request_body,
                &function_name,
                config,
            ) {
                Ok(request_body) => Some(request_body),
                Err(err) => {
                    return Err(GeneratorError::CodeGenerationError(
                        "request body".to_string(),
                        err.to_string(),
                    ))
                }
            }
        }
        None => None,
    };

    trace!("Generating source code");
    // function
    let path_definition = PathDefinition {
        name: function_name.clone(),
        url: path.to_owned(),
        method: method.to_owned(),
        response_entities,
        used_modules: module_imports,
        request_entity,
        path_parameters: path_parameters,
        query_parameters: query_parameter_code,
        description: description.to_owned(),
        request_body: request_body,
        ..Default::default() // description,
    };
    path_database.insert(function_name, path_definition);
    Ok(String::new())
}

fn media_type_enum_name(
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    transfer_media_type: &TransferMediaType,
) -> String {
    let name = match transfer_media_type {
        TransferMediaType::ApplicationJson(_) => "Json",
        TransferMediaType::TextPlain => "Text",
    };
    name_mapping.name_to_struct_name(definition_path, name)
}

fn generate_path_parameters(
    spec: &Spec,
    operation: &Operation,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    function_name: &str,
    path: &str,
) -> Result<PathParameters, GeneratorError> {
    trace!("Generating path parameters");
    let path_parameters_struct_name = name_mapping.name_to_struct_name(
        &definition_path,
        &format!("{}PathParameters", function_name),
    );

    let mut path_parameters_definition_path = definition_path.clone();
    path_parameters_definition_path.push(path_parameters_struct_name.clone());

    let path_parameters_ordered = path
        .split("/")
        .filter(|&path_component| is_path_parameter(&path_component))
        .map(|path_component| path_component.replace("{", "").replace("}", ""))
        .map(|path_component| {
            let mut description = None;
            let mut example: Option<serde_json::Value> = None;
            let type_name = "String".to_owned();
            operation.parameters.iter().find(|f| match f {
                oas3::spec::ObjectOrReference::Ref { ref_path } => false,
                oas3::spec::ObjectOrReference::Object(parameter) => {
                    if parameter.location != ParameterIn::Path {
                        return false;
                    }
                    if parameter.name != path_component {
                        return false;
                    }
                    description = parameter.description.clone();
                    example = parameter.example.clone();
                    true
                }
            });

            PropertyDefinition {
                module: None,
                name: name_mapping
                    .name_to_property_name(&path_parameters_definition_path, &path_component),
                real_name: path_component,
                required: true,
                type_name,
                description,
                example,
            }
        })
        .collect::<Vec<PropertyDefinition>>();
    let package_name = name_mapping.extract_package_name(&path_parameters_struct_name);
    let path_parameters_struct_name =
        name_mapping.extract_struct_name(&path_parameters_struct_name);

    let path_struct_definition = StructDefinition {
        package: package_name,
        name: path_parameters_struct_name,
        used_modules: vec![],
        local_objects: HashMap::new(),
        properties: path_parameters_ordered
            .iter()
            .map(|path_component| {
                (
                    path_component.name.clone(),
                    PropertyDefinition {
                        module: None,
                        name: path_component.name.clone(),
                        real_name: path_component.real_name.clone(),
                        required: path_component.required,
                        type_name: path_component.type_name.clone(),
                        description: path_component.description.clone(),
                        example: path_component.example.clone(),
                    },
                )
            })
            .collect::<HashMap<String, PropertyDefinition>>(),
        description: None,
    };

    let path_format_string = path
        .split("/")
        .map(|path_component| {
            return match is_path_parameter(path_component) {
                true => String::from("{}"),
                _ => path_component.to_owned(),
            };
        })
        .collect::<Vec<String>>()
        .join("/");

    Ok(PathParameters {
        parameters_struct_variable_name: name_mapping
            .name_to_property_name(definition_path, "path_parameters"),
        parameters_struct: path_struct_definition,
        path_format_string: path_format_string,
    })
}

fn generate_query_parameter_code(
    spec: &Spec,
    operation: &Operation,
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    object_database: &ObjectDatabase,
    function_name: &str,
    config: &Config,
) -> Result<QueryParameters, GeneratorError> {
    trace!("Generating query params");
    let mapping_name = name_mapping.name_to_struct_name(
        &definition_path,
        &format!("{}QueryParameters", function_name),
    );
    let package_name = name_mapping.extract_package_name(&mapping_name);
    let mapping_structure_name = name_mapping.extract_struct_name(&mapping_name);

    let mut query_struct = StructDefinition {
        package: package_name,
        name: mapping_structure_name,
        properties: HashMap::new(),
        used_modules: vec![],
        local_objects: HashMap::new(),
        description: None,
    };

    let query_struct_variable_name =
        name_mapping.name_to_property_name(&definition_path, "query_parameters");

    let mut query_parameters_definition_path = definition_path.clone();
    query_parameters_definition_path.push(query_struct.name.clone());

    for parameter_ref in &operation.parameters {
        let parameter = match parameter_ref.resolve(spec) {
            Ok(parameter) => parameter,
            Err(err) => {
                return Err(GeneratorError::ParameterError(
                    "Failed to resolve parameter".to_owned(),
                    err.to_string(),
                ))
            }
        };
        if parameter.location != ParameterIn::Query {
            continue;
        }

        let parameter_type = match parameter.schema {
            Some(schema) => match schema.resolve(spec) {
                Ok(object_schema) => get_type_from_schema(
                    spec,
                    object_database,
                    query_parameters_definition_path.clone(),
                    &object_schema,
                    Some(&parameter.name),
                    name_mapping,
                    config,
                ),
                Err(err) => {
                    return Err(GeneratorError::ParameterError(
                        format!("Failed to resolve parameter {}", parameter.name),
                        err.to_string(),
                    ))
                }
            },
            None => {
                return Err(GeneratorError::ParameterError(
                    "Parameter has no schema:".to_string(),
                    parameter.name,
                ))
            }
        };

        let _ = match parameter_type {
            Ok(parameter_type) => query_struct.properties.insert(
                name_mapping
                    .name_to_property_name(&query_parameters_definition_path, &parameter.name),
                PropertyDefinition {
                    name: name_mapping
                        .name_to_property_name(&query_parameters_definition_path, &parameter.name),
                    module: parameter_type.module,
                    real_name: parameter.name,
                    required: match parameter.required {
                        Some(required) => required,
                        None => false,
                    },
                    type_name: parameter_type.name,
                    description: parameter_type.description.clone(),
                    example: parameter_type.example.clone(),
                },
            ),
            Err(err) => return Err(err),
        };
    }

    let mut unroll_query_parameters_code = String::new();
    unroll_query_parameters_code += &format!(
        "  let {} request_query_parameters: Vec<(&str, String)> = vec![{}];\n",
        match query_struct
            .properties
            .iter()
            .filter(|(_, property)| !property.required || property.type_name.starts_with("Vec<"))
            .collect::<Vec<(&String, &PropertyDefinition)>>()
            .len()
        {
            0 => "",
            _ => "mut",
        },
        query_struct
            .properties
            .iter()
            .filter(|(_, property)| property.required && !property.type_name.starts_with("Vec<"))
            .map(|(_, property)| format!(
                "(\"{}\",{}.{}.to_string())",
                property.real_name, query_struct_variable_name, property.name
            ))
            .collect::<Vec<String>>()
            .join(",")
    );

    query_struct
        .properties
        .values()
        .filter(|&property| property.required && property.type_name.starts_with("Vec<"))
        .for_each(|vector_property|
    {
        unroll_query_parameters_code += &format!(
                "{}.{}.iter().for_each(|query_parameter_item| request_query_parameters.push((\"{}\", query_parameter_item.to_string())));\n",
                &query_struct_variable_name,
                name_mapping.name_to_property_name(&definition_path, &vector_property.name),
                vector_property.real_name
            );
    });

    for optional_property in query_struct
        .properties
        .values()
        .filter(|&property| !property.required)
        .collect::<Vec<&PropertyDefinition>>()
    {
        unroll_query_parameters_code += &format!(
            "  if let Some(ref query_parameter) = {}.{} {{\n",
            query_struct_variable_name, optional_property.name
        );
        if optional_property.type_name.starts_with("Vec<") {
            unroll_query_parameters_code += &format!(
                "  query_parameter.iter().for_each(|query_parameter_item| request_query_parameters.push((\"{}\", query_parameter_item.to_string())));\n",
                optional_property.real_name
            );
        } else {
            unroll_query_parameters_code += &format!(
                "  request_query_parameters.push((\"{}\", query_parameter.to_string()));\n",
                optional_property.real_name
            );
        }
        unroll_query_parameters_code += "}\n"
    }

    Ok(QueryParameters {
        query_struct_variable_name,
        query_struct,
        unroll_query_parameters_code,
    })
}

fn generate_multi_request_type_functions(
    definition_path: &Vec<String>,
    name_mapping: &NameMapping,
    function_name: &str,
    path_parameters: &PathParameters,
    module_imports: &mut Vec<ModuleInfo>,
    query_parameter_code: &QueryParameters,
    response_enum_name: &str,
    method: Method,
    request_entity: &RequestEntity,
) -> Option<String> {
    if request_entity.content.len() < 2 {
        return None;
    }

    let mut request_source_code = String::new();

    for (_, transfer_media_type) in &request_entity.content {
        let content_function_name = name_mapping.name_to_property_name(
            &definition_path,
            &format!(
                "{}{}",
                function_name,
                media_type_enum_name(&definition_path, name_mapping, &transfer_media_type)
            ),
        );
        let mut function_parameters = vec![
            "client: &reqwest::Client".to_owned(),
            "server: &str".to_owned(),
        ];

        if path_parameters.parameters_struct.properties.len() > 0 {
            function_parameters.push(format!(
                "{}: &{}",
                path_parameters.parameters_struct_variable_name,
                path_parameters.parameters_struct.name
            ));
        }

        let query_struct = &query_parameter_code.query_struct;
        if query_struct.properties.len() > 0 {
            function_parameters.push(format!(
                "{}: &{}",
                query_parameter_code.query_struct_variable_name, query_struct.name
            ));
        }

        let request_content_variable_name =
            name_mapping.name_to_property_name(definition_path, "content");
        match transfer_media_type {
            TransferMediaType::ApplicationJson(ref type_definition_opt) => {
                match type_definition_opt {
                    Some(ref type_definition) => {
                        if let Some(ref module) = type_definition.module {
                            if !module_imports.contains(module) {
                                module_imports.push(module.clone());
                            }
                        }
                        function_parameters.push(format!(
                            "{}: {}",
                            request_content_variable_name, type_definition.name
                        ))
                    }
                    None => trace!("Empty request body not added to function params"),
                }
            }
            TransferMediaType::TextPlain => function_parameters.push(format!(
                "{}: &{}",
                request_content_variable_name,
                oas3_type_to_string(&oas3::spec::SchemaType::String)
            )),
        }

        let function_name = name_mapping.extract_function_name(&content_function_name);

        request_source_code += &format!(
            "pub async fn {}({}) -> Result<{}, reqwest::Error> {{\n",
            &function_name,
            function_parameters.join(", "),
            response_enum_name,
        );

        // PRE request processing
        match transfer_media_type {
            TransferMediaType::TextPlain => {
                request_source_code += &format!(
                    "  let body = {}.to_owned();\n",
                    request_content_variable_name
                )
            }
            _ => (),
        }

        // Request attach
        let request_body = match transfer_media_type {
            TransferMediaType::ApplicationJson(type_definition) => match type_definition {
                Some(_) => {
                    format!(".json(&{})", request_content_variable_name)
                }
                None => ".json(&serde_json::json!({}))".to_owned(),
            },
            TransferMediaType::TextPlain => ".body(body)".to_owned(),
        };

        request_source_code += &format!(
            "  let request_builder = client.{}(format!(\"{{server}}{}\", {})){};\n",
            method.to_string().to_lowercase(),
            path_parameters.path_format_string,
            path_parameters
                .parameters_struct
                .properties
                .iter()
                .map(|(_, parameter)| format!(
                    "{}.{}",
                    path_parameters.parameters_struct_variable_name,
                    name_mapping.name_to_property_name(&definition_path, &parameter.name)
                ))
                .collect::<Vec<String>>()
                .join(","),
            request_body
        );

        let request_function_call_parameters = match query_struct.properties.len() {
            0 => vec!["request_builder".to_owned()],
            _ => vec![
                "request_builder".to_owned(),
                query_parameter_code.query_struct_variable_name.clone(),
            ],
        };

        request_source_code += &format!(
            "{}({}).await",
            function_name,
            request_function_call_parameters.join(",")
        );
        request_source_code += "}\n";

        let _ = PathDefinition {
            package: name_mapping.extract_package_name(&content_function_name),
            name: name_mapping.extract_struct_name(&content_function_name),
            used_modules: module_imports.clone(),
            ..Default::default()
        };
    }

    Some(request_source_code)
}
