/// Identifiant unique d'un provider AI
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Bedrock,
    Azure,
    Gemini,
    Cohere,
    Groq,
    Mistral,
    Cerebras,
    Perplexity,
    Fireworks,
    XAI,
    Nebius,
    Ollama,
    OpenRouter,
    Vertex,
    /// Provider custom via URL de base configurable
    Custom(String),
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderKind::OpenAI => write!(f, "openai"),
            ProviderKind::Anthropic => write!(f, "anthropic"),
            ProviderKind::Bedrock => write!(f, "bedrock"),
            ProviderKind::Azure => write!(f, "azure"),
            ProviderKind::Gemini => write!(f, "gemini"),
            ProviderKind::Cohere => write!(f, "cohere"),
            ProviderKind::Groq => write!(f, "groq"),
            ProviderKind::Mistral => write!(f, "mistral"),
            ProviderKind::Cerebras => write!(f, "cerebras"),
            ProviderKind::Perplexity => write!(f, "perplexity"),
            ProviderKind::Fireworks => write!(f, "fireworks"),
            ProviderKind::XAI => write!(f, "xai"),
            ProviderKind::Nebius => write!(f, "nebius"),
            ProviderKind::Ollama => write!(f, "ollama"),
            ProviderKind::OpenRouter => write!(f, "openrouter"),
            ProviderKind::Vertex => write!(f, "vertex"),
            ProviderKind::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// URLs de base par défaut pour les providers OpenAI-compatibles
pub fn default_base_url(kind: &ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::Groq => Some("https://api.groq.com/openai/v1"),
        ProviderKind::Mistral => Some("https://api.mistral.ai/v1"),
        ProviderKind::Cerebras => Some("https://api.cerebras.ai/v1"),
        ProviderKind::Perplexity => Some("https://api.perplexity.ai"),
        ProviderKind::Fireworks => Some("https://api.fireworks.ai/inference/v1"),
        ProviderKind::XAI => Some("https://api.x.ai/v1"),
        ProviderKind::Nebius => Some("https://api.studio.nebius.ai/v1"),
        ProviderKind::OpenRouter => Some("https://openrouter.ai/api/v1"),
        ProviderKind::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
        ProviderKind::Cohere => Some("https://api.cohere.ai"),
        _ => None,
    }
}

/// Configuration spécifique Azure OpenAI
/// Bifrost source: core/providers/azure/types.go AzureKeyConfig
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AzureConfig {
    /// Nom de la ressource Azure : {resource_name}.openai.azure.com
    pub resource_name: String,
    /// Nom du déploiement Azure (correspond au modèle déployé)
    pub deployment_name: String,
    /// Version de l'API Azure OpenAI (ex: "2024-02-01")
    #[serde(default = "default_azure_api_version")]
    pub api_version: String,
}

fn default_azure_api_version() -> String {
    "2024-02-01".to_string()
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
    /// Configuration Azure OpenAI spécifique
    pub azure: Option<AzureConfig>,
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
            azure: None,
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
