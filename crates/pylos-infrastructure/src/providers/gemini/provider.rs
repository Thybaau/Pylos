use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use tracing::{debug, warn};

use pylos_core::domain::embedding::{EmbeddingRequest, EmbeddingResponse};
use pylos_core::domain::provider::ProviderConfig;
use pylos_core::domain::request::{PylosRequest, PylosResponse};
use pylos_core::domain::traits::{ChunkStream, Provider};
use pylos_core::error::PylosError;

use super::converters::{
    from_gemini_embed_response, from_gemini_response, from_gemini_stream_chunk, map_gemini_error,
    to_gemini_embed_request, to_gemini_request, GeminiBatchEmbedResponse, GeminiResponse,
};

const DEFAULT_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

// ─────────────────────────────────────────────────────────────────────────────
// GeminiProvider — Google Gemini API
// Bifrost source: core/providers/gemini/gemini.go
//
// Auth: header "x-goog-api-key: <key>"
// URL: https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent
// Streaming: /models/{model}:streamGenerateContent?alt=sse
// ─────────────────────────────────────────────────────────────────────────────

pub struct GeminiProvider {
    client: Client,
}

impl GeminiProvider {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");
        Self { client }
    }

    fn base_url<'a>(&self, config: &'a ProviderConfig) -> &'a str {
        config.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL)
    }

    fn select_key<'a>(&self, config: &'a ProviderConfig) -> Option<&'a str> {
        if config.keys.is_empty() {
            return None;
        }
        let total_weight: f64 = config.keys.iter().map(|k| k.weight).sum();
        if total_weight <= 0.0 {
            return Some(&config.keys[0].value);
        }
        let mut rng_val = fastrand::f64() * total_weight;
        for key in &config.keys {
            rng_val -= key.weight;
            if rng_val <= 0.0 {
                return Some(&key.value);
            }
        }
        Some(&config.keys.last().unwrap().value)
    }

    /// Normalise le nom du modèle : "google/gemini-2.0-flash" → "gemini-2.0-flash"
    fn normalize_model<'a>(&self, model: &'a str) -> &'a str {
        if let Some(stripped) = model.strip_prefix("google/") {
            return stripped;
        }
        if let Some(stripped) = model.strip_prefix("gemini/") {
            return stripped;
        }
        model
    }
}

