use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn admin_guard_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let headers = request.headers();

    let is_admin = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            state
                .admin_key
                .as_ref()
                .is_some_and(|admin| verify_slices_are_equal(v.as_bytes(), admin.as_bytes()))
        })
        .unwrap_or(false);

    if is_admin {
        return next.run(request).await;
    }

    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if let Some(token) = auth_header {
        if let Ok(claims) = jsonwebtoken::decode::<serde_json::Value>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &jsonwebtoken::Validation::default(),
        ) {
            let role = claims.claims.get("role").and_then(|r| r.as_str());
            if role == Some("admin") {
                return next.run(request).await;
            }
        }
    }

    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": "ADMIN_REQUIRED",
            "message": "This endpoint requires the admin role."
        })),
    )
        .into_response()
}

fn verify_slices_are_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
