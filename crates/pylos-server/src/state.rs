use std::path::PathBuf;
use std::sync::Arc;

use pylos_application::{
    BudgetPlugin, BudgetStore, ConfigStore, GuardrailsPlugin, InferenceOrchestrator, LogStore,
    ModelCatalog, OrganizationStore, OtelConfig, PgLogStore, RateLimitPlugin, RateLimitStore,
    SemanticCachePlugin, StructuredOutputPlugin, VirtualKeyStore,
};
use pylos_core::domain::traits::LlmPlugin;

use crate::metrics::Metrics;

#[derive(Clone)]
pub enum LogStoreVariant {
    Sqlite(Arc<LogStore>),
    Postgres(Arc<PgLogStore>),
}

impl LogStoreVariant {
    pub async fn push(&self, entry: pylos_application::log_store::LogEntry) {
        match self {
            Self::Sqlite(s) => s.push(entry).await,
            Self::Postgres(s) => s.push(entry).await,
        }
    }

    pub async fn list(
        &self,
        limit: usize,
        offset: usize,
        filter: &pylos_application::log_store::LogFilter,
    ) -> (Vec<pylos_application::log_store::LogEntry>, u64) {
        match self {
            Self::Sqlite(s) => s.list(limit, offset, filter).await,
            Self::Postgres(s) => s.list(limit, offset, filter).await,
        }
    }

    pub async fn stats(
        &self,
        filter: &pylos_application::log_store::LogFilter,
    ) -> pylos_application::log_store::LogStats {
        match self {
            Self::Sqlite(s) => s.stats(filter).await,
            Self::Postgres(s) => s.stats(filter).await,
        }
    }

    pub async fn histogram(
        &self,
        filter: &pylos_application::log_store::LogFilter,
        bucket_secs: i64,
    ) -> Vec<pylos_application::log_store::HistogramBucket> {
        match self {
            Self::Sqlite(s) => s.histogram(filter, bucket_secs).await,
            Self::Postgres(s) => s.histogram(filter, bucket_secs).await,
        }
    }

    pub async fn token_histogram(
        &self,
        filter: &pylos_application::log_store::LogFilter,
        bucket_secs: i64,
    ) -> Vec<pylos_application::log_store::TokenBucket> {
        match self {
            Self::Sqlite(s) => s.token_histogram(filter, bucket_secs).await,
            Self::Postgres(s) => s.token_histogram(filter, bucket_secs).await,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub orchestrator: Arc<InferenceOrchestrator>,
    pub config_store: Arc<ConfigStore>,
    pub metrics: Arc<Metrics>,
    pub vk_registry: Arc<pylos_core::domain::virtual_key::VirtualKeyRegistry>,
    pub log_store: LogStoreVariant,
    pub model_catalog: Arc<ModelCatalog>,
    pub budget_store: Arc<BudgetStore>,
    pub rate_limit_store: Arc<RateLimitStore>,
    pub vk_store: Arc<VirtualKeyStore>,
    pub system_prompt_store: Arc<pylos_application::SystemPromptStore>,
    pub org_store: Arc<OrganizationStore>,
    pub admin_key: Option<String>,
    pub allowed_origins: Vec<String>,
    pub inference_semaphore: Arc<tokio::sync::Semaphore>,
    pub max_concurrency: usize,
    pub max_queue_size: usize,
    pub queue_timeout_ms: u64,
}

impl AppState {
    pub async fn from_config(config_path: Option<PathBuf>) -> anyhow::Result<Self> {
        Self::from_config_with_dir(config_path, None).await
    }

    pub async fn from_config_with_dir(
        config_path: Option<PathBuf>,
        data_dir: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let config_store = ConfigStore::load(config_path.as_deref()).await?;
        let config_store = Arc::new(config_store);

        let cfg = config_store.get().await;

        // ── Providers ────────────────────────────────────────────────────
        let providers = config_store.runtime_providers().await;
        if providers.is_empty() {
            tracing::warn!(
                "No providers available. Set OPENAI_API_KEY, ANTHROPIC_API_KEY, or create a pylos.json"
            );
        } else {
            tracing::info!(count = providers.len(), "Providers ready");
        }

        // ── Data directory ───────────────────────────────────────────────
        let database_url = cfg.server.database_url.as_ref().and_then(|e| e.resolve());

        let (
            log_store,
            model_catalog,
            budget_store,
            rate_limit_store,
            vk_store,
            system_prompt_store,
            org_store,
        ) = if let Some(ref db_url) = database_url {
            let db_scheme = db_url.split(':').next().unwrap_or("unknown");
            tracing::info!(database_url = %format!("{}://***@***/***", db_scheme), "Using PostgreSQL for all stores");

            // PostgreSQL
            let pg_log = Arc::new(
                PgLogStore::new(db_url, cfg.server.log_retention_days)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to connect PostgreSQL log store: {}", e)
                    })?,
            );

            let pg_catalog =
                Arc::new(ModelCatalog::open_postgres(db_url).await.map_err(|e| {
                    anyhow::anyhow!("Failed to open PostgreSQL model catalog: {}", e)
                })?);

            let pg_budget =
                Arc::new(BudgetStore::open_postgres(db_url).await.map_err(|e| {
                    anyhow::anyhow!("Failed to open PostgreSQL budget store: {}", e)
                })?);

            let pg_rl = Arc::new(RateLimitStore::open_postgres(db_url).await.map_err(|e| {
                anyhow::anyhow!("Failed to open PostgreSQL rate limit store: {}", e)
            })?);

            let pg_vk = Arc::new(VirtualKeyStore::open_postgres(db_url).await.map_err(|e| {
                anyhow::anyhow!("Failed to open PostgreSQL virtual key store: {}", e)
            })?);

            let pg_prompts = Arc::new(
                pylos_application::SystemPromptStore::open_postgres(db_url)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to open PostgreSQL system prompt store: {}", e)
                    })?,
            );

