use thiserror::Error;

#[derive(Error, Debug)]
pub enum TurboError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("Compilation failed")]
    CompilationFailed,

    #[error("Runtime not found: {0}:{1}")]
    RuntimeNotFound(String, String),
    
    #[error("Package error: {0}")]
    Package(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, TurboError>;
