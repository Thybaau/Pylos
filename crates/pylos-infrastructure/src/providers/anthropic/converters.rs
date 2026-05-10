use pylos_core::domain::openai::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionResponse, MessageRole, Usage,
};
use pylos_core::domain::request::{PylosResponse, StreamChoice, StreamChunk, StreamDelta};
use pylos_core::error::PylosError;
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────────────────────────────────────
// Format natif de l'API Anthropic Messages
// Ref : https://docs.anthropic.com/en/api/messages
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub(crate) struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Structures de réponse Anthropic
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicResponse {
    pub id: String,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub response_type: String,
    #[allow(dead_code)]
    pub role: String,
    pub content: Vec<AnthropicContent>,
    #[allow(dead_code)]
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicUsage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}

// ──────────────────────────────────────────────────────────────────────────────
// Structures de streaming Anthropic (SSE events)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub index: Option<i32>,
    pub delta: Option<AnthropicStreamDelta>,
    pub message: Option<AnthropicStreamMessage>,
    #[allow(dead_code)]
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicStreamDelta {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub delta_type: Option<String>,
    pub text: Option<String>,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicStreamMessage {
    pub id: String,
    pub model: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Contexte de streaming (pour accumuler les métadonnées du message)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub(crate) struct StreamContext {
    pub message_id: String,
    pub model: String,
    pub created: i64,
}

// ──────────────────────────────────────────────────────────────────────────────
// Conversions PylosRequest → Anthropic natif
// ──────────────────────────────────────────────────────────────────────────────

pub(crate) fn to_anthropic_request(
    req: &pylos_core::domain::openai::ChatCompletionRequest,
    stream: bool,
) -> AnthropicRequest {
    // Anthropic sépare les messages system des messages user/assistant
    // Le ou les messages `system` sont extraits dans le champ `system`
    let mut system_parts: Vec<String> = Vec::new();
    let mut messages: Vec<AnthropicMessage> = Vec::new();

    for msg in &req.messages {
        match msg.role {
            MessageRole::System => {
                system_parts.push(msg.content.clone());
            }
            MessageRole::User => {
                messages.push(AnthropicMessage {
                    role: "user".into(),
                    content: msg.content.clone(),
                });
            }
            MessageRole::Assistant => {
                messages.push(AnthropicMessage {
                    role: "assistant".into(),
                    content: msg.content.clone(),
                });
            }
            _ => {
                // Tool/Function → traité comme user pour simplifier
                messages.push(AnthropicMessage {
                    role: "user".into(),
                    content: msg.content.clone(),
                });
            }
        }
    }

    // Anthropic exige au moins un message et que le premier soit "user"
    if messages.is_empty() {
        messages.push(AnthropicMessage {
            role: "user".into(),
            content: String::new(),
        });
    }

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n\n"))
    };

    // max_tokens est obligatoire dans l'API Anthropic
    let max_tokens = req.max_tokens.unwrap_or(4096);

    // Les stop sequences sont une Vec<String> pour Anthropic
    let stop_sequences = match &req.stop {
        Some(pylos_core::domain::openai::StopCondition::String(s)) => Some(vec![s.clone()]),
        Some(pylos_core::domain::openai::StopCondition::Array(arr)) => Some(arr.clone()),
        None => None,
    };

    AnthropicRequest {
        model: req.model.clone(),
        messages,
        max_tokens,
        system,
        temperature: req.temperature,
        top_p: req.top_p,
        stream: if stream { Some(true) } else { None },
        stop_sequences,
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Conversions Anthropic natif → Pylos domain
// ──────────────────────────────────────────────────────────────────────────────

pub(crate) fn from_anthropic_response(
    resp: AnthropicResponse,
    requested_model: &str,
) -> PylosResponse {
    let text = resp
        .content
        .iter()
        .filter(|c| c.content_type == "text")
        .filter_map(|c| c.text.as_ref())
        .cloned()
        .collect::<Vec<_>>()
        .join("");

    let finish_reason = resp
        .stop_reason
        .as_deref()
        .map(|r| match r {
            "end_turn" => "stop",
            "max_tokens" => "length",
            "stop_sequence" => "stop",
            other => other,
        })
        .map(String::from);

    PylosResponse::ChatCompletion(ChatCompletionResponse {
        id: resp.id,
        object: "chat.completion".into(),
        created: chrono_now(),
        model: requested_model.to_string(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: MessageRole::Assistant,
                content: text,
                name: None,
            },
            finish_reason,
        }],
        usage: resp.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        }),
    })
}

pub(crate) fn from_anthropic_stream_event(
    event: AnthropicStreamEvent,
    ctx: &StreamContext,
) -> Option<StreamChunk> {
    match event.event_type.as_str() {
        "content_block_delta" => {
            let content = event.delta.as_ref().and_then(|d| d.text.clone());

            Some(StreamChunk {
                id: ctx.message_id.clone(),
                object: "chat.completion.chunk".into(),
                created: ctx.created,
                model: ctx.model.clone(),
                choices: vec![StreamChoice {
                    index: event.index.unwrap_or(0),
                    delta: StreamDelta {
                        role: None,
                        content,
                    },
                    finish_reason: None,
                }],
            })
        }
        "message_delta" => {
            // Stop reason
            let finish_reason = event
                .delta
                .as_ref()
                .and_then(|d| d.stop_reason.as_deref())
                .map(|r| match r {
                    "end_turn" => "stop",
                    "max_tokens" => "length",
                    other => other,
                })
                .map(String::from);

            Some(StreamChunk {
                id: ctx.message_id.clone(),
                object: "chat.completion.chunk".into(),
                created: ctx.created,
                model: ctx.model.clone(),
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
        _ => None, // message_start, ping, etc. — ignorés
    }
}

/// Erreurs Anthropic
#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicErrorBody {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: AnthropicErrorDetail,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AnthropicErrorDetail {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

pub(crate) fn map_anthropic_error(status: u16, body: &str) -> PylosError {
    let message = serde_json::from_str::<AnthropicErrorBody>(body)
        .map(|e| e.error.message)
        .unwrap_or_else(|_| body.to_string());

    match status {
        401 => PylosError::Unauthorized(message),
        429 => PylosError::RateLimitExceeded(message),
        408 | 504 => PylosError::Timeout(message),
        _ => PylosError::ProviderError {
            provider: "anthropic".into(),
            message,
        },
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
