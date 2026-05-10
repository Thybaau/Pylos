#[derive(Debug, thiserror::Error)]
pub enum PylosError {
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl From<anyhow::Error> for PylosError {
    fn from(err: anyhow::Error) -> Self {
        PylosError::Internal(err.to_string())
    }
}
