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
    // Extrait la clé depuis Authorization: Bearer ou X-Admin-Key
    let provided = extract_admin_key(request.headers());

    let Some(provided_key) = provided else {
        // Si pas de clé admin configurée globale → laisse passer (compatibilité)
        if state.admin_key.is_none() {
            return next.run(request).await;
        }
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": {
                    "message": "Management API requires Authorization: Bearer <token>",
                    "type": "unauthorized",
                    "code": 401
                }
            })),
        )
            .into_response();
    };

    // 1. Essayer de valider le token comme un JWT de session
    let validation = jsonwebtoken::Validation::default();
    let decoding_key = jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes());
    if let Ok(token_data) = jsonwebtoken::decode::<crate::interfaces::http::auth::PylosSessionClaims>(
        provided_key,
        &decoding_key,
        &validation,
    ) {
        let email = token_data.claims.sub.to_lowercase();
        // Vérifie si l'utilisateur est toujours actif dans le store
        if let Ok(users) = state.org_store.list_users().await {
            if users
                .iter()
                .any(|u| u.email.to_lowercase() == email && u.is_active)
            {
                return next.run(request).await;
            }
        }
    }

    // 2. Fallback sur la clé d'administration statique globale
    if let Some(expected) = &state.admin_key {
        if constant_time_eq(provided_key.as_bytes(), expected.as_bytes()) {
            return next.run(request).await;
        }
    } else {
        // Si pas de clé d'administration configurée
        return next.run(request).await;
    }

    (
        StatusCode::FORBIDDEN,
        Json(json!({
            "error": {
                "message": "Invalid token or admin key",
                "type": "forbidden",
                "code": 403
            }
        })),
    )
        .into_response()
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
