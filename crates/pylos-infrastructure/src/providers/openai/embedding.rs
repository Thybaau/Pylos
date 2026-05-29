use pylos_core::domain::embedding::{
    EmbeddingData, EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage,
};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Structures de requête/réponse OpenAI Embeddings
// Bifrost source: core/providers/openai/embedding.go
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct OpenAIEmbeddingRequest {
    pub model: String,
    pub input: OpenAIEmbeddingInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum OpenAIEmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAIEmbeddingResponse {
    #[serde(default)]
    pub object: String,
    pub data: Vec<OpenAIEmbeddingData>,
    pub model: String,
    #[serde(default)]
    pub usage: OpenAIEmbeddingUsage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAIEmbeddingData {
    pub index: usize,
    #[serde(default)]
    pub object: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct OpenAIEmbeddingUsage {
    #[serde(default)]
    pub prompt_tokens: i32,
    #[serde(default)]
    pub total_tokens: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Conversions
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn to_openai_embedding_request(req: &EmbeddingRequest) -> OpenAIEmbeddingRequest {
    let input = match &req.input {
        EmbeddingInput::Single(s) => OpenAIEmbeddingInput::Single(s.clone()),
        EmbeddingInput::Multiple(v) => OpenAIEmbeddingInput::Multiple(v.clone()),
    };

    OpenAIEmbeddingRequest {
        model: req.model.clone(),
        input,
        encoding_format: req.encoding_format.clone(),
        dimensions: req.dimensions,
        user: req.user.clone(),
    }
}

pub(crate) fn from_openai_embedding_response(resp: OpenAIEmbeddingResponse) -> EmbeddingResponse {
    EmbeddingResponse {
        object: resp.object,
        data: resp
            .data
            .into_iter()
            .map(|d| EmbeddingData {
                index: d.index,
                object: d.object,
                embedding: d.embedding,
            })
            .collect(),
        model: resp.model,
        usage: EmbeddingUsage {
            prompt_tokens: resp.usage.prompt_tokens,
            total_tokens: resp.usage.total_tokens,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pylos_core::domain::embedding::EmbeddingInput;

    #[test]
    fn test_single_input_conversion() {
        let req = EmbeddingRequest {
            model: "text-embedding-3-small".into(),
            input: EmbeddingInput::Single("hello world".into()),
            encoding_format: None,
            dimensions: None,
            user: None,
        };
        let openai_req = to_openai_embedding_request(&req);
        assert_eq!(openai_req.model, "text-embedding-3-small");
        let json = serde_json::to_string(&openai_req.input).unwrap();
        assert_eq!(json, r#""hello world""#);
    }

    #[test]
    fn test_multiple_input_conversion() {
        let req = EmbeddingRequest {
            model: "text-embedding-3-small".into(),
            input: EmbeddingInput::Multiple(vec!["hello".into(), "world".into()]),
            encoding_format: None,
            dimensions: None,
            user: None,
        };
        let openai_req = to_openai_embedding_request(&req);
        let json = serde_json::to_string(&openai_req.input).unwrap();
        assert_eq!(json, r#"["hello","world"]"#);
    }
}
