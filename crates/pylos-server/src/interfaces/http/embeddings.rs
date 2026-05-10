use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tracing::error;

use pylos_core::domain::embedding::EmbeddingRequest;
use pylos_core::error::PylosError;

use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// POST /v1/embeddings — compatible OpenAI Embeddings API
// Bifrost source: transports/bifrost-http/handlers/embeddings.go
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_embeddings(
    State(state): State<AppState>,
    Json(payload): Json<EmbeddingRequest>,
) -> impl IntoResponse {
    let model = payload.model.clone();

    // Trouve le provider qui supporte les embeddings pour ce modèle
    // On utilise l'orchestrateur pour bénéficier du fallback
    match state.orchestrator.embed(payload).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => {
            error!(model = %model, error = %e, "Embedding request failed");
            embedding_error_response(&e)
        }
    }
}

fn embedding_error_response(error: &PylosError) -> axum::response::Response {
    let (status, code) = match error {
        PylosError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
        PylosError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request_error"),
        PylosError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
        PylosError::RateLimitExceeded(_) => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_error"),
        PylosError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "timeout"),
        PylosError::Unsupported(_) => (StatusCode::NOT_IMPLEMENTED, "not_implemented"),
        PylosError::BudgetExceeded(_) => (StatusCode::PAYMENT_REQUIRED, "budget_exceeded"),
        PylosError::AllProvidersFailed(_) | PylosError::ProviderError { .. } => {
            (StatusCode::BAD_GATEWAY, "provider_error")
        }
        PylosError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
    };

    let body = json!({
        "error": {
            "message": error.to_string(),
            "type": code,
            "code": status.as_u16()
        }
    });

    (status, Json(body)).into_response()
}
