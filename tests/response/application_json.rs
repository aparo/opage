use opage::{
    generator::{
        path::default_request::generate_operation,
        types::{Method, ObjectDatabase, PathDatabase},
    },
    utils::{config, name_mapping::NameMapping},
};
use std::path::PathBuf;

#[test]
fn empty_json() {
    let mut spec_file_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    spec_file_path.push("tests/response/specs/empty_json.openapi.yaml");

    let spec = oas3::from_path(spec_file_path).expect("Failed to read spec");
    let path_spec = spec.paths.as_ref().unwrap().get("/test").unwrap();

    let object_database = ObjectDatabase::new();
    let path_database = PathDatabase::new();
    let name_mapping = NameMapping::new();
    let config = config::Config::default();

    generate_operation(
        &spec,
        &name_mapping,
        &Method::POST,
        "/test",
        &path_spec.post.as_ref().unwrap(),
        &object_database,
        &path_database,
        &config,
    )
    .expect("Failed to generated path");
}
