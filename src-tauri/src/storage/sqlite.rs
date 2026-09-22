/// SQLite implementation of all repository traits.
/// Uses sqlx with the sqlite driver. All timestamps are stored as UTC ISO8601 TEXT.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::error::{AppError, Result};
use crate::models::{Bar, BarSource, Interval, Symbol, SymbolMapping, SymbolStatus, SyncStatus};

use super::repository::*;

// ──────────────────────────────────────────────────────────────
// SqliteSymbolRepository
// ──────────────────────────────────────────────────────────────

pub struct SqliteSymbolRepository {
    pool: SqlitePool,
}

impl SqliteSymbolRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SymbolRepository for SqliteSymbolRepository {
    async fn insert_symbol(&self, s: &NewSymbol) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let id = sqlx::query(
            r#"INSERT INTO symbols
               (symbol, exchange, instrument_type, expiry, strike, option_type,
                tick_size, lot_size, status, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&s.symbol)
        .bind(&s.exchange)
        .bind(&s.instrument_type)
        .bind(&s.expiry)
        .bind(s.strike)
        .bind(&s.option_type)
        .bind(s.tick_size)
        .bind(s.lot_size)
        .bind(&s.status)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?
        .last_insert_rowid();
        Ok(id)
    }

    async fn upsert_symbol(&self, s: &NewSymbol) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let id = sqlx::query(
            r#"INSERT INTO symbols
               (symbol, exchange, instrument_type, expiry, strike, option_type,
                tick_size, lot_size, status, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(symbol, exchange, instrument_type, expiry, strike, option_type)
               DO UPDATE SET
                 tick_size  = excluded.tick_size,
                 lot_size   = excluded.lot_size,
                 updated_at = excluded.updated_at
               RETURNING id"#,
        )
        .bind(&s.symbol)
        .bind(&s.exchange)
        .bind(&s.instrument_type)
        .bind(&s.expiry)
        .bind(s.strike)
        .bind(&s.option_type)
        .bind(s.tick_size)
        .bind(s.lot_size)
        .bind(&s.status)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?
        .get::<i64, _>("id");
        Ok(id)
    }

