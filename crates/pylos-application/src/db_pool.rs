use std::path::Path;

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tracing::info;

/// Pool de base de données partagé pour tous les stores (SQLite ou Postgres).
/// Élimine la duplication du pattern `Pool` enum + migrations dans chaque store.
#[derive(Clone)]
pub(crate) enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl DbPool {
    pub async fn open_sqlite(db_path: &Path, pool_name: &str, max_connections: u32) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect_with(options)
            .await?;

        info!(path = %db_path.display(), pool = %pool_name, "Store opened (SQLite)");
        Ok(DbPool::Sqlite(pool))
    }

    pub async fn open_postgres(database_url: &str, pool_name: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await?;

        info!(pool = %pool_name, "Store opened (PostgreSQL)");
        Ok(DbPool::Postgres(pool))
    }

    pub async fn in_memory(max_connections: u32) -> Result<Self, sqlx::Error> {
        let pool = SqlitePoolOptions::new()
            .max_connections(max_connections)
            .connect("sqlite::memory:")
            .await?;
        Ok(DbPool::Sqlite(pool))
    }

    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        match self {
            DbPool::Sqlite(pool) => sqlx::migrate!("./migrations")
                .run(pool)
                .await
                .map_err(|e| sqlx::Error::Migrate(Box::new(e))),
            DbPool::Postgres(pool) => sqlx::migrate!("./migrations_postgres")
                .run(pool)
                .await
                .map_err(|e| sqlx::Error::Migrate(Box::new(e))),
        }
    }

    pub fn as_sqlite(&self) -> Option<&SqlitePool> {
        match self {
            DbPool::Sqlite(p) => Some(p),
            DbPool::Postgres(_) => None,
        }
    }

}
