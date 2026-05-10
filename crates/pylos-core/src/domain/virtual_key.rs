use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Préfixe standard des Virtual Keys Pylos (inspiré de bifrost: sk-bf-*)
pub const VIRTUAL_KEY_PREFIX: &str = "sk-pylos-";

/// Une Virtual Key avec ses limites
#[derive(Debug, Clone)]
pub struct VirtualKey {
    /// Valeur de la clé (sk-pylos-...)
    pub key: String,
    /// Alias lisible
    pub name: String,
    /// Nombre max de requêtes par minute (0 = illimité)
    pub rate_limit_rpm: u32,
    /// Nombre max de tokens par minute (0 = illimité)
    pub token_limit_tpm: u32,
    /// Provider autorisé (None = tous)
    pub allowed_provider: Option<String>,
}

impl VirtualKey {
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            rate_limit_rpm: 0,
            token_limit_tpm: 0,
            allowed_provider: None,
        }
    }

    pub fn with_rpm(mut self, rpm: u32) -> Self {
        self.rate_limit_rpm = rpm;
        self
    }
}

/// État de consommation d'une Virtual Key sur la fenêtre courante (1 minute glissante)
#[derive(Debug, Default)]
struct KeyUsage {
    /// Requêtes effectuées dans la fenêtre courante
    requests: u32,
    /// Tokens consommés dans la fenêtre courante
    tokens: u32,
    /// Début de la fenêtre courante
    window_start: Option<Instant>,
}

impl KeyUsage {
    fn reset_if_expired(&mut self) {
        if let Some(start) = self.window_start {
            if start.elapsed() >= Duration::from_secs(60) {
                self.requests = 0;
                self.tokens = 0;
                self.window_start = Some(Instant::now());
            }
        } else {
            self.window_start = Some(Instant::now());
        }
    }
}

/// Registre de Virtual Keys en mémoire
/// Version simplifiée du governance plugin de bifrost
/// (en production : stocker dans SQLite/Postgres via framework/configstore)
#[derive(Clone)]
pub struct VirtualKeyRegistry {
    keys: Arc<RwLock<HashMap<String, VirtualKey>>>,
    usage: Arc<RwLock<HashMap<String, KeyUsage>>>,
}

impl VirtualKeyRegistry {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Enregistre une nouvelle Virtual Key
    pub async fn register(&self, vk: VirtualKey) {
        self.keys.write().await.insert(vk.key.clone(), vk);
    }

    /// Vérifie si la clé existe et si le rate limit est respecté
    /// Retourne `Ok(VirtualKey)` si autorisé, `Err(message)` sinon
    pub async fn check_and_increment(&self, key: &str) -> Result<VirtualKey, String> {
        // La clé doit commencer par le préfixe
        if !key.starts_with(VIRTUAL_KEY_PREFIX) {
            return Err("Invalid virtual key format".into());
        }

        let keys = self.keys.read().await;
        let vk = keys
            .get(key)
            .ok_or_else(|| "Virtual key not found".to_string())?
            .clone();
        drop(keys);

        // Vérification du rate limit
        if vk.rate_limit_rpm > 0 {
            let mut usage_map = self.usage.write().await;
            let usage = usage_map.entry(key.to_string()).or_default();
            usage.reset_if_expired();

            if usage.requests >= vk.rate_limit_rpm {
                return Err(format!(
                    "Rate limit exceeded: {} requests/minute for key '{}'",
                    vk.rate_limit_rpm, vk.name
                ));
            }

            usage.requests += 1;
        }

        Ok(vk)
    }
}

impl Default for VirtualKeyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_virtual_key_registration_and_lookup() {
        let registry = VirtualKeyRegistry::new();
        let vk = VirtualKey::new("sk-pylos-test123", "Test Key").with_rpm(100);
        registry.register(vk).await;

        let result = registry.check_and_increment("sk-pylos-test123").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Key");
    }

    #[tokio::test]
    async fn test_invalid_key_rejected() {
        let registry = VirtualKeyRegistry::new();
        let result = registry.check_and_increment("sk-openai-invalid").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_rate_limit_enforcement() {
        let registry = VirtualKeyRegistry::new();
        let vk = VirtualKey::new("sk-pylos-limited", "Limited Key").with_rpm(2);
        registry.register(vk).await;

        // Premières requêtes OK
        assert!(registry
            .check_and_increment("sk-pylos-limited")
            .await
            .is_ok());
        assert!(registry
            .check_and_increment("sk-pylos-limited")
            .await
            .is_ok());
        // 3ème requête : dépassement
        assert!(registry
            .check_and_increment("sk-pylos-limited")
            .await
            .is_err());
    }
}
