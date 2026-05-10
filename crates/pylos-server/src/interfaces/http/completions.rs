use std::time::Instant;

use axum::{extract::Extension, response::IntoResponse, Json};
use tracing::error;

use pylos_application::log_store::{build_log_entry, LogStatus};
use pylos_core::domain::openai::TextCompletionRequest;
use pylos_core::domain::request::{PylosRequest, RequestContext};

use crate::interfaces::http::inference::error_response;
use crate::middleware::virtual_key::VirtualKeyInfo;
use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// POST /v1/completions — legacy text completion API
// Convertit en ChatCompletion en interne (bifrost compat pattern)
// ─────────────────────────────────────────────────────────────────────────────

pub async fn text_completions(
    axum::extract::State(state): axum::extract::State<AppState>,
    Extension(vk_info): Extension<Option<VirtualKeyInfo>>,
    Json(payload): Json<TextCompletionRequest>,
) -> impl IntoResponse {
    let model = payload.model.clone();
    let prompt_preview = Some(payload.prompt.first().chars().take(200).collect::<String>());
    let start = Instant::now();

    let request = PylosRequest::TextCompletion(payload);
    let mut ctx = RequestContext::default();
    if let Some(vk) = &vk_info {
        ctx.virtual_key = Some(vk.name.clone());
    }
    let vk_name = vk_info.map(|v| v.name);

    match state.orchestrator.complete(request, ctx).await {
        Ok(pylos_core::domain::request::PylosResponse::TextCompletion(resp)) => {
            let latency = start.elapsed().as_secs_f64() * 1000.0;
            let output_preview = resp
                .choices
                .first()
                .map(|c| c.text.chars().take(200).collect::<String>());
            let finish_reason = resp.choices.first().and_then(|c| c.finish_reason.clone());
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
                prompt_preview,
                output_preview,
                vk_name,
            );
            let state_clone = state.clone();
            tokio::spawn(async move {
                state_clone.log_store.push(entry).await;
            });

            Json(resp).into_response()
        }
        Ok(other) => {
            // Ne devrait pas arriver — mais au cas où
            error!(model = %model, "Unexpected response type for text completion");
            Json(other).into_response()
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
                prompt_preview,
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

fn guess_provider(model: &str) -> String {
    if model.starts_with("us.")
        || model.starts_with("eu.")
        || model.starts_with("ap.")
        || model.starts_with("amazon.")
        || model.contains("nova")
        || model.contains("titan")
        || model.starts_with("anthropic.")
    {
        return "bedrock".to_string();
    }
    if model.contains("claude") {
        return "anthropic".to_string();
    }
    if model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("o3") {
        return "openai".to_string();
    }
    "unknown".to_string()
}
