use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use pylos_core::domain::embedding::{EmbeddingRequest, EmbeddingResponse};
use pylos_core::domain::openai::{
    ChatCompletionMessage, ChatCompletionRequest, MessageRole, TextCompletionChoice,
    TextCompletionResponse,
};
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

    /// Calcule des embeddings avec fallback entre providers
    pub async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse, PylosError> {
        let providers = self.providers.read().await;
        let mut last_error: Option<PylosError> = None;

        let model = request.model.clone();
        let mut ordered: Vec<_> = providers.iter().collect();
        ordered.sort_by_key(|(_provider, config)| {
            if model_supported_by(config, &model) {
                0u8
            } else {
                1u8
            }
        });

        for (provider, config) in ordered {
            match provider.embed(&request, config).await {
                Ok(resp) => {
                    info!(provider = provider.name(), model = %model, "Embedding successful");
                    return Ok(resp);
                }
                Err(PylosError::Unsupported(_)) => {
                    debug!(
                        provider = provider.name(),
                        "Provider does not support embeddings, skipping"
                    );
                    continue;
                }
                Err(e) => {
                    warn!(provider = provider.name(), error = %e, "Embedding failed, trying fallback");
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            PylosError::AllProvidersFailed("No provider supports embeddings".into())
        }))
    }

    /// Envoie une requête non-streaming avec pre/post hooks et fallback
    pub async fn complete(
        &self,
        mut request: PylosRequest,
        mut ctx: RequestContext,
    ) -> Result<PylosResponse, PylosError> {
        // Convertit TextCompletion en ChatCompletion (compat)
        let text_completion_prompt = if let PylosRequest::TextCompletion(ref tc) = request {
            let prompt = tc.prompt.first().to_string();
            let chat_req = ChatCompletionRequest {
                model: tc.model.clone(),
                messages: vec![ChatCompletionMessage {
                    role: MessageRole::User,
                    content: Some(prompt.clone()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                }],
                temperature: tc.temperature,
                top_p: tc.top_p,
                n: tc.n,
                stream: Some(false),
                stop: tc.stop.clone(),
                max_tokens: tc.max_tokens,
                presence_penalty: tc.presence_penalty,
                frequency_penalty: tc.frequency_penalty,
                logit_bias: None,
                user: tc.user.clone(),
                tools: None,
                tool_choice: None,
                response_format: None,
                seed: None,
                top_k: None,
                min_p: None,
                repetition_penalty: None,
            };
            let original_model = tc.model.clone();
            request = PylosRequest::ChatCompletion(chat_req);
            Some((original_model, prompt))
        } else {
            None
        };

        // Pre-hooks
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
        ordered.sort_by_key(|(_provider, config)| {
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
                    // Post-hooks (ordre inverse — LIFO)
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

                    // Si c'était une TextCompletion, convertit la réponse ChatCompletion en TextCompletion
                    if let Some((model, _prompt)) = &text_completion_prompt {
                        if let PylosResponse::ChatCompletion(ref chat_resp) = response {
                            let text = chat_resp
                                .choices
                                .first()
                                .and_then(|c| c.message.content.clone())
                                .unwrap_or_default();
                            let finish = chat_resp
                                .choices
                                .first()
                                .and_then(|c| c.finish_reason.clone());
                            return Ok(PylosResponse::TextCompletion(TextCompletionResponse {
                                id: chat_resp.id.clone(),
                                object: "text_completion".to_string(),
                                created: chat_resp.created,
                                model: model.clone(),
                                choices: vec![TextCompletionChoice {
                                    text,
                                    index: 0,
                                    finish_reason: finish,
                                    logprobs: None,
                                }],
                                usage: chat_resp.usage.clone(),
                            }));
                        }
                    }

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
                Ok(Some(short_circuit)) => {
                    // Convertit la réponse short-circuit en stream d'un seul chunk de fin
                    debug!(
                        plugin = plugin.name(),
                        "Pre-hook short-circuited stream request"
                    );
                    let content = match &short_circuit {
                        PylosResponse::ChatCompletion(r) => r
                            .choices
                            .first()
                            .and_then(|c| c.message.content.clone())
                            .unwrap_or_default(),
                        _ => String::new(),
                    };
                    let model = request.model().to_string();
                    let chunk = make_terminal_chunk(&model, &content);
                    let stream: ChunkStream =
                        Box::pin(futures::stream::once(async move { Ok(chunk) }));
                    return Ok(stream);
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
///
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

    // Pour les autres providers : heuristique sur le nom du provider et du modèle
    match &config.kind {
        ProviderKind::OpenAI => {
            model.starts_with("gpt")
                || model.starts_with("o1")
                || model.starts_with("o3")
                || model.starts_with("o4")
        }
        ProviderKind::Anthropic => model.contains("claude"),
        ProviderKind::Gemini => {
            model.starts_with("gemini") || model.starts_with("google/") || model.contains("learnlm")
        }
        ProviderKind::Cohere => {
            model.starts_with("command")
                || model.starts_with("embed-")
                || model.starts_with("rerank-")
        }
        ProviderKind::Groq => {
            model.contains("llama")
                || model.contains("mixtral")
                || model.contains("whisper")
                || model.contains("gemma")
        }
        ProviderKind::Mistral => {
            model.starts_with("mistral")
                || model.starts_with("codestral")
                || model.starts_with("open-")
                || model.starts_with("pixtral")
        }
        ProviderKind::Cerebras => model.starts_with("llama") || model.starts_with("qwen"),
        ProviderKind::Perplexity => model.contains("sonar") || model.starts_with("llama-"),
        ProviderKind::Fireworks => {
            model.contains("firefunction")
                || model.contains("fireworks")
                || model.starts_with("accounts/fireworks/")
        }
        ProviderKind::XAI => model.starts_with("grok"),
        ProviderKind::Nebius => {
            model.contains("llama") || model.contains("qwen") || model.contains("deepseek")
        }
        ProviderKind::Ollama => {
            !model.starts_with("gpt")
                && !model.starts_with("claude")
                && !model.starts_with("gemini")
                && !model.starts_with("command")
                && !model.contains('/')
                && !model.starts_with("us.")
                && !model.starts_with("amazon.")
                && !model.starts_with("anthropic.")
        }
        ProviderKind::OpenRouter => model.contains('/'),
        ProviderKind::Custom(_) | ProviderKind::Vertex => true,
        _ => true,
    }
}

/// Crée un chunk de streaming terminal (finish_reason = "stop") à partir d'un contenu texte.
/// Utilisé pour convertir une réponse short-circuit (plugin pre-hook) en stream.
pub(crate) fn make_terminal_chunk(
    model: &str,
    content: &str,
) -> pylos_core::domain::request::StreamChunk {
    use pylos_core::domain::request::{StreamChoice, StreamDelta};
    use std::time::{SystemTime, UNIX_EPOCH};
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    pylos_core::domain::request::StreamChunk {
        id: format!("shortcircuit-{}", fastrand::u32(..)),
        object: "chat.completion.chunk".into(),
        created,
        model: model.to_string(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta {
                role: Some("assistant".into()),
                content: if content.is_empty() {
                    None
                } else {
                    Some(content.to_string())
                },
                tool_calls: None,
            },
            finish_reason: Some("stop".into()),
        }],
        usage: None,
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
