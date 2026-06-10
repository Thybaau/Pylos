use std::path::Path;

use sqlx::Row;
use tracing::info;

use crate::db_pool::DbPool;
use pylos_core::domain::organization::{
    AccessGroup, InternalUser, Organization, Policy, Team, ToolPolicy,
};
use pylos_core::error::PylosError;

#[derive(Clone)]
pub struct OrganizationStore {
    pool: DbPool,
}

impl OrganizationStore {
    pub async fn open(db_path: &Path) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_sqlite(db_path, "organization_store", 4).await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!(path = %db_path.display(), "Organization store opened (SQLite)");
        Ok(store)
    }

    pub async fn in_memory() -> Result<Self, sqlx::Error> {
        let pool = DbPool::in_memory(2).await?;

        if let Some(p) = pool.as_sqlite() {
            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS organizations (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL,
                    description TEXT,
                    is_active   INTEGER NOT NULL DEFAULT 1,
                    tags        TEXT NOT NULL DEFAULT '[]',
                    created_at  INTEGER NOT NULL,
                    updated_at  INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS teams (
                    id              TEXT PRIMARY KEY,
                    organization_id TEXT NOT NULL,
                    name            TEXT NOT NULL,
                    description     TEXT,
                    is_active       INTEGER NOT NULL DEFAULT 1,
                    tags            TEXT NOT NULL DEFAULT '[]',
                    created_at      INTEGER NOT NULL,
                    updated_at      INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS internal_users (
                    id              TEXT PRIMARY KEY,
                    email           TEXT NOT NULL UNIQUE,
                    name            TEXT NOT NULL,
                    role            TEXT NOT NULL DEFAULT 'member',
                    usr_group       TEXT NOT NULL DEFAULT 'default',
                    organization_id TEXT,
                    team_ids        TEXT NOT NULL DEFAULT '[]',
                    is_active       INTEGER NOT NULL DEFAULT 1,
                    created_at      INTEGER NOT NULL,
                    updated_at      INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS access_groups (
                    id              TEXT PRIMARY KEY,
                    name            TEXT NOT NULL,
                    description     TEXT,
                    organization_id TEXT,
                    team_ids        TEXT NOT NULL DEFAULT '[]',
                    user_ids        TEXT NOT NULL DEFAULT '[]',
                    model_ids       TEXT NOT NULL DEFAULT '[]',
                    provider_ids    TEXT NOT NULL DEFAULT '[]',
                    is_active       INTEGER NOT NULL DEFAULT 1,
                    tags            TEXT NOT NULL DEFAULT '[]',
                    created_at      INTEGER NOT NULL,
                    updated_at      INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS policies (
                    id          TEXT PRIMARY KEY,
                    name        TEXT NOT NULL,
                    description TEXT,
                    policy_type TEXT NOT NULL,
                    config      TEXT NOT NULL DEFAULT '{}',
                    is_active   INTEGER NOT NULL DEFAULT 1,
                    created_at  INTEGER NOT NULL,
                    updated_at  INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;

            sqlx::query(
                r#"
                CREATE TABLE IF NOT EXISTS tool_policies (
                    id                   TEXT PRIMARY KEY,
                    name                 TEXT NOT NULL,
                    description          TEXT,
                    tool_type            TEXT NOT NULL,
                    allowed_models       TEXT NOT NULL DEFAULT '[]',
                    allowed_providers    TEXT NOT NULL DEFAULT '[]',
                    max_tokens_per_call  INTEGER,
                    max_calls_per_minute INTEGER,
                    is_active            INTEGER NOT NULL DEFAULT 1,
                    created_at           INTEGER NOT NULL,
                    updated_at           INTEGER NOT NULL
                )"#,
            )
            .execute(p)
            .await?;
        }

        Ok(Self { pool })
    }

    pub async fn open_postgres(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = DbPool::open_postgres(database_url, "organization_store").await?;
        let store = Self { pool };
        store.pool.run_migrations().await?;
        info!("Organization store opened (PostgreSQL)");
        Ok(store)
    }

    // ── Organizations ──────────────────────────────────────────────────────────

    pub async fn list_organizations(
        &self,
        tag: Option<&str>,
    ) -> Result<Vec<Organization>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = if let Some(t) = tag {
                    sqlx::query("SELECT * FROM organizations WHERE tags LIKE $1 ORDER BY name")
                        .bind(format!("%\"{}\"%", t))
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list organizations: {}", e))
                        })?
                } else {
                    sqlx::query("SELECT * FROM organizations ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list organizations: {}", e))
                        })?
                };
                Ok(rows.iter().map(row_to_org_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows: Vec<sqlx::postgres::PgRow> = if let Some(t) = tag {
                    sqlx::query::<sqlx::Postgres>(
                        "SELECT * FROM organizations WHERE tags @> $1 ORDER BY name",
                    )
                    .bind(serde_json::json!([t]))
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list organizations: {}", e))
                    })?
                } else {
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM organizations ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list organizations: {}", e))
                        })?
                };
                Ok(rows.iter().map(row_to_org_pg).collect())
            }
        }
    }

    pub async fn get_organization(&self, id: &str) -> Result<Option<Organization>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM organizations WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get organization: {}", e))
                    })?;
                Ok(row.as_ref().map(row_to_org_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM organizations WHERE id = $1")
                        .bind(id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to get organization: {}", e))
                        })?;
                Ok(row.as_ref().map(row_to_org_pg))
            }
        }
    }

    pub async fn upsert_organization(&self, org: &Organization) -> Result<(), PylosError> {
        let tags_json = serde_json::to_string(&org.tags).unwrap_or_else(|_| "[]".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO organizations (id, name, description, is_active, tags, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&org.id)
                .bind(&org.name)
                .bind(&org.description)
                .bind(if org.is_active { 1 } else { 0 })
                .bind(&tags_json)
                .bind(org.created_at)
                .bind(org.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert organization: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let tags_val: serde_json::Value =
                    serde_json::from_str(&tags_json).unwrap_or_default();
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO organizations (id, name, description, is_active, tags, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&org.id)
                .bind(&org.name)
                .bind(&org.description)
                .bind(org.is_active)
                .bind(&tags_val)
                .bind(org.created_at)
                .bind(org.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert organization: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_organization(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM organizations WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete organization: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM organizations WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to delete organization: {}", e))
                    })?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }

    // ── Teams ──────────────────────────────────────────────────────────────────

    pub async fn list_teams(
        &self,
        organization_id: Option<&str>,
        tag: Option<&str>,
    ) -> Result<Vec<Team>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = match (organization_id, tag) {
                    (Some(org_id), Some(t)) => {
                        sqlx::query(
                            "SELECT * FROM teams WHERE organization_id = $1 AND tags LIKE $2 ORDER BY name",
                        )
                        .bind(org_id)
                        .bind(format!("%\"{}\"%", t))
                        .fetch_all(pool)
                        .await
                        .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                    (Some(org_id), None) => {
                        sqlx::query("SELECT * FROM teams WHERE organization_id = $1 ORDER BY name")
                            .bind(org_id)
                            .fetch_all(pool)
                            .await
                            .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                    (None, Some(t)) => {
                        sqlx::query("SELECT * FROM teams WHERE tags LIKE $1 ORDER BY name")
                            .bind(format!("%\"{}\"%", t))
                            .fetch_all(pool)
                            .await
                            .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                    (None, None) => {
                        sqlx::query("SELECT * FROM teams ORDER BY name")
                            .fetch_all(pool)
                            .await
                            .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                };
                Ok(rows.iter().map(row_to_team_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows: Vec<sqlx::postgres::PgRow> = match (organization_id, tag) {
                    (Some(org_id), Some(t)) => {
                        sqlx::query::<sqlx::Postgres>(
                            "SELECT * FROM teams WHERE organization_id = $1 AND tags @> $2 ORDER BY name",
                        )
                        .bind(org_id)
                        .bind(serde_json::json!([t]))
                        .fetch_all(pool)
                        .await
                        .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                    (Some(org_id), None) => {
                        sqlx::query::<sqlx::Postgres>(
                            "SELECT * FROM teams WHERE organization_id = $1 ORDER BY name",
                        )
                        .bind(org_id)
                        .fetch_all(pool)
                        .await
                        .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                    (None, Some(t)) => {
                        sqlx::query::<sqlx::Postgres>(
                            "SELECT * FROM teams WHERE tags @> $1 ORDER BY name",
                        )
                        .bind(serde_json::json!([t]))
                        .fetch_all(pool)
                        .await
                        .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                    (None, None) => {
                        sqlx::query::<sqlx::Postgres>("SELECT * FROM teams ORDER BY name")
                            .fetch_all(pool)
                            .await
                            .map_err(|e| PylosError::Internal(format!("Failed to list teams: {}", e)))?
                    }
                };
                Ok(rows.iter().map(row_to_team_pg).collect())
            }
        }
    }

    pub async fn get_team(&self, id: &str) -> Result<Option<Team>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM teams WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to get team: {}", e)))?;
                Ok(row.as_ref().map(row_to_team_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row = sqlx::query::<sqlx::Postgres>("SELECT * FROM teams WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to get team: {}", e)))?;
                Ok(row.as_ref().map(row_to_team_pg))
            }
        }
    }

    pub async fn upsert_team(&self, team: &Team) -> Result<(), PylosError> {
        let tags_json = serde_json::to_string(&team.tags).unwrap_or_else(|_| "[]".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO teams (id, organization_id, name, description, is_active, tags, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        organization_id = excluded.organization_id,
                        name = excluded.name,
                        description = excluded.description,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&team.id)
                .bind(&team.organization_id)
                .bind(&team.name)
                .bind(&team.description)
                .bind(if team.is_active { 1 } else { 0 })
                .bind(&tags_json)
                .bind(team.created_at)
                .bind(team.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert team: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let tags_val: serde_json::Value =
                    serde_json::from_str(&tags_json).unwrap_or_default();
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO teams (id, organization_id, name, description, is_active, tags, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        organization_id = excluded.organization_id,
                        name = excluded.name,
                        description = excluded.description,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&team.id)
                .bind(&team.organization_id)
                .bind(&team.name)
                .bind(&team.description)
                .bind(team.is_active)
                .bind(&tags_val)
                .bind(team.created_at)
                .bind(team.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert team: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_team(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM teams WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete team: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM teams WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to delete team: {}", e)))?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }

    // ── Internal Users ─────────────────────────────────────────────────────────

    pub async fn list_users(&self) -> Result<Vec<InternalUser>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT * FROM internal_users ORDER BY name")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to list users: {}", e)))?;
                Ok(rows.iter().map(row_to_user_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM internal_users ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list users: {}", e))
                        })?;
                Ok(rows.iter().map(row_to_user_pg).collect())
            }
        }
    }

    pub async fn get_user(&self, id: &str) -> Result<Option<InternalUser>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM internal_users WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to get user: {}", e)))?;
                Ok(row.as_ref().map(row_to_user_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM internal_users WHERE id = $1")
                        .bind(id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| PylosError::Internal(format!("Failed to get user: {}", e)))?;
                Ok(row.as_ref().map(row_to_user_pg))
            }
        }
    }

    pub async fn upsert_user(&self, user: &InternalUser) -> Result<(), PylosError> {
        let team_ids_json =
            serde_json::to_string(&user.team_ids).unwrap_or_else(|_| "[]".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO internal_users (id, email, name, role, usr_group, organization_id, team_ids, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT(id) DO UPDATE SET
                        email = excluded.email,
                        name = excluded.name,
                        role = excluded.role,
                        usr_group = excluded.usr_group,
                        organization_id = excluded.organization_id,
                        team_ids = excluded.team_ids,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&user.id)
                .bind(&user.email)
                .bind(&user.name)
                .bind(&user.role)
                .bind(&user.group)
                .bind(&user.organization_id)
                .bind(&team_ids_json)
                .bind(if user.is_active { 1 } else { 0 })
                .bind(user.created_at)
                .bind(user.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert user: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let team_ids_val: serde_json::Value = serde_json::from_str(&team_ids_json)
                    .unwrap_or(serde_json::Value::Array(vec![]));
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO internal_users (id, email, name, role, usr_group, organization_id, team_ids, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                    ON CONFLICT(id) DO UPDATE SET
                        email = excluded.email,
                        name = excluded.name,
                        role = excluded.role,
                        usr_group = excluded.usr_group,
                        organization_id = excluded.organization_id,
                        team_ids = excluded.team_ids,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&user.id)
                .bind(&user.email)
                .bind(&user.name)
                .bind(&user.role)
                .bind(&user.group)
                .bind(&user.organization_id)
                .bind(&team_ids_val)
                .bind(user.is_active)
                .bind(user.created_at)
                .bind(user.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert user: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_user(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM internal_users WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete user: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM internal_users WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to delete user: {}", e)))?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }

    // ── Access Groups ──────────────────────────────────────────────────────────

    pub async fn list_access_groups(
        &self,
        tag: Option<&str>,
    ) -> Result<Vec<AccessGroup>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = if let Some(t) = tag {
                    sqlx::query("SELECT * FROM access_groups WHERE tags LIKE $1 ORDER BY name")
                        .bind(format!("%\"{}\"%", t))
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list access groups: {}", e))
                        })?
                } else {
                    sqlx::query("SELECT * FROM access_groups ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list access groups: {}", e))
                        })?
                };
                Ok(rows.iter().map(row_to_access_group_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows: Vec<sqlx::postgres::PgRow> = if let Some(t) = tag {
                    sqlx::query::<sqlx::Postgres>(
                        "SELECT * FROM access_groups WHERE tags @> $1 ORDER BY name",
                    )
                    .bind(serde_json::json!([t]))
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list access groups: {}", e))
                    })?
                } else {
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM access_groups ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list access groups: {}", e))
                        })?
                };
                Ok(rows.iter().map(row_to_access_group_pg).collect())
            }
        }
    }

    pub async fn get_access_group(&self, id: &str) -> Result<Option<AccessGroup>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let row = sqlx::query("SELECT * FROM access_groups WHERE id = $1")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to get access group: {}", e))
                    })?;
                Ok(row.as_ref().map(row_to_access_group_sqlite))
            }
            DbPool::Postgres(pool) => {
                let row =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM access_groups WHERE id = $1")
                        .bind(id)
                        .fetch_optional(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to get access group: {}", e))
                        })?;
                Ok(row.as_ref().map(row_to_access_group_pg))
            }
        }
    }

    pub async fn upsert_access_group(&self, ag: &AccessGroup) -> Result<(), PylosError> {
        let team_ids_json =
            serde_json::to_string(&ag.team_ids).unwrap_or_else(|_| "[]".to_string());
        let user_ids_json =
            serde_json::to_string(&ag.user_ids).unwrap_or_else(|_| "[]".to_string());
        let model_ids_json =
            serde_json::to_string(&ag.model_ids).unwrap_or_else(|_| "[]".to_string());
        let provider_ids_json =
            serde_json::to_string(&ag.provider_ids).unwrap_or_else(|_| "[]".to_string());
        let tags_json = serde_json::to_string(&ag.tags).unwrap_or_else(|_| "[]".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO access_groups (id, name, description, organization_id, team_ids, user_ids, model_ids, provider_ids, is_active, tags, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        organization_id = excluded.organization_id,
                        team_ids = excluded.team_ids,
                        user_ids = excluded.user_ids,
                        model_ids = excluded.model_ids,
                        provider_ids = excluded.provider_ids,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&ag.id)
                .bind(&ag.name)
                .bind(&ag.description)
                .bind(&ag.organization_id)
                .bind(&team_ids_json)
                .bind(&user_ids_json)
                .bind(&model_ids_json)
                .bind(&provider_ids_json)
                .bind(if ag.is_active { 1 } else { 0 })
                .bind(&tags_json)
                .bind(ag.created_at)
                .bind(ag.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert access group: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let team_ids_val: serde_json::Value =
                    serde_json::from_str(&team_ids_json).unwrap_or_default();
                let user_ids_val: serde_json::Value =
                    serde_json::from_str(&user_ids_json).unwrap_or_default();
                let model_ids_val: serde_json::Value =
                    serde_json::from_str(&model_ids_json).unwrap_or_default();
                let provider_ids_val: serde_json::Value =
                    serde_json::from_str(&provider_ids_json).unwrap_or_default();
                let tags_val: serde_json::Value =
                    serde_json::from_str(&tags_json).unwrap_or_default();
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO access_groups (id, name, description, organization_id, team_ids, user_ids, model_ids, provider_ids, is_active, tags, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        organization_id = excluded.organization_id,
                        team_ids = excluded.team_ids,
                        user_ids = excluded.user_ids,
                        model_ids = excluded.model_ids,
                        provider_ids = excluded.provider_ids,
                        is_active = excluded.is_active,
                        tags = excluded.tags,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&ag.id)
                .bind(&ag.name)
                .bind(&ag.description)
                .bind(&ag.organization_id)
                .bind(&team_ids_val)
                .bind(&user_ids_val)
                .bind(&model_ids_val)
                .bind(&provider_ids_val)
                .bind(ag.is_active)
                .bind(&tags_val)
                .bind(ag.created_at)
                .bind(ag.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert access group: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_access_group(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM access_groups WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete access group: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM access_groups WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to delete access group: {}", e))
                    })?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }

    // ── Policies ───────────────────────────────────────────────────────────────

    pub async fn list_policies(&self) -> Result<Vec<Policy>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT * FROM policies ORDER BY name")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to list policies: {}", e)))?;
                Ok(rows.iter().map(row_to_policy_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows = sqlx::query::<sqlx::Postgres>("SELECT * FROM policies ORDER BY name")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to list policies: {}", e)))?;
                Ok(rows.iter().map(row_to_policy_pg).collect())
            }
        }
    }

    pub async fn upsert_policy(&self, policy: &Policy) -> Result<(), PylosError> {
        let config_json =
            serde_json::to_string(&policy.config).unwrap_or_else(|_| "{}".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO policies (id, name, description, policy_type, config, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        policy_type = excluded.policy_type,
                        config = excluded.config,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&policy.id)
                .bind(&policy.name)
                .bind(&policy.description)
                .bind(&policy.policy_type)
                .bind(&config_json)
                .bind(if policy.is_active { 1 } else { 0 })
                .bind(policy.created_at)
                .bind(policy.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert policy: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let config_val: serde_json::Value = serde_json::from_str(&config_json)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO policies (id, name, description, policy_type, config, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        policy_type = excluded.policy_type,
                        config = excluded.config,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&policy.id)
                .bind(&policy.name)
                .bind(&policy.description)
                .bind(&policy.policy_type)
                .bind(&config_val)
                .bind(policy.is_active)
                .bind(policy.created_at)
                .bind(policy.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert policy: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_policy(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM policies WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete policy: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM policies WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| PylosError::Internal(format!("Failed to delete policy: {}", e)))?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }

    // ── Tool Policies ──────────────────────────────────────────────────────────

    pub async fn list_tool_policies(&self) -> Result<Vec<ToolPolicy>, PylosError> {
        match &self.pool {
            DbPool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT * FROM tool_policies ORDER BY name")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to list tool policies: {}", e))
                    })?;
                Ok(rows.iter().map(row_to_tool_policy_sqlite).collect())
            }
            DbPool::Postgres(pool) => {
                let rows =
                    sqlx::query::<sqlx::Postgres>("SELECT * FROM tool_policies ORDER BY name")
                        .fetch_all(pool)
                        .await
                        .map_err(|e| {
                            PylosError::Internal(format!("Failed to list tool policies: {}", e))
                        })?;
                Ok(rows.iter().map(row_to_tool_policy_pg).collect())
            }
        }
    }

    pub async fn upsert_tool_policy(&self, tp: &ToolPolicy) -> Result<(), PylosError> {
        let models_json =
            serde_json::to_string(&tp.allowed_models).unwrap_or_else(|_| "[]".to_string());
        let providers_json =
            serde_json::to_string(&tp.allowed_providers).unwrap_or_else(|_| "[]".to_string());
        match &self.pool {
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO tool_policies (id, name, description, tool_type, allowed_models, allowed_providers, max_tokens_per_call, max_calls_per_minute, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        tool_type = excluded.tool_type,
                        allowed_models = excluded.allowed_models,
                        allowed_providers = excluded.allowed_providers,
                        max_tokens_per_call = excluded.max_tokens_per_call,
                        max_calls_per_minute = excluded.max_calls_per_minute,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&tp.id)
                .bind(&tp.name)
                .bind(&tp.description)
                .bind(&tp.tool_type)
                .bind(&models_json)
                .bind(&providers_json)
                .bind(tp.max_tokens_per_call)
                .bind(tp.max_calls_per_minute)
                .bind(if tp.is_active { 1 } else { 0 })
                .bind(tp.created_at)
                .bind(tp.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert tool policy: {}", e)))?;
            }
            DbPool::Postgres(pool) => {
                let models_val: serde_json::Value =
                    serde_json::from_str(&models_json).unwrap_or_default();
                let providers_val: serde_json::Value =
                    serde_json::from_str(&providers_json).unwrap_or_default();
                sqlx::query::<sqlx::Postgres>(
                    r#"INSERT INTO tool_policies (id, name, description, tool_type, allowed_models, allowed_providers, max_tokens_per_call, max_calls_per_minute, is_active, created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                    ON CONFLICT(id) DO UPDATE SET
                        name = excluded.name,
                        description = excluded.description,
                        tool_type = excluded.tool_type,
                        allowed_models = excluded.allowed_models,
                        allowed_providers = excluded.allowed_providers,
                        max_tokens_per_call = excluded.max_tokens_per_call,
                        max_calls_per_minute = excluded.max_calls_per_minute,
                        is_active = excluded.is_active,
                        updated_at = excluded.updated_at"#,
                )
                .bind(&tp.id)
                .bind(&tp.name)
                .bind(&tp.description)
                .bind(&tp.tool_type)
                .bind(&models_val)
                .bind(&providers_val)
                .bind(tp.max_tokens_per_call)
                .bind(tp.max_calls_per_minute)
                .bind(tp.is_active)
                .bind(tp.created_at)
                .bind(tp.updated_at)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to upsert tool policy: {}", e)))?;
            }
        }
        Ok(())
    }

    pub async fn delete_tool_policy(&self, id: &str) -> Result<bool, PylosError> {
        let rows = match &self.pool {
            DbPool::Sqlite(pool) => sqlx::query("DELETE FROM tool_policies WHERE id = $1")
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| PylosError::Internal(format!("Failed to delete tool policy: {}", e)))?
                .rows_affected(),
            DbPool::Postgres(pool) => {
                sqlx::query::<sqlx::Postgres>("DELETE FROM tool_policies WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        PylosError::Internal(format!("Failed to delete tool policy: {}", e))
                    })?
                    .rows_affected()
            }
        };
        Ok(rows > 0)
    }
}

