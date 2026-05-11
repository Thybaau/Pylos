use pylos_core::domain::provider::ProviderKind;

/// Déduit le provider depuis le nom du modèle.
pub fn guess_provider(model: &str) -> String {
    ProviderKind::guess_from_model(model).to_string()
}
