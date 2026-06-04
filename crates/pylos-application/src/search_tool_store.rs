use std::path::Path;

use sqlx::Row;
use tracing::info;

use crate::db_pool::DbPool;
use pylos_core::domain::search_tool::SearchToolConfig;
use pylos_core::error::PylosError;

#[derive(Clone)]
pub struct SearchToolStore {
    pool: DbPool,
}

impl SearchToolStore {
    pub async fn open(db_path: &Path) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_sqlite(db_path, "search_tool_store", 4).await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!(path = %db_path.display(), "Search tool store opened (SQLite)");
        Ok(store)
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = DbPool::in_memory(2).await?;

        if let Some(p) = pool.as_sqlite() {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS search_tools (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL,
                    description TEXT,
                    tool_type   TEXT NOT NULL,
                    config      TEXT NOT NULL DEFAULT '{}',
                    is_active   INTEGER NOT NULL DEFAULT 1,
                    created_at  INTEGER NOT NULL,
                    updated_at  INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;
        }

        Ok(Self { pool })
    }

    pub async fn open_postgres(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_postgres(database_url, "search_tool_store").await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!("Search tool store opened (PostgreSQL)");
        Ok(store)
    }

    pub async fn list_search_tools(&self) -> Result<Vec<SearchToolConfig>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT * FROM search_tools ORDER BY name")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list search tools: {}", e))
                    })?;
                Ok(rows.iter().map(row_to_search_tool_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM search_tools ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list search tools: {}", e))
                        })?;
                Ok(rows.iter().map(row_to_search_tool_pg).collect())
            }
        }
    }

    pub async fn get_search_tool(&self, id: &str) -> Result<Option<SearchToolConfig>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM search_tools WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get search tool: {}", e))
                    })?;
                Ok(row.as_ref().map(row_to_search_tool_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query::<sqlx::Postgres>("SELECT * FROM search_tools WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get search tool: {}", e))
                    })?;
                Ok(row.as_ref().map(row_to_search_tool_pg))
            }
        }
    }

    pub async fn upsert_search_tool(&self, st: &SearchToolConfig) -> Result<(), PylosError> {
        let config_json = serde_json::to_string(&st.config).unwrap_or_else(|_| "{}".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO search_tools (id, name, description, tool_type, config, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        tool_type = excluded.tool_type,
                        config = excluded.config,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&st.id)
                .bind(&st.name)
                .bind(&st.description)
                .bind(&st.tool_type)
                .bind(&config_json)
                .bind(if st.is_active { 1 } else { 0 })
                .bind(st.created_at)
                .bind(st.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert search tool: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let config_val: serde_json::Value = serde_json::from_str(&config_json)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO search_tools (id, name, description, tool_type, config, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        tool_type = excluded.tool_type,
                        config = excluded.config,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&st.id)
                .bind(&st.name)
                .bind(&st.description)
                .bind(&st.tool_type)
                .bind(&config_val)
                .bind(st.is_active)
                .bind(st.created_at)
                .bind(st.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert search tool: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_search_tool(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM search_tools WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete search tool: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM search_tools WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to delete search tool: {}", e))
                    })?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }
}

fn row_to_search_tool_sqlite(row: &sqlx::sqlite::SqliteRow) -> SearchToolConfig {
    let config_str: String = row.try_get("config").unwrap_or_default();
    SearchToolConfig {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        tool_type: row.try_get("tool_type").unwrap_or_default(),
        config: serde_json::from_str(&config_str).unwrap_or_default(),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_search_tool_pg(row: &sqlx::postgres::PgRow) -> SearchToolConfig {
    let config_val: serde_json::Value = row.try_get("config").unwrap_or_default();
    SearchToolConfig {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        tool_type: row.try_get("tool_type").unwrap_or_default(),
        config: config_val,
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}
