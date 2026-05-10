use crate::{
    domain::{
        provider::ProviderConfig,
        request::{PylosRequest, PylosResponse, StreamChunk},
    },
    error::PylosError,
};
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

/// Type alias pour un stream de chunks SSE
pub type ChunkStream = Pin<Box<dyn Stream<Item = Result<StreamChunk, PylosError>> + Send>>;

/// Trait central — chaque provider implémente cette interface
/// Équivalent de l'interface ModelProvider en Go (bifrost/core/providers/)
#[async_trait]
pub trait Provider: Send + Sync {
    /// Identifiant lisible du provider (ex: "openai", "anthropic")
    fn name(&self) -> &str;

    /// Envoie une requête non-streaming et retourne la réponse complète
    async fn complete(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<PylosResponse, PylosError>;

    /// Envoie une requête streaming et retourne un stream de chunks
    async fn stream(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<ChunkStream, PylosError>;

    /// Vérifie la santé du provider (optionnel — défaut : Ok)
    async fn health_check(&self, config: &ProviderConfig) -> Result<(), PylosError> {
        let _ = config;
        Ok(())
    }
}

/// Trait pour les plugins pre/post hook — équivalent de LLMPlugin en Go
#[async_trait]
pub trait LlmPlugin: Send + Sync {
    fn name(&self) -> &str;

    /// Hook exécuté AVANT l'envoi au provider
    /// Peut modifier la requête ou court-circuiter (retourner une réponse directement)
    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        ctx: &mut crate::domain::request::RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        let _ = (request, ctx);
        Ok(None) // Pas de court-circuit par défaut
    }

    /// Hook exécuté APRÈS réception de la réponse du provider
    /// Peut modifier ou enrichir la réponse
    async fn post_hook(
        &self,
        request: &PylosRequest,
        response: &mut PylosResponse,
        ctx: &mut crate::domain::request::RequestContext,
    ) -> Result<(), PylosError> {
        let _ = (request, response, ctx);
        Ok(())
    }
}
