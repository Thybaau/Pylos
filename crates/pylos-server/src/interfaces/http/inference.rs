use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures::StreamExt;
use serde_json::json;
use tracing::error;

use pylos_core::domain::openai::ChatCompletionRequest;
use pylos_core::domain::request::{PylosRequest, RequestContext};
use pylos_core::error::PylosError;

use crate::state::AppState;

/// Handler POST /v1/chat/completions
/// Compatible OpenAI — gère à la fois les requêtes normales et streaming
pub async fn chat_completions(
    State(state): State<AppState>,
    Json(payload): Json<ChatCompletionRequest>,
) -> Response {
    let is_stream = payload.stream.unwrap_or(false);
    let request = PylosRequest::ChatCompletion(payload);
    let ctx = RequestContext::default();

    if is_stream {
        stream_response(state, request, ctx).await
    } else {
        complete_response(state, request, ctx).await
    }
}

/// Réponse non-streaming — retourne un JSON complet
async fn complete_response(
    state: AppState,
    request: PylosRequest,
    ctx: RequestContext,
) -> Response {
    match state.orchestrator.complete(request, ctx).await {
        Ok(pylos_core::domain::request::PylosResponse::ChatCompletion(resp)) => {
            Json(resp).into_response()
        }
        Err(e) => error_response(&e),
    }
}

/// Réponse streaming — retourne un flux SSE text/event-stream
async fn stream_response(state: AppState, request: PylosRequest, ctx: RequestContext) -> Response {
    match state.orchestrator.stream(request, ctx).await {
        Ok(chunk_stream) => {
            let sse_stream = chunk_stream.map(|result| match result {
                Ok(chunk) => {
                    let data = serde_json::to_string(&chunk).unwrap_or_default();
                    Ok::<Event, axum::Error>(Event::default().data(data))
                }
                Err(e) => {
                    error!(error = %e, "SSE chunk error");
                    let err_data = json!({ "error": e.to_string() }).to_string();
                    Ok::<Event, axum::Error>(Event::default().event("error").data(err_data))
                }
            });

            // Sentinel [DONE] à la fin du stream (compatibilité OpenAI)
            let done_event = futures::stream::once(async {
                Ok::<Event, axum::Error>(Event::default().data("[DONE]"))
            });

            let full_stream = sse_stream.chain(done_event);

            Sse::new(full_stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(e) => error_response(&e),
    }
}

/// Convertit une PylosError en réponse HTTP avec le bon code de statut
fn error_response(error: &PylosError) -> Response {
    let (status, code) = match error {
        PylosError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
        PylosError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request_error"),
        PylosError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
        PylosError::RateLimitExceeded(_) => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_error"),
        PylosError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "timeout"),
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
