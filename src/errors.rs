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
    #[error("{0} {1}")]
    ParameterError(String, String),
    #[error("Failed to parse status code {0} {1}")]
    StatusCodeError(String, String),
    #[error("{0} is not supported")]
    UnsupportedError(String),
    #[error("Unable to determine property name of {0} {1}")]
    UnsupportedPropertyError(String, String),
    #[error("{0}")]
    ParseError(String),
    #[error("{0}")]
    ResolveError(String),
    #[error("ObjectDatabase already contains an object {0}")]
    ObjectDatabaseDuplicateError(String),
}
