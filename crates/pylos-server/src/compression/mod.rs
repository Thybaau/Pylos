pub mod cache_aligner;
pub mod smart_crusher;

use pylos_core::domain::openai::ChatCompletionRequest;

/// Applique les différentes passes d'optimisation / compression sur la requête LLM
pub fn optimize_request(request: &mut ChatCompletionRequest) {
    // 1. Minifier le JSON (Smart Crusher)
    smart_crusher::minify_json_content(request);

    // 2. Réaligner le cache (Cache Aligner)
    cache_aligner::align_system_messages(request);
}
