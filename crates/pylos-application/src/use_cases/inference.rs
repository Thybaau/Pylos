use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use pylos_core::domain::provider::ProviderConfig;
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::{ChunkStream, LlmPlugin, Provider};
use pylos_core::error::PylosError;

type ProviderList = Vec<(Arc<dyn Provider>, ProviderConfig)>;

/// Orchestre une requête sur un ou plusieurs providers avec retry et fallback
/// Équivalent de core/inference.go (bifrost) — SendRequest / inferenceLoopHelper
pub struct InferenceOrchestrator {
    /// RwLock pour permettre le hot-reload des providers sans redémarrage
    /// Identique au mécanisme atomic.Pointer de bifrost pour les plugins
    providers: Arc<RwLock<ProviderList>>,
    plugins: Vec<Arc<dyn LlmPlugin>>,
}

impl InferenceOrchestrator {
    pub fn new(providers: ProviderList, plugins: Vec<Arc<dyn LlmPlugin>>) -> Self {
        Self {
            providers: Arc::new(RwLock::new(providers)),
            plugins,
        }
    }

    /// Hot-reload des providers sans interruption de service
    pub async fn update_providers(&self, new_providers: ProviderList) {
        let mut guard = self.providers.write().await;
        *guard = new_providers;
        info!("Inference providers updated (hot-reload)");
    }

