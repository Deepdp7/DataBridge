/// Storage Engine — repository traits
///
/// All persistence is accessed exclusively through these traits.
/// The current implementation uses SQLite; swapping to DuckDB/Parquet
/// only requires a new struct implementing the same traits (PRD §11.2).

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::models::{
    Bar, ImportSummary, Interval, Symbol, SymbolMapping, SymbolStatus,
};

// ──────────────────────────────────────────────────────────────
// Symbol repository
// ──────────────────────────────────────────────────────────────

#[async_trait]
pub trait SymbolRepository: Send + Sync {
    async fn insert_symbol(&self, symbol: &NewSymbol) -> Result<i64>;
    async fn upsert_symbol(&self, symbol: &NewSymbol) -> Result<i64>;
    async fn get_symbol_by_id(&self, id: i64) -> Result<Option<Symbol>>;
    async fn get_symbol_by_key(&self, symbol: &str, exchange: &str) -> Result<Option<Symbol>>;
    async fn list_symbols(&self, filter: &SymbolFilter) -> Result<Vec<Symbol>>;
    async fn count_symbols(&self, status: Option<SymbolStatus>) -> Result<i64>;
    async fn set_symbol_status(&self, id: i64, status: SymbolStatus) -> Result<()>;
    async fn delete_symbol(&self, id: i64) -> Result<()>;

    async fn upsert_symbol_mapping(&self, mapping: &NewSymbolMapping) -> Result<i64>;
    async fn get_mapping_by_token(&self, broker_id: i64, token: &str) -> Result<Option<SymbolMapping>>;
    async fn get_mapping_for_symbol(&self, symbol_id: i64, broker_id: i64) -> Result<Option<SymbolMapping>>;
    async fn list_mappings_for_broker(&self, broker_id: i64) -> Result<Vec<SymbolMapping>>;
}

// ──────────────────────────────────────────────────────────────
// Historical data repository
// ──────────────────────────────────────────────────────────────

#[async_trait]
pub trait HistoricalRepository: Send + Sync {
    /// Idempotent upsert — uses ON CONFLICT DO UPDATE on (symbol_id, interval, timestamp).
    async fn upsert_bars(&self, bars: &[NewBar]) -> Result<usize>;

    async fn query_bars(
        &self,
        symbol_id: i64,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<Bar>>;

    /// Latest stored bar timestamp for a symbol/interval combo.
    /// Used by the Historical Engine to determine the incremental sync window.
    async fn latest_bar_timestamp(
        &self,
        symbol_id: i64,
        interval: Interval,
    ) -> Result<Option<DateTime<Utc>>>;

    /// Oldest stored bar timestamp.
    async fn oldest_bar_timestamp(
        &self,
        symbol_id: i64,
        interval: Interval,
    ) -> Result<Option<DateTime<Utc>>>;

    async fn count_bars(&self, symbol_id: i64, interval: Interval) -> Result<i64>;

    /// Delete all bars for a symbol (used when a symbol is removed).
    async fn delete_bars_for_symbol(&self, symbol_id: i64) -> Result<()>;
}

// ──────────────────────────────────────────────────────────────
// Backfill job repository
// ──────────────────────────────────────────────────────────────

#[async_trait]
pub trait BackfillRepository: Send + Sync {
    async fn create_job(&self, job: &NewBackfillJob) -> Result<i64>;
    async fn update_job_status(&self, job_id: i64, update: &BackfillJobUpdate) -> Result<()>;
    async fn get_job(&self, job_id: i64) -> Result<Option<BackfillJob>>;
    async fn list_jobs_by_status(&self, status: &[BackfillJobStatus]) -> Result<Vec<BackfillJob>>;
    async fn list_jobs_for_symbol(&self, symbol_id: i64) -> Result<Vec<BackfillJob>>;
    /// Load all non-terminal jobs at startup for resume.
    async fn load_resumable_jobs(&self) -> Result<Vec<BackfillJob>>;

    async fn create_gap(&self, gap: &NewBackfillGap) -> Result<i64>;
    async fn list_open_gaps(&self, symbol_id: i64) -> Result<Vec<BackfillGap>>;
    async fn mark_gap_repaired(&self, gap_id: i64) -> Result<()>;
}

// ──────────────────────────────────────────────────────────────
// Settings repository
// ──────────────────────────────────────────────────────────────

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>>;
    async fn set(&self, key: &str, value: &str) -> Result<()>;
    async fn get_all(&self) -> Result<std::collections::HashMap<String, String>>;
    async fn delete(&self, key: &str) -> Result<()>;
}

// ──────────────────────────────────────────────────────────────
// Logs repository
// ──────────────────────────────────────────────────────────────

