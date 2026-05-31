use std::path::Path;
use tracing::info;

use crate::db_pool::DbPool;
use pylos_core::domain::system_prompt::SystemPrompt;
use pylos_core::error::PylosError;

#[derive(Clone)]
pub struct SystemPromptStore {
    pool: DbPool,
}

impl SystemPromptStore {
    pub async fn open(db_path: &Path) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_sqlite(db_path, "system_prompt_store", 4).await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!(path = %db_path.display(), "System prompt store opened (SQLite)");
        Ok(store)
    }

    pub async fn open_postgres(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_postgres(database_url, "system_prompt_store").await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!("System prompt store opened (PostgreSQL)");
        Ok(store)
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = DbPool::in_memory(2).await?;

        if let Some(p) = pool.as_sqlite() {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS system_prompts (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL,
                    prompt      TEXT NOT NULL,
                    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                )
                "#,
            )
            .execute(p)
            .await?;
        }

        Ok(Self { pool })
    }

    pub async fn list_prompts(&self) -> Result<Vec<SystemPrompt>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT id, name, prompt FROM system_prompts ORDER BY id")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list system prompts: {}", e))
                    })?;

                let list = rows
                    .iter()
                    .map(|r| SystemPrompt {
                        id: sqlx::Row::try_get(r, "id").unwrap_or_default(),
                        name: sqlx::Row::try_get(r, "name").unwrap_or_default(),
                        prompt: sqlx::Row::try_get(r, "prompt").unwrap_or_default(),
                    })
                    .collect();

                Ok(list)
            }
            DbPool::Postgres(pool) => {
                let rows = sqlx::query::<sqlx::Postgres>(
                    "SELECT id, name, prompt FROM system_prompts ORDER BY id",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| {
                    PylosError::Internal(format!("Failed to list system prompts: {}", e))
                })?;

                let list = rows
                    .iter()
                    .map(|r| SystemPrompt {
                        id: sqlx::Row::try_get(r, "id").unwrap_or_default(),
                        name: sqlx::Row::try_get(r, "name").unwrap_or_default(),
                        prompt: sqlx::Row::try_get(r, "prompt").unwrap_or_default(),
                    })
                    .collect();

                Ok(list)
            }
        }
    }

    pub async fn get_prompt_by_id(&self, id: &str) -> Result<Option<SystemPrompt>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT id, name, prompt FROM system_prompts WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get system prompt: {}", e))
                    })?;

                let prompt = row.map(|r| SystemPrompt {
                    id: sqlx::Row::try_get(&r, "id").unwrap_or_default(),
                    name: sqlx::Row::try_get(&r, "name").unwrap_or_default(),
                    prompt: sqlx::Row::try_get(&r, "prompt").unwrap_or_default(),
                });

                Ok(prompt)
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query::<sqlx::Postgres>(
                    "SELECT id, name, prompt FROM system_prompts WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to get system prompt: {}", e)))?;

                let prompt = row.map(|r| SystemPrompt {
                    id: sqlx::Row::try_get(&r, "id").unwrap_or_default(),
                    name: sqlx::Row::try_get(&r, "name").unwrap_or_default(),
                    prompt: sqlx::Row::try_get(&r, "prompt").unwrap_or_default(),
                });

                Ok(prompt)
            }
        }
    }

    pub async fn upsert_prompt(&self, sp: &SystemPrompt) -> Result<(), PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO system_prompts (id, name, prompt)
                    VALUES ($1, $2, $3)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        prompt = excluded.prompt
                    "#,
                )
                .bind(&sp.id)
                .bind(&sp.name)
                .bind(&sp.prompt)
                .execute(pool)
                .await
                .map_err(|e| {
                    PylosError::Internal(format!("Failed to upsert system prompt: {}", e))
                })?;
            }
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>(
                    r#"
                    INSERT INTO system_prompts (id, name, prompt)
                    VALUES ($1, $2, $3)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        prompt = excluded.prompt
                    "#,
                )
                .bind(&sp.id)
                .bind(&sp.name)
                .bind(&sp.prompt)
                .execute(pool)
                .await
                .map_err(|e| {
                    PylosError::Internal(format!("Failed to upsert system prompt: {}", e))
                })?;
            }
        }

        Ok(())
    }

    pub async fn delete_prompt(&self, id: &str) -> Result<bool, PylosError> {
        let rows_affected = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM system_prompts WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| {
                    PylosError::Internal(format!("Failed to delete system prompt: {}", e))
                })?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM system_prompts WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to delete system prompt: {}", e))
                    })?
                    .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }
}
