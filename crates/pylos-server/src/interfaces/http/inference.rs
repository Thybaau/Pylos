use std::sync::Arc;
use std::time::Instant;

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use futures::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::error;

use pylos_application::log_store::{build_log_entry, LogStatus};
use pylos_core::domain::openai::ChatCompletionRequest;
use pylos_core::domain::request::{PylosRequest, RequestContext, StreamChunk};
use pylos_core::error::PylosError;

use crate::middleware::virtual_key::VirtualKeyInfo;
use crate::provider_utils::guess_provider;
use crate::state::AppState;

/// Handler POST /v1/chat/completions
pub async fn chat_completions(
    State(state): State<AppState>,
    Extension(vk_info): Extension<Option<VirtualKeyInfo>>,
    Json(payload): Json<ChatCompletionRequest>,
) -> Response {
    let is_stream = payload.stream.unwrap_or(false);
    let model = payload.model.clone();
    let input_preview = payload
        .messages
        .iter()
        .find(|m| matches!(m.role, pylos_core::domain::openai::MessageRole::User))
        .and_then(|m| m.content.clone());

    let request = PylosRequest::ChatCompletion(payload);
    let mut ctx = RequestContext::default();
    if let Some(vk) = &vk_info {
        ctx.virtual_key = Some(vk.name.clone());
    }

    if is_stream {
        stream_response(state, request, ctx, model, input_preview, vk_info).await
    } else {
        complete_response(state, request, ctx, model, input_preview, vk_info).await
    }
}

async fn complete_response(
    state: AppState,
    request: PylosRequest,
    ctx: RequestContext,
    model: String,
    input_preview: Option<String>,
    vk_info: Option<VirtualKeyInfo>,
) -> Response {
    let start = Instant::now();

    match state.orchestrator.complete(request, ctx).await {
        Ok(pylos_core::domain::request::PylosResponse::ChatCompletion(resp)) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let output_preview = resp.choices.first().and_then(|c| c.message.content.clone());
            let finish_reason = resp.choices.first().and_then(|c| c.finish_reason.clone());
            let provider = guess_provider(&resp.model);
            let vk_name = vk_info.map(|v| v.name);

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
                vk_name,
            );

            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.log_store.push(entry).await;
            });

            Json(resp).into_response()
        }
        Ok(pylos_core::domain::request::PylosResponse::Embedding(resp)) => {
            Json(resp).into_response()
        }
        Ok(pylos_core::domain::request::PylosResponse::TextCompletion(resp)) => {
            Json(resp).into_response()
        }
        Err(e) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let provider = guess_provider(&model);
            let vk_name = vk_info.map(|v| v.name);
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
                vk_name,
            );
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.log_store.push(entry).await;
            });
            error_response(&e)
        }
    }
}

/// Accumulateur partagé entre les closures du stream SSE
#[derive(Default)]
struct StreamAccumulator {
    output_parts: Vec<String>,
    finish_reason: Option<String>,
    completion_tokens: usize,
}

impl StreamAccumulator {
    fn collect(&mut self, chunk: &StreamChunk) {
        for choice in &chunk.choices {
            if let Some(content) = &choice.delta.content {
                // ~4 chars ≈ 1 token (approximation standard tiktoken)
                self.completion_tokens += (content.len() / 4).max(1);
                self.output_parts.push(content.clone());
            }
            if let Some(fr) = &choice.finish_reason {
                self.finish_reason = Some(fr.clone());
            }
        }
    }
}

