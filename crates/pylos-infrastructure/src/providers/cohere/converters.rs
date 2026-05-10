use serde::{Deserialize, Serialize};

use pylos_core::domain::embedding::{EmbeddingData, EmbeddingResponse, EmbeddingUsage};
use pylos_core::domain::openai::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionResponse, MessageRole, Usage,
};
use pylos_core::domain::request::{PylosResponse, StreamChoice, StreamChunk, StreamDelta};

// ─────────────────────────────────────────────────────────────────────────────
// Structures de requête Cohere v2
// Bifrost source: core/providers/cohere/types.go
// Doc: https://docs.cohere.com/reference/chat
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct CohereChatRequest {
    pub model: String,
    pub messages: Vec<CohereMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p: Option<f32>, // top_p
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    pub stream: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct CohereMessage {
    pub role: String, // "system" | "user" | "assistant" | "tool"
    pub content: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Structures de réponse Cohere
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct CohereChatResponse {
    pub id: Option<String>,
    pub finish_reason: Option<String>,
    pub message: Option<CohereResponseMessage>,
    pub usage: Option<CohereUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereResponseMessage {
    #[allow(dead_code)]
    pub role: Option<String>,
    pub content: Option<Vec<CohereContentBlock>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereContentBlock {
    #[serde(rename = "type")]
    pub block_type: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereUsage {
    pub billed_units: Option<CohereTokens>,
    pub tokens: Option<CohereTokens>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereTokens {
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Streaming Cohere
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct CohereStreamEvent {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub delta: Option<CohereStreamDelta>,
    #[allow(dead_code)]
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereStreamDelta {
    pub message: Option<CohereStreamMessage>,
    pub finish_reason: Option<String>,
    #[allow(dead_code)]
    pub usage: Option<CohereUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereStreamMessage {
    pub content: Option<CohereStreamContent>,
    #[allow(dead_code)]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereStreamContent {
    pub text: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub content_type: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Embeddings Cohere v2
// POST /v2/embed
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct CohereEmbedRequest {
    pub model: String,
    pub texts: Vec<String>,
    pub input_type: String, // "search_document" | "search_query" | ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_dimension: Option<u32>,
    pub embedding_types: Vec<String>, // ["float"]
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereEmbedResponse {
    #[allow(dead_code)]
    pub id: Option<String>,
    pub embeddings: Option<CohereEmbeddings>,
    pub meta: Option<CohereEmbedMeta>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereEmbeddings {
    pub float: Option<Vec<Vec<f32>>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CohereEmbedMeta {
    pub billed_units: Option<CohereTokens>,
    pub tokens: Option<CohereTokens>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Conversions
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn to_cohere_request(
    req: &pylos_core::domain::openai::ChatCompletionRequest,
    stream: bool,
) -> CohereChatRequest {
    let messages = req
        .messages
        .iter()
        .map(|m| {
            let role = match m.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool | MessageRole::Function => "tool",
            };
            CohereMessage {
                role: role.to_string(),
                content: m.content.clone().unwrap_or_default(),
            }
        })
        .collect();

    let stop_sequences = match &req.stop {
        Some(pylos_core::domain::openai::StopCondition::Single(s)) => Some(vec![s.clone()]),
        Some(pylos_core::domain::openai::StopCondition::Multiple(v)) => Some(v.clone()),
        None => None,
    };

    CohereChatRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        max_tokens: req.max_tokens,
        p: req.top_p,
        stop_sequences,
        frequency_penalty: req.frequency_penalty,
        presence_penalty: req.presence_penalty,
        stream,
    }
}

pub(crate) fn from_cohere_response(resp: CohereChatResponse) -> PylosResponse {
    let text = resp
        .message
        .as_ref()
        .and_then(|m| m.content.as_ref())
        .and_then(|blocks| {
            blocks
                .iter()
                .filter(|b| b.block_type.as_deref() == Some("text"))
                .filter_map(|b| b.text.as_ref())
                .cloned()
                .reduce(|a, b| a + &b)
        })
        .unwrap_or_default();

    let finish_reason = resp.finish_reason.as_deref().map(map_cohere_finish_reason);
    let usage = resp
        .usage
        .as_ref()
        .and_then(|u| u.billed_units.as_ref().or(u.tokens.as_ref()))
        .map(|t| Usage {
            prompt_tokens: t.input_tokens.unwrap_or(0),
            completion_tokens: t.output_tokens.unwrap_or(0),
            total_tokens: t.input_tokens.unwrap_or(0) + t.output_tokens.unwrap_or(0),
        });

    let id = resp
        .id
        .unwrap_or_else(|| format!("cohere-{}", fastrand::u64(..)));

    PylosResponse::ChatCompletion(ChatCompletionResponse {
        id,
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        model: String::new(), // will be filled from request model
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: MessageRole::Assistant,
                content: Some(text),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason,
        }],
        usage,
    })
}

/// Convertit un event SSE Cohere en StreamChunk (retourne None si à ignorer)
pub(crate) fn from_cohere_stream_event(
    event: CohereStreamEvent,
    model: &str,
) -> Option<StreamChunk> {
    let event_type = event.event_type.as_deref()?;

    match event_type {
        "content-delta" => {
            let text = event
                .delta
                .as_ref()
                .and_then(|d| d.message.as_ref())
                .and_then(|m| m.content.as_ref())
                .and_then(|c| c.text.clone());

            Some(StreamChunk {
                id: format!("cohere-{}", fastrand::u64(..)),
                object: "chat.completion.chunk".to_string(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                model: model.to_string(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: StreamDelta {
                        role: None,
                        content: text,
                    },
                    finish_reason: None,
                }],
            })
        }
        "message-end" => {
            let finish_reason = event
                .delta
                .as_ref()
                .and_then(|d| d.finish_reason.as_deref())
                .map(map_cohere_finish_reason);

            Some(StreamChunk {
                id: format!("cohere-{}", fastrand::u64(..)),
                object: "chat.completion.chunk".to_string(),
                created: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                model: model.to_string(),
                choices: vec![StreamChoice {
                    index: 0,
                    delta: StreamDelta {
                        role: None,
                        content: None,
                    },
                    finish_reason,
                }],
            })
        }
        _ => None, // message-start, content-start, content-end, etc.
    }
}

fn map_cohere_finish_reason(reason: &str) -> String {
    match reason {
        "COMPLETE" | "STOP_SEQUENCE" => "stop",
        "MAX_TOKENS" => "length",
        "TOOL_CALL" => "tool_calls",
        other => other,
    }
    .to_string()
}

pub(crate) fn to_cohere_embed_request(
    req: &pylos_core::domain::embedding::EmbeddingRequest,
) -> CohereEmbedRequest {
    let texts = req
        .input
        .as_strings()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    CohereEmbedRequest {
        model: req.model.clone(),
        texts,
        input_type: "search_document".to_string(),
        output_dimension: req.dimensions,
        embedding_types: vec!["float".to_string()],
    }
}

pub(crate) fn from_cohere_embed_response(
    resp: CohereEmbedResponse,
    model: &str,
) -> EmbeddingResponse {
    let floats = resp.embeddings.and_then(|e| e.float).unwrap_or_default();

    let total_tokens = resp
        .meta
        .and_then(|m| m.billed_units.or(m.tokens))
        .and_then(|t| t.input_tokens)
        .unwrap_or(0);

    EmbeddingResponse {
        object: "list".to_string(),
        data: floats
            .into_iter()
            .enumerate()
            .map(|(i, vec)| EmbeddingData {
                index: i,
                object: "embedding".to_string(),
                embedding: vec,
            })
            .collect(),
        model: model.to_string(),
        usage: EmbeddingUsage {
            prompt_tokens: total_tokens,
            total_tokens,
        },
    }
}

pub(crate) fn map_cohere_error(status: u16, body: &str) -> pylos_core::error::PylosError {
    use pylos_core::error::PylosError;
    #[derive(serde::Deserialize)]
    struct CohereErr {
        message: Option<String>,
    }
    let message = serde_json::from_str::<CohereErr>(body)
        .ok()
        .and_then(|e| e.message)
        .unwrap_or_else(|| body.to_string());

    match status {
        401 | 403 => PylosError::Unauthorized(message),
        429 => PylosError::RateLimitExceeded(message),
        408 | 504 => PylosError::Timeout(message),
        _ => PylosError::ProviderError {
            provider: "cohere".into(),
            message,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finish_reason_mapping() {
        assert_eq!(map_cohere_finish_reason("COMPLETE"), "stop");
        assert_eq!(map_cohere_finish_reason("MAX_TOKENS"), "length");
        assert_eq!(map_cohere_finish_reason("TOOL_CALL"), "tool_calls");
    }

    #[test]
    fn test_to_cohere_request_roles() {
        use pylos_core::domain::openai::{
            ChatCompletionMessage, ChatCompletionRequest, MessageRole,
        };
        let req = ChatCompletionRequest {
            model: "command-a-03-2025".to_string(),
            messages: vec![
                ChatCompletionMessage {
                    role: MessageRole::System,
                    content: Some("Be helpful".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                ChatCompletionMessage {
                    role: MessageRole::User,
                    content: Some("Hello".to_string()),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            stream: Some(false),
            temperature: None,
            top_p: None,
            n: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            stop: None,
            logit_bias: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            seed: None,
            top_k: None,
            min_p: None,
            repetition_penalty: None,
        };
        let cohere_req = to_cohere_request(&req, false);
        assert_eq!(cohere_req.messages[0].role, "system");
        assert_eq!(cohere_req.messages[1].role, "user");
        assert!(!cohere_req.stream);
    }
}
