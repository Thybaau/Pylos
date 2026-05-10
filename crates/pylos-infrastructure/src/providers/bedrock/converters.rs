use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, InferenceConfiguration, Message, SystemContentBlock,
    ToolResultBlock, ToolResultContentBlock,
};

use pylos_core::domain::openai::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionResponse, MessageRole, Usage,
};
use pylos_core::domain::request::{PylosResponse, StreamChoice, StreamChunk, StreamDelta};
use pylos_core::error::PylosError;

// ─────────────────────────────────────────────────────────────────────────────
// Conversion PylosRequest → types SDK Bedrock Converse
// ─────────────────────────────────────────────────────────────────────────────

/// Convertit les messages OpenAI en messages Bedrock Converse
/// Identique à convertMessages() dans bifrost/core/providers/bedrock/utils.go
pub(crate) fn convert_messages(
    messages: &[pylos_core::domain::openai::ChatCompletionMessage],
) -> Result<(Vec<Message>, Vec<SystemContentBlock>), PylosError> {
    let mut bedrock_messages: Vec<Message> = Vec::new();
    let mut system_blocks: Vec<SystemContentBlock> = Vec::new();

    for msg in messages {
        match msg.role {
            // Les messages system sont extraits séparément (format Bedrock)
            MessageRole::System => {
                system_blocks.push(SystemContentBlock::Text(
                    msg.content.clone().unwrap_or_default(),
                ));
            }

            MessageRole::User => {
                let content = ContentBlock::Text(msg.content.clone().unwrap_or_default());
                bedrock_messages.push(
                    Message::builder()
                        .role(ConversationRole::User)
                        .content(content)
                        .build()
                        .map_err(|e| PylosError::Internal(e.to_string()))?,
                );
            }

            MessageRole::Assistant => {
                let content = ContentBlock::Text(msg.content.clone().unwrap_or_default());
                bedrock_messages.push(
                    Message::builder()
                        .role(ConversationRole::Assistant)
                        .content(content)
                        .build()
                        .map_err(|e| PylosError::Internal(e.to_string()))?,
                );
            }

            // Tool / Function → traité comme user avec ToolResult
            MessageRole::Tool | MessageRole::Function => {
                let tool_result = ToolResultBlock::builder()
                    .tool_use_id(msg.name.clone().unwrap_or_default())
                    .content(ToolResultContentBlock::Text(
                        msg.content.clone().unwrap_or_default(),
                    ))
                    .build()
                    .map_err(|e| PylosError::Internal(e.to_string()))?;

                let content = ContentBlock::ToolResult(tool_result);
                bedrock_messages.push(
                    Message::builder()
                        .role(ConversationRole::User)
                        .content(content)
                        .build()
                        .map_err(|e| PylosError::Internal(e.to_string()))?,
                );
            }
        }
    }

    Ok((bedrock_messages, system_blocks))
}

/// Construit l'InferenceConfiguration depuis les paramètres de la requête
pub(crate) fn build_inference_config(
    req: &pylos_core::domain::openai::ChatCompletionRequest,
) -> InferenceConfiguration {
    let mut builder = InferenceConfiguration::builder();

    if let Some(max_tokens) = req.max_tokens {
        builder = builder.max_tokens(max_tokens);
    } else {
        // Bedrock exige max_tokens — on met un défaut raisonnable
        builder = builder.max_tokens(4096);
    }

    if let Some(temp) = req.temperature {
        builder = builder.temperature(temp);
    }

    if let Some(top_p) = req.top_p {
        builder = builder.top_p(top_p);
    }

    if let Some(stop) = &req.stop {
        match stop {
            pylos_core::domain::openai::StopCondition::Single(s) => {
                builder = builder.stop_sequences(s.clone());
            }
            pylos_core::domain::openai::StopCondition::Multiple(arr) => {
                for s in arr {
                    builder = builder.stop_sequences(s.clone());
                }
            }
        }
    }

    builder.build()
}

// ─────────────────────────────────────────────────────────────────────────────
// Conversion réponse Bedrock → PylosResponse
// ─────────────────────────────────────────────────────────────────────────────

