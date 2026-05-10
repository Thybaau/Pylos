use std::sync::Arc;

use prometheus::{
    register_histogram_vec_with_registry, register_int_counter_vec_with_registry,
    register_int_gauge_with_registry, Encoder, HistogramVec, IntCounterVec, IntGauge, Registry,
    TextEncoder,
};

/// Métriques Prometheus exposées par Pylos
/// Équivalent du plugin telemetry de bifrost
#[allow(dead_code)]
#[derive(Clone)]
pub struct Metrics {
    pub registry: Arc<Registry>,

    /// Nombre total de requêtes d'inférence
    pub inference_requests_total: IntCounterVec,

    /// Nombre de requêtes réussies
    pub inference_success_total: IntCounterVec,

    /// Nombre de requêtes échouées
    pub inference_errors_total: IntCounterVec,

    /// Latence des requêtes d'inférence en secondes
    pub inference_duration_seconds: HistogramVec,

    /// Nombre de tokens utilisés (prompt)
    pub prompt_tokens_total: IntCounterVec,

    /// Nombre de tokens générés (completion)
    pub completion_tokens_total: IntCounterVec,

    /// Requêtes en cours
    pub inference_in_flight: IntGauge,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let inference_requests_total = register_int_counter_vec_with_registry!(
            "pylos_inference_requests_total",
            "Total number of inference requests",
            &["provider", "model", "request_type"],
            registry
        )
        .expect("Failed to register inference_requests_total");

        let inference_success_total = register_int_counter_vec_with_registry!(
            "pylos_inference_success_total",
            "Total number of successful inference requests",
            &["provider", "model"],
            registry
        )
        .expect("Failed to register inference_success_total");

        let inference_errors_total = register_int_counter_vec_with_registry!(
            "pylos_inference_errors_total",
            "Total number of failed inference requests",
            &["provider", "error_type"],
            registry
        )
        .expect("Failed to register inference_errors_total");

        let inference_duration_seconds = register_histogram_vec_with_registry!(
            "pylos_inference_duration_seconds",
            "Inference request duration in seconds",
            &["provider", "model"],
            vec![0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0],
            registry
        )
        .expect("Failed to register inference_duration_seconds");

        let prompt_tokens_total = register_int_counter_vec_with_registry!(
            "pylos_prompt_tokens_total",
            "Total number of prompt tokens used",
            &["provider", "model"],
            registry
        )
        .expect("Failed to register prompt_tokens_total");

        let completion_tokens_total = register_int_counter_vec_with_registry!(
            "pylos_completion_tokens_total",
            "Total number of completion tokens generated",
            &["provider", "model"],
            registry
        )
        .expect("Failed to register completion_tokens_total");

        let inference_in_flight = register_int_gauge_with_registry!(
            "pylos_inference_in_flight",
            "Number of inference requests currently in flight",
            registry
        )
        .expect("Failed to register inference_in_flight");

        Self {
            registry: Arc::new(registry),
            inference_requests_total,
            inference_success_total,
            inference_errors_total,
            inference_duration_seconds,
            prompt_tokens_total,
            completion_tokens_total,
            inference_in_flight,
        }
    }

    /// Sérialise les métriques au format texte Prometheus
    pub fn export(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .unwrap_or_default();
        String::from_utf8(buffer).unwrap_or_default()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
