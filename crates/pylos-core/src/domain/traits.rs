use crate::{
    domain::{
        embedding::{EmbeddingRequest, EmbeddingResponse},
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
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    /// Envoie une requête non-streaming et retourne la réponse complète
    /// Les providers doivent gérer PylosRequest::ChatCompletion.
    /// PylosRequest::TextCompletion est converti en ChatCompletion par l'orchestrateur.
    /// PylosRequest::Embedding doit retourner Err(Unsupported) ici.
    async fn complete(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<PylosResponse, PylosError>;

    /// Envoie une requête streaming
    async fn stream(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<ChunkStream, PylosError>;

    /// Calcule des embeddings
    async fn embed(
        &self,
        _request: &EmbeddingRequest,
        _config: &ProviderConfig,
    ) -> Result<EmbeddingResponse, PylosError> {
        Err(PylosError::Unsupported(format!(
            "Provider '{}' does not support embeddings",
            self.name()
        )))
    }

    /// Vérifie la santé du provider
    async fn health_check(&self, config: &ProviderConfig) -> Result<(), PylosError> {
        let _ = config;
        Ok(())
    }
}

/// Trait pour les plugins pre/post hook
#[async_trait]
pub trait LlmPlugin: Send + Sync {
    fn name(&self) -> &str;

    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        ctx: &mut crate::domain::request::RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        let _ = (request, ctx);
        Ok(None)
    }

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