    async fn get_symbol_by_id(&self, id: i64) -> Result<Option<Symbol>> {
        let row = sqlx::query_as!(
            SymbolRow,
            "SELECT * FROM symbols WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row.map(symbol_row_to_model))
    }

    async fn get_symbol_by_key(&self, symbol: &str, exchange: &str) -> Result<Option<Symbol>> {
        let row = sqlx::query_as!(
            SymbolRow,
            "SELECT * FROM symbols WHERE symbol = ? AND exchange = ? LIMIT 1",
            symbol, exchange
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row.map(symbol_row_to_model))
    }

    async fn list_symbols(&self, filter: &SymbolFilter) -> Result<Vec<Symbol>> {
        // Build a dynamic query — sqlx doesn't support fully dynamic queries
        // with compile-time checks, so we use the runtime query builder.
        let mut query = String::from("SELECT * FROM symbols WHERE 1=1");
        let mut args: Vec<String> = Vec::new();

        if let Some(exchange) = &filter.exchange {
            query.push_str(" AND exchange = ?");
            args.push(exchange.clone());
        }
        if let Some(status) = &filter.status {
            query.push_str(" AND status = ?");
            args.push(status.to_string());
        }
        if let Some(search) = &filter.search {
            query.push_str(" AND (symbol LIKE ? OR exchange LIKE ?)");
            let pattern = format!("%{}%", search);
            args.push(pattern.clone());
            args.push(pattern);
        }

        query.push_str(" ORDER BY symbol ASC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut q = sqlx::query(&query);
        for arg in &args {
            q = q.bind(arg);
        }

        let rows = q.fetch_all(&self.pool).await.map_err(AppError::Database)?;
        let symbols: Vec<Symbol> = rows.iter().map(|r| {
            Symbol {
                id: r.get("id"),
                symbol: r.get("symbol"),
                exchange: r.get("exchange"),
                instrument_type: r.get::<String, _>("instrument_type").parse().unwrap_or(crate::models::InstrumentType::Other),
                expiry: r.get("expiry"),
                strike: r.get("strike"),
                option_type: r.get::<Option<String>, _>("option_type").and_then(|s| s.parse().ok()),
                tick_size: r.get("tick_size"),
                lot_size: r.get("lot_size"),
                status: if r.get::<String, _>("status") == "enabled" {
                    SymbolStatus::Enabled
                } else {
                    SymbolStatus::Disabled
                },
                created_at: r.get::<String, _>("created_at").parse().unwrap_or_else(|_| Utc::now()),
                updated_at: r.get::<String, _>("updated_at").parse().unwrap_or_else(|_| Utc::now()),
            }
        }).collect();
        Ok(symbols)
    }

    async fn count_symbols(&self, status: Option<SymbolStatus>) -> Result<i64> {
        let count = if let Some(s) = status {
            sqlx::query_scalar!("SELECT COUNT(*) FROM symbols WHERE status = ?", s.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            sqlx::query_scalar!("SELECT COUNT(*) FROM symbols")
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?
        };
        Ok(count)
    }

    async fn set_symbol_status(&self, id: i64, status: SymbolStatus) -> Result<()> {
        sqlx::query!(
            "UPDATE symbols SET status = ?, updated_at = ? WHERE id = ?",
            status.to_string(), Utc::now().to_rfc3339(), id
        )
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn delete_symbol(&self, id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM symbols WHERE id = ?", id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    async fn upsert_symbol_mapping(&self, m: &NewSymbolMapping) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let id = sqlx::query(
            r#"INSERT INTO symbol_mappings
               (symbol_id, broker_id, broker_symbol, broker_token, sync_status, last_synced_at)
               VALUES (?, ?, ?, ?, 'not_started', NULL)
               ON CONFLICT(symbol_id, broker_id)
               DO UPDATE SET
                 broker_symbol = excluded.broker_symbol,
                 broker_token  = excluded.broker_token
               ON CONFLICT(broker_id, broker_token)
               DO UPDATE SET
                 symbol_id    = excluded.symbol_id,
                 broker_symbol = excluded.broker_symbol
               RETURNING id"#,
        )
        .bind(m.symbol_id)
        .bind(m.broker_id)
        .bind(&m.broker_symbol)
        .bind(&m.broker_token)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?
        .get::<i64, _>("id");
        let _ = now;
        Ok(id)
    }

    async fn get_mapping_by_token(&self, broker_id: i64, token: &str) -> Result<Option<SymbolMapping>> {
        let row = sqlx::query(
            "SELECT * FROM symbol_mappings WHERE broker_id = ? AND broker_token = ?"
        )
        .bind(broker_id)
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row.map(|r| mapping_row_to_model(&r)))
    }

    async fn get_mapping_for_symbol(&self, symbol_id: i64, broker_id: i64) -> Result<Option<SymbolMapping>> {
        let row = sqlx::query(
            "SELECT * FROM symbol_mappings WHERE symbol_id = ? AND broker_id = ?"
        )
        .bind(symbol_id)
        .bind(broker_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row.map(|r| mapping_row_to_model(&r)))
    }

    async fn list_mappings_for_broker(&self, broker_id: i64) -> Result<Vec<SymbolMapping>> {
        let rows = sqlx::query(
            "SELECT * FROM symbol_mappings WHERE broker_id = ?"
        )
        .bind(broker_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows.iter().map(|r| mapping_row_to_model(r)).collect())
    }
}

// ──────────────────────────────────────────────────────────────
// SqliteHistoricalRepository
// ──────────────────────────────────────────────────────────────

pub struct SqliteHistoricalRepository {
    pool: SqlitePool,
}

impl SqliteHistoricalRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
}

#[async_trait]
impl HistoricalRepository for SqliteHistoricalRepository {
    async fn upsert_bars(&self, bars: &[NewBar]) -> Result<usize> {
        if bars.is_empty() { return Ok(0); }

        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;
        let mut inserted = 0usize;

        for bar in bars {
            let ts = bar.timestamp.to_rfc3339();
            sqlx::query(
                r#"INSERT INTO historical_bars
                   (symbol_id, interval, timestamp, open, high, low, close, volume, source)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(symbol_id, interval, timestamp)
                   DO UPDATE SET
                     open   = excluded.open,
                     high   = excluded.high,
                     low    = excluded.low,
                     close  = excluded.close,
                     volume = excluded.volume,
                     source = excluded.source"#,
            )
            .bind(bar.symbol_id)
            .bind(&bar.interval)
            .bind(&ts)
            .bind(bar.open)
            .bind(bar.high)
            .bind(bar.low)
            .bind(bar.close)
            .bind(bar.volume as i64)
            .bind(&bar.source)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;
            inserted += 1;
        }

        tx.commit().await.map_err(AppError::Database)?;
        Ok(inserted)
    }

