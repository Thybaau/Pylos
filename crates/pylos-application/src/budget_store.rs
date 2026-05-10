use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use tokio::sync::Mutex;
use tracing::{debug, info};

use pylos_core::domain::config::BudgetConfig;
use pylos_core::error::PylosError;

// ─────────────────────────────────────────────────────────────────────────────
// BudgetStore — persistance SQLite des budgets USD par virtual key
// Bifrost source: plugins/governance/budget_resolver.go + usage_tracker.go
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BudgetStore {
    pool: SqlitePool,
}

impl BudgetStore {
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

        info!(path = %db_path.display(), "Budget store opened");
        Ok(Self { pool })
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS vk_budgets (
                virtual_key_id TEXT NOT NULL,
                period         TEXT NOT NULL,
                max_usd        REAL NOT NULL,
                current_usd    REAL NOT NULL DEFAULT 0.0,
                reset_at       INTEGER NOT NULL,
                PRIMARY KEY (virtual_key_id, period)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// Initialise ou met à jour un budget depuis la config
    pub async fn upsert_budget(
        &self,
        vk_id: &str,
        budget: &BudgetConfig,
    ) -> Result<(), sqlx::Error> {
        let period = detect_period(&budget.reset_duration.0);
        let reset_at = next_reset_ms(&budget.reset_duration.0);

        sqlx::query(
            r#"
            INSERT INTO vk_budgets (virtual_key_id, period, max_usd, current_usd, reset_at)
            VALUES (?, ?, ?, 0.0, ?)
            ON CONFLICT(virtual_key_id, period) DO UPDATE SET
                max_usd = excluded.max_usd,
                reset_at = CASE WHEN reset_at < ? THEN excluded.reset_at ELSE reset_at END
            "#,
        )
        .bind(vk_id)
        .bind(&period)
        .bind(budget.max_limit)
        .bind(reset_at)
        .bind(now_ms())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Vérifie si un virtual key a dépassé son budget
    /// Retourne Ok(()) si dans le budget, Err(BudgetExceeded) sinon
    pub async fn check_budget(&self, vk_id: &str, estimated_cost: f64) -> Result<(), PylosError> {
        let now = now_ms();

        let rows = sqlx::query(
            "SELECT period, max_usd, current_usd, reset_at FROM vk_budgets WHERE virtual_key_id = ?",
        )
        .bind(vk_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for row in &rows {
            let period: String = row.try_get("period").unwrap_or_default();
            let max_usd: f64 = row.try_get("max_usd").unwrap_or(f64::MAX);
            let current_usd: f64 = row.try_get("current_usd").unwrap_or(0.0);
            let reset_at: i64 = row.try_get("reset_at").unwrap_or(0);

            // Reset si la période est expirée
            if now >= reset_at {
                debug!(vk_id = %vk_id, period = %period, "Budget period expired, will reset on next charge");
                continue; // On laisse la charge faire le reset
            }

            if current_usd + estimated_cost > max_usd {
                return Err(PylosError::BudgetExceeded(format!(
                    "Virtual key '{}' has exceeded its {} budget: ${:.4} used / ${:.2} max",
                    vk_id, period, current_usd, max_usd
                )));
            }
        }

        Ok(())
    }

    /// Enregistre l'utilisation après une requête réussie
    /// Gère le reset automatique si la période est expirée
    pub async fn record_usage(&self, vk_id: &str, cost_usd: f64) {
        if cost_usd <= 0.0 {
            return;
        }
        let now = now_ms();

        let rows = sqlx::query(
            "SELECT rowid, period, max_usd, current_usd, reset_at FROM vk_budgets WHERE virtual_key_id = ?",
        )
        .bind(vk_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for row in rows {
            let rowid: i64 = row.try_get("rowid").unwrap_or(0);
            let period: String = row.try_get("period").unwrap_or_default();
            let reset_at: i64 = row.try_get("reset_at").unwrap_or(0);

            if now >= reset_at {
                // Période expirée → reset + nouvel incrément
                let new_reset = reset_at + period_ms(&period);
                let _ = sqlx::query(
                    "UPDATE vk_budgets SET current_usd = ?, reset_at = ? WHERE rowid = ?",
                )
                .bind(cost_usd)
                .bind(new_reset)
                .bind(rowid)
                .execute(&self.pool)
                .await;
            } else {
                let _ = sqlx::query(
                    "UPDATE vk_budgets SET current_usd = current_usd + ? WHERE rowid = ?",
                )
                .bind(cost_usd)
                .bind(rowid)
                .execute(&self.pool)
                .await;
            }
        }
    }

    /// Retourne l'utilisation actuelle d'un VK
    pub async fn get_usage(&self, vk_id: &str) -> Vec<BudgetUsage> {
        let rows = sqlx::query(
            "SELECT period, max_usd, current_usd, reset_at FROM vk_budgets WHERE virtual_key_id = ? ORDER BY period",
        )
        .bind(vk_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.iter()
            .map(|row| BudgetUsage {
                period: row.try_get("period").unwrap_or_default(),
                max_usd: row.try_get("max_usd").unwrap_or(0.0),
                current_usd: row.try_get("current_usd").unwrap_or(0.0),
                reset_at_ms: row.try_get("reset_at").unwrap_or(0),
            })
            .collect()
    }
}

/// Usage courant d'un budget
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetUsage {
    pub period: String,
    pub max_usd: f64,
    pub current_usd: f64,
    pub reset_at_ms: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// BudgetPlugin — plugin LLM qui vérifie/enregistre les budgets
// S'installe dans le pipeline pre/post hook de l'InferenceOrchestrator
// ─────────────────────────────────────────────────────────────────────────────

pub struct BudgetPlugin {
    store: Arc<BudgetStore>,
    /// Coût estimé de la requête courante (partagé pre → post hook)
    pending_cost: Arc<Mutex<std::collections::HashMap<String, f64>>>,
}

impl BudgetPlugin {
    pub fn new(store: Arc<BudgetStore>) -> Self {
        Self {
            store,
            pending_cost: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl pylos_core::domain::traits::LlmPlugin for BudgetPlugin {
    fn name(&self) -> &str {
        "budget"
    }

    async fn pre_hook(
        &self,
        request: &mut pylos_core::domain::request::PylosRequest,
        ctx: &mut pylos_core::domain::request::RequestContext,
    ) -> Result<Option<pylos_core::domain::request::PylosResponse>, PylosError> {
        let vk_id = match &ctx.virtual_key {
            Some(vk) => vk.clone(),
            None => return Ok(None), // Pas de VK → pas de budget
        };

        // Estimation conservatrice du coût (basée sur le modèle)
        let estimated = estimate_request_cost(request.model());

        // Vérifie le budget avant d'envoyer
        self.store.check_budget(&vk_id, estimated).await?;

        // Stocke le coût estimé pour le post-hook
        let mut pending = self.pending_cost.lock().await;
        pending.insert(vk_id, estimated);

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

        // Calcule le coût réel depuis l'usage de la réponse
        let actual_cost = match response {
            pylos_core::domain::request::PylosResponse::ChatCompletion(r) => {
                r.usage
                    .as_ref()
                    .map(|u| {
                        crate::log_store::estimate_cost_pub(
                            // On utilise le provider depuis le modèle
                            guess_provider_from_model(&r.model),
                            &r.model,
                            u.prompt_tokens,
                            u.completion_tokens,
                        )
                    })
                    .unwrap_or(0.0)
            }
            _ => 0.0,
        };

        // Retire l'estimation et enregistre le coût réel
        let mut pending = self.pending_cost.lock().await;
        pending.remove(&vk_id);
        drop(pending);

        if actual_cost > 0.0 {
            self.store.record_usage(&vk_id, actual_cost).await;
            debug!(vk_id = %vk_id, cost_usd = actual_cost, "Budget usage recorded");
        }

        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn detect_period(duration_str: &str) -> String {
    let s = duration_str.trim();
    // Weekly : "7d" ou "1w"
    if s == "1w" || s == "7d" || (s.ends_with('w') && s[..s.len() - 1].parse::<u64>().is_ok()) {
        return "weekly".to_string();
    }
    if s.ends_with('d') && s[..s.len() - 1].parse::<u64>().unwrap_or(0) == 1 {
        "daily".to_string()
    } else if s.ends_with('M')
        || (s.ends_with('d') && s[..s.len() - 1].parse::<u64>().unwrap_or(0) >= 28)
    {
        "monthly".to_string()
    } else if s == "total" || s.ends_with('Y') {
        "total".to_string()
    } else {
        // Durée arbitraire → label basé sur la string
        format!("window_{}", s)
    }
}

fn period_ms(period: &str) -> i64 {
    match period {
        "daily" => 86_400_000,
        "weekly" => 7 * 86_400_000,
        "monthly" => 30 * 86_400_000,
        "total" => i64::MAX / 2,
        _ => 3_600_000, // 1h par défaut pour les fenêtres custom
    }
}

fn next_reset_ms(duration_str: &str) -> i64 {
    let period = detect_period(duration_str);
    now_ms() + period_ms(&period)
}

fn estimate_request_cost(model: &str) -> f64 {
    // Estimation très conservatrice pour la vérification pre-hook
    // On suppose ~1000 tokens prompt + 500 tokens completion
    let (input_per_1m, output_per_1m): (f64, f64) = if model.contains("gpt-4o") {
        (5.0, 15.0)
    } else if model.contains("claude-3-5") || model.contains("claude-3.5") {
        (3.0, 15.0)
    } else if model.contains("gemini-2.5-pro") {
        (7.0, 21.0)
    } else {
        (1.0, 3.0) // conservatif
    };
    (1000.0 / 1_000_000.0) * input_per_1m + (500.0 / 1_000_000.0) * output_per_1m
}

fn guess_provider_from_model(model: &str) -> &str {
    if model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("o3") {
        "openai"
    } else if model.contains("claude") {
        "anthropic"
    } else if model.starts_with("gemini") {
        "gemini"
    } else if model.starts_with("command") {
        "cohere"
    } else {
        "unknown"
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use pylos_core::domain::config::{BudgetConfig, Duration};

    async fn make_store() -> BudgetStore {
        BudgetStore::in_memory()
            .await
            .expect("in-memory budget store")
    }

    #[tokio::test]
    async fn test_upsert_and_check_budget_under_limit() {
        let store = make_store().await;
        let budget = BudgetConfig {
            id: "b1".into(),
            max_limit: 10.0,
            reset_duration: Duration("1d".into()),
            current_usage: 0.0,
            virtual_key_id: Some("vk-1".into()),
        };
        store.upsert_budget("vk-1", &budget).await.unwrap();

        // Under limit: should pass
        let result = store.check_budget("vk-1", 1.0).await;
        assert!(result.is_ok(), "Should be within budget");
    }

    #[tokio::test]
    async fn test_budget_exceeded() {
        let store = make_store().await;
        let budget = BudgetConfig {
            id: "b1".into(),
            max_limit: 1.0, // très petit budget
            reset_duration: Duration("1d".into()),
            current_usage: 0.0,
            virtual_key_id: Some("vk-2".into()),
        };
        store.upsert_budget("vk-2", &budget).await.unwrap();

        // Record near-max usage
        store.record_usage("vk-2", 0.95).await;

        // Exceeds limit
        let result = store.check_budget("vk-2", 0.10).await;
        assert!(result.is_err(), "Should exceed budget");
        assert!(matches!(result.unwrap_err(), PylosError::BudgetExceeded(_)));
    }

    #[tokio::test]
    async fn test_no_vk_no_budget_check() {
        let store = make_store().await;
        // No budget registered for this VK — check should pass (no constraint)
        let result = store.check_budget("vk-unknown", 1000.0).await;
        assert!(
            result.is_ok(),
            "Unknown VK should have no budget constraint"
        );
    }

    #[tokio::test]
    async fn test_get_usage() {
        let store = make_store().await;
        let budget = BudgetConfig {
            id: "b1".into(),
            max_limit: 5.0,
            reset_duration: Duration("1d".into()),
            current_usage: 0.0,
            virtual_key_id: Some("vk-3".into()),
        };
        store.upsert_budget("vk-3", &budget).await.unwrap();
        store.record_usage("vk-3", 1.50).await;

        let usages = store.get_usage("vk-3").await;
        assert_eq!(usages.len(), 1);
        assert!((usages[0].current_usd - 1.50).abs() < 0.001);
        assert!((usages[0].max_usd - 5.0).abs() < 0.001);
    }
}
