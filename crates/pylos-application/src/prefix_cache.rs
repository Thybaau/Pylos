use async_trait::async_trait;
use moka::future::Cache;
use std::time::Duration;
use tracing::{debug, info};

use pylos_core::domain::openai::ChatCompletionResponse;
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::LlmPlugin;
use pylos_core::error::PylosError;

pub struct PrefixCachePlugin {
    cache: Cache<String, ChatCompletionResponse>,
}

impl PrefixCachePlugin {
    pub fn new(ttl_secs: u64, max_capacity: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(ttl_secs))
            .build();
        Self { cache }
    }

    fn generate_key(&self, request: &PylosRequest) -> Option<String> {
        let chat_req = match request {
            PylosRequest::ChatCompletion(ref req) => req,
            _ => return None,
        };

        let mut key_builder = String::new();
        key_builder.push_str(request.model());
        key_builder.push('|');

        for msg in &chat_req.messages {
            key_builder.push_str(&format!("{:?}:{:?}|", msg.role, msg.content));
        }

        Some(key_builder)
    }
}

#[async_trait]
impl LlmPlugin for PrefixCachePlugin {
    fn name(&self) -> &str {
        "prefix_cache"
    }

    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        ctx: &mut RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        // Skip streaming requests
        if let PylosRequest::ChatCompletion(ref req) = request {
            if req.stream.unwrap_or(false) {
                return Ok(None);
            }
        }

        let key = match self.generate_key(request) {
            Some(k) => k,
            None => return Ok(None),
        };

        if let Some(cached_resp) = self.cache.get(&key).await {
            info!("PrefixCachePlugin: Cache HIT for key hash");
            ctx.headers
                .insert("x-prefix-cache-hit".to_string(), "true".to_string());
            return Ok(Some(PylosResponse::ChatCompletion(cached_resp)));
        }

        Ok(None)
    }

    async fn post_hook(
        &self,
        request: &PylosRequest,
        response: &mut PylosResponse,
        ctx: &mut RequestContext,
    ) -> Result<(), PylosError> {
        if ctx.headers.contains_key("x-prefix-cache-hit") {
            return Ok(());
        }

        let chat_resp = match response {
            PylosResponse::ChatCompletion(ref resp) => resp,
            _ => return Ok(()),
        };

        let key = match self.generate_key(request) {
            Some(k) => k,
            None => return Ok(()),
        };

        self.cache.insert(key, chat_resp.clone()).await;
        debug!("PrefixCachePlugin: Cached response in-memory");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pylos_core::domain::openai::{
        ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse,
        MessageRole,
    };

    #[tokio::test]
    async fn test_prefix_cache_hit_and_miss() {
        let plugin = PrefixCachePlugin::new(60, 100);
        let mut request = PylosRequest::ChatCompletion(ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![ChatCompletionMessage {
                role: MessageRole::User,
                content: Some("Hello".to_string()),
                ..Default::default()
            }],
            stream: None,
            temperature: None,
            max_tokens: None,
            response_format: None,
            top_p: None,
            n: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            seed: None,
            top_k: None,
            min_p: None,
            repetition_penalty: None,
            max_completion_tokens: None,
        });

        let mut ctx = RequestContext::default();

        // 1. First request -> Miss
        let pre_result = plugin.pre_hook(&mut request, &mut ctx).await.unwrap();
        assert!(pre_result.is_none());

        // Save mock response
        let mut mock_response = PylosResponse::ChatCompletion(ChatCompletionResponse {
            id: "chat-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1234567,
            model: "gpt-4".to_string(),
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatCompletionMessage {
                    role: MessageRole::Assistant,
                    content: Some("World".to_string()),
                    ..Default::default()
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        });

        plugin
            .post_hook(&request, &mut mock_response, &mut ctx)
            .await
            .unwrap();

        // 2. Second identical request -> Hit
        let mut ctx2 = RequestContext::default();
        let hit_result = plugin.pre_hook(&mut request, &mut ctx2).await.unwrap();
        assert!(hit_result.is_some());
        assert_eq!(ctx2.headers.get("x-prefix-cache-hit").unwrap(), "true");
    }
}