    async fn query_bars(
        &self,
        symbol_id: i64,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Bar>> {
        let from_s = from.to_rfc3339();
        let to_s = to.to_rfc3339();
        let interval_s = interval.as_str();

        let rows = sqlx::query(
            r#"SELECT symbol_id, interval, timestamp, open, high, low, close, volume, source
               FROM historical_bars
               WHERE symbol_id = ? AND interval = ?
                 AND timestamp >= ? AND timestamp <= ?
               ORDER BY timestamp ASC"#,
        )
        .bind(symbol_id)
        .bind(interval_s)
        .bind(&from_s)
        .bind(&to_s)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        let bars: Vec<Bar> = rows.iter().map(|r| {
            let src: String = r.get("source");
            Bar {
                symbol: String::new(), // caller can join if needed
                exchange: String::new(),
                interval,
                timestamp: r.get::<String, _>("timestamp").parse().unwrap_or_else(|_| Utc::now()),
                open: r.get("open"),
                high: r.get("high"),
                low: r.get("low"),
                close: r.get("close"),
                volume: r.get::<i64, _>("volume") as u64,
                source: if src == "live_aggregation" { BarSource::LiveAggregation } else { BarSource::Backfill },
            }
        }).collect();

        Ok(bars)
    }

    async fn latest_bar_timestamp(&self, symbol_id: i64, interval: Interval) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query(
            "SELECT MAX(timestamp) as ts FROM historical_bars WHERE symbol_id = ? AND interval = ?"
        )
        .bind(symbol_id)
        .bind(interval.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.and_then(|r| {
            r.get::<Option<String>, _>("ts")
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        }))
    }

    async fn oldest_bar_timestamp(&self, symbol_id: i64, interval: Interval) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query(
            "SELECT MIN(timestamp) as ts FROM historical_bars WHERE symbol_id = ? AND interval = ?"
        )
        .bind(symbol_id)
        .bind(interval.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.and_then(|r| {
            r.get::<Option<String>, _>("ts")
                .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        }))
    }

