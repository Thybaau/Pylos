use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use tracing::info;

#[derive(Clone)]
enum Pool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl Pool {
    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        match self {
            Pool::Sqlite(pool) => sqlx::migrate!("./migrations")
                .run(pool)
                .await
                .map_err(|e| sqlx::Error::Migrate(Box::new(e))),
            Pool::Postgres(pool) => sqlx::migrate!("./migrations_postgres")
                .run(pool)
                .await
                .map_err(|e| sqlx::Error::Migrate(Box::new(e))),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub provider: String,
    pub model_id: String,
    pub display_name: Option<String>,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub input_price_per_1m_usd: f64,
    pub output_price_per_1m_usd: f64,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub supports_embeddings: bool,
    pub is_deprecated: bool,
}

#[derive(Clone)]
pub struct ModelCatalog {
    pool: Pool,
}

impl ModelCatalog {
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

        let catalog = Self {
            pool: Pool::Sqlite(pool),
        };
        catalog.pool.run_migrations().await?;
        catalog.seed_builtin_models().await;

        info!(path = %db_path.display(), "Model catalog opened (SQLite)");
        Ok(catalog)
    }

    pub async fn open_postgres(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await?;

        let catalog = Self {
            pool: Pool::Postgres(pool),
        };
        catalog.pool.run_migrations().await?;
        catalog.seed_builtin_models().await;

        info!("Model catalog opened (PostgreSQL)");
        Ok(catalog)
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite::memory:")
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_catalog (
                id                      TEXT PRIMARY KEY,
                provider                TEXT NOT NULL,
                model_id                TEXT NOT NULL,
                display_name            TEXT,
                context_window          INTEGER NOT NULL DEFAULT 0,
                max_output_tokens       INTEGER NOT NULL DEFAULT 0,
                input_price_per_1m_usd  REAL    NOT NULL DEFAULT 0.0,
                output_price_per_1m_usd REAL    NOT NULL DEFAULT 0.0,
                supports_vision         INTEGER NOT NULL DEFAULT 0,
                supports_tools          INTEGER NOT NULL DEFAULT 1,
                supports_streaming      INTEGER NOT NULL DEFAULT 1,
                supports_embeddings     INTEGER NOT NULL DEFAULT 0,
                is_deprecated           INTEGER NOT NULL DEFAULT 0,
                updated_at_ms           INTEGER NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_catalog_uniq ON model_catalog(provider, model_id);
        "#,
        )
        .execute(&pool)
        .await?;

        let catalog = Self {
            pool: Pool::Sqlite(pool),
        };
        catalog.seed_builtin_models().await;
        Ok(catalog)
    }

    pub async fn list_models(
        &self,
        provider: Option<&str>,
        include_deprecated: bool,
    ) -> Vec<ModelInfo> {
        let mut sql = String::from("SELECT * FROM model_catalog WHERE 1=1");
        let mut params: Vec<String> = vec![];

        if let Some(p) = provider {
            sql.push_str(&format!(" AND provider = ${}", params.len() + 1));
            params.push(p.to_string());
        }
        if !include_deprecated {
            sql.push_str(" AND is_deprecated = 0");
        }
        sql.push_str(" ORDER BY provider, model_id");

        match &self.pool {
            Pool::Sqlite(pool) => {
                let mut q = sqlx::query(&sql);
                for p in &params {
                    q = q.bind(p);
                }
                q.fetch_all(pool)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .map(row_to_model_info_sqlite)
                    .collect()
            }
            Pool::Postgres(pool) => {
                let mut q = sqlx::query::<sqlx::Postgres>(&sql);
                for p in &params {
                    q = q.bind(p);
                }
                q.fetch_all(pool)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .map(row_to_model_info_pg)
                    .collect()
            }
        }
    }

