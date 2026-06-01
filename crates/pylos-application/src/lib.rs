pub mod budget_store;
pub mod config_store;
pub(crate) mod db_pool;
pub mod guardrails;
pub mod log_store;
pub mod model_catalog;
pub mod otel_plugin;
pub mod pg_log_store;
pub mod rag_plugin;
pub mod rate_limit_store;
pub mod semantic_cache;
pub mod system_prompt_store;

pub mod batching;
pub mod organization_store;
pub mod prefix_cache;
pub mod prompt_registry;
pub mod structured_output;
pub mod use_cases;
pub mod virtual_key_store;

pub use organization_store::OrganizationStore;
pub use system_prompt_store::SystemPromptStore;
pub use virtual_key_store::VirtualKeyStore;

pub use batching::BatchingPlugin;
pub use budget_store::{BudgetPlugin, BudgetStore};
pub use config_store::ConfigStore;
pub use guardrails::GuardrailsPlugin;
pub use log_store::{
    build_log_entry, generate_log_id, now_ms, LogEntry, LogFilter, LogStats, LogStatus, LogStore,
};
pub use model_catalog::{ModelCatalog, ModelInfo};
pub use otel_plugin::{OtelConfig, OtelPlugin};
pub use pg_log_store::PgLogStore;
pub use prefix_cache::PrefixCachePlugin;
pub use prompt_registry::PromptRegistryPlugin;
pub use rag_plugin::RagPlugin;
pub use rate_limit_store::{RateLimitPlugin, RateLimitStatus, RateLimitStore};
pub use semantic_cache::SemanticCachePlugin;

pub use structured_output::StructuredOutputPlugin;
pub use use_cases::InferenceOrchestrator;

pub fn init() {
    tracing::debug!("Initializing Pylos Application");
}
