use axum::{extract::State, http::Request, middleware::Next, response::Response};
use tracing::warn;

use crate::interfaces::http::inference::error_response;
use crate::state::AppState;
use pylos_core::error::PylosError;

pub async fn queuing_middleware(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let max_concurrency = state.max_concurrency;

    // Check queue limit using in-flight requests count
    let in_flight = state.metrics.inference_in_flight.get() as usize;
    if in_flight >= max_concurrency + state.max_queue_size {
        warn!(
            in_flight = in_flight,
            max = max_concurrency + state.max_queue_size,
            "Inference queue is full"
        );
        return error_response(&PylosError::RateLimitExceeded(
            "Inference queue is full".into(),
        ));
    }

    // RAII guard to decrement when request completes/fails/cancels
    struct InFlightGuard {
        metrics: std::sync::Arc<crate::metrics::Metrics>,
    }
    impl Drop for InFlightGuard {
        fn drop(&mut self) {
            self.metrics.inference_in_flight.dec();
        }
    }

    state.metrics.inference_in_flight.inc();
    let _guard = InFlightGuard {
        metrics: state.metrics.clone(),
    };

    // Acquire permit with timeout
    let semaphore = state.inference_semaphore.clone();
    let timeout_duration = std::time::Duration::from_millis(state.queue_timeout_ms);

    let _permit =
        match tokio::time::timeout(
            timeout_duration,
            async move { semaphore.acquire_owned().await },
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            Ok(Err(e)) => {
                return error_response(&PylosError::Internal(format!(
                    "Semaphore acquisition failed: {}",
                    e
                )));
            }
            Err(_) => {
                warn!("Queue timeout exceeded waiting for concurrency permit");
                return error_response(&PylosError::Timeout("Queue timeout exceeded".into()));
            }
        };

    // Forward to next handler
    next.run(req).await
}
