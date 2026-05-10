use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use pylos_core::domain::config::ProviderConfig;

use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// GET /config
// ─────────────────────────────────────────────────────────────────────────────

/// Retourne la configuration complète en mémoire (clés API masquées)
pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_store.get().await;

    // Masquage des valeurs sensibles (identique à Redacted() dans bifrost)
    let redacted = redact_config(&cfg);
    Json(redacted)
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /config/reload
// ─────────────────────────────────────────────────────────────────────────────

/// Recharge la config depuis le fichier pylos.json sur disque (hot reload)
/// Si le hash n'a pas changé, répond avec changed: false sans modifier la mémoire
pub async fn reload_config(State(state): State<AppState>) -> impl IntoResponse {
    match state.config_store.reload().await {
        Ok(summary) => {
            // Rebuild de l'orchestrateur avec les nouveaux providers
            let providers = state.config_store.runtime_providers().await;
            state.orchestrator.update_providers(providers).await;

            Json(json!({
                "changed": summary.changed,
                "providers_reloaded": summary.providers_reloaded,
                "message": if summary.changed {
                    "Config reloaded successfully"
                } else {
                    "Config unchanged (hash match)"
                }
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /providers
// ─────────────────────────────────────────────────────────────────────────────

/// Liste les providers configurés avec leur statut
pub async fn list_providers(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_store.get().await;
    let providers: Vec<_> = cfg
        .providers
        .iter()
        .map(|(name, p)| {
            json!({
                "name": name,
                "keys_count": p.keys.len(),
                "keys": p.keys.iter().map(|k| json!({
                    "name": k.name,
                    "value": k.value.redacted(),
                    "models": k.models,
                    "weight": k.weight
                })).collect::<Vec<_>>(),
                "network": {
                    "base_url": p.network.base_url,
                    "timeout_secs": p.network.timeout_secs,
                    "max_retries": p.network.max_retries
                }
            })
        })
        .collect();

    Json(json!({ "providers": providers, "total": providers.len() }))
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT /providers/:name
// ─────────────────────────────────────────────────────────────────────────────

/// Met à jour ou crée un provider en mémoire (hot-reload du provider)
pub async fn upsert_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(provider): Json<ProviderConfig>,
) -> impl IntoResponse {
    match state
        .config_store
        .upsert_provider(name.clone(), provider)
        .await
    {
        Ok(()) => {
            // Rebuild de l'orchestrateur
            let providers = state.config_store.runtime_providers().await;
            state.orchestrator.update_providers(providers).await;

            Json(json!({
                "message": format!("Provider '{}' updated", name),
                "provider": name
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /virtual-keys
// ─────────────────────────────────────────────────────────────────────────────

pub async fn list_virtual_keys(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_store.get().await;
    let vks: Vec<_> = cfg
        .governance
        .virtual_keys
        .iter()
        .map(|vk| {
            json!({
                "id": vk.id,
                "name": vk.name,
                "description": vk.description,
                "is_active": vk.is_active,
                "value": vk.value.as_ref().map(|v| v.redacted()).unwrap_or_default(),
                "provider_configs": vk.provider_configs.iter().map(|p| json!({
                    "provider": p.provider,
                    "allowed_models": p.allowed_models,
                    "weight": p.weight
                })).collect::<Vec<_>>()
            })
        })
        .collect();

    Json(json!({ "virtual_keys": vks, "total": vks.len() }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers — masquage des données sensibles
// ─────────────────────────────────────────────────────────────────────────────

fn redact_config(cfg: &pylos_core::domain::config::PylosConfig) -> serde_json::Value {
    let mut v = serde_json::to_value(cfg).unwrap_or_default();

    // Masque les valeurs de clés API dans providers[*].keys[*].value
    if let Some(providers) = v.get_mut("providers").and_then(|p| p.as_object_mut()) {
        for provider in providers.values_mut() {
            if let Some(keys) = provider.get_mut("keys").and_then(|k| k.as_array_mut()) {
                for key in keys.iter_mut() {
                    if let Some(val) = key.get_mut("value") {
                        let raw = val.as_str().unwrap_or("").to_string();
                        let env_var = pylos_core::domain::config::EnvVar::from(raw.as_str());
                        *val = serde_json::Value::String(env_var.redacted());
                    }
                }
            }
        }
    }

    v
}
