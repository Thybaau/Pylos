use async_trait::async_trait;
use tracing::warn;

use pylos_core::domain::request::{PylosRequest, PylosResponse, RequestContext};
use pylos_core::domain::traits::LlmPlugin;
use pylos_core::error::PylosError;

#[derive(Default)]
pub struct CacheAlignerPlugin;

impl CacheAlignerPlugin {
    pub fn new() -> Self {
        Self
    }
}

fn is_uuid(token: &str) -> bool {
    if token.len() != 36 {
        return false;
    }
    let bytes = token.as_bytes();
    if bytes[8] != b'-' || bytes[13] != b'-' || bytes[18] != b'-' || bytes[23] != b'-' {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            continue;
        }
        if !b.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn is_iso8601(token: &str) -> bool {
    if token.len() < 8 {
        return false;
    }
    let bytes = token.as_bytes();
    // Check YYYY-MM-DD shape
    if bytes.len() >= 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        let is_date = bytes[0..4].iter().all(|b| b.is_ascii_digit())
            && bytes[5..7].iter().all(|b| b.is_ascii_digit())
            && bytes[8..10].iter().all(|b| b.is_ascii_digit());
        if is_date {
            return true;
        }
    }
    false
}

fn is_jwt_shape(token: &str) -> bool {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    for part in parts {
        if part.len() < 4 {
            return false;
        }
        // Base64url character set check
        for &b in part.as_bytes() {
            let is_base64url = b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
            if !is_base64url {
                return false;
            }
        }
    }
    true
}

fn is_hex_hash(token: &str) -> bool {
    let len = token.len();
    if len != 32 && len != 40 && len != 64 {
        return false;
    }
    token.bytes().all(|b| b.is_ascii_hexdigit())
}

fn split_tokens(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .map(|raw| {
            // Trim surrounding punctuation commonly wrapping inline tokens
            raw.trim_matches(|c: char| {
                c == '.'
                    || c == ','
                    || c == ';'
                    || c == ':'
                    || c == '!'
                    || c == '?'
                    || c == '"'
                    || c == '\''
                    || c == '('
                    || c == ')'
                    || c == '['
                    || c == ']'
                    || c == '{'
                    || c == '}'
                    || c == '<'
                    || c == '>'
            })
            .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn detect_volatile_content(content: &str) -> Vec<(String, String)> {
    let mut findings = Vec::new();
    for token in split_tokens(content) {
        let label = if is_uuid(&token) {
            Some("uuid")
        } else if is_jwt_shape(&token) {
            Some("jwt")
        } else if is_iso8601(&token) {
            Some("iso8601")
        } else if is_hex_hash(&token) {
            Some("hex_hash")
        } else {
            None
        };

        if let Some(lbl) = label {
            let sample = if token.len() <= 16 {
                token
            } else {
                format!("{}...{}", &token[..8], &token[token.len() - 4..])
            };
            findings.push((lbl.to_string(), sample));
        }
    }
    findings
}

#[async_trait]
impl LlmPlugin for CacheAlignerPlugin {
    fn name(&self) -> &str {
        "cache_aligner"
    }

    async fn pre_hook(
        &self,
        request: &mut PylosRequest,
        ctx: &mut RequestContext,
    ) -> Result<Option<PylosResponse>, PylosError> {
        let chat_req = match request {
            PylosRequest::ChatCompletion(ref req) => req,
            _ => return Ok(None),
        };

        let mut findings = Vec::new();
        for msg in &chat_req.messages {
            if msg.role == pylos_core::domain::openai::MessageRole::System {
                if let Some(ref content) = msg.content {
                    findings.extend(detect_volatile_content(content));
                }
            }
        }

        if !findings.is_empty() {
            let summary = findings
                .iter()
                .map(|(lbl, sample)| format!("{}={}", lbl, sample))
                .collect::<Vec<String>>()
                .join(", ");
            warn!(
                "CacheAligner: detected volatile content in system prompt ({}); cache prefix unstable. Move dynamic values out of the system prompt to recover cache hits.",
                summary
            );
            ctx.headers
                .insert("x-cache-unstable".to_string(), "true".to_string());
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_uuid() {
        assert!(is_uuid("123e4567-e89b-12d3-a456-426614174000"));
        assert!(!is_uuid("123e4567-e89b-12d3-a456-42661417400")); // too short
        assert!(!is_uuid("123e4567-e89b-12d3-a456-4266141740000")); // too long
        assert!(!is_uuid("123e4567-e89b-12d3-a456-42661417400g")); // non-hex character
    }

    #[test]
    fn test_is_iso8601() {
        assert!(is_iso8601("2024-01-15T08:30:00Z"));
        assert!(is_iso8601("2024-01-15"));
        assert!(!is_iso8601("not-a-date"));
    }

    #[test]
    fn test_is_jwt_shape() {
        assert!(is_jwt_shape("eyJhbGci.eyJzdWIi.signature"));
        assert!(!is_jwt_shape("not.jwt"));
    }

    #[test]
    fn test_is_hex_hash() {
        assert!(is_hex_hash("098f6bcd4621d373cade4e832627b4f6")); // MD5
        assert!(!is_hex_hash("not-hex"));
    }

    #[test]
    fn test_detect_volatile_content() {
        let content =
            "System initialized on 2024-01-15 with session 123e4567-e89b-12d3-a456-426614174000.";
        let findings = detect_volatile_content(content);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].0, "iso8601");
        assert_eq!(findings[1].0, "uuid");
    }
}
