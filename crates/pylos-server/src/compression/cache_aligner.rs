use pylos_core::domain::openai::{ChatCompletionRequest, MessageRole};

/// Réorganise les messages pour regrouper tous les messages System au début.
/// Cela maximise les chances de réutiliser le prefix cache (Anthropic / OpenAI).
pub fn align_system_messages(req: &mut ChatCompletionRequest) {
    let mut system_messages = Vec::new();
    let mut other_messages = Vec::new();

    for msg in req.messages.drain(..) {
        if matches!(msg.role, MessageRole::System) {
            system_messages.push(msg);
        } else {
            other_messages.push(msg);
        }
    }

    // On remet les system messages en premier
    req.messages.extend(system_messages);
    req.messages.extend(other_messages);
}
