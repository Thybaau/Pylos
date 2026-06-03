use pylos_core::domain::openai::ChatCompletionRequest;
use serde_json::Value;

/// Compresse/Minifie le contenu JSON s'il est détecté dans les messages
/// Retourne le nombre d'octets économisés
pub fn minify_json_content(req: &mut ChatCompletionRequest) -> usize {
    let mut saved_bytes = 0;
    for msg in &mut req.messages {
        if let Some(content) = &mut msg.content {
            if let Ok(parsed) = serde_json::from_str::<Value>(content) {
                if parsed.is_object() || parsed.is_array() {
                    if let Ok(minified) = serde_json::to_string(&parsed) {
                        let original_len = content.len();
                        let minified_len = minified.len();
                        if original_len > minified_len {
                            saved_bytes += original_len - minified_len;
                            *content = minified;
                        }
                    }
                }
            }
        }
    }
    saved_bytes
}
