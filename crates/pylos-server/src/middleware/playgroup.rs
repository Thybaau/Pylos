use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn playgroup_check_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let headers = request.headers();
    let is_management = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            state
                .admin_key
                .as_ref()
                .is_some_and(|admin| constant_time_eq(v, admin))
        })
        .unwrap_or(false);

    if is_management {
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
            if role == Some("playgroup") {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "error": "ACCESS_RESTRICTED",
                        "message": "Your account is in quarantine (playgroup). \
                            An administrator must approve your access before you can use the gateway. \
                            Please contact your Pylos administrator."
                    })),
                )
                    .into_response();
            }
        }
    }

    next.run(request).await
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
