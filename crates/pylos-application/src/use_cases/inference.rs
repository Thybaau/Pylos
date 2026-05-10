use std::sync::Arc;
use std::time::Duration;

use tracing::{debug, info, warn};

use pylos_core::domain::provider::ProviderConfig;
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::{ChunkStream, LlmPlugin, Provider};
use pylos_core::error::PylosError;

/// Orchestre une requête sur un ou plusieurs providers avec retry et fallback
/// Équivalent de core/inference.go (bifrost) — SendRequest / inferenceLoopHelper
pub struct InferenceOrchestrator {
    providers: Vec<(Arc<dyn Provider>, ProviderConfig)>,
    plugins: Vec<Arc<dyn LlmPlugin>>,
}

impl InferenceOrchestrator {
    pub fn new(
        providers: Vec<(Arc<dyn Provider>, ProviderConfig)>,
        plugins: Vec<Arc<dyn LlmPlugin>>,
    ) -> Self {
        Self { providers, plugins }
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

        // Essaie chaque provider dans l'ordre (fallback chain)
        let mut last_error: Option<PylosError> = None;

        for (provider, config) in &self.providers {
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
                    // Court-circuit sur streaming : on retourne un stream d'une seule réponse
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

        let mut last_error: Option<PylosError> = None;

        for (provider, config) in &self.providers {
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
/// Formule : min(initial * 2^(attempt-1) + jitter, max)
fn exponential_backoff(attempt: u32, initial_ms: u64, max_ms: u64) -> u64 {
    let shift = attempt.saturating_sub(1).min(62);
    let base = initial_ms.saturating_mul(1u64 << shift);
    // Jitter : ±20% pour éviter les thundering herds
    let jitter = (base as f64 * 0.2 * (fastrand::f64() * 2.0 - 1.0)) as i64;
    let backoff = (base as i64 + jitter).max(0) as u64;
    backoff.min(max_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff_bounds() {
        // Le backoff ne doit jamais dépasser max_ms
        for attempt in 1..=10 {
            let b = exponential_backoff(attempt, 100, 5_000);
            assert!(
                b <= 5_000,
                "Backoff {} exceeded max at attempt {}",
                b,
                attempt
            );
        }
        // Le premier essai doit être proche de initial_ms
        for _ in 0..20 {
            let b = exponential_backoff(1, 100, 5_000);
            // Avec ±20% de jitter : 80..120 ms
            assert!(b <= 120, "First backoff too high: {}", b);
        }
    }
}
