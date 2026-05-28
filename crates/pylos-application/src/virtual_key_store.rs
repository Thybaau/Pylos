use std::path::Path;

use sqlx::Row;
use tracing::info;

use crate::db_pool::DbPool;
use pylos_core::domain::config::{EnvVar, VirtualKeyConfig, VkProviderConfig};
use pylos_core::error::PylosError;

#[derive(Clone)]
pub struct VirtualKeyStore {
    pool: DbPool,
}

impl VirtualKeyStore {
    pub async fn open(db_path: &Path) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_sqlite(db_path, "virtual_key_store", 4).await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!(path = %db_path.display(), "Virtual key store opened (SQLite)");
        Ok(store)
    }

    pub async fn open_postgres(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_postgres(database_url, "virtual_key_store").await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!("Virtual key store opened (PostgreSQL)");
        Ok(store)
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = DbPool::in_memory(2).await?;

        if let Some(p) = pool.as_sqlite() {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS virtual_keys (
                    id                TEXT PRIMARY KEY,
                    name              TEXT NOT NULL,
                    description       TEXT,
                    value             TEXT NOT NULL UNIQUE,
                    is_active         INTEGER NOT NULL DEFAULT 1,
                    rate_limit_id     TEXT,
                    provider_configs  TEXT
                )
                "#,
            )
            .execute(p)
            .await?;
        }

        Ok(Self { pool })
    }

    pub async fn list_keys(&self) -> Result<Vec<VirtualKeyConfig>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT * FROM virtual_keys ORDER BY id")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list virtual keys: {}", e))
                    })?;

                Ok(rows.iter().map(row_to_vk_config_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows = sqlx::query::<sqlx::Postgres>("SELECT * FROM virtual_keys ORDER BY id")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list virtual keys: {}", e))
                    })?;

                Ok(rows.iter().map(row_to_vk_config_pg).collect())
            }
        }
    }

    pub async fn get_key_by_value(
        &self,
        value: &str,
    ) -> Result<Option<VirtualKeyConfig>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM virtual_keys WHERE value = $1")
                    .bind(value)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get virtual key: {}", e))
                    })?;

                Ok(row.as_ref().map(row_to_vk_config_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM virtual_keys WHERE value = $1")
                        .bind(value)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to get virtual key: {}", e))
                        })?;

                Ok(row.as_ref().map(row_to_vk_config_pg))
            }
        }
    }

    pub async fn get_key_by_id(&self, id: &str) -> Result<Option<VirtualKeyConfig>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM virtual_keys WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get virtual key: {}", e))
                    })?;

                Ok(row.as_ref().map(row_to_vk_config_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query::<sqlx::Postgres>("SELECT * FROM virtual_keys WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get virtual key: {}", e))
                    })?;

                Ok(row.as_ref().map(row_to_vk_config_pg))
            }
        }
    }

    pub async fn upsert_key(&self, vk: &VirtualKeyConfig) -> Result<(), PylosError> {
        let key_value =
            vk.value.as_ref().and_then(|v| v.resolve()).ok_or_else(|| {
                PylosError::InvalidRequest("Virtual key must have a value".into())
            })?;

        let provider_configs_json =
            serde_json::to_string(&vk.provider_configs).unwrap_or_else(|_| "[]".to_string());

        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO virtual_keys
                        (id, name, description, value, is_active, rate_limit_id, provider_configs)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        value = excluded.value,
                        is_active = excluded.is_active,
                        rate_limit_id = excluded.rate_limit_id,
                        provider_configs = excluded.provider_configs
                    "#,
                )
                .bind(&vk.id)
                .bind(&vk.name)
                .bind(&vk.description)
                .bind(&key_value)
                .bind(if vk.is_active { 1 } else { 0 })
                .bind(&vk.rate_limit_id)
                .bind(&provider_configs_json)
                .execute(pool)
                .await
                .map_err(|e| {
                    PylosError::Internal(format!("Failed to upsert virtual key: {}", e))
                })?;
            }
            DbPool::Postgres(pool) => {
                let provider_configs_val: serde_json::Value =
                    serde_json::from_str(&provider_configs_json)
                        .unwrap_or(serde_json::Value::Array(vec![]));

                sqlx::query::<sqlx::Postgres>(
                    r#"
                    INSERT INTO virtual_keys
                        (id, name, description, value, is_active, rate_limit_id, provider_configs)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        value = excluded.value,
                        is_active = excluded.is_active,
                        rate_limit_id = excluded.rate_limit_id,
                        provider_configs = excluded.provider_configs
                    "#,
                )
                .bind(&vk.id)
                .bind(&vk.name)
                .bind(&vk.description)
                .bind(&key_value)
                .bind(vk.is_active)
                .bind(&vk.rate_limit_id)
                .bind(&provider_configs_val)
                .execute(pool)
                .await
                .map_err(|e| {
                    PylosError::Internal(format!("Failed to upsert virtual key: {}", e))
                })?;
            }
        }

        Ok(())
    }

    pub async fn delete_key(&self, id: &str) -> Result<bool, PylosError> {
        let rows_affected = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM virtual_keys WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete virtual key: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM virtual_keys WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to delete virtual key: {}", e))
                    })?
                    .rows_affected()
            }
        };

        Ok(rows_affected > 0)
    }
}

