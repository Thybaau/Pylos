pub mod use_cases;

pub use use_cases::InferenceOrchestrator;

/// Initialise la couche application
pub fn init() {
    tracing::debug!("Initializing Pylos Application");
}
