use async_trait::async_trait;
use std::collections::HashMap;
use tracing::{debug, warn};

use pylos_core::domain::openai::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionResponse, MessageRole,
};
use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::LlmPlugin;
use pylos_core::error::PylosError;

pub struct GuardrailsPlugin {
    mask_pii: bool,
    blocked_keywords: Vec<String>,
}

impl GuardrailsPlugin {
    pub fn new(mask_pii: bool, blocked_keywords: Vec<String>) -> Self {
        Self {
            mask_pii,
            blocked_keywords,
        }
    }

    fn mask_text(&self, text: &str, pii_map: &mut HashMap<String, String>) -> String {
        let mut masked = text.to_string();

        // 1. Emails
        let email_regex = match regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}")
        {
            Ok(re) => re,
            Err(_) => return masked,
        };

        let start_email_idx = pii_map.len() + 1;
        let mut next_masked = masked.clone();
        for (idx, mat) in (start_email_idx..).zip(email_regex.find_iter(&masked)) {
            let original = mat.as_str().to_string();
            let placeholder = format!("[EMAIL_{}]", idx);
            pii_map.insert(placeholder.clone(), original);
            next_masked = next_masked.replace(mat.as_str(), &placeholder);
        }
        masked = next_masked;

        // 2. Phone Numbers (matches standard international and national formats)
        let phone_regex =
            regex::Regex::new(r"\+?\d{1,4}[-.\s]?\(?\d{1,3}?\)?[-.\s]?\d{3,4}[-.\s]?\d{3,4}")
                .unwrap();
        let mut phone_idx = pii_map.len() + 1;
        let mut next_masked = masked.clone();
        for mat in phone_regex.find_iter(&masked) {
            let original = mat.as_str().to_string();
            // Avoid matching short integers as phone numbers
            if original.chars().filter(|c| c.is_ascii_digit()).count() >= 7 {
                let placeholder = format!("[PHONE_{}]", phone_idx);
                pii_map.insert(placeholder.clone(), original);
                next_masked = next_masked.replace(mat.as_str(), &placeholder);
                phone_idx += 1;
            }
        }
        masked = next_masked;

        // 3. Credit cards
        let cc_regex = regex::Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap();
        let start_cc_idx = pii_map.len() + 1;
        let mut next_masked = masked.clone();
        for (idx, mat) in (start_cc_idx..).zip(cc_regex.find_iter(&masked)) {
            let original = mat.as_str().to_string();
            let placeholder = format!("[CREDIT_CARD_{}]", idx);
            pii_map.insert(placeholder.clone(), original);
            next_masked = next_masked.replace(mat.as_str(), &placeholder);
        }
        masked = next_masked;

        masked
    }

    fn restore_text(&self, text: &str, pii_map: &HashMap<String, String>) -> String {
        let mut restored = text.to_string();
        for (placeholder, original) in pii_map {
            restored = restored.replace(placeholder, original);
        }
        restored
    }
}

#[async_trait]
impl LlmPlugin for GuardrailsPlugin {
    fn name(&self) -> &str {
        "guardrails"
    }

    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        ctx: &mut RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        let chat_req = match request {
            PylosRequest::ChatCompletion(ref mut req) => req,
            _ => return Ok(None),
        };

        // 1. Keyword Blocklist Check
        for message in &chat_req.messages {
            if let Some(ref content) = message.content {
                let lower_content = content.to_lowercase();
                for keyword in &self.blocked_keywords {
                    if lower_content.contains(&keyword.to_lowercase()) {
                        warn!(keyword = %keyword, "Guardrails: Blocked request due to flagged keyword match");
                        // Return short-circuit response
                        let blocked_response = ChatCompletionResponse {
                            id: format!("blocked-{}", fastrand::u32(..)),
                            object: "chat.completion".to_string(),
                            created: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64,
                            model: chat_req.model.clone(),
                            choices: vec![ChatCompletionChoice {
                                index: 0,
                                message: ChatCompletionMessage {
                                    role: MessageRole::Assistant,
                                    content: Some(
                                        "Request flagged and blocked by guardrails safety policy."
                                            .to_string(),
                                    ),
                                    ..Default::default()
                                },
                                finish_reason: Some("content_filter".to_string()),
                            }],
                            usage: Some(Default::default()),
                        };
                        return Ok(Some(PylosResponse::ChatCompletion(blocked_response)));
                    }
                }
            }
        }

        // 2. PII Masking
        if self.mask_pii {
            let mut pii_map = HashMap::new();
            for message in &mut chat_req.messages {
                if let Some(ref content) = message.content {
                    let masked = self.mask_text(content, &mut pii_map);
                    message.content = Some(masked);
                }
            }

            if !pii_map.is_empty() {
                debug!(
                    pii_items = pii_map.len(),
                    "Guardrails: Masked PII items in user request"
                );
                // Save mapping in request context headers to restore it in post_hook
                if let Ok(serialized) = serde_json::to_string(&pii_map) {
                    ctx.headers.insert("x-pii-mapping".to_string(), serialized);
                }
            }
        }

        Ok(None)
    }

    async fn post_hook(
        &self,
        _request: &PylosRequest,
        response: &mut PylosResponse,
        ctx: &mut RequestContext,
    ) -> Result<(), PylosError> {
        let chat_resp = match response {
            PylosResponse::ChatCompletion(ref mut resp) => resp,
            _ => return Ok(()),
        };

        // Retrieve PII unmasking map
        if let Some(serialized) = ctx.headers.get("x-pii-mapping") {
            if let Ok(pii_map) = serde_json::from_str::<HashMap<String, String>>(serialized) {
                if !pii_map.is_empty() {
                    debug!("Guardrails: Unmasking response choices");
                    for choice in &mut chat_resp.choices {
                        if let Some(ref mut content) = choice.message.content {
                            *content = self.restore_text(content, &pii_map);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