impl Default for GeminiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn complete(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<PylosResponse, PylosError> {
        let api_key = self
            .select_key(config)
            .ok_or_else(|| PylosError::InvalidRequest("No API key configured for Gemini".into()))?;

        let base_url = self.base_url(config);

        match request {
            PylosRequest::ChatCompletion(req) => {
                let model = self.normalize_model(&req.model);
                let url = format!("{}/models/{}:generateContent", base_url, model);
                let gemini_req = to_gemini_request(req);

                debug!(provider = "gemini", model = %model, url = %url, "Sending Gemini generateContent");

                let response = self
                    .client
                    .post(&url)
                    .header("x-goog-api-key", api_key)
                    .json(&gemini_req)
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            PylosError::Timeout(e.to_string())
                        } else {
                            PylosError::ProviderError {
                                provider: "gemini".into(),
                                message: e.to_string(),
                            }
                        }
                    })?;

                let status = response.status().as_u16();
                if !response.status().is_success() {
                    let body = response.text().await.unwrap_or_default();
                    warn!(provider = "gemini", status = status, body = %body, "Gemini returned error");
                    return Err(map_gemini_error(status, &body));
                }

                let gemini_resp: GeminiResponse = response.json().await.map_err(|e| {
                    PylosError::Internal(format!("Failed to parse Gemini response: {}", e))
                })?;

                debug!(provider = "gemini", model = %model, "Gemini generateContent successful");
                Ok(from_gemini_response(gemini_resp, model))
            }
            PylosRequest::TextCompletion(_)
            | PylosRequest::Embedding(_)
            | PylosRequest::Image(_) => Err(PylosError::InvalidRequest(
                "Request type not supported by complete() on Gemini".into(),
            )),
        }
    }

    async fn stream(
        &self,
        request: &PylosRequest,
        config: &ProviderConfig,
    ) -> Result<ChunkStream, PylosError> {
        let api_key = self
            .select_key(config)
            .ok_or_else(|| PylosError::InvalidRequest("No API key configured for Gemini".into()))?;

        let base_url = self.base_url(config);

        match request {
            PylosRequest::ChatCompletion(req) => {
                let model = self.normalize_model(&req.model).to_string();
                // Streaming: ?alt=sse pour recevoir les chunks en SSE
                let url = format!(
                    "{}/models/{}:streamGenerateContent?alt=sse",
                    base_url, model
                );
                let gemini_req = to_gemini_request(req);

                debug!(provider = "gemini", model = %model, url = %url, "Sending Gemini streaming request");

                let response = self
                    .client
                    .post(&url)
                    .header("x-goog-api-key", api_key)
                    .json(&gemini_req)
                    .send()
                    .await
                    .map_err(|e| {
                        if e.is_timeout() {
                            PylosError::Timeout(e.to_string())
                        } else {
                            PylosError::ProviderError {
                                provider: "gemini".into(),
                                message: e.to_string(),
                            }
                        }
                    })?;

                let status = response.status().as_u16();
                if !response.status().is_success() {
                    let body = response.text().await.unwrap_or_default();
                    return Err(map_gemini_error(status, &body));
                }

                let model_clone = model.clone();
                let stream = response
                    .bytes_stream()
                    .eventsource()
                    .filter_map(move |event| {
                        let m = model_clone.clone();
                        async move {
                            match event {
                                Ok(e) => {
                                    let data = e.data.trim().to_string();
                                    if data.is_empty() {
                                        return None;
                                    }
                                    match serde_json::from_str::<GeminiResponse>(&data) {
                                        Ok(chunk) => from_gemini_stream_chunk(chunk, &m).map(Ok),
                                        Err(_) => None,
                                    }
                                }
                                Err(e) => Some(Err(PylosError::ProviderError {
                                    provider: "gemini".into(),
                                    message: e.to_string(),
                                })),
                            }
                        }
                    });

                Ok(Box::pin(stream))
            }
            PylosRequest::TextCompletion(_)
            | PylosRequest::Embedding(_)
            | PylosRequest::Image(_) => Err(PylosError::InvalidRequest(
                "Streaming not supported for this request type".into(),
            )),
        }
    }

    /// Gemini embeddings — POST /models/{model}:batchEmbedContents
    async fn embed(
        &self,
        request: &EmbeddingRequest,
        config: &ProviderConfig,
    ) -> Result<EmbeddingResponse, PylosError> {
        let api_key = self
            .select_key(config)
            .ok_or_else(|| PylosError::InvalidRequest("No API key configured for Gemini".into()))?;

        let base_url = self.base_url(config);
        let model = self.normalize_model(&request.model);
        let url = format!("{}/models/{}:batchEmbedContents", base_url, model);

        let texts = request.input.as_strings();
        let gemini_req = to_gemini_embed_request(model, texts, request.dimensions);

        debug!(provider = "gemini", model = %model, url = %url, "Sending Gemini batchEmbedContents");

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&gemini_req)
            .send()
            .await
            .map_err(|e| PylosError::ProviderError {
                provider: "gemini".into(),
                message: e.to_string(),
            })?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(map_gemini_error(status, &body));
        }

        let gemini_resp: GeminiBatchEmbedResponse = response.json().await.map_err(|e| {
            PylosError::Internal(format!("Failed to parse Gemini embed response: {}", e))
        })?;

        Ok(from_gemini_embed_response(gemini_resp, model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_model() {
        let p = GeminiProvider::new();
        assert_eq!(p.normalize_model("gemini-2.0-flash"), "gemini-2.0-flash");
        assert_eq!(
            p.normalize_model("google/gemini-2.0-flash"),
            "gemini-2.0-flash"
        );
        assert_eq!(
            p.normalize_model("gemini/gemini-2.0-flash"),
            "gemini-2.0-flash"
        );
    }
}
