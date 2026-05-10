#[derive(Debug, thiserror::Error)]
pub enum PylosError {
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Provider error from {provider}: {message}")]
    ProviderError { provider: String, message: String },

    #[error("All providers failed: {0}")]
    AllProvidersFailed(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl From<anyhow::Error> for PylosError {
    fn from(err: anyhow::Error) -> Self {
        PylosError::Internal(err.to_string())
    }
}

impl PylosError {
    /// Indique si l'erreur est retriable (fallback possible)
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            PylosError::ProviderError { .. }
                | PylosError::Timeout(_)
                | PylosError::RateLimitExceeded(_)
        )
    }
}
