use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Requête unifiée Pylos — équivalent de BifrostRequest en Go
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PylosRequest {
    ChatCompletion(crate::domain::openai::ChatCompletionRequest),
    TextCompletion(crate::domain::openai::TextCompletionRequest),
    Embedding(crate::domain::embedding::EmbeddingRequest),
}

impl PylosRequest {
    pub fn model(&self) -> &str {
        match self {
            PylosRequest::ChatCompletion(req) => &req.model,
            PylosRequest::TextCompletion(req) => &req.model,
            PylosRequest::Embedding(req) => &req.model,
        }
    }

    pub fn is_stream(&self) -> bool {
        match self {
            PylosRequest::ChatCompletion(req) => req.stream.unwrap_or(false),
            PylosRequest::TextCompletion(req) => req.stream.unwrap_or(false),
            PylosRequest::Embedding(_) => false,
        }
    }
}

/// Réponse unifiée Pylos
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PylosResponse {
    ChatCompletion(crate::domain::openai::ChatCompletionResponse),
    TextCompletion(crate::domain::openai::TextCompletionResponse),
    Embedding(crate::domain::embedding::EmbeddingResponse),
}

/// Un chunk de streaming SSE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: i32,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Metadata d'une requête en transit
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    pub headers: HashMap<String, String>,
    pub virtual_key: Option<String>,
    pub customer_id: Option<String>,
    pub team_id: Option<String>,
    pub trace_id: Option<String>,
    pub tried_providers: Vec<String>,
}