#[async_trait]
pub trait LogRepository: Send + Sync {
    async fn insert_log(&self, entry: &NewLogEntry) -> Result<()>;
    async fn query_logs(&self, filter: &LogFilter) -> Result<Vec<LogEntry>>;
    async fn count_errors_last_hour(&self) -> Result<i64>;
    async fn purge_old_logs(&self, keep_days: u32) -> Result<u64>;
}

// ──────────────────────────────────────────────────────────────
// Broker / session repository
// ──────────────────────────────────────────────────────────────

#[async_trait]
pub trait BrokerRepository: Send + Sync {
    async fn get_broker_by_key(&self, key: &str) -> Result<Option<BrokerRow>>;
    async fn list_brokers(&self) -> Result<Vec<BrokerRow>>;
    async fn upsert_session(&self, session: &NewSessionRow) -> Result<i64>;
    async fn get_active_session(&self, broker_id: i64) -> Result<Option<SessionRow>>;
    async fn update_session_status(&self, broker_id: i64, status: &str, expires_at: Option<DateTime<Utc>>) -> Result<()>;
}

// ──────────────────────────────────────────────────────────────
// Data transfer objects (write-side structs)
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NewSymbol {
    pub symbol: String,
    pub exchange: String,
    pub instrument_type: String,
    pub expiry: Option<String>,
    pub strike: Option<f64>,
    pub option_type: Option<String>,
    pub tick_size: Option<f64>,
    pub lot_size: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct NewSymbolMapping {
    pub symbol_id: i64,
    pub broker_id: i64,
    pub broker_symbol: String,
    pub broker_token: String,
}

#[derive(Debug, Clone)]
pub struct NewBar {
    pub symbol_id: i64,
    pub interval: String,
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct SymbolFilter {
    pub exchange: Option<String>,
    pub status: Option<SymbolStatus>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ──────────────────────────────────────────────────────────────
// Backfill job structs
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackfillJobStatus {
    Queued,
    Downloading,
    Processing,
    Completed,
    Partial,
    Failed,
    Retrying,
    Cancelled,
}

impl BackfillJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued      => "queued",
            Self::Downloading => "downloading",
            Self::Processing  => "processing",
            Self::Completed   => "completed",
            Self::Partial     => "partial",
            Self::Failed      => "failed",
            Self::Retrying    => "retrying",
            Self::Cancelled   => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

impl std::fmt::Display for BackfillJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for BackfillJobStatus {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "queued"      => Ok(Self::Queued),
            "downloading" => Ok(Self::Downloading),
            "processing"  => Ok(Self::Processing),
            "completed"   => Ok(Self::Completed),
            "partial"     => Ok(Self::Partial),
            "failed"      => Ok(Self::Failed),
            "retrying"    => Ok(Self::Retrying),
            "cancelled"   => Ok(Self::Cancelled),
            other         => Err(format!("Unknown status: '{}'", other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewBackfillJob {
    pub symbol_id: i64,
    pub requested_from: DateTime<Utc>,
    pub requested_to: DateTime<Utc>,
    pub priority: i64,
}

#[derive(Debug, Clone)]
pub struct BackfillJobUpdate {
    pub status: BackfillJobStatus,
    pub fetched_up_to: Option<DateTime<Utc>>,
    pub attempt_count: Option<i64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackfillJob {
    pub id: i64,
    pub symbol_id: i64,
    pub requested_from: DateTime<Utc>,
    pub requested_to: DateTime<Utc>,
    pub fetched_up_to: Option<DateTime<Utc>>,
    pub status: BackfillJobStatus,
    pub attempt_count: i64,
    pub last_error: Option<String>,
    pub priority: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewBackfillGap {
    pub symbol_id: i64,
    pub gap_start: DateTime<Utc>,
    pub gap_end: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct BackfillGap {
    pub id: i64,
    pub symbol_id: i64,
    pub gap_start: DateTime<Utc>,
    pub gap_end: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
    pub repaired_at: Option<DateTime<Utc>>,
    pub status: String,
}

// ──────────────────────────────────────────────────────────────
// Log structs
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct NewLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub module: String,
    pub message: String,
    pub symbol: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub module: String,
    pub message: String,
    pub symbol: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub level: Option<String>,
    pub module: Option<String>,
    pub symbol: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ──────────────────────────────────────────────────────────────
// Broker / session row structs
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrokerRow {
    pub id: i64,
    pub broker_key: String,
    pub display_name: String,
    pub api_base_url: Option<String>,
    pub ws_base_url: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct NewSessionRow {
    pub broker_id: i64,
    pub credential_ref: String,
    pub session_status: String,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: i64,
    pub broker_id: i64,
    pub credential_ref: String,
    pub session_status: String,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_validated_at: Option<DateTime<Utc>>,
}
