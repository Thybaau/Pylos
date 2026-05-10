use crate::interfaces::http::{health, inference};
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/v1/chat/completions", post(inference::chat_completions))
        .with_state(state)
}
