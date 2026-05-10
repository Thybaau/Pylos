use crate::interfaces::http::{config, health, inference, logs, metrics};
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

    // Routes API observabilité (logs)
    let logs_routes = Router::new()
        .route("/api/logs", get(logs::get_logs))
        .route("/api/logs/stats", get(logs::get_logs_stats))
        .route("/api/logs/histogram", get(logs::get_logs_histogram))
        .route("/api/logs/histogram/tokens", get(logs::get_token_histogram))
        .route("/api/logs/filterdata", get(logs::get_filter_data));

    // Routes de gestion de la config (hot-reload, providers, virtual keys)
    let config_routes = Router::new()
        .route("/config", get(config::get_config))
        .route("/config/reload", post(config::reload_config))
        .route("/providers", get(config::list_providers))
        .route("/providers/:name", put(config::upsert_provider))
        .route("/virtual-keys", get(config::list_virtual_keys));

    Router::new()
        // Racine et health
        .route("/", get(health::root))
        .route("/health", get(health::health_check))
        // Observabilité
        .route("/metrics", get(metrics::metrics))
        // Inférence
        .merge(inference_routes)
        // Logs API
        .merge(logs_routes)
        // Config API
        .merge(config_routes)
        // Middleware global
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