    async fn count_bars(&self, symbol_id: i64, interval: Interval) -> Result<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM historical_bars WHERE symbol_id = ? AND interval = ?",
            symbol_id, interval.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(count)
    }

    async fn delete_bars_for_symbol(&self, symbol_id: i64) -> Result<()> {
        sqlx::query!("DELETE FROM historical_bars WHERE symbol_id = ?", symbol_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// SqliteBackfillRepository
// ──────────────────────────────────────────────────────────────

pub struct SqliteBackfillRepository {
    pool: SqlitePool,
}

impl SqliteBackfillRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
}

#[async_trait]
impl BackfillRepository for SqliteBackfillRepository {
    async fn create_job(&self, job: &NewBackfillJob) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let from_s = job.requested_from.to_rfc3339();
        let to_s = job.requested_to.to_rfc3339();
        let id = sqlx::query(
            r#"INSERT INTO backfill_jobs
               (symbol_id, requested_from, requested_to, status, attempt_count, priority, created_at, updated_at)
               VALUES (?, ?, ?, 'queued', 0, ?, ?, ?)
               RETURNING id"#,
        )
        .bind(job.symbol_id)
        .bind(&from_s)
        .bind(&to_s)
        .bind(job.priority)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?
        .get::<i64, _>("id");
        Ok(id)
    }

    async fn update_job_status(&self, job_id: i64, update: &BackfillJobUpdate) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let fetched = update.fetched_up_to.as_ref().map(|d| d.to_rfc3339());
        sqlx::query(
            r#"UPDATE backfill_jobs SET
               status = ?,
               fetched_up_to = COALESCE(?, fetched_up_to),
               attempt_count = COALESCE(?, attempt_count),
               last_error    = ?,
               updated_at    = ?
               WHERE id = ?"#,
        )
        .bind(update.status.as_str())
        .bind(fetched)
        .bind(update.attempt_count)
        .bind(&update.last_error)
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn get_job(&self, job_id: i64) -> Result<Option<BackfillJob>> {
        let row = sqlx::query(
            "SELECT * FROM backfill_jobs WHERE id = ?"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row.map(|r| backfill_row_to_job(&r)))
    }

    async fn list_jobs_by_status(&self, statuses: &[BackfillJobStatus]) -> Result<Vec<BackfillJob>> {
        if statuses.is_empty() { return Ok(vec![]); }
        let placeholders: String = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT * FROM backfill_jobs WHERE status IN ({}) ORDER BY priority DESC, created_at ASC",
            placeholders
        );
        let mut q = sqlx::query(&query);
        for s in statuses {
            q = q.bind(s.as_str());
        }
        let rows = q.fetch_all(&self.pool).await.map_err(AppError::Database)?;
        Ok(rows.iter().map(|r| backfill_row_to_job(r)).collect())
    }

    async fn list_jobs_for_symbol(&self, symbol_id: i64) -> Result<Vec<BackfillJob>> {
        let rows = sqlx::query(
            "SELECT * FROM backfill_jobs WHERE symbol_id = ? ORDER BY created_at DESC"
        )
        .bind(symbol_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows.iter().map(|r| backfill_row_to_job(r)).collect())
    }

    async fn load_resumable_jobs(&self) -> Result<Vec<BackfillJob>> {
        let rows = sqlx::query(
            r#"SELECT * FROM backfill_jobs
               WHERE status NOT IN ('completed', 'failed', 'cancelled')
               ORDER BY priority DESC, created_at ASC"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(rows.iter().map(|r| backfill_row_to_job(r)).collect())
    }

    async fn create_gap(&self, gap: &NewBackfillGap) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let id = sqlx::query(
            r#"INSERT INTO backfill_gaps (symbol_id, gap_start, gap_end, detected_at, status)
               VALUES (?, ?, ?, ?, 'open')
               RETURNING id"#,
        )
        .bind(gap.symbol_id)
        .bind(gap.gap_start.to_rfc3339())
        .bind(gap.gap_end.to_rfc3339())
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?
        .get::<i64, _>("id");
        Ok(id)
    }

    async fn list_open_gaps(&self, symbol_id: i64) -> Result<Vec<BackfillGap>> {
        let rows = sqlx::query(
            "SELECT * FROM backfill_gaps WHERE symbol_id = ? AND status = 'open'"
        )
        .bind(symbol_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.iter().map(|r| BackfillGap {
            id: r.get("id"),
            symbol_id: r.get("symbol_id"),
            gap_start: r.get::<String, _>("gap_start").parse().unwrap_or_else(|_| Utc::now()),
            gap_end: r.get::<String, _>("gap_end").parse().unwrap_or_else(|_| Utc::now()),
            detected_at: r.get::<String, _>("detected_at").parse().unwrap_or_else(|_| Utc::now()),
            repaired_at: r.get::<Option<String>, _>("repaired_at").and_then(|s| s.parse().ok()),
            status: r.get("status"),
        }).collect())
    }

    async fn mark_gap_repaired(&self, gap_id: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE backfill_gaps SET status = 'repaired', repaired_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(gap_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// SqliteSettingsRepository
// ──────────────────────────────────────────────────────────────

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get(&self, key: &str) -> Result<Option<String>> {
        let row = sqlx::query_scalar!(
            "SELECT value FROM app_settings WHERE key = ?", key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row)
    }

    async fn set(&self, key: &str, value: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT INTO app_settings (key, value, updated_at) VALUES (?, ?, ?)
               ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"#,
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn get_all(&self) -> Result<std::collections::HashMap<String, String>> {
        let rows = sqlx::query!("SELECT key, value FROM app_settings")
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(rows.into_iter().map(|r| (r.key, r.value)).collect())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        sqlx::query!("DELETE FROM app_settings WHERE key = ?", key)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// SqliteLogRepository
// ──────────────────────────────────────────────────────────────

pub struct SqliteLogRepository {
    pool: SqlitePool,
}

impl SqliteLogRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
}

#[async_trait]
impl LogRepository for SqliteLogRepository {
    async fn insert_log(&self, entry: &NewLogEntry) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO system_logs (timestamp, level, module, message, symbol, error_code)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(entry.timestamp.to_rfc3339())
        .bind(&entry.level)
        .bind(&entry.module)
        .bind(&entry.message)
        .bind(&entry.symbol)
        .bind(&entry.error_code)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<LogEntry>> {
        let mut query = String::from("SELECT * FROM system_logs WHERE 1=1");
        let mut args: Vec<String> = Vec::new();

        if let Some(level) = &filter.level {
            query.push_str(" AND level = ?");
            args.push(level.clone());
        }
        if let Some(module) = &filter.module {
            query.push_str(" AND module = ?");
            args.push(module.clone());
        }
        if let Some(symbol) = &filter.symbol {
            query.push_str(" AND symbol = ?");
            args.push(symbol.clone());
        }
        if let Some(since) = &filter.since {
            query.push_str(" AND timestamp >= ?");
            args.push(since.to_rfc3339());
        }

        query.push_str(" ORDER BY timestamp DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let mut q = sqlx::query(&query);
        for arg in &args {
            q = q.bind(arg);
        }

        let rows = q.fetch_all(&self.pool).await.map_err(AppError::Database)?;
        Ok(rows.iter().map(|r| LogEntry {
            id: r.get("id"),
            timestamp: r.get::<String, _>("timestamp").parse().unwrap_or_else(|_| Utc::now()),
            level: r.get("level"),
            module: r.get("module"),
            message: r.get("message"),
            symbol: r.get("symbol"),
            error_code: r.get("error_code"),
        }).collect())
    }

    async fn count_errors_last_hour(&self) -> Result<i64> {
        let since = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM system_logs WHERE level = 'ERROR' AND timestamp >= ?",
            since
        )
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(count)
    }

    async fn purge_old_logs(&self, keep_days: u32) -> Result<u64> {
        let cutoff = (Utc::now() - chrono::Duration::days(keep_days as i64)).to_rfc3339();
        let result = sqlx::query!(
            "DELETE FROM system_logs WHERE timestamp < ?", cutoff
        )
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(result.rows_affected())
    }
}

// ──────────────────────────────────────────────────────────────
// SqliteBrokerRepository
// ──────────────────────────────────────────────────────────────

pub struct SqliteBrokerRepository {
    pool: SqlitePool,
}

impl SqliteBrokerRepository {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
}

#[async_trait]
impl BrokerRepository for SqliteBrokerRepository {
    async fn get_broker_by_key(&self, key: &str) -> Result<Option<BrokerRow>> {
        let row = sqlx::query!(
            "SELECT id, broker_key, display_name, api_base_url, ws_base_url, is_active FROM brokers WHERE broker_key = ?",
            key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| BrokerRow {
            id: r.id,
            broker_key: r.broker_key,
            display_name: r.display_name,
            api_base_url: r.api_base_url,
            ws_base_url: r.ws_base_url,
            is_active: r.is_active == 1,
        }))
    }

    async fn list_brokers(&self) -> Result<Vec<BrokerRow>> {
        let rows = sqlx::query!(
            "SELECT id, broker_key, display_name, api_base_url, ws_base_url, is_active FROM brokers"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(|r| BrokerRow {
            id: r.id,
            broker_key: r.broker_key,
            display_name: r.display_name,
            api_base_url: r.api_base_url,
            ws_base_url: r.ws_base_url,
            is_active: r.is_active == 1,
        }).collect())
    }

    async fn upsert_session(&self, session: &NewSessionRow) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let expires = session.token_expires_at.as_ref().map(|d| d.to_rfc3339());
        let validated = session.last_validated_at.as_ref().map(|d| d.to_rfc3339());

        let id = sqlx::query(
            r#"INSERT INTO broker_sessions
               (broker_id, credential_ref, session_status, token_expires_at, last_validated_at, created_at, updated_at)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(broker_id) DO UPDATE SET
                 credential_ref    = excluded.credential_ref,
                 session_status    = excluded.session_status,
                 token_expires_at  = excluded.token_expires_at,
                 last_validated_at = excluded.last_validated_at,
                 updated_at        = excluded.updated_at
               RETURNING id"#,
        )
        .bind(session.broker_id)
        .bind(&session.credential_ref)
        .bind(&session.session_status)
        .bind(expires)
        .bind(validated)
        .bind(&now)
        .bind(&now)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?
        .get::<i64, _>("id");
        Ok(id)
    }

    async fn get_active_session(&self, broker_id: i64) -> Result<Option<SessionRow>> {
        let row = sqlx::query(
            "SELECT * FROM broker_sessions WHERE broker_id = ? ORDER BY updated_at DESC LIMIT 1"
        )
        .bind(broker_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| SessionRow {
            id: r.get("id"),
            broker_id: r.get("broker_id"),
            credential_ref: r.get("credential_ref"),
            session_status: r.get("session_status"),
            token_expires_at: r.get::<Option<String>, _>("token_expires_at").and_then(|s| s.parse().ok()),
            last_validated_at: r.get::<Option<String>, _>("last_validated_at").and_then(|s| s.parse().ok()),
        }))
    }

    async fn update_session_status(&self, broker_id: i64, status: &str, expires_at: Option<DateTime<Utc>>) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let expires = expires_at.map(|d| d.to_rfc3339());
        sqlx::query(
            r#"UPDATE broker_sessions SET
               session_status = ?, token_expires_at = ?, last_validated_at = ?, updated_at = ?
               WHERE broker_id = ?"#,
        )
        .bind(status)
        .bind(expires)
        .bind(&now)
        .bind(&now)
        .bind(broker_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────

struct SymbolRow {
    id: i64,
    symbol: String,
    exchange: String,
    instrument_type: String,
    expiry: Option<String>,
    strike: Option<f64>,
    option_type: Option<String>,
    tick_size: Option<f64>,
    lot_size: Option<i64>,
    status: String,
    created_at: String,
    updated_at: String,
}

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for SymbolRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        Ok(SymbolRow {
            id: row.get("id"),
            symbol: row.get("symbol"),
            exchange: row.get("exchange"),
            instrument_type: row.get("instrument_type"),
            expiry: row.get("expiry"),
            strike: row.get("strike"),
            option_type: row.get("option_type"),
            tick_size: row.get("tick_size"),
            lot_size: row.get("lot_size"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

fn symbol_row_to_model(r: SymbolRow) -> Symbol {
    Symbol {
        id: r.id,
        symbol: r.symbol,
        exchange: r.exchange,
        instrument_type: r.instrument_type.parse().unwrap_or(crate::models::InstrumentType::Other),
        expiry: r.expiry,
        strike: r.strike,
        option_type: r.option_type.and_then(|s| s.parse().ok()),
        tick_size: r.tick_size,
        lot_size: r.lot_size,
        status: if r.status == "enabled" { SymbolStatus::Enabled } else { SymbolStatus::Disabled },
        created_at: r.created_at.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: r.updated_at.parse().unwrap_or_else(|_| Utc::now()),
    }
}

fn mapping_row_to_model(r: &sqlx::sqlite::SqliteRow) -> SymbolMapping {
    SymbolMapping {
        id: r.get("id"),
        symbol_id: r.get("symbol_id"),
        broker_id: r.get("broker_id"),
        broker_symbol: r.get("broker_symbol"),
        broker_token: r.get("broker_token"),
        sync_status: r.get::<String, _>("sync_status").parse().unwrap_or(SyncStatus::NotStarted),
        last_synced_at: r.get::<Option<String>, _>("last_synced_at").and_then(|s| s.parse().ok()),
    }
}

fn backfill_row_to_job(r: &sqlx::sqlite::SqliteRow) -> BackfillJob {
    BackfillJob {
        id: r.get("id"),
        symbol_id: r.get("symbol_id"),
        requested_from: r.get::<String, _>("requested_from").parse().unwrap_or_else(|_| Utc::now()),
        requested_to: r.get::<String, _>("requested_to").parse().unwrap_or_else(|_| Utc::now()),
        fetched_up_to: r.get::<Option<String>, _>("fetched_up_to").and_then(|s| s.parse().ok()),
        status: r.get::<String, _>("status").parse().unwrap_or(BackfillJobStatus::Queued),
        attempt_count: r.get("attempt_count"),
        last_error: r.get("last_error"),
        priority: r.get("priority"),
        created_at: r.get::<String, _>("created_at").parse().unwrap_or_else(|_| Utc::now()),
        updated_at: r.get::<String, _>("updated_at").parse().unwrap_or_else(|_| Utc::now()),
    }
}
