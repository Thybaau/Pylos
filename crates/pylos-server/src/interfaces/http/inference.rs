use std::time::Instant;

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

use pylos_application::log_store::{build_log_entry, LogStatus};
use pylos_core::domain::openai::ChatCompletionRequest;
use pylos_core::domain::request::{PylosRequest, RequestContext};
use pylos_core::error::PylosError;

use crate::state::AppState;

/// Handler POST /v1/chat/completions
pub async fn chat_completions(
    State(state): State<AppState>,
    Json(payload): Json<ChatCompletionRequest>,
) -> Response {
    let is_stream = payload.stream.unwrap_or(false);
    let model = payload.model.clone();
    // Preview du premier message user
    let input_preview = payload
        .messages
        .iter()
        .find(|m| matches!(m.role, pylos_core::domain::openai::MessageRole::User))
        .map(|m| m.content.clone());

    let request = PylosRequest::ChatCompletion(payload);
    let ctx = RequestContext::default();

    if is_stream {
        stream_response(state, request, ctx, model, input_preview).await
    } else {
        complete_response(state, request, ctx, model, input_preview).await
    }
}

async fn complete_response(
    state: AppState,
    request: PylosRequest,
    ctx: RequestContext,
    model: String,
    input_preview: Option<String>,
) -> Response {
    let start = Instant::now();

    match state.orchestrator.complete(request, ctx).await {
        Ok(pylos_core::domain::request::PylosResponse::ChatCompletion(resp)) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let output_preview = resp.choices.first().map(|c| c.message.content.clone());
            let finish_reason = resp.choices.first().and_then(|c| c.finish_reason.clone());

            // Détermine le provider depuis l'orchestrateur (approximation: premier tenté)
            let provider = guess_provider(&resp.model);

            let entry = build_log_entry(
                &provider,
                &resp.model,
                false,
                LogStatus::Success,
                latency,
                resp.usage.as_ref(),
                finish_reason,
                None,
                input_preview,
                output_preview,
                None,
            );

            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.log_store.push(entry).await;
            });

            Json(resp).into_response()
        }
        Err(e) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let provider = guess_provider(&model);

            let entry = build_log_entry(
                &provider,
                &model,
                false,
                LogStatus::Error,
                latency,
                None,
                None,
                Some(e.to_string()),
                input_preview,
                None,
                None,
            );
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.log_store.push(entry).await;
            });

            error_response(&e)
        }
    }
}

async fn stream_response(
    state: AppState,
    request: PylosRequest,
    ctx: RequestContext,
    model: String,
    input_preview: Option<String>,
) -> Response {
    let start = Instant::now();

    match state.orchestrator.stream(request, ctx).await {
        Ok(chunk_stream) => {
            let provider = guess_provider(&model);
            let model_clone = model.clone();
            let state_for_log = state.clone();
            let input_prev = input_preview.clone();

            let sse_stream = chunk_stream.map(move |result| match result {
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

            let done_event = futures::stream::once(async {
                Ok::<Event, axum::Error>(Event::default().data("[DONE]"))
            });

            // Log après fin du stream
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            tokio::spawn(async move {
                let entry = build_log_entry(
                    &provider,
                    &model_clone,
                    true,
                    LogStatus::Success,
                    latency,
                    None,
                    Some("stop".into()),
                    None,
                    input_prev,
                    None,
                    None,
                );
                state_for_log.log_store.push(entry).await;
            });

            Sse::new(sse_stream.chain(done_event))
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(e) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let provider = guess_provider(&model);
            let entry = build_log_entry(
                &provider,
                &model,
                true,
                LogStatus::Error,
                latency,
                None,
                None,
                Some(e.to_string()),
                input_preview,
                None,
                None,
            );
            tokio::spawn(async move {
                state.log_store.push(entry).await;
            });
            error_response(&e)
        }
    }
}

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

/// Déduit le provider depuis le nom du modèle (heuristique)
fn guess_provider(model: &str) -> String {
    if model.starts_with("us.") || model.starts_with("eu.") || model.starts_with("ap.")
        || model.starts_with("amazon.") || model.contains("nova") || model.contains("titan")
    {
        return "bedrock".to_string();
    }
    if model.contains("claude") {
        return "anthropic".to_string();
    }
    if model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("o3") {
        return "openai".to_string();
    }
    if model.contains('/') {
        return "openrouter".to_string();
    }
    // Ollama : modèles sans préfixe de provider (llama3.1:8b, codestral:22b, etc.)
    if model.contains(':') || model.contains("llama") || model.contains("qwen")
        || model.contains("codestral") || model.contains("deepseek")
        || model.contains("starcoder") || model.contains("nomic")
        || model.contains("mistral") || model.contains("gemma")
        || model.contains("phi") || model.contains("falcon")
    {
        return "ollama".to_string();
    }
    "unknown".to_string()
}
    } else if model.contains("nova")
        || model.starts_with("amazon.")
        || model.starts_with("us.amazon")
    {
        "bedrock".to_string()
    } else if model.contains("gpt") || model.contains("openai") {
        "openai".to_string()
    } else if model.contains('/') {
        // Format OpenRouter : "openai/gpt-4o-mini"
        "openrouter".to_string()
    } else {
        "unknown".to_string()
    }
}
