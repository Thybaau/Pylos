use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use pylos_core::domain::config::{EnvVar, ProviderConfig, VirtualKeyConfig, VkProviderConfig};

use crate::state::AppState;

// ─────────────────────────────────────────────────────────────────────────────
// GET /config
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_store.get().await;
    Json(redact_config(&cfg))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /config/reload
// ─────────────────────────────────────────────────────────────────────────────

pub async fn reload_config(State(state): State<AppState>) -> impl IntoResponse {
    match state.config_store.reload().await {
        Ok(summary) => {
            let providers = state.config_store.runtime_providers().await;
            state.orchestrator.update_providers(providers).await;

            Json(json!({
                "changed": summary.changed,
                "providers_reloaded": summary.providers_reloaded,
                "message": if summary.changed { "Config reloaded" } else { "Config unchanged" }
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
// POST /providers — crée un provider
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    #[serde(flatten)]
    pub config: ProviderConfig,
}

pub async fn create_provider(
    State(state): State<AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> impl IntoResponse {
    let name = req.name.clone();
    match state
        .config_store
        .upsert_provider(name.clone(), req.config)
        .await
    {
        Ok(()) => {
            let providers = state.config_store.runtime_providers().await;
            state.orchestrator.update_providers(providers).await;
            (
                StatusCode::CREATED,
                Json(
                    json!({ "message": format!("Provider '{}' created", name), "provider": name }),
                ),
            )
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
// PUT /providers/:name — met à jour un provider
// ─────────────────────────────────────────────────────────────────────────────

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
            let providers = state.config_store.runtime_providers().await;
            state.orchestrator.update_providers(providers).await;
            Json(json!({ "message": format!("Provider '{}' updated", name), "provider": name }))
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
// DELETE /providers/:name — supprime un provider
// ─────────────────────────────────────────────────────────────────────────────

pub async fn delete_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match state.config_store.remove_provider(&name).await {
        Ok(true) => {
            let providers = state.config_store.runtime_providers().await;
            state.orchestrator.update_providers(providers).await;
            Json(json!({ "message": format!("Provider '{}' deleted", name) })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Provider '{}' not found", name) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
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
                "rate_limit_id": vk.rate_limit_id,
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
// POST /virtual-keys — crée une virtual key
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateVirtualKeyRequest {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub value: Option<String>,
    #[serde(default = "default_true")]
    pub is_active: bool,
    #[serde(default)]
    pub provider_configs: Vec<VkProviderConfig>,
    pub rate_limit_id: Option<String>,
}

fn default_true() -> bool {
    true
}

pub async fn create_virtual_key(
    State(state): State<AppState>,
    Json(req): Json<CreateVirtualKeyRequest>,
) -> impl IntoResponse {
    let id = req
        .id
        .unwrap_or_else(|| format!("vk-{}", fastrand::u32(..)));
    let key_value = req
        .value
        .unwrap_or_else(|| format!("sk-pylos-{}", fastrand::u64(..)));

    let vk_cfg = VirtualKeyConfig {
        id: id.clone(),
        name: req.name.clone(),
        description: req.description,
        value: Some(EnvVar::Literal(key_value.clone())),
        is_active: req.is_active,
        rate_limit_id: req.rate_limit_id,
        provider_configs: req.provider_configs,
    };

    match state.config_store.add_virtual_key(vk_cfg).await {
        Ok(()) => {
            // Enregistre dans le VK registry
            let vk = pylos_core::domain::virtual_key::VirtualKey::new(key_value.clone(), &req.name);
            state.vk_registry.register(vk).await;

            (
                StatusCode::CREATED,
                Json(json!({
                    "id": id,
                    "name": req.name,
                    "value": key_value,
                    "message": "Virtual key created"
                })),
            )
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
// PUT /virtual-keys/:id — met à jour une virtual key
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateVirtualKeyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub provider_configs: Option<Vec<VkProviderConfig>>,
    pub rate_limit_id: Option<String>,
}

pub async fn update_virtual_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateVirtualKeyRequest>,
) -> impl IntoResponse {
    match state
        .config_store
        .update_virtual_key(&id, |vk| {
            if let Some(name) = req.name {
                vk.name = name;
            }
            if let Some(desc) = req.description {
                vk.description = Some(desc);
            }
            if let Some(active) = req.is_active {
                vk.is_active = active;
            }
            if let Some(pcs) = req.provider_configs {
                vk.provider_configs = pcs;
            }
            if req.rate_limit_id.is_some() {
                vk.rate_limit_id = req.rate_limit_id;
            }
        })
        .await
    {
        Ok(true) => Json(json!({ "id": id, "message": "Virtual key updated" })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Virtual key '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DELETE /virtual-keys/:id
// ─────────────────────────────────────────────────────────────────────────────

pub async fn delete_virtual_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.config_store.remove_virtual_key(&id).await {
        Ok(true) => {
            Json(json!({ "message": format!("Virtual key '{}' deleted", id) })).into_response()
        }
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Virtual key '{}' not found", id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /virtual-keys/:id/budget — statut du budget d'une VK (stub)
// ─────────────────────────────────────────────────────────────────────────────

pub async fn get_virtual_key_budget(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Budget tracking persistant prévu dans une prochaine phase
    Json(json!({
        "virtual_key_id": id,
        "budget": [],
        "rate_limits": []
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn redact_config(cfg: &pylos_core::domain::config::PylosConfig) -> serde_json::Value {
    let mut v = serde_json::to_value(cfg).unwrap_or_default();

    if let Some(providers) = v.get_mut("providers").and_then(|p| p.as_object_mut()) {
        for provider in providers.values_mut() {
            if let Some(keys) = provider.get_mut("keys").and_then(|k| k.as_array_mut()) {
                for key in keys.iter_mut() {
                    if let Some(val) = key.get_mut("value") {
                        let raw = val.as_str().unwrap_or("").to_string();
                        let env_var = EnvVar::from(raw.as_str());
                        *val = serde_json::Value::String(env_var.redacted());
                    }
                }
            }
        }
    }

    v
}
