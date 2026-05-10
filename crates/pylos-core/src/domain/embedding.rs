use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Embeddings API — compatible OpenAI /v1/embeddings
// Bifrost source: core/schemas/embedding.go
// ─────────────────────────────────────────────────────────────────────────────

/// Input pour les embeddings : string unique ou liste de strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Multiple(Vec<String>),
}

impl EmbeddingInput {
    /// Retourne toutes les chaînes à encoder
    pub fn as_strings(&self) -> Vec<&str> {
        match self {
            EmbeddingInput::Single(s) => vec![s.as_str()],
            EmbeddingInput::Multiple(v) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}

/// Requête POST /v1/embeddings — format OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// Modèle d'embedding (ex: "text-embedding-3-small", "nomic-embed-text")
    pub model: String,

    /// Texte(s) à encoder
    pub input: EmbeddingInput,

    /// Format de sortie : "float" (défaut) ou "base64"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,

    /// Nombre de dimensions de sortie (si le modèle le supporte, ex: text-embedding-3-*)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,

    /// Identifiant utilisateur pour la journalisation OpenAI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Réponse /v1/embeddings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Toujours "list"
    pub object: String,
    /// Liste des vecteurs d'embedding (un par input)
    pub data: Vec<EmbeddingData>,
    /// Modèle utilisé
    pub model: String,
    /// Usage en tokens
    pub usage: EmbeddingUsage,
}

/// Un embedding (vecteur) avec son index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    /// Index dans la liste d'inputs (commence à 0)
    pub index: usize,
    /// Toujours "embedding"
    pub object: String,
    /// Vecteur de floats
    pub embedding: Vec<f32>,
}

/// Usage en tokens pour les embeddings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingUsage {
    pub prompt_tokens: i32,
    pub total_tokens: i32,
}
