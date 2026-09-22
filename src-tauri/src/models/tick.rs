/// Internal Tick model — PRD §15.1
///
/// All broker-specific tick shapes are normalized into this struct by the
/// Data Normalizer before any downstream processing. Fields not supported
/// by a given broker are `None`, never a sentinel value like `0`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    /// Canonical symbol name (e.g. "RELIANCE")
    pub symbol: String,
    /// Exchange name (e.g. "NSE", "BSE", "NFO")
    pub exchange: String,
    /// Broker-specific instrument token / identifier
    pub broker_token: String,
    /// UTC timestamp, normalized from broker-supplied time
    pub timestamp: DateTime<Utc>,

    /// Last traded price
    pub ltp: f64,
    /// Quantity traded in this tick
    pub last_quantity: u64,

    // ── Optional fields — None when broker does not provide them ──
    /// Cumulative traded volume for the day
    pub volume: Option<u64>,
    /// Best bid price
    pub bid: Option<f64>,
    /// Best bid quantity
    pub bid_quantity: Option<u64>,
    /// Best ask price
    pub ask: Option<f64>,
    /// Best ask quantity
    pub ask_quantity: Option<u64>,
    /// Day open price
    pub open: Option<f64>,
    /// Day high price
    pub high: Option<f64>,
    /// Day low price
    pub low: Option<f64>,
    /// Previous day close price
    pub previous_close: Option<f64>,
}

impl Tick {
    /// Deduplication fingerprint. Ticks sharing the same fingerprint within
    /// a short time window are considered duplicates (PRD §14.3).
    pub fn fingerprint(&self) -> TickFingerprint {
        TickFingerprint {
            symbol: self.symbol.clone(),
            exchange: self.exchange.clone(),
            timestamp_ms: self.timestamp.timestamp_millis(),
            ltp_micros: (self.ltp * 1_000_000.0) as i64,
            last_quantity: self.last_quantity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TickFingerprint {
    pub symbol: String,
    pub exchange: String,
    pub timestamp_ms: i64,
    pub ltp_micros: i64,
    pub last_quantity: u64,
}

/// Raw tick as received from a broker adapter before normalization.
/// This is what `BrokerAdapter::connect_websocket` + the tick stream delivers.
/// The Data Normalizer converts `RawTick` → `Tick`.
#[derive(Debug, Clone)]
pub struct RawTick {
    /// Broker's own token/identifier
    pub broker_token: String,
    /// Timestamp as provided by the broker (may be local, may be UTC)
    pub broker_timestamp_ms: i64,
    /// Last traded price
    pub ltp: f64,
    /// Quantity in this tick
    pub last_quantity: u64,
    /// Optional raw fields from broker payload
    pub volume: Option<u64>,
    pub bid: Option<f64>,
    pub bid_quantity: Option<u64>,
    pub ask: Option<f64>,
    pub ask_quantity: Option<u64>,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub previous_close: Option<f64>,
}
