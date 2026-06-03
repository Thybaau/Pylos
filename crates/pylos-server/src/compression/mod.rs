pub mod cache_aligner;
pub mod smart_crusher;

use pylos_core::domain::openai::ChatCompletionRequest;

/// Applique les différentes passes d'optimisation / compression sur la requête LLM
/// Retourne le nombre d'octets économisés
pub fn optimize_request(request: &mut ChatCompletionRequest) -> usize {
    // 1. Minifier le JSON (Smart Crusher)
    let saved_bytes = smart_crusher::minify_json_content(request);

    // 2. Réaligner le cache (Cache Aligner)
    cache_aligner::align_system_messages(request);

    saved_bytes
}
