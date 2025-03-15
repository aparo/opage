use clap::Parser;

use opage::generator::generator::Generator;
use opage::utils::config::Config;
use tracing::{error, info};

use std::path::PathBuf;

use opage::Language;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Turn debugging information on
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// (json) Configuration with name mappings and ignores
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Client output location
    #[arg(short, long, value_name = "FILE")]
    pub output_dir: PathBuf,

    /// SInput OpenAPI spec/specs
    #[arg(short, long, value_name = "FILE")]
    pub specs: Vec<PathBuf>,
    /// What mode to run the program in
    #[arg(value_enum, default_value = "rust")]
    pub language: Language,
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
    let mut config = match config_file_path {
        Some(mapping_file) => Config::from(mapping_file.as_path()).expect("Failed to parse config"),
        None => Config::new(),
    };

    config.set_language(cli.language);

    let generator = Generator::new(config, output_dir, spec_file_paths);

    match generator.generate_paths() {
        Ok(_) => info!("Generation paths completed"),
        Err(err) => error!("Generation failed: {}", err),
    }
    generator.generate_objects();
    generator.populate_client_files();
}
