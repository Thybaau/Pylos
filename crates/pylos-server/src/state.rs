use std::path::PathBuf;
use std::sync::Arc;

use pylos_application::{ConfigStore, InferenceOrchestrator, LogStore};

use crate::metrics::Metrics;

/// État global partagé entre tous les handlers Axum
#[derive(Clone)]
pub struct AppState {
    pub orchestrator: Arc<InferenceOrchestrator>,
    pub config_store: Arc<ConfigStore>,
    pub metrics: Arc<Metrics>,
    pub vk_registry: Arc<pylos_core::domain::virtual_key::VirtualKeyRegistry>,
    pub log_store: Arc<LogStore>,
}

impl AppState {
    pub async fn from_config(config_path: Option<PathBuf>) -> anyhow::Result<Self> {
        let config_store = ConfigStore::load(config_path.as_deref()).await?;
        let config_store = Arc::new(config_store);

        let providers = config_store.runtime_providers().await;

        if providers.is_empty() {
            tracing::warn!(
                "No providers available. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, or create a pylos.json config file."
            );
        } else {
            tracing::info!(count = providers.len(), "Providers ready");
        }

        let orchestrator = Arc::new(InferenceOrchestrator::new(providers, vec![]));
        let metrics = Arc::new(Metrics::new());
        let log_store = Arc::new(LogStore::new(10_000));

        let vk_registry = Arc::new(pylos_core::domain::virtual_key::VirtualKeyRegistry::new());
        let cfg = config_store.get().await;
        for vk_cfg in &cfg.governance.virtual_keys {
            if !vk_cfg.is_active {
                continue;
            }
            let key_value = vk_cfg
                .value
                .as_ref()
                .and_then(|v| v.resolve())
                .unwrap_or_else(|| {
                    format!("sk-pylos-{}", &vk_cfg.id.replace(' ', "-").to_lowercase())
                });

            let rate_limit = cfg
                .governance
                .rate_limits
                .iter()
                .find(|rl| Some(&rl.id) == vk_cfg.rate_limit_id.as_ref())
                .map(|rl| rl.request_max_limit)
                .unwrap_or(0);

            let vk = pylos_core::domain::virtual_key::VirtualKey::new(key_value, &vk_cfg.name)
                .with_rpm(rate_limit);

            vk_registry.register(vk).await;
        }

        Ok(Self {
            orchestrator,
            config_store,
            metrics,
            vk_registry,
            log_store,
        })
    }
}
