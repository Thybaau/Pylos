use std::sync::Arc;

use pylos_application::InferenceOrchestrator;
use pylos_core::domain::provider::{ProviderConfig, ProviderKey, ProviderKind};
use pylos_core::domain::traits::Provider;
use pylos_core::domain::virtual_key::VirtualKeyRegistry;
use pylos_infrastructure::{AnthropicProvider, OpenAIProvider};

use crate::metrics::Metrics;

/// État global partagé entre tous les handlers Axum
#[derive(Clone)]
pub struct AppState {
    pub orchestrator: Arc<InferenceOrchestrator>,
    pub metrics: Arc<Metrics>,
    pub vk_registry: Arc<VirtualKeyRegistry>,
}

impl AppState {
    /// Construit l'AppState depuis les variables d'environnement
    pub fn from_env() -> anyhow::Result<Self> {
        let mut providers: Vec<(Arc<dyn Provider>, ProviderConfig)> = Vec::new();

        // OpenAI — activé si OPENAI_API_KEY est défini
        if let Ok(openai_key) = std::env::var("OPENAI_API_KEY") {
            if !openai_key.is_empty() {
                let base_url = std::env::var("OPENAI_BASE_URL").ok();
                let is_custom = base_url
                    .as_deref()
                    .map(|u| !u.contains("api.openai.com"))
                    .unwrap_or(false);

                let kind = if is_custom {
                    let name = base_url.as_deref().unwrap_or("custom").to_string();
                    tracing::info!(base_url = %name, "Custom OpenAI-compatible provider configured");
                    ProviderKind::Custom(name)
                } else {
                    tracing::info!("OpenAI provider configured");
                    ProviderKind::OpenAI
                };

                let mut config = ProviderConfig::new(kind, vec![ProviderKey::new(openai_key)]);
                if let Some(url) = base_url {
                    config.base_url = Some(url);
                }
                providers.push((Arc::new(OpenAIProvider::new()), config));
            }
        }

        // Anthropic — activé si ANTHROPIC_API_KEY est défini
        if let Ok(anthropic_key) = std::env::var("ANTHROPIC_API_KEY") {
            if !anthropic_key.is_empty() {
                tracing::info!("Anthropic provider configured");
                let config = ProviderConfig::new(
                    ProviderKind::Anthropic,
                    vec![ProviderKey::new(anthropic_key)],
                );
                providers.push((Arc::new(AnthropicProvider::new()), config));
            }
        }

        if providers.is_empty() {
            tracing::warn!("No providers configured. Set OPENAI_API_KEY or ANTHROPIC_API_KEY.");
        }

        let orchestrator = Arc::new(InferenceOrchestrator::new(providers, vec![]));
        let metrics = Arc::new(Metrics::new());
        let vk_registry = Arc::new(VirtualKeyRegistry::new());

        Ok(Self {
            orchestrator,
            metrics,
            vk_registry,
        })
    }
}
