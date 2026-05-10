pub mod config_store;
pub mod use_cases;

pub use config_store::ConfigStore;
pub use use_cases::InferenceOrchestrator;

/// Initialise la couche application
pub fn init() {
    tracing::debug!("Initializing Pylos Application");
}
