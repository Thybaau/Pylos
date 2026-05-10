use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot

use pylos_application::{
    BudgetStore, ConfigStore, InferenceOrchestrator, LogStore, ModelCatalog, RateLimitStore,
};
use pylos_core::domain::openai::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionResponse, MessageRole, Usage,
};
use pylos_core::domain::provider::{ProviderConfig, ProviderKind};
use pylos_core::domain::request::{
    PylosRequest, PylosResponse, StreamChoice, StreamChunk, StreamDelta,
};
use pylos_core::domain::traits::{ChunkStream, Provider};
use pylos_core::error::PylosError;
use pylos_server::metrics::Metrics;
use pylos_server::routes::create_router;
use pylos_server::state::AppState;

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
        request: &PylosRequest,
        _config: &ProviderConfig,
    ) -> Result<ChunkStream, PylosError> {
        let model = request.model().to_string();
        let chunk1 = StreamChunk {
            id: "mock-id".into(),
            object: "chat.completion.chunk".into(),
            created: 123456789,
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: Some("assistant".into()),
                    content: None,
                },
                finish_reason: None,
            }],
        };
        let chunk2 = StreamChunk {
            id: "mock-id".into(),
            object: "chat.completion.chunk".into(),
            created: 123456789,
            model,
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some("Hello".into()),
                },
                finish_reason: Some("stop".into()),
            }],
        };

        let stream = futures::stream::iter(vec![Ok(chunk1), Ok(chunk2)]);
        Ok(Box::pin(stream))
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
    let model_catalog = Arc::new(ModelCatalog::in_memory().await.unwrap());
    let budget_store = Arc::new(BudgetStore::in_memory().await.unwrap());
    let rate_limit_store = Arc::new(RateLimitStore::in_memory().await.unwrap());

    let state = AppState {
        orchestrator,
        config_store,
        metrics,
        vk_registry,
        log_store,
        model_catalog,
        budget_store,
        rate_limit_store,
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

#[tokio::test]
async fn test_chat_completions_stream() {
    // 1. Setup Mock State
    let mock_provider = Arc::new(MockProvider);
    let config = ProviderConfig::new(ProviderKind::OpenAI, vec![]);

    let orchestrator = Arc::new(InferenceOrchestrator::new(
        vec![(mock_provider, config)],
        vec![],
    ));

    let config_store = Arc::new(ConfigStore::load(None).await.unwrap());
    let metrics = Arc::new(Metrics::new());
    let vk_registry = Arc::new(pylos_core::domain::virtual_key::VirtualKeyRegistry::new());
    let log_store = Arc::new(LogStore::new(None, 1, 100));
    let model_catalog = Arc::new(ModelCatalog::in_memory().await.unwrap());
    let budget_store = Arc::new(BudgetStore::in_memory().await.unwrap());
    let rate_limit_store = Arc::new(RateLimitStore::in_memory().await.unwrap());

    let state = AppState {
        orchestrator,
        config_store,
        metrics,
        vk_registry,
        log_store,
        model_catalog,
        budget_store,
        rate_limit_store,
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
                        "messages": [{"role": "user", "content": "Hi"}],
                        "stream": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // 4. Assertions
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/event-stream");

    let body = response.into_body();
    let bytes = axum::body::to_bytes(body, 1024 * 1024).await.unwrap();
    let text = String::from_utf8(bytes.to_vec()).unwrap();

    // Check SSE format
    assert!(text.contains("data: {\"id\":\"mock-id\""));
    assert!(text.contains("data: [DONE]"));
    assert!(text.contains("content\":\"Hello\""));
}

async fn ax_body_to_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 1024 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
