use async_trait::async_trait;
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::LlmPlugin;
use pylos_core::error::PylosError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::debug;

pub struct BatchingPlugin {
    delay: Duration,
    // Mutex pour simuler l'orchestration du batching des requêtes concurrentes vers l'amont
    lock: Arc<Mutex<()>>,
}

impl BatchingPlugin {
    pub fn new(delay_ms: u64) -> Self {
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

        // Attente dynamique pour accumuler d'autres requêtes concurrentes
        tokio::time::sleep(self.delay).await;

        // Acquisition exclusive du canal d'inférence par lot pour simuler l'exécution
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
