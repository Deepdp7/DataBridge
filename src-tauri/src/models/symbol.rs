/// Symbol models — PRD §9.2, §12.1
///
/// `Symbol` is the canonical internal representation.
/// `SymbolMapping` ties it to a broker-specific token.
/// `BrokerSymbol` is what a BrokerAdapter returns from `get_symbol_master()`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Canonical symbol — broker-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: i64,
    pub symbol: String,
    pub exchange: String,
    pub instrument_type: InstrumentType,
    /// Expiry date string (e.g. "2026-09-25") for derivatives; None for equities.
    pub expiry: Option<String>,
    /// Strike price for options; None for non-option instruments.
    pub strike: Option<f64>,
    /// Option type for options.
    pub option_type: Option<OptionType>,
    pub tick_size: Option<f64>,
    pub lot_size: Option<i64>,
    pub status: SymbolStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Symbol {
    /// The canonical display key used in logs and IPC (e.g. "RELIANCE:NSE")
    pub fn display_key(&self) -> String {
        format!("{}:{}", self.symbol, self.exchange)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "UPPERCASE")]
pub enum InstrumentType {
    #[serde(rename = "EQ")]
    Equity,
    #[serde(rename = "FUT")]
    Future,
    #[serde(rename = "OPT")]
    Option,
    #[serde(rename = "IDX")]
    Index,
    #[serde(rename = "ETF")]
    Etf,
    #[serde(rename = "OTHER")]
    Other,
}

impl std::fmt::Display for InstrumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstrumentType::Equity  => write!(f, "EQ"),
            InstrumentType::Future  => write!(f, "FUT"),
            InstrumentType::Option  => write!(f, "OPT"),
            InstrumentType::Index   => write!(f, "IDX"),
            InstrumentType::Etf     => write!(f, "ETF"),
            InstrumentType::Other   => write!(f, "OTHER"),
        }
    }
}

impl std::str::FromStr for InstrumentType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "EQ"     => Ok(InstrumentType::Equity),
            "FUT"    => Ok(InstrumentType::Future),
            "OPT"    => Ok(InstrumentType::Option),
            "IDX"    => Ok(InstrumentType::Index),
            "ETF"    => Ok(InstrumentType::Etf),
            "OTHER"  => Ok(InstrumentType::Other),
            other    => Err(format!("Unknown instrument type: '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum OptionType {
    #[serde(rename = "CE")]
    Call,
    #[serde(rename = "PE")]
    Put,
}

impl std::fmt::Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OptionType::Call => write!(f, "CE"),
            OptionType::Put  => write!(f, "PE"),
        }
    }
}

impl std::str::FromStr for OptionType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "CE" | "CALL" => Ok(OptionType::Call),
            "PE" | "PUT"  => Ok(OptionType::Put),
            other         => Err(format!("Unknown option type: '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "snake_case")]
pub enum SymbolStatus {
    Enabled,
    Disabled,
}

impl std::fmt::Display for SymbolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolStatus::Enabled  => write!(f, "enabled"),
            SymbolStatus::Disabled => write!(f, "disabled"),
        }
    }
}

/// Broker-specific mapping for a symbol (one row per broker per symbol).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMapping {
    pub id: i64,
    pub symbol_id: i64,
    pub broker_id: i64,
    pub broker_symbol: String,
    pub broker_token: String,
    pub sync_status: SyncStatus,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    NotStarted,
    Queued,
    Downloading,
    Processing,
    Completed,
    Partial,
    Failed,
    Retrying,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SyncStatus::NotStarted  => "not_started",
            SyncStatus::Queued      => "queued",
            SyncStatus::Downloading => "downloading",
            SyncStatus::Processing  => "processing",
            SyncStatus::Completed   => "completed",
            SyncStatus::Partial     => "partial",
            SyncStatus::Failed      => "failed",
            SyncStatus::Retrying    => "retrying",
        };
        write!(f, "{}", s)
    }
}

/// Symbol as returned by a BrokerAdapter from `get_symbol_master()`.
/// Adapter-specific; the Data Normalizer maps this into `Symbol` + `SymbolMapping`.
#[derive(Debug, Clone)]
pub struct BrokerSymbol {
    pub broker_symbol: String,
    pub broker_token: String,
    pub symbol: String,
    pub exchange: String,
    pub instrument_type: String,
    pub expiry: Option<String>,
    pub strike: Option<f64>,
    pub option_type: Option<String>,
    pub tick_size: Option<f64>,
    pub lot_size: Option<i64>,
}

/// A row parsed from a user-supplied CSV import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvSymbolRow {
    pub symbol: String,
    pub exchange: String,
    pub token: String,
    pub instrument_type: Option<String>,
    pub expiry: Option<String>,
    pub strike: Option<f64>,
    pub option_type: Option<String>,
}

/// Summary returned to the UI after a CSV import — PRD §9.2 / §20.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSummary {
    pub total_rows: usize,
    pub valid_rows: usize,
    pub duplicate_rows: usize,
    pub invalid_rows: usize,
    pub imported_rows: usize,
    pub skipped_rows: usize,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub row: usize,
    pub symbol: Option<String>,
    pub reason: String,
}
