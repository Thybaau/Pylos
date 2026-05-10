use std::path::PathBuf;
use std::sync::Arc;

use pylos_application::{ConfigStore, InferenceOrchestrator, LogStore};

use crate::metrics::Metrics;

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
                "No providers available. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, or create a pylos.json"
            );
        } else {
            tracing::info!(count = providers.len(), "Providers ready");
        }

        let orchestrator = Arc::new(InferenceOrchestrator::new(providers, vec![]));
        let metrics = Arc::new(Metrics::new());

        let db_path = {
            let dir = std::env::var("PYLOS_DATA_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            std::fs::create_dir_all(&dir).ok();
            let p = dir.join("pylos-logs.db");
            tracing::info!(path = %p.display(), "Log store path");
            p
        };

        let cfg = config_store.get().await;
        let retention_days = cfg.server.log_retention_days;
        let log_store = Arc::new(LogStore::new(Some(db_path), retention_days, 10_000));

        let vk_registry = Arc::new(pylos_core::domain::virtual_key::VirtualKeyRegistry::new());
        for vk_cfg in &cfg.governance.virtual_keys {
            if !vk_cfg.is_active {
                continue;
            }
            let key_value = vk_cfg
                .value
                .as_ref()
                .and_then(|v| v.resolve())
                .unwrap_or_else(|| {
                    format!("sk-pylos-{}", vk_cfg.id.replace(' ', "-").to_lowercase())
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
