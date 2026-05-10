use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use pylos_core::domain::virtual_key::VIRTUAL_KEY_PREFIX;

use crate::state::AppState;

/// Middleware Axum de vérification des Virtual Keys
///
/// Si le header Authorization est présent avec un sk-pylos-* :
///   - Vérifie la clé dans le registre
///   - Vérifie le rate limit
///   - Enrichit le contexte de la requête
///
/// Si le header est absent ou est une clé provider directe (pas sk-pylos-*),
/// on laisse passer (mode non-gouverné)
pub async fn virtual_key_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // Extraction du Bearer token depuis Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match auth_header {
        Some(token) if token.starts_with(VIRTUAL_KEY_PREFIX) => {
            // C'est une Virtual Key Pylos — on la vérifie
            match state.vk_registry.check_and_increment(token).await {
                Ok(vk) => {
                    // Injecte le nom de la VK dans les extensions de la requête
                    req.extensions_mut().insert(VirtualKeyInfo {
                        name: vk.name.clone(),
                        key: vk.key.clone(),
                    });
                    next.run(req).await
                }
                Err(reason) => {
                    let status = if reason.contains("Rate limit") {
                        StatusCode::TOO_MANY_REQUESTS
                    } else if reason.contains("not found") {
                        StatusCode::UNAUTHORIZED
                    } else {
                        StatusCode::FORBIDDEN
                    };

                    (
                        status,
                        Json(json!({
                            "error": {
                                "message": reason,
                                "type": "governance_error",
                                "code": status.as_u16()
                            }
                        })),
                    )
                        .into_response()
                }
            }
        }
        _ => {
            // Pas de VK ou clé provider directe — laisse passer
            next.run(req).await
        }
    }
}

/// Information sur la Virtual Key injectée dans les extensions de requête
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct VirtualKeyInfo {
    pub name: String,
    pub key: String,
}
