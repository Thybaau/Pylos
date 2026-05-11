use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use pylos_application::ModelInfo;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ModelsQuery {
    /// Filtre par provider (optionnel)
    pub provider: Option<String>,
    #[serde(default)]
    pub include_deprecated: bool,
}

/// GET /v1/models — liste tous les modèles disponibles
/// Si ?provider=X est donné, retourne tous les modèles du catalog pour ce provider.
/// Sinon, retourne uniquement les modèles des providers configurés.
pub async fn list_models(
    State(state): State<AppState>,
    Query(query): Query<ModelsQuery>,
) -> impl IntoResponse {
    // Filtre par provider direct depuis le catalog
    if let Some(ref prov) = query.provider {
        let catalog_models = state
            .model_catalog
            .list_models(Some(prov), query.include_deprecated)
            .await;
        let data: Vec<_> = catalog_models.iter().map(model_info_to_entry).collect();
        return Json(json!({ "object": "list", "data": data }));
    }

    let cfg = state.config_store.get().await;
    let mut models = Vec::new();

    for (provider_name, provider_cfg) in &cfg.providers {
        // ── Ollama : interrogation en direct ─────────────────────────────────
        if provider_name == "ollama" {
            if let Some(base_url) = &provider_cfg.network.base_url {
                let tags_url = base_url.trim_end_matches("/v1").to_string() + "/api/tags";
                if let Ok(resp) = reqwest::get(&tags_url).await {
                    if let Ok(body) = resp.json::<serde_json::Value>().await {
                        if let Some(ollama_models) = body["models"].as_array() {
                            for m in ollama_models {
                                let name = m["name"].as_str().unwrap_or("");
                                let family = m["details"]["family"].as_str().unwrap_or("unknown");
                                let size = m["details"]["parameter_size"].as_str().unwrap_or("");
                                let info = state.model_catalog.get_model("ollama", name).await;
                                let pylos_field = info
                                    .as_ref()
                                    .map(model_info_pylos_field)
                                    .unwrap_or_else(|| make_minimal_pylos("ollama", name, None));
                                models.push(json!({
                                    "id": name,
                                    "provider": "ollama",
                                    "object": "model",
                                    "owned_by": "ollama",
                                    "details": { "family": family, "parameter_size": size },
                                    "pylos": pylos_field,
                                }));
                            }
                            continue;
                        }
                    }
                }
            }
            // Fallback catalog
            let catalog_models = state.model_catalog.list_models(Some("ollama"), false).await;
            for info in catalog_models {
                models.push(model_info_to_entry(&info));
            }
            continue;
        }

        // ── Autres providers ──────────────────────────────────────────────────
        let mut provider_model_ids: Vec<String> = Vec::new();
        let mut wildcard = false;

        for key in &provider_cfg.keys {
            for model in &key.models {
                if model == "*" {
                    wildcard = true;
                    break;
                }
                if !provider_model_ids.contains(model) {
                    provider_model_ids.push(model.clone());
                }
            }
            if wildcard { break; }
        }

        if wildcard {
            let catalog_models = state
                .model_catalog
                .list_models(Some(provider_name), false)
                .await;
            for info in catalog_models {
                models.push(model_info_to_entry(&info));
            }
        } else {
            for model_id in &provider_model_ids {
                let info = state.model_catalog.get_model(provider_name, model_id).await;
                let entry = if let Some(ref info) = info {
                    model_info_to_entry(info)
                } else {
                    json!({
                        "id": model_id,
                        "provider": provider_name,
                        "object": "model",
                        "owned_by": provider_name,
                        "pylos": make_minimal_pylos(provider_name, model_id, None),
                    })
                };
                models.push(entry);
            }
        }
    }

    models.sort_by(|a, b| {
        let pa = a["provider"].as_str().unwrap_or("");
        let pb = b["provider"].as_str().unwrap_or("");
        pa.cmp(pb).then(a["id"].as_str().unwrap_or("").cmp(b["id"].as_str().unwrap_or("")))
    });
    models.dedup_by(|a, b| a["provider"] == b["provider"] && a["id"] == b["id"]);

    Json(json!({ "object": "list", "data": models }))
}

