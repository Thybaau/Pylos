/// Identifiant unique d'un provider AI
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Bedrock,
    Gemini,
    Vertex,
    Cohere,
    Mistral,
    Groq,
    Ollama,
    OpenRouter,
    /// Provider custom via URL de base configurable
    Custom(String),
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderKind::OpenAI => write!(f, "openai"),
            ProviderKind::Anthropic => write!(f, "anthropic"),
            ProviderKind::Bedrock => write!(f, "bedrock"),
            ProviderKind::Gemini => write!(f, "gemini"),
            ProviderKind::Vertex => write!(f, "vertex"),
            ProviderKind::Cohere => write!(f, "cohere"),
            ProviderKind::Mistral => write!(f, "mistral"),
            ProviderKind::Groq => write!(f, "groq"),
            ProviderKind::Ollama => write!(f, "ollama"),
            ProviderKind::OpenRouter => write!(f, "openrouter"),
            ProviderKind::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Configuration d'une clé API pour un provider
#[derive(Debug, Clone)]
pub struct ProviderKey {
    pub value: String,
    /// Poids pour le load balancing pondéré (0.0 à 1.0)
    pub weight: f64,
    /// Metadata (région AWS, projet GCP, etc.)
    pub metadata: std::collections::HashMap<String, String>,
}

impl ProviderKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            weight: 1.0,
            metadata: Default::default(),
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Configuration d'un provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub keys: Vec<ProviderKey>,
    /// URL de base optionnelle (pour custom providers, Ollama, etc.)
    pub base_url: Option<String>,
    /// Timeout HTTP en millisecondes
    pub timeout_ms: u64,
    /// Nombre max de retries
    pub max_retries: u32,
    /// Délai initial de backoff en ms
    pub retry_backoff_initial_ms: u64,
    /// Délai maximum de backoff en ms
    pub retry_backoff_max_ms: u64,
    /// Configuration Bedrock spécifique (region, credentials, role_arn…)
    pub bedrock: Option<crate::domain::config::BedrockKeyConfig>,
}

impl ProviderConfig {
    pub fn new(kind: ProviderKind, keys: Vec<ProviderKey>) -> Self {
        Self {
            kind,
            keys,
            base_url: None,
            timeout_ms: 30_000,
            max_retries: 3,
            retry_backoff_initial_ms: 100,
            retry_backoff_max_ms: 5_000,
            bedrock: None,
        }
    }
}

/// Cible de routing : provider + modèle optionnel
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoutingTarget {
    pub provider: ProviderKind,
    /// Modèle override (si None, utilise le modèle de la requête)
    pub model_override: Option<String>,
    /// Poids pour routing probabiliste
    pub weight: f64,
}

impl RoutingTarget {
    pub fn new(provider: ProviderKind) -> Self {
        Self {
            provider,
            model_override: None,
            weight: 1.0,
        }
    }
}
