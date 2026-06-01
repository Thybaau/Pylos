use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// Management API auth middleware (H-2 fix)
//
// Protège les endpoints /providers, /virtual-keys, /config, /v1/models/catalog.
// Auth via header : Authorization: Bearer <PYLOS_ADMIN_KEY>
// ou                X-Admin-Key: <PYLOS_ADMIN_KEY>
//
// Si PYLOS_ADMIN_KEY n'est pas défini → les endpoints management sont ouverts
// (comportement legacy, avec warning au démarrage).
// ─────────────────────────────────────────────────────────────────────────────

pub async fn management_auth_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let admin_key = state.admin_key.as_deref();

    // Si pas de clé admin configurée → warn mais laisse passer (compatibilité)
    let Some(expected) = admin_key else {
        return next.run(request).await;
    };

    // Extrait la clé depuis Authorization: Bearer ou X-Admin-Key
    let provided = extract_admin_key(request.headers());

    match provided {
        Some(key) if constant_time_eq(key.as_bytes(), expected.as_bytes()) => {
            next.run(request).await
        }
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": {
                    "message": "Invalid admin key",
                    "type": "forbidden",
                    "code": 403
                }
            })),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": {
                    "message": "Management API requires Authorization: Bearer <PYLOS_ADMIN_KEY>",
                    "type": "unauthorized",
                    "code": 401
                }
            })),
        )
            .into_response(),
    }
}

/// Constant-time string comparison to prevent timing side-channel attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

fn extract_admin_key(headers: &axum::http::HeaderMap) -> Option<&str> {
    // Authorization: Bearer <key>
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(key) = auth_str.strip_prefix("Bearer ") {
                return Some(key);
            }
        }
    }
    // X-Admin-Key: <key>
    if let Some(key) = headers.get("x-admin-key") {
        return key.to_str().ok();
    }
    None
}
