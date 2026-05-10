pub mod providers;

pub use providers::AnthropicProvider;
pub use providers::OpenAIProvider;

/// Initialise l'infrastructure (garde pour compatibilité et future initialisation globale)
pub fn init() {
    tracing::debug!("Initializing Pylos Infrastructure");
}
