use crate::interfaces::http::{config, health, inference, metrics};
use crate::middleware::virtual_key_middleware;
use crate::state::AppState;
use axum::{
    middleware,
    routing::{get, post, put},
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

    // Routes de gestion de la config (hot-reload, providers, virtual keys)
    let config_routes = Router::new()
        .route("/config", get(config::get_config))
        .route("/config/reload", post(config::reload_config))
        .route("/providers", get(config::list_providers))
        .route("/providers/:name", put(config::upsert_provider))
        .route("/virtual-keys", get(config::list_virtual_keys));

    Router::new()
        // Racine et health (pas de middleware d'auth)
        .route("/", get(health::root))
        .route("/health", get(health::health_check))
        // Observabilité
        .route("/metrics", get(metrics::metrics))
        // Routes d'inférence avec governance
        .merge(inference_routes)
        // Routes de config
        .merge(config_routes)
        // Middleware stack global
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
