use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::state::AppState;

/// GET /v1/models — liste tous les modèles disponibles par provider
/// Utilise le ModelCatalog pour les métadonnées enrichies (pricing, capacités…)
/// Interroge Ollama en direct pour les modèles locaux.
pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_store.get().await;
    let mut models = Vec::new();

    for (provider_name, provider_cfg) in &cfg.providers {
        // ── Ollama : interrogation en direct ──────────────────────────────
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
                                // Enrichissement optionnel depuis le catalog
                                let info = state.model_catalog.get_model("ollama", name).await;
                                let mut entry = json!({
                                    "id": name,
                                    "provider": "ollama",
                                    "object": "model",
                                    "owned_by": "ollama",
                                    "details": { "family": family, "parameter_size": size }
                                });
                                if let Some(info) = info {
                                    enrich_entry(&mut entry, &info);
                                }
                                models.push(entry);
                            }
                            continue;
                        }
                    }
                }
            }
            // Fallback : modèles Ollama dans le catalog
            let catalog_models = state.model_catalog.list_models(Some("ollama"), false).await;
            for info in catalog_models {
                models.push(model_info_to_entry(&info));
            }
            continue;
        }

        // ── Autres providers ─────────────────────────────────────────────
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
            if wildcard {
                break;
            }
        }

        if wildcard {
            // Retourner tous les modèles du catalog pour ce provider
            let catalog_models = state
                .model_catalog
                .list_models(Some(provider_name), false)
                .await;
            for info in catalog_models {
                models.push(model_info_to_entry(&info));
            }
        } else {
            // Retourner uniquement les modèles configurés, enrichis si possible
            for model_id in &provider_model_ids {
                let info = state.model_catalog.get_model(provider_name, model_id).await;
                let entry = if let Some(info) = info {
                    model_info_to_entry(&info)
                } else {
                    json!({
                        "id": model_id,
                        "provider": provider_name,
                        "object": "model",
                        "owned_by": provider_name
                    })
                };
                models.push(entry);
            }
        }
    }

    models.sort_by(|a, b| {
        let pa = a["provider"].as_str().unwrap_or("");
        let pb = b["provider"].as_str().unwrap_or("");
        let ia = a["id"].as_str().unwrap_or("");
        let ib = b["id"].as_str().unwrap_or("");
        pa.cmp(pb).then(ia.cmp(ib))
    });
    models.dedup_by(|a, b| a["provider"] == b["provider"] && a["id"] == b["id"]);

    Json(json!({ "object": "list", "data": models }))
}

/// Convertit un ModelInfo en entrée JSON pour l'API /v1/models
fn model_info_to_entry(info: &pylos_application::ModelInfo) -> serde_json::Value {
    let mut entry = json!({
        "id": info.model_id,
        "provider": info.provider,
        "object": "model",
        "owned_by": info.provider,
    });
    enrich_entry(&mut entry, info);
    entry
}

/// Enrichit une entrée JSON avec les métadonnées du catalog
fn enrich_entry(entry: &mut serde_json::Value, info: &pylos_application::ModelInfo) {
    if let Some(name) = &info.display_name {
        entry["display_name"] = json!(name);
    }
    entry["context_window"] = json!(info.context_window);
    entry["max_output_tokens"] = json!(info.max_output_tokens);
    entry["pricing"] = json!({
        "input_per_1m_usd": info.input_price_per_1m_usd,
        "output_per_1m_usd": info.output_price_per_1m_usd,
    });
    entry["capabilities"] = json!({
        "vision": info.supports_vision,
        "tools": info.supports_tools,
        "streaming": info.supports_streaming,
        "embeddings": info.supports_embeddings,
    });
}