async fn stream_response(
    state: AppState,
    request: PylosRequest,
    ctx: RequestContext,
    model: String,
    input_preview: Option<String>,
    vk_info: Option<VirtualKeyInfo>,
) -> Response {
    let start = Instant::now();

    match state.orchestrator.stream(request, ctx).await {
        Ok(chunk_stream) => {
            let provider = guess_provider(&model);
            let vk_name = vk_info.map(|v| v.name);

            // Accumulateur partagé entre le stream et le done_event.
            // Utilise std::sync::Mutex (sync) pour pouvoir s'acquérir dans les
            // closures synchrones du .map() sans risque de bloquer ni de perdre des chunks.
            let accumulator = Arc::new(std::sync::Mutex::new(StreamAccumulator::default()));
            let acc_stream = Arc::clone(&accumulator);

            // Canal pour déclencher le logging après [DONE]
            let (log_tx, mut log_rx) = mpsc::channel::<()>(1);
            let log_tx = Arc::new(log_tx);
            let log_tx_done = Arc::clone(&log_tx);

            // Stream principal : accumule les chunks + émet les événements SSE
            let sse_stream = chunk_stream.map(move |result| {
                match &result {
                    Ok(chunk) => {
                        // lock() ne peut jamais paniquer ici : pas de poison possible
                        // dans ce contexte single-producer
                        if let Ok(mut acc) = acc_stream.lock() {
                            acc.collect(chunk);
                        }
                    }
                    Err(e) => {
                        error!(error = %e, "SSE chunk error");
                    }
                }
                let data = match result {
                    Ok(chunk) => serde_json::to_string(&chunk).unwrap_or_default(),
                    Err(e) => json!({ "error": e.to_string() }).to_string(),
                };
                Ok::<Event, axum::Error>(Event::default().data(data))
            });

            // Événement terminal [DONE] — déclenche le logging
            let done_event = futures::stream::once(async move {
                let _ = log_tx_done.send(()).await;
                Ok::<Event, axum::Error>(Event::default().data("[DONE]"))
            });

            // Task de logging : attend le signal [DONE] puis persiste
            let state_for_log = state.clone();
            let model_clone = model.clone();
            let input_prev = input_preview.clone();
            let acc_log = Arc::clone(&accumulator);
            tokio::spawn(async move {
                if log_rx.recv().await.is_some() {
                    let latency = start.elapsed().as_secs_f64() * 1000.0;
                    let (completion_tokens, output_preview, finish) = {
                        let acc = acc_log.lock().expect("accumulator lock");
                        let tokens = acc.completion_tokens as i32;
                        let preview = if acc.output_parts.is_empty() {
                            None
                        } else {
                            Some(acc.output_parts.join(""))
                        };
                        let finish = acc.finish_reason.clone().or_else(|| Some("stop".into()));
                        (tokens, preview, finish)
                    };

                    let pseudo_usage = pylos_core::domain::openai::Usage {
                        prompt_tokens: 0,
                        completion_tokens,
                        total_tokens: completion_tokens,
                    };

                    let entry = build_log_entry(
                        &provider,
                        &model_clone,
                        true,
                        LogStatus::Success,
                        latency,
                        Some(&pseudo_usage),
                        finish,
                        None,
                        input_prev,
                        output_preview,
                        vk_name,
                    );
                    state_for_log.log_store.push(entry).await;
                }
            });

            Sse::new(sse_stream.chain(done_event))
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        Err(e) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let provider = guess_provider(&model);
            let vk_name = vk_info.map(|v| v.name);
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
                vk_name,
            );
            tokio::spawn(async move {
                state.log_store.push(entry).await;
            });
            error_response(&e)
        }
    }
}

pub fn error_response(error: &PylosError) -> Response {
    let (status, code) = match error {
        PylosError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
        PylosError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request_error"),
        PylosError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
        PylosError::RateLimitExceeded(_) => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_error"),
        PylosError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "timeout"),
        PylosError::AllProvidersFailed(_) | PylosError::ProviderError { .. } => {
            (StatusCode::BAD_GATEWAY, "provider_error")
        }
        PylosError::Unsupported(_) => (StatusCode::NOT_IMPLEMENTED, "not_implemented"),
        PylosError::BudgetExceeded(_) => (StatusCode::PAYMENT_REQUIRED, "budget_exceeded"),
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
