use axum::{extract::State, http::header, response::IntoResponse};

use crate::state::AppState;

/// GET /metrics — expose les métriques Prometheus
pub async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let body = state.metrics.export();
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