    pub async fn get_model(&self, provider: &str, model_id: &str) -> Option<ModelInfo> {
        match &self.pool {
            Pool::Sqlite(pool) => {
                sqlx::query("SELECT * FROM model_catalog WHERE provider = $1 AND model_id = $2")
                    .bind(provider)
                    .bind(model_id)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten()
                    .as_ref()
                    .map(row_to_model_info_sqlite)
            }
            Pool::Postgres(pool) => sqlx::query::<sqlx::Postgres>(
                "SELECT * FROM model_catalog WHERE provider = $1 AND model_id = $2",
            )
            .bind(provider)
            .bind(model_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .as_ref()
            .map(row_to_model_info_pg),
        }
    }

    pub async fn get_pricing(&self, provider: &str, model_id: &str) -> (f64, f64) {
        if let Some(info) = self.get_model(provider, model_id).await {
            return (info.input_price_per_1m_usd, info.output_price_per_1m_usd);
        }
        (1.0, 3.0)
    }

    pub async fn upsert_model(&self, model: &ModelInfo) -> Result<(), sqlx::Error> {
        let now = now_ms();
        match &self.pool {
            Pool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO model_catalog
                        (id, provider, model_id, display_name, context_window, max_output_tokens,
                         input_price_per_1m_usd, output_price_per_1m_usd,
                         supports_vision, supports_tools, supports_streaming, supports_embeddings,
                         is_deprecated, updated_at_ms)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                    ON CONFLICT(id) DO UPDATE SET
                        display_name = excluded.display_name,
                        context_window = excluded.context_window,
                        max_output_tokens = excluded.max_output_tokens,
                        input_price_per_1m_usd = excluded.input_price_per_1m_usd,
                        output_price_per_1m_usd = excluded.output_price_per_1m_usd,
                        supports_vision = excluded.supports_vision,
                        supports_tools = excluded.supports_tools,
                        supports_streaming = excluded.supports_streaming,
                        supports_embeddings = excluded.supports_embeddings,
                        is_deprecated = excluded.is_deprecated,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                )
                .bind(&model.id)
                .bind(&model.provider)
                .bind(&model.model_id)
                .bind(&model.display_name)
                .bind(model.context_window)
                .bind(model.max_output_tokens)
                .bind(model.input_price_per_1m_usd)
                .bind(model.output_price_per_1m_usd)
                .bind(model.supports_vision as i32)
                .bind(model.supports_tools as i32)
                .bind(model.supports_streaming as i32)
                .bind(model.supports_embeddings as i32)
                .bind(model.is_deprecated as i32)
                .bind(now)
                .execute(pool)
                .await?;
            }
            Pool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>(
                    r#"
                    INSERT INTO model_catalog
                        (id, provider, model_id, display_name, context_window, max_output_tokens,
                         input_price_per_1m_usd, output_price_per_1m_usd,
                         supports_vision, supports_tools, supports_streaming, supports_embeddings,
                         is_deprecated, updated_at_ms)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                    ON CONFLICT(id) DO UPDATE SET
                        display_name = excluded.display_name,
                        context_window = excluded.context_window,
                        max_output_tokens = excluded.max_output_tokens,
                        input_price_per_1m_usd = excluded.input_price_per_1m_usd,
                        output_price_per_1m_usd = excluded.output_price_per_1m_usd,
                        supports_vision = excluded.supports_vision,
                        supports_tools = excluded.supports_tools,
                        supports_streaming = excluded.supports_streaming,
                        supports_embeddings = excluded.supports_embeddings,
                        is_deprecated = excluded.is_deprecated,
                        updated_at_ms = excluded.updated_at_ms
                    "#,
                )
                .bind(&model.id)
                .bind(&model.provider)
                .bind(&model.model_id)
                .bind(&model.display_name)
                .bind(model.context_window as i32)
                .bind(model.max_output_tokens as i32)
                .bind(model.input_price_per_1m_usd)
                .bind(model.output_price_per_1m_usd)
                .bind(model.supports_vision as i32)
                .bind(model.supports_tools as i32)
                .bind(model.supports_streaming as i32)
                .bind(model.supports_embeddings as i32)
                .bind(model.is_deprecated as i32)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn seed_builtin_models(&self) {
        let models = builtin_models();
        for m in &models {
            if let Err(e) = self.upsert_model(m).await {
                tracing::warn!(model = %m.id, error = %e, "Failed to seed model");
            }
        }
        info!(count = models.len(), "Model catalog seeded");
    }

    pub async fn delete_model(&self, provider: &str, model_id: &str) -> Result<bool, sqlx::Error> {
        let rows_affected = match &self.pool {
            Pool::Sqlite(pool) => {
                sqlx::query("DELETE FROM model_catalog WHERE provider = $1 AND model_id = $2")
                    .bind(provider)
                    .bind(model_id)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            Pool::Postgres(pool) => sqlx::query::<sqlx::Postgres>(
                "DELETE FROM model_catalog WHERE provider = $1 AND model_id = $2",
            )
            .bind(provider)
            .bind(model_id)
            .execute(pool)
            .await?
            .rows_affected(),
        };
        Ok(rows_affected > 0)
    }
}

fn row_to_model_info_sqlite(row: &sqlx::sqlite::SqliteRow) -> ModelInfo {
    ModelInfo {
        id: row.try_get("id").unwrap_or_default(),
        provider: row.try_get("provider").unwrap_or_default(),
        model_id: row.try_get("model_id").unwrap_or_default(),
        display_name: row.try_get("display_name").ok(),
        context_window: row.try_get::<i64, _>("context_window").unwrap_or(0) as u32,
        max_output_tokens: row.try_get::<i64, _>("max_output_tokens").unwrap_or(0) as u32,
        input_price_per_1m_usd: row.try_get("input_price_per_1m_usd").unwrap_or(0.0),
        output_price_per_1m_usd: row.try_get("output_price_per_1m_usd").unwrap_or(0.0),
        supports_vision: row.try_get::<i64, _>("supports_vision").unwrap_or(0) != 0,
        supports_tools: row.try_get::<i64, _>("supports_tools").unwrap_or(1) != 0,
        supports_streaming: row.try_get::<i64, _>("supports_streaming").unwrap_or(1) != 0,
        supports_embeddings: row.try_get::<i64, _>("supports_embeddings").unwrap_or(0) != 0,
        is_deprecated: row.try_get::<i64, _>("is_deprecated").unwrap_or(0) != 0,
    }
}

fn row_to_model_info_pg(row: &sqlx::postgres::PgRow) -> ModelInfo {
    ModelInfo {
        id: row.try_get("id").unwrap_or_default(),
        provider: row.try_get("provider").unwrap_or_default(),
        model_id: row.try_get("model_id").unwrap_or_default(),
        display_name: row.try_get("display_name").ok(),
        context_window: row.try_get::<i32, _>("context_window").unwrap_or(0) as u32,
        max_output_tokens: row.try_get::<i32, _>("max_output_tokens").unwrap_or(0) as u32,
        input_price_per_1m_usd: row.try_get("input_price_per_1m_usd").unwrap_or(0.0),
        output_price_per_1m_usd: row.try_get("output_price_per_1m_usd").unwrap_or(0.0),
        supports_vision: row.try_get::<bool, _>("supports_vision").unwrap_or(false),
        supports_tools: row.try_get::<bool, _>("supports_tools").unwrap_or(true),
        supports_streaming: row.try_get::<bool, _>("supports_streaming").unwrap_or(true),
        supports_embeddings: row
            .try_get::<bool, _>("supports_embeddings")
            .unwrap_or(false),
        is_deprecated: row.try_get::<bool, _>("is_deprecated").unwrap_or(false),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[allow(clippy::too_many_arguments)]
fn m(
    provider: &str,
    model_id: &str,
    display: &str,
    ctx: u32,
    max_out: u32,
    in_price: f64,
    out_price: f64,
    vision: bool,
    tools: bool,
    embeddings: bool,
) -> ModelInfo {
    ModelInfo {
        id: format!("{}/{}", provider, model_id),
        provider: provider.to_string(),
        model_id: model_id.to_string(),
        display_name: Some(display.to_string()),
        context_window: ctx,
        max_output_tokens: max_out,
        input_price_per_1m_usd: in_price,
        output_price_per_1m_usd: out_price,
        supports_vision: vision,
        supports_tools: tools,
        supports_streaming: true,
        supports_embeddings: embeddings,
        is_deprecated: false,
    }
}

fn builtin_models() -> Vec<ModelInfo> {
    vec![
        m(
            "openai", "gpt-4o", "GPT-4o", 128_000, 16_384, 5.0, 15.0, true, true, false,
        ),
        m(
            "openai",
            "gpt-4o-mini",
            "GPT-4o Mini",
            128_000,
            16_384,
            0.15,
            0.60,
            true,
            true,
            false,
        ),
        m(
            "openai",
            "gpt-4-turbo",
            "GPT-4 Turbo",
            128_000,
            4_096,
            10.0,
            30.0,
            true,
            true,
            false,
        ),
        m(
            "openai", "o1", "o1", 200_000, 100_000, 15.0, 60.0, false, true, false,
        ),
        m(
            "openai", "o1-mini", "o1-mini", 128_000, 65_536, 3.0, 12.0, false, false, false,
        ),
        m(
            "openai", "o3-mini", "o3-mini", 200_000, 100_000, 1.1, 4.4, false, true, false,
        ),
        m(
            "openai",
            "text-embedding-3-small",
            "Embedding 3 Small",
            8_191,
            0,
            0.02,
            0.0,
            false,
            false,
            true,
        ),
        m(
            "openai",
            "text-embedding-3-large",
            "Embedding 3 Large",
            8_191,
            0,
            0.13,
            0.0,
            false,
            false,
            true,
        ),
        m(
            "anthropic",
            "claude-opus-4-5",
            "Claude Opus 4.5",
            200_000,
            32_000,
            15.0,
            75.0,
            true,
            true,
            false,
        ),
        m(
            "anthropic",
            "claude-sonnet-4-5",
            "Claude Sonnet 4.5",
            200_000,
            64_000,
            3.0,
            15.0,
            true,
            true,
            false,
        ),
        m(
            "anthropic",
            "claude-haiku-3-5",
            "Claude Haiku 3.5",
            200_000,
            8_096,
            0.8,
            4.0,
            true,
            true,
            false,
        ),
        m(
            "anthropic",
            "claude-3-5-sonnet-20241022",
            "Claude 3.5 Sonnet",
            200_000,
            8_096,
            3.0,
            15.0,
            true,
            true,
            false,
        ),
        m(
            "anthropic",
            "claude-3-opus-20240229",
            "Claude 3 Opus",
            200_000,
            4_096,
            15.0,
            75.0,
            true,
            true,
            false,
        ),
        m(
            "anthropic",
            "claude-3-haiku-20240307",
            "Claude 3 Haiku",
            200_000,
            4_096,
            0.25,
            1.25,
            true,
            true,
            false,
        ),
        m(
            "gemini",
            "gemini-2.5-pro",
            "Gemini 2.5 Pro",
            1_000_000,
            65_536,
            7.0,
            21.0,
            true,
            true,
            false,
        ),
        m(
            "gemini",
            "gemini-2.5-flash",
            "Gemini 2.5 Flash",
            1_000_000,
            65_536,
            0.3,
            2.5,
            true,
            true,
            false,
        ),
        m(
            "gemini",
            "gemini-2.0-flash",
            "Gemini 2.0 Flash",
            1_000_000,
            8_192,
            0.1,
            0.4,
            true,
            true,
            false,
        ),
        m(
            "gemini",
            "gemini-1.5-pro",
            "Gemini 1.5 Pro",
            2_000_000,
            8_192,
            3.5,
            10.5,
            true,
            true,
            false,
        ),
        m(
            "gemini",
            "text-embedding-004",
            "Gemini Embedding",
            2_048,
            0,
            0.025,
            0.0,
            false,
            false,
            true,
        ),
        m(
            "gemini",
            "gemini-embedding-001",
            "Gemini Embedding 001",
            2_048,
            0,
            0.0,
            0.0,
            false,
            false,
            true,
        ),
        m(
            "cohere",
            "command-a-03-2025",
            "Command A",
            256_000,
            8_000,
            2.5,
            10.0,
            true,
            true,
            false,
        ),
        m(
            "cohere",
            "command-r-plus",
            "Command R+",
            128_000,
            4_096,
            3.0,
            15.0,
            false,
            true,
            false,
        ),
        m(
            "cohere",
            "command-r",
            "Command R",
            128_000,
            4_096,
            0.15,
            0.60,
            false,
            true,
            false,
        ),
        m(
            "cohere",
            "embed-v4.0",
            "Embed v4",
            512,
            0,
            0.1,
            0.0,
            false,
            false,
            true,
        ),
        m(
            "groq",
            "llama-3.3-70b-versatile",
            "Llama 3.3 70B",
            128_000,
            32_768,
            0.59,
            0.79,
            false,
            true,
            false,
        ),
        m(
            "groq",
            "llama-3.1-8b-instant",
            "Llama 3.1 8B",
            128_000,
            8_192,
            0.05,
            0.08,
            false,
            true,
            false,
        ),
        m(
            "groq",
            "gemma2-9b-it",
            "Gemma 2 9B",
            8_192,
            8_192,
            0.2,
            0.2,
            false,
            true,
            false,
        ),
        m(
            "groq",
            "mixtral-8x7b-32768",
            "Mixtral 8x7B",
            32_768,
            32_768,
            0.24,
            0.24,
            false,
            true,
            false,
        ),
        m(
            "mistral",
            "mistral-large-latest",
            "Mistral Large",
            128_000,
            8_192,
            3.0,
            9.0,
            true,
            true,
            false,
        ),
        m(
            "mistral",
            "mistral-small-latest",
            "Mistral Small",
            128_000,
            8_192,
            0.2,
            0.6,
            false,
            true,
            false,
        ),
        m(
            "mistral",
            "codestral-latest",
            "Codestral",
            256_000,
            8_192,
            0.3,
            0.9,
            false,
            true,
            false,
        ),
        m(
            "xai", "grok-3", "Grok 3", 131_072, 8_192, 5.0, 15.0, true, true, false,
        ),
        m(
            "xai",
            "grok-3-mini",
            "Grok 3 Mini",
            131_072,
            8_192,
            0.3,
            0.5,
            true,
            true,
            false,
        ),
        m(
            "bedrock",
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "Claude 3.5 Sonnet (Bedrock)",
            200_000,
            8_096,
            3.0,
            15.0,
            true,
            true,
            false,
        ),
        m(
            "bedrock",
            "amazon.nova-pro-v1:0",
            "Nova Pro",
            300_000,
            5_120,
            0.8,
            3.2,
            true,
            true,
            false,
        ),
        m(
            "bedrock",
            "amazon.nova-lite-v1:0",
            "Nova Lite",
            300_000,
            5_120,
            0.06,
            0.24,
            true,
            true,
            false,
        ),
        m(
            "bedrock",
            "amazon.nova-micro-v1:0",
            "Nova Micro",
            128_000,
            5_120,
            0.035,
            0.14,
            false,
            false,
            false,
        ),
        m(
            "deepseek",
            "deepseek-v4-pro",
            "DeepSeek V4 Pro",
            1_000_000,
            128_000,
            0.435,
            0.87,
            true,
            true,
            false,
        ),
        m(
            "deepseek",
            "deepseek-v4-flash",
            "DeepSeek V4 Flash",
            1_000_000,
            128_000,
            0.14,
            0.28,
            true,
            true,
            false,
        ),
        m(
            "deepseek",
            "deepseek-r1-v4",
            "DeepSeek R1 V4",
            128_000,
            16_384,
            0.55,
            2.19,
            false,
            true,
            false,
        ),
        m(
            "deepseek",
            "deepseek-chat",
            "DeepSeek V3",
            64_000,
            8_192,
            0.14,
            0.28,
            false,
            true,
            false,
        ),
        m(
            "graphon-rag",
            "graphon-rag",
            "Graphon RAG",
            8192,
            2048,
            0.0,
            0.0,
            false,
            false,
            false,
        ),
        m(
            "mnemosyne",
            "mnemosyne-search",
            "Mnemosyne Search Engine",
            8192,
            2048,
            0.0,
            0.0,
            false,
            false,
            false,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_seed_and_list() {
        let catalog = ModelCatalog::in_memory().await.unwrap();
        let models = catalog.list_models(None, false).await;
        assert!(!models.is_empty());
    }

    #[tokio::test]
    async fn test_list_by_provider() {
        let catalog = ModelCatalog::in_memory().await.unwrap();
        let models = catalog.list_models(Some("openai"), false).await;
        assert!(!models.is_empty());
        assert!(models.iter().all(|m| m.provider == "openai"));
    }

    #[tokio::test]
    async fn test_get_model() {
        let catalog = ModelCatalog::in_memory().await.unwrap();
        let info = catalog.get_model("openai", "gpt-4o").await;
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.input_price_per_1m_usd, 5.0);
        assert!(info.supports_vision);
    }

    #[tokio::test]
    async fn test_get_pricing() {
        let catalog = ModelCatalog::in_memory().await.unwrap();
        let (inp, out) = catalog
            .get_pricing("anthropic", "claude-3-5-sonnet-20241022")
            .await;
        assert_eq!(inp, 3.0);
        assert_eq!(out, 15.0);
    }

    #[tokio::test]
    async fn test_upsert_custom_model() {
        let catalog = ModelCatalog::in_memory().await.unwrap();
        let custom = m(
            "ollama",
            "llama3.2:3b",
            "Llama 3.2 3B",
            128_000,
            8_192,
            0.0,
            0.0,
            false,
            true,
            false,
        );
        catalog.upsert_model(&custom).await.unwrap();
        let found = catalog.get_model("ollama", "llama3.2:3b").await;
        assert!(found.is_some());
    }
}
