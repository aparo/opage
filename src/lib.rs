mod errors;
pub mod generator;
pub mod utils;

use clap::ValueEnum;
pub use errors::GeneratorError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Deserialize, Serialize)] // ArgEnum here
#[clap(rename_all = "kebab_case")]
pub enum Language {
    Rust,
    Scala,
}
