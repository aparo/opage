pub mod generator;
pub mod utils;

use std::{fs::File, io::Write};

use clap::Parser;
use generator::{
    cargo::generate_cargo_content,
    component::{
        generate_components, object_definition::types::ObjectDatabase, write_object_database,
    },
    paths::generate_paths,
};
use utils::config::Config;

use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Turn debugging information on
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// (json) Configuration with name mappings and ignores
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Client output location
    #[arg(short, long, value_name = "FILE")]
    output_dir: PathBuf,

    /// SInput OpenAPI spec/specs
    #[arg(short, long, value_name = "FILE")]
    specs: Vec<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    // we setup logging
    let tracing_level = match cli.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .compact()
        .with_thread_names(true)
        // enable everything
        .with_max_level(tracing_level)
        // sets this to be the default, global subscriber for this application.
        .init();

    let output_dir = cli.output_dir;
    let spec_file_paths = cli.specs;
    let config_file_path = cli.config;

    // Start generating

    // 1. Load config (Get mapper for invalid language names, ignores...)
    let config = match config_file_path {
        Some(mapping_file) => Config::from(mapping_file.as_path()).expect("Failed to parse config"),
        None => Config::new(),
    };

    // 2. Read spec
    let mut object_database = ObjectDatabase::new();
    let mut generated_paths = 0;
    for spec_file_path in spec_file_paths {
        let spec = oas3::from_path(spec_file_path).expect("Failed to read spec");
        // 3. Generate Code
        // 3.1 Components and database for type referencing
        let odb = generate_components(&spec, &config, object_database).unwrap();
        object_database = odb;
        // 3.2 Generate paths requests
        generated_paths += generate_paths(&output_dir, &spec, &mut object_database, &config)
            .expect("Failed to generated paths");
    }

    // 3.3 Write all registered objects to individual type definitions
    write_object_database(&output_dir, &mut object_database, &config)
        .expect("Write objects failed");
    // 4. Project setup
    let lib_target_file = output_dir.join("src").join("lib.rs");

    let mut lib_file = File::create(lib_target_file).expect("Failed to create lib.rs");

    if object_database.len() > 0 {
        lib_file
            .write("pub mod objects;\n".to_string().as_bytes())
            .unwrap();
    }

    if generated_paths > 0 {
        lib_file
            .write("pub mod paths;\n".to_string().as_bytes())
            .unwrap();
    }

    let cargo_target_file = output_dir.join("cargo.toml");
    let mut cargo_file = File::create(cargo_target_file).expect("Failed to create Cargo.toml");
    cargo_file
        .write(
            generate_cargo_content(&config.project_metadata)
                .expect("Failed to generate Cargo.toml")
                .as_bytes(),
        )
        .expect("Failed to write Cargo.toml");
}
