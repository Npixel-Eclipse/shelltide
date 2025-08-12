use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API request failed: {0}")]
    ApiRequest(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Environment '{0}' not found in configuration.")]
    EnvNotFound(String),

    #[error("Invalid command arguments: {0}")]
    InvalidArgs(String),

    #[error("Invalid revision version: {0}")]
    InvalidRevisionVersion(String),
}
