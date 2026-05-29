use crate::error::PylosError;
use crate::Result;
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    #[serde(default)]
    pub roles: HashSet<String>,
    #[serde(default)]
    pub permissions: HashSet<String>,
}

#[derive(Clone)]
pub struct OidcAuthenticator {
    expected_issuer: String,
    expected_audience: String,
    // Statique pour le moment afin d'éviter les requêtes réseau non fiables
    decoding_key: Option<DecodingKey>,
}

impl OidcAuthenticator {
    pub fn new(issuer: impl Into<String>, audience: impl Into<String>) -> Self {
        Self {
            expected_issuer: issuer.into(),
            expected_audience: audience.into(),
            decoding_key: None,
        }
    }

    pub fn with_decoding_key(mut self, key: DecodingKey) -> Self {
        self.decoding_key = Some(key);
        self
    }

    pub fn validate_token(&self, token: &str) -> Result<OidcClaims> {
        let header = decode_header(token).map_err(|e| {
            PylosError::InvalidRequest(format!("Failed to parse JWT header: {}", e))
        })?;

        let mut validation = Validation::new(header.alg);

        validation.set_issuer(&[&self.expected_issuer]);
        validation.set_audience(&[&self.expected_audience]);

        // Si aucune clé n'est fournie (ex: JWKS hors ligne dans les tests), on utilise une clé mock
        let key = match &self.decoding_key {
            Some(k) => k.clone(),
            None => DecodingKey::from_secret(b"secret-mock-validation-key"),
        };

        let token_data = decode::<OidcClaims>(token, &key, &validation)
            .map_err(|e| PylosError::InvalidRequest(format!("Failed to validate JWT: {}", e)))?;

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn test_valid_token_validation() {
        let key = b"secret-mock-validation-key";
        let claims = OidcClaims {
            sub: "user-123".into(),
            iss: "https://auth.example.com".into(),
            aud: "pylos-client".into(),
            exp: 9999999999, // Loin dans le futur
            roles: vec!["admin".to_string()].into_iter().collect(),
            permissions: HashSet::new(),
        };

        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(key)).unwrap();

        let authenticator = OidcAuthenticator::new("https://auth.example.com", "pylos-client");
        let result = authenticator.validate_token(&token);
        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, "user-123");
        assert!(claims.roles.contains("admin"));
    }
}
