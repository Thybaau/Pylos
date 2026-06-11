use axum::{
    body::Body,
    http::{HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{info_span, Instrument};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct RequestTrace {
    pub request_id: String,
    pub source: Option<String>,
}

/// Middleware qui injecte un X-Request-ID unique dans chaque requête,
/// et crée un span tracing pour le cycle de vie complet.
pub async fn request_id_middleware(mut req: Request<Body>, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            format!("pylos-{:x}-{:x}", fastrand::u32(..), counter)
        });

    let source = req
        .headers()
        .get("x-pylos-source")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let trace = RequestTrace {
        request_id: request_id.clone(),
        source: source.clone(),
    };
    req.extensions_mut().insert(trace.clone());

    let span = info_span!(
        "request",
        request_id = %request_id,
        source = %source.as_deref().unwrap_or("unknown"),
        method = %req.method(),
        uri = %req.uri(),
    );

    let mut res = next.run(req).instrument(span).await;

    // Injecter le X-Request-ID dans la réponse pour le client
    if let Ok(header_val) = HeaderValue::from_str(&request_id) {
        res.headers_mut().insert("x-request-id", header_val);
    }

    res
}
