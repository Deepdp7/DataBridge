/// DataBridge — Internal error types
///
/// Every subsystem maps its failures into one of these variants.
/// BrokerError is kept separate (in broker::error) so broker-specific
/// error cases never leak into the core engine.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    // ── Storage ──────────────────────────────────────────────
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    // ── IO ────────────────────────────────────────────────────
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // ── Serialization ─────────────────────────────────────────
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // ── CSV ───────────────────────────────────────────────────
    #[error("CSV parse error: {0}")]
    Csv(#[from] csv::Error),

    // ── HTTP / API ────────────────────────────────────────────
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    // ── WebSocket ─────────────────────────────────────────────
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    // ── Credential storage ────────────────────────────────────
    #[error("Credential storage error: {0}")]
    CredentialStore(String),

    // ── Configuration ─────────────────────────────────────────
    #[error("Configuration error: {0}")]
    Config(String),

    // ── Symbol management ─────────────────────────────────────
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Duplicate symbol: {0}")]
    DuplicateSymbol(String),

    // ── Backfill ──────────────────────────────────────────────
    #[error("Backfill error: {0}")]
    Backfill(String),

    // ── Session / auth ────────────────────────────────────────
    #[error("Session expired")]
    SessionExpired,

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    // ── Broker (generic wrapper — broker-specific errors stay in broker::error) ──
    #[error("Broker error: {0}")]
    Broker(String),

    // ── IPC ───────────────────────────────────────────────────
    #[error("IPC error: {0}")]
    Ipc(String),

    // ── Catch-all ─────────────────────────────────────────────
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn other(msg: impl Into<String>) -> Self {
        AppError::Other(msg.into())
    }
}

/// Convenience alias used throughout the codebase
pub type Result<T> = std::result::Result<T, AppError>;

/// Convert AppError into a Tauri-serializable string for frontend error reporting.
/// Secrets are never included — the error message is the already-sanitized display string.
impl From<AppError> for String {
    fn from(e: AppError) -> Self {
        e.to_string()
    }
}