/// Construit l'objet `pylos` avec la structure complète ModelInfo attendue par le TS
fn model_info_pylos_field(info: &pylos_application::ModelInfo) -> serde_json::Value {
    json!({
        "id": info.id,
        "provider": info.provider,
        "model_id": info.model_id,
        "display_name": info.display_name,
        "context_window": info.context_window,
        "max_output_tokens": info.max_output_tokens,
        "input_price_per_1m_usd": info.input_price_per_1m_usd,
        "output_price_per_1m_usd": info.output_price_per_1m_usd,
        "supports_vision": info.supports_vision,
        "supports_tools": info.supports_tools,
        "supports_streaming": info.supports_streaming,
        "supports_embeddings": info.supports_embeddings,
        "is_deprecated": info.is_deprecated,
    })
}

/// Construit un `pylos` minimal pour les modèles sans entrée dans le catalog
fn make_minimal_pylos(
    provider: &str,
    model_id: &str,
    display_name: Option<&str>,
) -> serde_json::Value {
    json!({
        "id": format!("{}/{}", provider, model_id),
        "provider": provider,
        "model_id": model_id,
        "display_name": display_name,
        "context_window": 0,
        "max_output_tokens": 0,
        "input_price_per_1m_usd": 0.0,
        "output_price_per_1m_usd": 0.0,
        "supports_vision": false,
        "supports_tools": true,
        "supports_streaming": true,
        "supports_embeddings": false,
        "is_deprecated": false,
    })
}

/// Convertit un ModelInfo en entrée complète pour l'API /v1/models
fn model_info_to_entry(info: &pylos_application::ModelInfo) -> serde_json::Value {
    json!({
        "id": info.model_id,
        "provider": info.provider,
        "object": "model",
        "owned_by": info.provider,
        "pylos": model_info_pylos_field(info),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /v1/models/catalog — upsert un modèle custom dans le catalog
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UpsertModelRequest {
    pub provider: String,
    pub model_id: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub context_window: u32,
    #[serde(default)]
    pub max_output_tokens: u32,
    #[serde(default)]
    pub input_price_per_1m_usd: f64,
    #[serde(default)]
    pub output_price_per_1m_usd: f64,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default = "default_true")]
    pub supports_tools: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default)]
    pub supports_embeddings: bool,
    #[serde(default)]
    pub is_deprecated: bool,
}

fn default_true() -> bool {
    true
}

pub async fn upsert_catalog_model(
    State(state): State<AppState>,
    Json(req): Json<UpsertModelRequest>,
) -> impl IntoResponse {
    let info = ModelInfo {
        id: format!("{}/{}", req.provider, req.model_id),
        provider: req.provider.clone(),
        model_id: req.model_id.clone(),
        display_name: req.display_name,
        context_window: req.context_window,
        max_output_tokens: req.max_output_tokens,
        input_price_per_1m_usd: req.input_price_per_1m_usd,
        output_price_per_1m_usd: req.output_price_per_1m_usd,
        supports_vision: req.supports_vision,
        supports_tools: req.supports_tools,
        supports_streaming: req.supports_streaming,
        supports_embeddings: req.supports_embeddings,
        is_deprecated: req.is_deprecated,
    };

    match state.model_catalog.upsert_model(&info).await {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "message": format!("Model '{}/{}' upserted in catalog", req.provider, req.model_id),
                "model": model_info_pylos_field(&info),
            })),
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
// DELETE /v1/models/catalog/:provider/:model_id — supprime un modèle du catalog
// ─────────────────────────────────────────────────────────────────────────────

pub async fn delete_catalog_model(
    State(state): State<AppState>,
    Path((provider, model_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match state.model_catalog.delete_model(&provider, &model_id).await {
        Ok(true) => Json(json!({
            "message": format!("Model '{}/{}' removed from catalog", provider, model_id)
        }))
        .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Model '{}/{}' not found in catalog", provider, model_id) })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
