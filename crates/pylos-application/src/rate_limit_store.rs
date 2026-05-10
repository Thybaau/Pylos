use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use tracing::{debug, info};

use pylos_core::domain::config::RateLimitConfig;
use pylos_core::error::PylosError;

// ─────────────────────────────────────────────────────────────────────────────
// RateLimitStore — rate limiting persistant par virtual key
// Supporte: RPM, TPM, RPD, TPD (requests/tokens per minute/day)
// Bifrost source: plugins/governance/ratelimit.go
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RateLimitStore {
    pool: SqlitePool,
}

impl RateLimitStore {
    pub async fn open(db_path: &Path) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?;

        info!(path = %db_path.display(), "Rate limit store opened");
        Ok(Self { pool })
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS vk_rate_limits (
                virtual_key_id     TEXT    NOT NULL,
                window_type        TEXT    NOT NULL,
                max_value          INTEGER NOT NULL,
                current_value      INTEGER NOT NULL DEFAULT 0,
                window_start_ms    INTEGER NOT NULL,
                window_duration_ms INTEGER NOT NULL,
                PRIMARY KEY (virtual_key_id, window_type)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// Configure les rate limits d'un virtual key depuis sa config
    pub async fn upsert_rate_limit(
        &self,
        vk_id: &str,
        config: &RateLimitConfig,
    ) -> Result<(), sqlx::Error> {
        let now = now_ms();

        // Requests per minute
        if config.request_max_limit > 0 {
            let window_ms = config
                .request_reset_duration
                .as_ref()
                .map(|d| (d.as_secs() * 1000) as i64)
                .unwrap_or(60_000); // défaut: 1 minute

            sqlx::query(
                r#"
                INSERT INTO vk_rate_limits (virtual_key_id, window_type, max_value, current_value, window_start_ms, window_duration_ms)
                VALUES (?, 'requests', ?, 0, ?, ?)
                ON CONFLICT(virtual_key_id, window_type) DO UPDATE SET
                    max_value = excluded.max_value,
                    window_duration_ms = excluded.window_duration_ms
                "#,
            )
            .bind(vk_id)
            .bind(config.request_max_limit)
            .bind(now)
            .bind(window_ms)
            .execute(&self.pool)
            .await?;
        }

        // Tokens per window
        if config.token_max_limit > 0 {
            let window_ms = config
                .token_reset_duration
                .as_ref()
                .map(|d| (d.as_secs() * 1000) as i64)
                .unwrap_or(60_000);

            sqlx::query(
                r#"
                INSERT INTO vk_rate_limits (virtual_key_id, window_type, max_value, current_value, window_start_ms, window_duration_ms)
                VALUES (?, 'tokens', ?, 0, ?, ?)
                ON CONFLICT(virtual_key_id, window_type) DO UPDATE SET
                    max_value = excluded.max_value,
                    window_duration_ms = excluded.window_duration_ms
                "#,
            )
            .bind(vk_id)
            .bind(config.token_max_limit as i64)
            .bind(now)
            .bind(window_ms)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Vérifie et incrémente le compteur de requêtes
    /// Retourne Ok(()) si dans les limites, Err(RateLimitExceeded) sinon
    pub async fn check_and_increment_requests(&self, vk_id: &str) -> Result<(), PylosError> {
        self.check_and_increment(vk_id, "requests", 1).await
    }

    /// Vérifie et enregistre l'usage en tokens
    pub async fn record_tokens(&self, vk_id: &str, tokens: i64) {
        if tokens <= 0 {
            return;
        }
        if let Err(e) = self.check_and_increment(vk_id, "tokens", tokens).await {
            // Pour les tokens on log seulement (on ne bloque pas après coup)
            debug!(error = %e, vk_id = %vk_id, "Token rate limit would be exceeded (post-hoc)");
        }
    }

    async fn check_and_increment(
        &self,
        vk_id: &str,
        window_type: &str,
        increment: i64,
    ) -> Result<(), PylosError> {
        let now = now_ms();

        let row = sqlx::query(
            "SELECT max_value, current_value, window_start_ms, window_duration_ms FROM vk_rate_limits WHERE virtual_key_id = ? AND window_type = ?",
        )
        .bind(vk_id)
        .bind(window_type)
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);

        let Some(row) = row else {
            return Ok(()); // Pas de limite configurée
        };

        let max_value: i64 = row.try_get("max_value").unwrap_or(0);
        let current_value: i64 = row.try_get("current_value").unwrap_or(0);
        let window_start_ms: i64 = row.try_get("window_start_ms").unwrap_or(now);
        let window_duration_ms: i64 = row.try_get("window_duration_ms").unwrap_or(60_000);

        // Vérifie si la fenêtre est expirée → reset
        let (new_current, new_start) = if now - window_start_ms >= window_duration_ms {
            (0i64, now)
        } else {
            (current_value, window_start_ms)
        };

        // Vérifie la limite
        if max_value > 0 && new_current + increment > max_value {
            let reset_in_secs = (window_duration_ms - (now - new_start)) / 1000;
            return Err(PylosError::RateLimitExceeded(format!(
                "Rate limit exceeded for {} ({}): {}/{} — resets in {}s",
                vk_id, window_type, new_current, max_value, reset_in_secs
            )));
        }

        // Met à jour atomiquement
        let _ = sqlx::query(
            "UPDATE vk_rate_limits SET current_value = ?, window_start_ms = ? WHERE virtual_key_id = ? AND window_type = ?",
        )
        .bind(new_current + increment)
        .bind(new_start)
        .bind(vk_id)
        .bind(window_type)
        .execute(&self.pool)
        .await;

        Ok(())
    }

    /// Retourne l'état des rate limits d'un VK
    pub async fn get_status(&self, vk_id: &str) -> Vec<RateLimitStatus> {
        let now = now_ms();
        let rows = sqlx::query(
            "SELECT window_type, max_value, current_value, window_start_ms, window_duration_ms FROM vk_rate_limits WHERE virtual_key_id = ?",
        )
        .bind(vk_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.iter()
            .map(|row| {
                let window_start: i64 = row.try_get("window_start_ms").unwrap_or(0);
                let window_dur: i64 = row.try_get("window_duration_ms").unwrap_or(60_000);
                let current: i64 = row.try_get("current_value").unwrap_or(0);
                let max: i64 = row.try_get("max_value").unwrap_or(0);
                // Si la fenêtre est expirée, current effectif = 0
                let effective_current = if now - window_start >= window_dur {
                    0
                } else {
                    current
                };

                RateLimitStatus {
                    window_type: row.try_get("window_type").unwrap_or_default(),
                    max_value: max as u64,
                    current_value: effective_current as u64,
                    reset_at_ms: window_start + window_dur,
                }
            })
            .collect()
    }
}

/// Statut d'un rate limit
#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimitStatus {
    pub window_type: String,
    pub max_value: u64,
    pub current_value: u64,
    pub reset_at_ms: i64,
}

/// Plugin de rate limiting — s'installe dans le pipeline pre/post hook
pub struct RateLimitPlugin {
    store: Arc<RateLimitStore>,
}

impl RateLimitPlugin {
    pub fn new(store: Arc<RateLimitStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl pylos_core::domain::traits::LlmPlugin for RateLimitPlugin {
    fn name(&self) -> &str {
        "rate_limit"
    }

    async fn pre_hook(
        &self,
        _request: &mut pylos_core::domain::request::PylosRequest,
        ctx: &mut pylos_core::domain::request::RequestContext,
    ) -> Result<Option<pylos_core::domain::request::PylosResponse>, PylosError> {
        let vk_id = match &ctx.virtual_key {
            Some(vk) => vk.clone(),
            None => return Ok(None),
        };

        self.store.check_and_increment_requests(&vk_id).await?;
        Ok(None)
    }

    async fn post_hook(
        &self,
        _request: &pylos_core::domain::request::PylosRequest,
        response: &mut pylos_core::domain::request::PylosResponse,
        ctx: &mut pylos_core::domain::request::RequestContext,
    ) -> Result<(), PylosError> {
        let vk_id = match &ctx.virtual_key {
            Some(vk) => vk.clone(),
            None => return Ok(()),
        };

        // Enregistre les tokens utilisés
        let tokens = match response {
            pylos_core::domain::request::PylosResponse::ChatCompletion(r) => {
                r.usage.as_ref().map(|u| u.total_tokens as i64).unwrap_or(0)
            }
            pylos_core::domain::request::PylosResponse::Embedding(r) => {
                r.usage.prompt_tokens as i64
            }
            _ => 0,
        };

        if tokens > 0 {
            self.store.record_tokens(&vk_id, tokens).await;
        }

        Ok(())
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pylos_core::domain::config::{Duration, RateLimitConfig};

    async fn make_store() -> RateLimitStore {
        RateLimitStore::in_memory()
            .await
            .expect("in-memory rate limit store")
    }

    fn make_config(req_limit: u32, window: &str) -> RateLimitConfig {
        RateLimitConfig {
            id: "rl-test".into(),
            token_max_limit: 0,
            token_reset_duration: None,
            request_max_limit: req_limit,
            request_reset_duration: Some(Duration(window.into())),
        }
    }

    #[tokio::test]
    async fn test_request_under_limit() {
        let store = make_store().await;
        store
            .upsert_rate_limit("vk-1", &make_config(10, "1m"))
            .await
            .unwrap();

        for _ in 0..10 {
            store.check_and_increment_requests("vk-1").await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_request_exceeds_limit() {
        let store = make_store().await;
        store
            .upsert_rate_limit("vk-2", &make_config(3, "1m"))
            .await
            .unwrap();

        for _ in 0..3 {
            store.check_and_increment_requests("vk-2").await.unwrap();
        }
        let result = store.check_and_increment_requests("vk-2").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PylosError::RateLimitExceeded(_)
        ));
    }

    #[tokio::test]
    async fn test_no_limit_configured() {
        let store = make_store().await;
        // Sans config → toujours OK
        for _ in 0..1000 {
            store
                .check_and_increment_requests("vk-unknown")
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_get_status() {
        let store = make_store().await;
        store
            .upsert_rate_limit("vk-3", &make_config(100, "1m"))
            .await
            .unwrap();
        store.check_and_increment_requests("vk-3").await.unwrap();
        store.check_and_increment_requests("vk-3").await.unwrap();

        let status = store.get_status("vk-3").await;
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].current_value, 2);
        assert_eq!(status[0].max_value, 100);
    }
}
