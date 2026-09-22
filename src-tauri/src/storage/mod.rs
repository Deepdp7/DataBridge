/// storage/mod.rs — Storage subsystem
///
/// The AppStorage struct bundles all repository implementations behind
/// a single shared handle. It is created once at startup and injected
/// into the Tauri state and all core engine modules.

pub mod credentials;
pub mod repository;
pub mod sqlite;

use std::sync::Arc;

use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

use crate::error::Result;

use self::repository::*;
use self::sqlite::*;

/// All repository impls bundled together for easy injection.
#[derive(Clone)]
pub struct AppStorage {
    pub symbols: Arc<dyn SymbolRepository>,
    pub historical: Arc<dyn HistoricalRepository>,
    pub backfill: Arc<dyn BackfillRepository>,
    pub settings: Arc<dyn SettingsRepository>,
    pub logs: Arc<dyn LogRepository>,
    pub brokers: Arc<dyn BrokerRepository>,
    pub pool: SqlitePool,
}

impl AppStorage {
    /// Initialize storage: open/create the SQLite database and run migrations.
    pub async fn init(db_path: &str) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(crate::error::AppError::Database)?;

        // Run migrations (embedded at compile time)
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(crate::error::AppError::Migration)?;

        Ok(AppStorage {
            symbols:    Arc::new(SqliteSymbolRepository::new(pool.clone())),
            historical: Arc::new(SqliteHistoricalRepository::new(pool.clone())),
            backfill:   Arc::new(SqliteBackfillRepository::new(pool.clone())),
            settings:   Arc::new(SqliteSettingsRepository::new(pool.clone())),
            logs:       Arc::new(SqliteLogRepository::new(pool.clone())),
            brokers:    Arc::new(SqliteBrokerRepository::new(pool.clone())),
            pool,
        })
    }
}
