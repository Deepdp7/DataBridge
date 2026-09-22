/// Internal Bar (OHLCV) model — PRD §15.2
///
/// Represents a completed time-period candle. Used for both historical bars
/// fetched from the broker API and bars constructed in real time from ticks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::interval::Interval;

/// Source of a bar — distinguishes broker-supplied history from live aggregations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum BarSource {
    /// Fetched from the broker's historical REST API
    #[serde(rename = "backfill")]
    Backfill,
    /// Constructed in real time by the Tick→Bar Aggregation engine
    #[serde(rename = "live_aggregation")]
    LiveAggregation,
}

impl std::fmt::Display for BarSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BarSource::Backfill => write!(f, "backfill"),
            BarSource::LiveAggregation => write!(f, "live_aggregation"),
        }
    }
}

/// A completed OHLCV bar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub symbol: String,
    pub exchange: String,
    pub interval: Interval,
    /// Bar open time (UTC). This is the start of the interval window.
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
    pub source: BarSource,
}

impl Bar {
    /// Validate OHLC relationships per PRD §13.5.
    /// Returns `true` if the bar is internally consistent.
    pub fn is_valid_ohlc(&self) -> bool {
        self.high >= self.low
            && self.high >= self.open
            && self.high >= self.close
            && self.low <= self.open
            && self.low <= self.close
    }
}

/// Raw historical bar as returned by the broker's REST API before normalization.
/// The Data Normalizer converts `RawBar` → `Bar`.
#[derive(Debug, Clone)]
pub struct RawBar {
    /// Broker-supplied timestamp (interpretation is broker-specific; normalizer converts to UTC)
    pub broker_timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: u64,
}
