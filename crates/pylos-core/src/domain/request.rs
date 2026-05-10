use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Requête unifiée Pylos — équivalent de BifrostRequest en Go
/// Chaque variant porte la requête spécifique à ce type d'inférence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PylosRequest {
    ChatCompletion(crate::domain::openai::ChatCompletionRequest),
    // Extensible : Embedding, Speech, ImageGeneration, etc.
}

impl PylosRequest {
    /// Extrait le modèle de la requête
    pub fn model(&self) -> &str {
        match self {
            PylosRequest::ChatCompletion(req) => &req.model,
        }
    }

    /// Indique si la requête demande du streaming
    pub fn is_stream(&self) -> bool {
        match self {
            PylosRequest::ChatCompletion(req) => req.stream.unwrap_or(false),
        }
    }
}

/// Réponse unifiée Pylos
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PylosResponse {
    ChatCompletion(crate::domain::openai::ChatCompletionResponse),
}

/// Un chunk de streaming SSE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Identifiant de la complétion (même que la réponse finale)
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

/// Metadata d'une requête en transit (headers, trace, virtual key, etc.)
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// Headers HTTP originaux de l'appelant
    pub headers: HashMap<String, String>,
    /// Virtual Key utilisée (si governance activée)
    pub virtual_key: Option<String>,
    /// Customer ID pour tracking
    pub customer_id: Option<String>,
    /// Team ID pour tracking
    pub team_id: Option<String>,
    /// Trace ID W3C (traceparent)
    pub trace_id: Option<String>,
    /// Providers déjà essayés (pour fallback)
    pub tried_providers: Vec<String>,
}
