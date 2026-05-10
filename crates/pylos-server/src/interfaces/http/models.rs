use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::state::AppState;

/// GET /v1/models — liste tous les modèles disponibles par provider
/// Interroge Ollama en direct + retourne les modèles connus pour les autres
pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.config_store.get().await;
    let mut models = Vec::new();

    for (provider_name, provider_cfg) in &cfg.providers {
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
                                models.push(json!({
                                    "id": name,
                                    "provider": "ollama",
                                    "object": "model",
                                    "owned_by": "ollama",
                                    "details": { "family": family, "parameter_size": size }
                                }));
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // Autres providers : modèles depuis la config ou liste connue
        for key in &provider_cfg.keys {
            for model in &key.models {
                if model == "*" {
                    for m in known_models(provider_name) {
                        models.push(json!({
                            "id": m,
                            "provider": provider_name,
                            "object": "model",
                            "owned_by": provider_name
                        }));
                    }
                    break;
                } else {
                    models.push(json!({
                        "id": model,
                        "provider": provider_name,
                        "object": "model",
                        "owned_by": provider_name
                    }));
                }
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

fn known_models(provider: &str) -> Vec<&'static str> {
    match provider {
        "openrouter" => vec![
            "openai/gpt-4o",
            "openai/gpt-4o-mini",
            "anthropic/claude-3.5-sonnet",
            "anthropic/claude-3-haiku",
            "meta-llama/llama-3.1-8b-instruct",
            "google/gemini-flash-1.5",
        ],
        "bedrock" => vec![
            "us.amazon.nova-lite-v1:0",
            "us.amazon.nova-pro-v1:0",
            "us.anthropic.claude-sonnet-4-6",
            "us.anthropic.claude-haiku-4-5-20251001-v1:0",
        ],
        "anthropic" => vec![
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229",
        ],
        "openai" => vec!["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "gpt-3.5-turbo"],
        _ => vec![],
    }
}
