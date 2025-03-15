use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Unable to create file {0} {1}")]
    FileCreationError(String, String),
    #[error("Failed to generated {0} code {1}")]
    CodeGenerationError(String, String),
    #[error("Invalid Value {0}")]
    InvalidValueError(String),
    #[error("{0} {1} has no id")]
    MissingIdError(String, String),
}