fn row_to_vk_config_sqlite(row: &sqlx::sqlite::SqliteRow) -> VirtualKeyConfig {
    let raw_prov_configs: String = row.try_get("provider_configs").unwrap_or_default();
    let provider_configs: Vec<VkProviderConfig> =
        serde_json::from_str(&raw_prov_configs).unwrap_or_default();
    let value_str: String = row.try_get("value").unwrap_or_default();

    VirtualKeyConfig {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        value: Some(EnvVar::Literal(value_str)),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        rate_limit_id: row.try_get("rate_limit_id").ok(),
        provider_configs,
    }
}

fn row_to_vk_config_pg(row: &sqlx::postgres::PgRow) -> VirtualKeyConfig {
    let prov_configs_val: serde_json::Value = row
        .try_get("provider_configs")
        .unwrap_or(serde_json::Value::Array(vec![]));
    let provider_configs: Vec<VkProviderConfig> =
        serde_json::from_value(prov_configs_val).unwrap_or_default();
    let value_str: String = row.try_get("value").unwrap_or_default();

    VirtualKeyConfig {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        value: Some(EnvVar::Literal(value_str)),
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        rate_limit_id: row.try_get("rate_limit_id").ok(),
        provider_configs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_virtual_key_store_crud() {
        let store = VirtualKeyStore::in_memory().await.unwrap();
        let vk = VirtualKeyConfig {
            id: "vk-test-1".into(),
            name: "Test Key".into(),
            description: Some("Description test".into()),
            value: Some(EnvVar::Literal("sk-pylos-secret123".into())),
            is_active: true,
            rate_limit_id: Some("rl-1".into()),
            provider_configs: vec![VkProviderConfig {
                provider: "openai".into(),
                allowed_models: vec!["*".into()],
                key_names: vec!["*".into()],
                weight: 1.0,
            }],
        };

        store.upsert_key(&vk).await.unwrap();

        let list = store.list_keys().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "vk-test-1");
        assert_eq!(list[0].name, "Test Key");
        assert_eq!(list[0].provider_configs.len(), 1);
        assert_eq!(list[0].provider_configs[0].provider, "openai");

        let fetched = store.get_key_by_value("sk-pylos-secret123").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, "vk-test-1");

        let deleted = store.delete_key("vk-test-1").await.unwrap();
        assert!(deleted);

        let list_empty = store.list_keys().await.unwrap();
        assert_eq!(list_empty.len(), 0);
    }
}