    /// Envoie une requête non-streaming avec pre/post hooks et fallback
    pub async fn complete(
        &self,
        mut request: PylosRequest,
        mut ctx: RequestContext,
    ) -> Result<PylosResponse, PylosError> {
        // Pre-hooks (ordre d'enregistrement — comme bifrost)
        for plugin in &self.plugins {
            match plugin.pre_hook(&mut request, &mut ctx).await {
                Ok(Some(short_circuit)) => {
                    debug!(plugin = plugin.name(), "Pre-hook short-circuited request");
                    return Ok(short_circuit);
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(plugin = plugin.name(), error = %e, "Pre-hook error (ignored)");
                }
            }
        }

        // Snapshot des providers (lecture lock-free pendant l'inférence)
        let providers = self.providers.read().await;
        let mut last_error: Option<PylosError> = None;

        // Tri des providers : ceux qui supportent explicitement le modèle demandé passent en premier
        let model = request.model().to_string();
        let mut ordered: Vec<_> = providers.iter().collect();
        ordered.sort_by_key(|(provider, config)| {
            let supports = model_supported_by(config, &model);
            if supports {
                0u8
            } else {
                1u8
            }
        });

        for (provider, config) in ordered {
            if ctx.tried_providers.contains(&provider.name().to_string()) {
                debug!(
                    provider = provider.name(),
                    "Skipping already-tried provider"
                );
                continue;
            }

            ctx.tried_providers.push(provider.name().to_string());

            match self
                .try_complete_with_retry(provider.as_ref(), config, &request)
                .await
            {
                Ok(mut response) => {
                    // Post-hooks (ordre inverse — LIFO comme bifrost)
                    for plugin in self.plugins.iter().rev() {
                        if let Err(e) = plugin.post_hook(&request, &mut response, &mut ctx).await {
                            warn!(plugin = plugin.name(), error = %e, "Post-hook error (ignored)");
                        }
                    }
                    info!(
                        provider = provider.name(),
                        model = request.model(),
                        "Inference successful"
                    );
                    return Ok(response);
                }
                Err(e) => {
                    warn!(
                        provider = provider.name(),
                        error = %e,
                        retriable = e.is_retriable(),
                        "Provider failed, trying fallback"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| PylosError::AllProvidersFailed("No providers configured".into())))
    }

    /// Envoie une requête streaming avec fallback
    pub async fn stream(
        &self,
        mut request: PylosRequest,
        mut ctx: RequestContext,
    ) -> Result<ChunkStream, PylosError> {
        // Pre-hooks
        for plugin in &self.plugins {
            match plugin.pre_hook(&mut request, &mut ctx).await {
                Ok(Some(_short_circuit)) => {
                    return Err(PylosError::Internal(
                        "Streaming short-circuit not yet supported".into(),
                    ));
                }
                Ok(None) => {}
                Err(e) => {
                    warn!(plugin = plugin.name(), error = %e, "Pre-hook stream error (ignored)");
                }
            }
        }

        let providers = self.providers.read().await;
        let mut last_error: Option<PylosError> = None;

        let stream_model = request.model().to_string();
        let mut ordered_stream: Vec<_> = providers.iter().collect();
        ordered_stream.sort_by_key(|(_, config)| {
            if model_supported_by(config, &stream_model) {
                0u8
            } else {
                1u8
            }
        });

        for (provider, config) in ordered_stream {
            if ctx.tried_providers.contains(&provider.name().to_string()) {
                continue;
            }
            ctx.tried_providers.push(provider.name().to_string());

            match provider.stream(&request, config).await {
                Ok(stream) => {
                    info!(
                        provider = provider.name(),
                        model = request.model(),
                        "Streaming inference started"
                    );
                    return Ok(stream);
                }
                Err(e) => {
                    warn!(provider = provider.name(), error = %e, "Provider stream failed, trying fallback");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| PylosError::AllProvidersFailed("No providers configured".into())))
    }

    /// Retry avec backoff exponentiel sur un provider donné
    async fn try_complete_with_retry(
        &self,
        provider: &dyn Provider,
        config: &ProviderConfig,
        request: &PylosRequest,
    ) -> Result<PylosResponse, PylosError> {
        let mut attempt = 0u32;
        let max_retries = config.max_retries;

        loop {
            match provider.complete(request, config).await {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_retriable() && attempt < max_retries => {
                    attempt += 1;
                    let backoff = exponential_backoff(
                        attempt,
                        config.retry_backoff_initial_ms,
                        config.retry_backoff_max_ms,
                    );
                    debug!(
                        provider = provider.name(),
                        attempt = attempt,
                        backoff_ms = backoff,
                        error = %e,
                        "Retrying after backoff"
                    );
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

/// Calcule le délai de backoff exponentiel avec jitter
fn exponential_backoff(attempt: u32, initial_ms: u64, max_ms: u64) -> u64 {
    let shift = attempt.saturating_sub(1).min(62);
    let base = initial_ms.saturating_mul(1u64 << shift);
    let jitter = (base as f64 * 0.2 * (fastrand::f64() * 2.0 - 1.0)) as i64;
    let backoff = (base as i64 + jitter).max(0) as u64;
    backoff.min(max_ms)
}

/// Détermine si un provider supporte explicitement un modèle donné.
/// Retourne true si :
///   - le provider a une clé avec ["*"] (wildcard)
///   - le provider a une clé avec le modèle exact dans sa liste
///   - Bedrock : le provider est bedrock ET le modèle contient des marqueurs bedrock
/// Retourne false uniquement si toutes les clés ont des listes explicites
/// qui n'incluent pas le modèle — ce qui déclenche le fallback vers les autres providers.
fn model_supported_by(config: &ProviderConfig, model: &str) -> bool {
    use pylos_core::domain::provider::ProviderKind;

    // Bedrock : supporte les modèles avec préfixe us./eu./ap. ou "amazon."/"anthropic."
    if config.kind == ProviderKind::Bedrock {
        return model.starts_with("us.")
            || model.starts_with("eu.")
            || model.starts_with("ap.")
            || model.starts_with("amazon.")
            || model.starts_with("anthropic.")
            || model.contains("nova")
            || model.contains("titan");
    }

    // Pour les autres providers : regarde les modèles déclarés dans la config source
    // Le runtime ProviderConfig ne contient pas les listes de modèles —
    // on se base sur l'heuristique du nom du provider et du modèle
    match &config.kind {
        ProviderKind::OpenAI => {
            model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("o3")
        }
        ProviderKind::Anthropic => model.contains("claude"),
        ProviderKind::Custom(name) => {
            match name.as_str() {
                // Ollama : pas de préfixe de provider dans le nom de modèle
                "ollama" => {
                    !model.starts_with("gpt")
                        && !model.starts_with("claude")
                        && !model.contains('/')
                        && !model.starts_with("us.")
                        && !model.starts_with("amazon.")
                        && !model.starts_with("anthropic.")
                }
                // OpenRouter : format "provider/model"
                "openrouter" | _ if name.contains("openrouter") => model.contains('/'),
                // Custom générique : tente toujours
                _ => true,
            }
        }
        _ => true, // Tente pour les providers non reconnus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_bounds() {
        for attempt in 1..=10 {
            let b = exponential_backoff(attempt, 100, 5_000);
            assert!(
                b <= 5_000,
                "Backoff {} exceeded max at attempt {}",
                b,
                attempt
            );
        }
        for _ in 0..20 {
            let b = exponential_backoff(1, 100, 5_000);
            assert!(b <= 120, "First backoff too high: {}", b);
        }
    }
}
