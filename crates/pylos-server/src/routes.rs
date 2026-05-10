use crate::interfaces::http::{health, inference, metrics};
use crate::middleware::virtual_key_middleware;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    // Routes d'inférence — protégées par le middleware Virtual Key
    let inference_routes = Router::new()
        .route("/v1/chat/completions", post(inference::chat_completions))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            virtual_key_middleware,
        ));

    Router::new()
        // Health (pas de middleware d'auth)
        .route("/health", get(health::health_check))
        // Observabilité (pas de middleware d'auth)
        .route("/metrics", get(metrics::metrics))
        // Routes d'inférence avec governance
        .merge(inference_routes)
        // Middleware stack global
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