/// Convertit une réponse Bedrock Converse en PylosResponse
/// Identique à ToBifrostChatResponse() dans bifrost/core/providers/bedrock/chat.go
pub(crate) fn from_bedrock_response(
    output_message: &Message,
    stop_reason: &str,
    usage: Option<&aws_sdk_bedrockruntime::types::TokenUsage>,
    model: &str,
    id: String,
) -> PylosResponse {
    // Extraction du contenu texte depuis les blocs
    let mut text_content = String::new();

    for block in output_message.content() {
        // Les autres types (ToolUse, etc.) seront gérés plus tard
        if let ContentBlock::Text(t) = block {
            text_content.push_str(t);
        }
    }

    let finish_reason = map_stop_reason(stop_reason);

    let usage_data = usage.map(|u| Usage {
        prompt_tokens: u.input_tokens(),
        completion_tokens: u.output_tokens(),
        total_tokens: u.total_tokens(),
    });

    PylosResponse::ChatCompletion(ChatCompletionResponse {
        id,
        object: "chat.completion".into(),
        created: now_unix(),
        model: model.to_string(),
        choices: vec![ChatCompletionChoice {
            index: 0,
            message: ChatCompletionMessage {
                role: MessageRole::Assistant,
                content: Some(text_content),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            finish_reason: Some(finish_reason.to_string()),
        }],
        usage: usage_data,
    })
}

/// Construit un StreamChunk depuis les événements Bedrock
pub(crate) fn make_stream_chunk(
    id: &str,
    model: &str,
    content: Option<String>,
    role: Option<String>,
    finish_reason: Option<String>,
) -> StreamChunk {
    StreamChunk {
        id: id.to_string(),
        object: "chat.completion.chunk".into(),
        created: now_unix(),
        model: model.to_string(),
        choices: vec![StreamChoice {
            index: 0,
            delta: StreamDelta { role, content },
            finish_reason,
        }],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Mapping stop reasons  (Bedrock → OpenAI)
// ─────────────────────────────────────────────────────────────────────────────

/// Identique à bedrockFinishReasonToBifrost dans bifrost/core/providers/bedrock/chat.go
pub(crate) fn map_stop_reason(reason: &str) -> &'static str {
    match reason {
        "end_turn" => "stop",
        "max_tokens" => "length",
        "stop_sequence" => "stop",
        "tool_use" => "tool_calls",
        "guardrail_intervened" | "content_filtered" => "content_filter",
        _ => "stop",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Génère un ID unique pour une complétion Bedrock
pub(crate) fn generate_completion_id() -> String {
    format!("bedrock-{}", fastrand::u64(..))
}

/// Extrait le nom du modèle pour l'URL Bedrock Converse
/// Bedrock accepte soit l'ID natif soit le cross-region inference profile
/// Exemples :
///   "anthropic.claude-3-5-sonnet-20241022-v2:0"
///   "us.anthropic.claude-3-5-sonnet-20241022-v2:0"  (cross-region)
///   "amazon.nova-pro-v1:0"
pub(crate) fn normalize_model_id(model: &str) -> &str {
    model
}

#[cfg(test)]
mod tests {
    use super::*;
    use pylos_core::domain::openai::{ChatCompletionMessage, MessageRole};

    #[test]
    fn test_stop_reason_mapping() {
        assert_eq!(map_stop_reason("end_turn"), "stop");
        assert_eq!(map_stop_reason("max_tokens"), "length");
        assert_eq!(map_stop_reason("stop_sequence"), "stop");
        assert_eq!(map_stop_reason("tool_use"), "tool_calls");
        assert_eq!(map_stop_reason("guardrail_intervened"), "content_filter");
        assert_eq!(map_stop_reason("unknown"), "stop");
    }

    #[test]
    fn test_convert_messages_system_extracted() {
        let messages = vec![
            ChatCompletionMessage {
                role: MessageRole::System,
                content: Some("You are helpful.".into()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatCompletionMessage {
                role: MessageRole::User,
                content: Some("Hello".into()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let (bedrock_msgs, system_blocks) = convert_messages(&messages).unwrap();
        assert_eq!(system_blocks.len(), 1, "System message should be extracted");
        assert_eq!(bedrock_msgs.len(), 1, "Only user message should remain");
        assert_eq!(bedrock_msgs[0].role(), &ConversationRole::User);
    }

    #[test]
    fn test_convert_messages_roles() {
        let messages = vec![
            ChatCompletionMessage {
                role: MessageRole::User,
                content: Some("Hi".into()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            ChatCompletionMessage {
                role: MessageRole::Assistant,
                content: Some("Hello!".into()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let (bedrock_msgs, system_blocks) = convert_messages(&messages).unwrap();
        assert!(system_blocks.is_empty());
        assert_eq!(bedrock_msgs.len(), 2);
        assert_eq!(bedrock_msgs[0].role(), &ConversationRole::User);
        assert_eq!(bedrock_msgs[1].role(), &ConversationRole::Assistant);
    }

    #[test]
    fn test_inference_config_defaults() {
        use pylos_core::domain::openai::ChatCompletionRequest;

        let req = ChatCompletionRequest {
            model: "anthropic.claude-3-5-sonnet-20241022-v2:0".into(),
            messages: vec![],
            temperature: None,
            top_p: None,
            n: None,
            stream: None,
            stop: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            tools: None,
            tool_choice: None,
            response_format: None,
            seed: None,
            top_k: None,
            min_p: None,
            repetition_penalty: None,
        };

        let config = build_inference_config(&req);
        assert_eq!(config.max_tokens(), Some(4096));
    }
}