// ── Row mappers (SQLite) ──────────────────────────────────────────────────────

fn row_to_org_sqlite(row: &sqlx::sqlite::SqliteRow) -> Organization {
    Organization {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        tags: serde_json::from_str(&row.try_get::<String, _>("tags").unwrap_or_default())
            .unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_team_sqlite(row: &sqlx::sqlite::SqliteRow) -> Team {
    Team {
        id: row.try_get("id").unwrap_or_default(),
        organization_id: row.try_get("organization_id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        tags: serde_json::from_str(&row.try_get::<String, _>("tags").unwrap_or_default())
            .unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_user_sqlite(row: &sqlx::sqlite::SqliteRow) -> InternalUser {
    let team_ids_str: String = row.try_get("team_ids").unwrap_or_default();
    let team_ids: Vec<String> = serde_json::from_str(&team_ids_str).unwrap_or_default();
    InternalUser {
        id: row.try_get("id").unwrap_or_default(),
        email: row.try_get("email").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        role: row.try_get("role").unwrap_or_default(),
        group: row
            .try_get("usr_group")
            .unwrap_or_else(|_| "default".to_string()),
        organization_id: row.try_get("organization_id").ok(),
        team_ids,
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_access_group_sqlite(row: &sqlx::sqlite::SqliteRow) -> AccessGroup {
    AccessGroup {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        organization_id: row.try_get("organization_id").ok(),
        team_ids: serde_json::from_str(&row.try_get::<String, _>("team_ids").unwrap_or_default())
            .unwrap_or_default(),
        user_ids: serde_json::from_str(&row.try_get::<String, _>("user_ids").unwrap_or_default())
            .unwrap_or_default(),
        model_ids: serde_json::from_str(&row.try_get::<String, _>("model_ids").unwrap_or_default())
            .unwrap_or_default(),
        provider_ids: serde_json::from_str(
            &row.try_get::<String, _>("provider_ids").unwrap_or_default(),
        )
        .unwrap_or_default(),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        tags: serde_json::from_str(&row.try_get::<String, _>("tags").unwrap_or_default())
            .unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_policy_sqlite(row: &sqlx::sqlite::SqliteRow) -> Policy {
    let config_str: String = row.try_get("config").unwrap_or_default();
    Policy {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        policy_type: row.try_get("policy_type").unwrap_or_default(),
        config: serde_json::from_str(&config_str).unwrap_or_default(),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_tool_policy_sqlite(row: &sqlx::sqlite::SqliteRow) -> ToolPolicy {
    ToolPolicy {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        tool_type: row.try_get("tool_type").unwrap_or_default(),
        allowed_models: serde_json::from_str(
            &row.try_get::<String, _>("allowed_models")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        allowed_providers: serde_json::from_str(
            &row.try_get::<String, _>("allowed_providers")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        max_tokens_per_call: row.try_get("max_tokens_per_call").ok(),
        max_calls_per_minute: row.try_get("max_calls_per_minute").ok(),
        is_active: row.try_get::<i64, _>("is_active").unwrap_or(1) != 0,
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

// ── Row mappers (Postgres) ────────────────────────────────────────────────────

fn row_to_org_pg(row: &sqlx::postgres::PgRow) -> Organization {
    Organization {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        tags: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("tags")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_team_pg(row: &sqlx::postgres::PgRow) -> Team {
    Team {
        id: row.try_get("id").unwrap_or_default(),
        organization_id: row.try_get("organization_id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        tags: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("tags")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_user_pg(row: &sqlx::postgres::PgRow) -> InternalUser {
    let team_ids_val: serde_json::Value = row
        .try_get("team_ids")
        .unwrap_or(serde_json::Value::Array(vec![]));
    let team_ids: Vec<String> = serde_json::from_value(team_ids_val).unwrap_or_default();
    InternalUser {
        id: row.try_get("id").unwrap_or_default(),
        email: row.try_get("email").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        role: row.try_get("role").unwrap_or_default(),
        group: row
            .try_get("usr_group")
            .unwrap_or_else(|_| "default".to_string()),
        organization_id: row.try_get("organization_id").ok(),
        team_ids,
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_access_group_pg(row: &sqlx::postgres::PgRow) -> AccessGroup {
    AccessGroup {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        organization_id: row.try_get("organization_id").ok(),
        team_ids: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("team_ids")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        user_ids: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("user_ids")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        model_ids: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("model_ids")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        provider_ids: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("provider_ids")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        tags: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("tags")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_policy_pg(row: &sqlx::postgres::PgRow) -> Policy {
    let config_val: serde_json::Value = row.try_get("config").unwrap_or_default();
    Policy {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        policy_type: row.try_get("policy_type").unwrap_or_default(),
        config: config_val,
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}

fn row_to_tool_policy_pg(row: &sqlx::postgres::PgRow) -> ToolPolicy {
    ToolPolicy {
        id: row.try_get("id").unwrap_or_default(),
        name: row.try_get("name").unwrap_or_default(),
        description: row.try_get("description").ok(),
        tool_type: row.try_get("tool_type").unwrap_or_default(),
        allowed_models: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("allowed_models")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        allowed_providers: serde_json::from_value(
            row.try_get::<serde_json::Value, _>("allowed_providers")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
        max_tokens_per_call: row.try_get("max_tokens_per_call").ok(),
        max_calls_per_minute: row.try_get("max_calls_per_minute").ok(),
        is_active: row.try_get::<bool, _>("is_active").unwrap_or(true),
        created_at: row.try_get("created_at").unwrap_or_default(),
        updated_at: row.try_get("updated_at").unwrap_or_default(),
    }
}
