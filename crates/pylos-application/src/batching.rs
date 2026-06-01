use async_trait::async_trait;
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::LlmPlugin;
use pylos_core::error::PylosError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Plugin de batching dynamique.
///
/// ATTENTION : Implémentation partielle — ce plugin accumule les requêtes
/// concurrentes pendant un délai configurable, mais ne les batch pas encore
/// au niveau du transport. Une vraie implémentation nécessite de grouper
/// les payloads avant l'envoi au provider amont (ex: OpenAI batching API).
///
/// TODO: Implémenter le batching via l'API OpenAI batch (/v1/batch)
///       ou un mécanisme de coalescing des requêtes.
pub struct BatchingPlugin {
    delay: Duration,
    lock: Arc<Mutex<()>>,
}

impl BatchingPlugin {
    pub fn new(delay_ms: u64) -> Self {
        warn!(
            "BatchingPlugin: NOT YET FULLY IMPLEMENTED. Configured with {}ms delay (stub behavior).",
            delay_ms
        );
        Self {
            delay: Duration::from_millis(delay_ms),
            lock: Arc::new(Mutex::new(())),
        }
    }
}

#[async_trait]
impl LlmPlugin for BatchingPlugin {
    fn name(&self) -> &str {
        "batching"
    }

    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        _ctx: &mut RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        let model = request.model();
        debug!(
            "BatchingPlugin: Queueing request for model '{}' in dynamic batch",
            model
        );

        tokio::time::sleep(self.delay).await;

        let _guard = self.lock.lock().await;
        debug!(
            "BatchingPlugin: Executing batched request group for model '{}'",
            model
        );

        Ok(None)
    }

    async fn post_hook(
        &self,
        _request: &PylosRequest,
        _response: &mut PylosResponse,
        _ctx: &mut RequestContext,
    ) -> Result<(), PylosError> {
        Ok(())
    }
}