            let pg_org = Arc::new(
                OrganizationStore::open_postgres(db_url)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to open PostgreSQL organization store: {}", e)
                    })?,
            );

            if let Err(e) = config_store.init_database(db_url).await {
                tracing::warn!(error = %e, "Failed to initialize PostgreSQL config store");
            }

            (
                LogStoreVariant::Postgres(pg_log),
                pg_catalog,
                pg_budget,
                pg_rl,
                pg_vk,
                pg_prompts,
                pg_org,
            )
        } else {
            // SQLite
            let data_dir = data_dir.unwrap_or_else(|| {
                std::env::var("PYLOS_DATA_DIR")
                    .ok()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."))
            });
            std::fs::create_dir_all(&data_dir).ok();

            let log_db_path = data_dir.join("pylos-logs.db");
            tracing::info!(path = %log_db_path.display(), "Log store path (SQLite)");
            let sqlite_log = Arc::new(LogStore::new(
                Some(log_db_path),
                cfg.server.log_retention_days,
                10_000,
            ));

            let catalog_db_path = data_dir.join("pylos-catalog.db");
            let sqlite_catalog = Arc::new(
                ModelCatalog::open(&catalog_db_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open model catalog: {}", e))?,
            );

            let budget_db_path = data_dir.join("pylos-budget.db");
            let sqlite_budget = Arc::new(
                BudgetStore::open(&budget_db_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open budget store: {}", e))?,
            );

            let rl_db_path = data_dir.join("pylos-ratelimit.db");
            let sqlite_rl = Arc::new(
                RateLimitStore::open(&rl_db_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open rate limit store: {}", e))?,
            );

            let vk_db_path = data_dir.join("pylos-virtualkeys.db");
            let sqlite_vk = Arc::new(
                VirtualKeyStore::open(&vk_db_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open virtual key store: {}", e))?,
            );

            let org_db_path = data_dir.join("pylos-org.db");
            let sqlite_org = Arc::new(
                OrganizationStore::open(&org_db_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open organization store: {}", e))?,
            );

            let prompts_db_path = data_dir.join("pylos-prompts.db");
            let sqlite_prompts = Arc::new(
                pylos_application::SystemPromptStore::open(&prompts_db_path)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to open system prompt store: {}", e))?,
            );

            let config_db_path = data_dir.join("pylos-config.db");
            let sqlite_config_db_url =
                format!("sqlite://{}?mode=rwc", config_db_path.to_string_lossy());
            if let Err(e) = config_store.init_database(&sqlite_config_db_url).await {
                tracing::warn!(error = %e, "Failed to initialize SQLite config store");
            }

            (
                LogStoreVariant::Sqlite(sqlite_log),
                sqlite_catalog,
                sqlite_budget,
                sqlite_rl,
                sqlite_vk,
                sqlite_prompts,
                sqlite_org,
            )
        };

        for budget_cfg in &cfg.governance.budgets {
            if let Some(vk_id) = &budget_cfg.virtual_key_id {
                if let Err(e) = budget_store.upsert_budget(vk_id, budget_cfg).await {
                    tracing::warn!(budget_id = %budget_cfg.id, error = %e, "Failed to init budget");
                }
            }
        }

        for vk_cfg in &cfg.governance.virtual_keys {
            if let Some(rl_id) = &vk_cfg.rate_limit_id {
                if let Some(rl_cfg) = cfg.governance.rate_limits.iter().find(|r| &r.id == rl_id) {
                    if let Err(e) = rate_limit_store.upsert_rate_limit(&vk_cfg.id, rl_cfg).await {
                        tracing::warn!(vk_id = %vk_cfg.id, error = %e, "Failed to init rate limit");
                    }
                }
            }
        }

        // ── Plugins ────────────────────────────────────────────────────────
        let mut plugins: Vec<Arc<dyn LlmPlugin>> = Vec::new();

        // Structured output verification plugin (unconditional)
        plugins.push(Arc::new(StructuredOutputPlugin::new()));

        // Rag plugin
        let qdrant_url =
            std::env::var("QDRANT_URL").unwrap_or_else(|_| "http://qdrant:6333".to_string());
        let collection_name =
            std::env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "emails".to_string());
        let pylos_base_url =
            std::env::var("PYLOS_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let pylos_api_key = std::env::var("PYLOS_API_KEY").ok();
        let embedding_model = std::env::var("PYLOS_EMBEDDING_MODEL")
            .unwrap_or_else(|_| "nomic-embed-text-v2-moe-GGUF".to_string());
        let pylos_model =
            std::env::var("PYLOS_MODEL").unwrap_or_else(|_| "deepseek-coder-v2:16b".to_string());

        plugins.push(Arc::new(pylos_application::RagPlugin::new(
            qdrant_url,
            collection_name,
            pylos_base_url,
            pylos_api_key,
            embedding_model,
            pylos_model,
        )));
        tracing::info!("RagPlugin registered");

        // Budget plugin
        if !cfg.governance.budgets.is_empty() {
            plugins.push(Arc::new(BudgetPlugin::new(Arc::clone(&budget_store))));
            tracing::info!(
                count = cfg.governance.budgets.len(),
                "Budget plugin enabled"
            );
        }

        // Rate limit plugin (SQLite persistant)
        let has_rl = cfg
            .governance
            .rate_limits
            .iter()
            .any(|r| r.request_max_limit > 0 || r.token_max_limit > 0);
        if has_rl {
            plugins.push(Arc::new(RateLimitPlugin::new(Arc::clone(
                &rate_limit_store,
            ))));
            tracing::info!("Rate limit plugin enabled");
        }

        // Plugins déclarés dans la config (OTel, etc.)
        for plugin_cfg in &cfg.plugins {
            if !plugin_cfg.enabled {
                continue;
            }
            match plugin_cfg.name.as_str() {
                "otel" => {
                    let otel_cfg = OtelConfig::from_plugin_config(&plugin_cfg.config);
                    plugins.push(Arc::new(otel_cfg.build_plugin()));
                    tracing::info!(name = "otel", "Plugin registered");
                }
                "semantic_cache" => {
                    let qdrant_url = std::env::var("QDRANT_URL")
                        .unwrap_or_else(|_| "http://qdrant:6333".to_string());
                    let collection_name = plugin_cfg
                        .config
                        .get("collection_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pylos-cache")
                        .to_string();
                    let pylos_base_url = std::env::var("PYLOS_BASE_URL")
                        .unwrap_or_else(|_| "http://localhost:3000".to_string());
                    let pylos_api_key = std::env::var("PYLOS_API_KEY").ok();
                    let embedding_model = plugin_cfg
                        .config
                        .get("embedding_model")
                        .and_then(|v| v.as_str())
                        .unwrap_or("nomic-embed-text-v2-moe-GGUF")
                        .to_string();
                    let similarity_threshold = plugin_cfg
                        .config
                        .get("similarity_threshold")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.9);
                    let ttl_secs = plugin_cfg
                        .config
                        .get("ttl_secs")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(86400);

                    let plugin = SemanticCachePlugin::new(
                        qdrant_url,
                        collection_name,
                        pylos_base_url,
                        pylos_api_key,
                        embedding_model,
                        similarity_threshold,
                        ttl_secs,
                    );
                    plugins.push(Arc::new(plugin));
                    tracing::info!(name = "semantic_cache", "Semantic Cache plugin enabled");
                }

                "guardrails" => {
                    let plugin = GuardrailsPlugin::new(Arc::clone(&config_store));
                    plugins.push(Arc::new(plugin));
                    tracing::info!(name = "guardrails", "Guardrails plugin enabled");
                }
                name => {
                    tracing::debug!(name = %name, "Unknown plugin, skipping");
                }
            }
        }

        // ── Orchestrator ──────────────────────────────────────────────────
        let orchestrator = Arc::new(InferenceOrchestrator::new(providers, plugins));
        let metrics = Arc::new(Metrics::new());

        // ── Virtual key registry ──────────────────────────────────────────
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

        if let Ok(db_vks) = vk_store.list_keys().await {
            for vk_cfg in db_vks {
                if !vk_cfg.is_active {
                    continue;
                }
                let key_value = vk_cfg
                    .value
                    .as_ref()
                    .map(|v| v.resolve().unwrap_or_default())
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
        }

        let max_concurrency = cfg.server.queuing.max_concurrency;
        let max_queue_size = cfg.server.queuing.max_queue_size;
        let queue_timeout_ms = cfg.server.queuing.queue_timeout_ms;
        let inference_semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));

        Ok(Self {
            orchestrator,
            config_store,
            metrics,
            vk_registry,
            log_store,
            model_catalog,
            budget_store,
            rate_limit_store,
            vk_store,
            system_prompt_store,
            org_store,
            admin_key: std::env::var("PYLOS_ADMIN_KEY").ok(),
            allowed_origins: cfg.server.allowed_origins.clone(),
            inference_semaphore,
            max_concurrency,
            max_queue_size,
            queue_timeout_ms,
        })
    }
}
