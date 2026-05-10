use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

use pylos_core::domain::openai::{ChatCompletionResponse, ChatCompletionChoice, ChatCompletionMessage, MessageRole, Usage};
use pylos_core::domain::provider::{ProviderConfig, ProviderKind};
use pylos_core::domain::request::{PylosRequest, PylosResponse};
use pylos_core::domain::traits::{Provider, ChunkStream};
use pylos_core::error::PylosError;
use pylos_application::{InferenceOrchestrator, ConfigStore, LogStore};
use pylos_server::routes::create_router;
use pylos_server::state::AppState;
use pylos_server::metrics::Metrics;

use async_trait::async_trait;

struct MockProvider;

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn complete(
        &self,
        request: &PylosRequest,
        _config: &ProviderConfig,
    ) -> Result<PylosResponse, PylosError> {
        let model = request.model().to_string();
        Ok(PylosResponse::ChatCompletion(ChatCompletionResponse {
            id: "mock-id".into(),
            object: "chat.completion".into(),
            created: 123456789,
            model,
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatCompletionMessage {
                    role: MessageRole::Assistant,
                    content: Some("Hello from Mock!".into()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        }))
    }

    async fn stream(
        &self,
        _request: &PylosRequest,
        _config: &ProviderConfig,
    ) -> Result<ChunkStream, PylosError> {
        Err(PylosError::Unsupported("Mock does not support streaming yet".into()))
    }
}

#[tokio::test]
async fn test_chat_completions_unary() {
    // 1. Setup Mock State
    let mock_provider = Arc::new(MockProvider);
    let mut config = ProviderConfig::new(ProviderKind::OpenAI, vec![]);
    config.timeout_ms = 1000;
    
    let orchestrator = Arc::new(InferenceOrchestrator::new(
        vec![(mock_provider, config)],
        vec![],
    ));
    
    // Minimal mock stores
    let config_store = Arc::new(ConfigStore::load(None).await.unwrap());
    let metrics = Arc::new(Metrics::new());
    let vk_registry = Arc::new(pylos_core::domain::virtual_key::VirtualKeyRegistry::new());
    let log_store = Arc::new(LogStore::new(None, 1, 100)); // In-memory log store
    
    let state = AppState {
        orchestrator,
        config_store,
        metrics,
        vk_registry,
        log_store,
    };

    // 2. Create Router
    let app = create_router(state);

    // 3. Send Request
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/chat/completions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "model": "gpt-4o",
                        "messages": [{"role": "user", "content": "Hi"}]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 4. Assertions
    assert_eq!(response.status(), StatusCode::OK);

    let body = ax_body_to_json(response.into_body()).await;
    assert_eq!(body["id"], "mock-id");
    assert_eq!(body["choices"][0]["message"]["content"], "Hello from Mock!");
}

async fn ax_body_to_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
