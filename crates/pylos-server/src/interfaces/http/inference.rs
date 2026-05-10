use axum::{extract::State, response::IntoResponse, Json};
use pylos_core::domain::openai::ChatCompletionRequest;
use serde_json::json;
use crate::state::AppState;

pub async fn chat_completions(
    State(_state): State<AppState>,
    Json(_payload): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    Json(json!({
        "error": "Not implemented yet",
        "info": "L'orchestrateur de providers est en cours de développement"
    }))
}
