use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;
use pylos_core::domain::system_prompt::SystemPrompt;

// GET /api/system-prompts
pub async fn list_system_prompts(State(state): State<AppState>) -> impl IntoResponse {
    match state.system_prompt_store.list_prompts().await {
        Ok(prompts) => {
            Json(json!({ "system_prompts": prompts, "total": prompts.len() })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// POST /api/system-prompts
pub async fn create_system_prompt(
    State(state): State<AppState>,
    Json(prompt): Json<SystemPrompt>,
) -> impl IntoResponse {
    if prompt.id.is_empty() || prompt.name.is_empty() || prompt.prompt.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Fields id, name, and prompt cannot be empty" })),
        )
            .into_response();
    }

    match state.system_prompt_store.upsert_prompt(&prompt).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({ "message": "System prompt saved", "id": prompt.id })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// DELETE /api/system-prompts/:id
pub async fn delete_system_prompt(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.system_prompt_store.delete_prompt(&id).await {
        Ok(true) => {
            Json(json!({ "message": format!("System prompt '{}' deleted", id) })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("System prompt '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
