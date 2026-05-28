pub mod budget_store;
pub mod config_store;
pub(crate) mod db_pool;
pub mod log_store;
pub mod model_catalog;
pub mod otel_plugin;
pub mod pg_log_store;
pub mod rag_plugin;
pub mod rate_limit_store;
pub mod use_cases;
pub mod virtual_key_store;

pub use virtual_key_store::VirtualKeyStore;

pub use budget_store::{BudgetPlugin, BudgetStore};
pub use config_store::ConfigStore;
pub use log_store::{
    build_log_entry, generate_log_id, now_ms, LogEntry, LogFilter, LogStats, LogStatus, LogStore,
};
pub use model_catalog::{ModelCatalog, ModelInfo};
pub use otel_plugin::{OtelConfig, OtelPlugin};
pub use pg_log_store::PgLogStore;
pub use rag_plugin::RagPlugin;
pub use rate_limit_store::{RateLimitPlugin, RateLimitStatus, RateLimitStore};
pub use use_cases::InferenceOrchestrator;

pub fn init() {
    tracing::debug!("Initializing Pylos Application");
}
