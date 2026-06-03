use pylos_core::domain::openai::ChatCompletionRequest;
use serde_json::Value;

/// Compresse/Minifie le contenu JSON s'il est détecté dans les messages
pub fn minify_json_content(req: &mut ChatCompletionRequest) {
    for msg in &mut req.messages {
        if let Some(content) = &mut msg.content {
            // Tente de parser en JSON (très naïf : si ça parse, on le ré-encode minifié)
            if let Ok(parsed) = serde_json::from_str::<Value>(content) {
                // Si c'est bien un objet ou un array (pour éviter de parser de simples strings)
                if parsed.is_object() || parsed.is_array() {
                    // On peut aussi ajouter une logique pour tronquer les très longues listes
                    // Mais pour la v1, on se contente de la minification.
                    if let Ok(minified) = serde_json::to_string(&parsed) {
                        *content = minified;
                    }
                }
            }
        }
    }
}
