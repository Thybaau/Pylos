use crate::interfaces::http::{
    completions, config, embeddings, health, images, inference, logs, metrics, models,
};
use crate::middleware::{management_auth_middleware, virtual_key_middleware};
use crate::state::AppState;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    // Routes d'inférence — protégées par le middleware Virtual Key
    let inference_routes = Router::new()
        .route("/v1/chat/completions", post(inference::chat_completions))
        .route("/v1/completions", post(completions::text_completions))
        .route("/v1/embeddings", post(embeddings::create_embeddings))
        .route("/v1/images/generations", post(images::generate_image))
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

    // Routes de gestion — protégées par le middleware Management Auth
    let management_routes = Router::new()
        .route("/config", get(config::get_config))
        .route("/config/reload", post(config::reload_config))
        // Providers CRUD
        .route(
            "/providers",
            get(config::list_providers).post(config::create_provider),
        )
        .route(
            "/providers/:name",
            put(config::upsert_provider).delete(config::delete_provider),
        )
        // Virtual Keys CRUD
        .route(
            "/virtual-keys",
            get(config::list_virtual_keys).post(config::create_virtual_key),
        )
        .route(
            "/virtual-keys/:id",
            put(config::update_virtual_key).delete(config::delete_virtual_key),
        )
        .route(
            "/virtual-keys/:id/budget",
            get(config::get_virtual_key_budget),
        )
        // Model catalog CRUD
        .route("/v1/models/catalog", post(models::upsert_catalog_model))
        .route(
            "/v1/models/catalog/:provider/:model_id",
            delete(models::delete_catalog_model),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            management_auth_middleware,
        ));

    Router::new()
        .route("/", get(health::root))
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics::metrics))
        // Models read-only (pas d'auth)
        .route("/v1/models", get(models::list_models))
        // Inférence
        .merge(inference_routes)
        // Logs API
        .merge(logs_routes)
        // Management API (protégée)
        .merge(management_routes)
        // Middleware global
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
