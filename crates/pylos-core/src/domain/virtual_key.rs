use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Préfixe standard des Virtual Keys Pylos (inspiré de bifrost: sk-bf-*)
pub const VIRTUAL_KEY_PREFIX: &str = "sk-pylos-";

use std::collections::HashSet;

/// Une Virtual Key avec ses limites et rôles (RBAC)
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
    /// Rôles associés à cette clé (RBAC)
    pub roles: HashSet<String>,
    /// Permissions spécifiques associées à cette clé (RBAC)
    pub permissions: HashSet<String>,
}

impl VirtualKey {
    pub fn new(key: impl Into<String>, name: impl Into<String>) -> Self {
        let mut key_str = key.into();
        if !key_str.starts_with(VIRTUAL_KEY_PREFIX) {
            key_str = format!("{}{}", VIRTUAL_KEY_PREFIX, key_str);
        }
        Self {
            key: key_str,
            name: name.into(),
            rate_limit_rpm: 0,
            token_limit_tpm: 0,
            allowed_provider: None,
            roles: HashSet::new(),
            permissions: HashSet::new(),
        }
    }

    pub fn with_rpm(mut self, rpm: u32) -> Self {
        self.rate_limit_rpm = rpm;
        self
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.insert(role.into());
        self
    }

    pub fn with_permission(mut self, perm: impl Into<String>) -> Self {
        self.permissions.insert(perm.into());
        self
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.contains(perm) || self.roles.contains("admin")
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
#[derive(Clone)]
pub struct VirtualKeyRegistry {
    /// Table combinée clé → (VirtualKey, KeyUsage) sous un unique RwLock
    /// Évite tout TOCTOU : check + incrément se font sous un seul write lock
    inner: Arc<RwLock<HashMap<String, (VirtualKey, KeyUsage)>>>,
}

impl VirtualKeyRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Enregistre une nouvelle Virtual Key
    pub async fn register(&self, vk: VirtualKey) {
        self.inner
            .write()
            .await
            .insert(vk.key.clone(), (vk, KeyUsage::default()));
    }

    /// Retire une Virtual Key du registre
    pub async fn deregister(&self, key: &str) {
        self.inner.write().await.remove(key);
    }

    /// Vérifie si la clé existe et si le rate limit est respecté (atomique, sans TOCTOU).
    /// Check + incrément se font sous un unique write lock.
    pub async fn check_and_increment(&self, key: &str) -> Result<VirtualKey, String> {
        if !key.starts_with(VIRTUAL_KEY_PREFIX) {
            return Err("Invalid virtual key format".into());
        }

        let mut map = self.inner.write().await;

        let (vk, usage) = map
            .get_mut(key)
            .ok_or_else(|| "Virtual key not found".to_string())?;

        if vk.rate_limit_rpm > 0 {
            usage.reset_if_expired();
            if usage.requests >= vk.rate_limit_rpm {
                return Err(format!(
                    "Rate limit exceeded: {} requests/minute for key '{}'",
                    vk.rate_limit_rpm, vk.name
                ));
            }
            usage.requests += 1;
        }

        Ok(vk.clone())
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
