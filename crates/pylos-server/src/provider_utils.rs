/// Déduit le provider depuis le nom du modèle.
/// Utilisé par les handlers d'inférence pour le logging et le calcul de coût.
///
/// Centralise la logique dupliquée qui était dans inference.rs, completions.rs
/// et budget_store.rs.
pub fn guess_provider(model: &str) -> String {
    // Bedrock : préfixes régionaux ou familles AWS
    if model.starts_with("us.")
        || model.starts_with("eu.")
        || model.starts_with("ap.")
        || model.starts_with("amazon.")
        || model.contains("nova")
        || model.contains("titan")
        || model.starts_with("anthropic.")
    {
        return "bedrock".to_string();
    }
    // Anthropic direct
    if model.contains("claude") {
        return "anthropic".to_string();
    }
    // OpenAI : GPT, o-series
    if model.starts_with("gpt")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
    {
        return "openai".to_string();
    }
    // OpenAI embeddings
    if model.starts_with("text-embedding") {
        return "openai".to_string();
    }
    // Google Gemini (direct + Vertex)
    if model.starts_with("gemini") || model.starts_with("gemma") {
        return "gemini".to_string();
    }
    // Cohere
    if model.starts_with("command") || model.starts_with("embed-") {
        return "cohere".to_string();
    }
    // xAI
    if model.starts_with("grok") {
        return "xai".to_string();
    }
    // Mistral / Codestral (direct)
    if (model.starts_with("mistral") || model.starts_with("codestral")) && !model.contains(':') {
        return "mistral".to_string();
    }
    // OpenRouter : format "provider/model"
    if model.contains('/') {
        return "openrouter".to_string();
    }
    // Ollama : modèles locaux (tag `:` ou noms connus)
    if model.contains(':')
        || model.contains("llama")
        || model.contains("qwen")
        || model.contains("deepseek")
        || model.contains("starcoder")
        || model.contains("nomic")
        || model.contains("phi")
    {
        return "ollama".to_string();
    }
    "unknown".to_string()
}
