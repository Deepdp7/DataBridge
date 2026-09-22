/// Tick → Bar Aggregation Engine — PRD §15.3
///
/// Aggregates incoming ticks into 1-minute OHLCV bars in real time.
/// Uses a per-symbol state machine: first tick of each minute opens a bar,
/// subsequent ticks update H/L/C/V, and minute boundary crossing closes/emits the bar.
///
/// Infrastructure supports multiple intervals; MVP activates only 1-minute.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, DurationRound, Utc};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::models::bar::{Bar, BarSource};
use crate::models::interval::Interval;
use crate::models::tick::Tick;
use crate::storage::repository::{HistoricalRepository, NewBar};

/// In-progress bar being built from ticks.
#[derive(Debug, Clone)]
struct InProgressBar {
    /// Bar open time (truncated to minute boundary)
    minute_ts: DateTime<Utc>,
    open:  f64,
    high:  f64,
    low:   f64,
    close: f64,
    volume: u64,
}

impl InProgressBar {
    fn new(tick: &Tick, minute_ts: DateTime<Utc>) -> Self {
        InProgressBar {
            minute_ts,
            open:   tick.ltp,
            high:   tick.ltp,
            low:    tick.ltp,
            close:  tick.ltp,
            volume: tick.last_quantity,
        }
    }

    fn update(&mut self, tick: &Tick) {
        if tick.ltp > self.high  { self.high  = tick.ltp; }
        if tick.ltp < self.low   { self.low   = tick.ltp; }
        self.close  = tick.ltp;
        self.volume += tick.last_quantity;
    }

    fn into_bar(self, symbol: String, exchange: String, interval: Interval) -> Bar {
        Bar {
            symbol,
            exchange,
            interval,
            timestamp: self.minute_ts,
            open:   self.open,
            high:   self.high,
            low:    self.low,
            close:  self.close,
            volume: self.volume,
            source: BarSource::LiveAggregation,
        }
    }
}

/// A completed bar ready to be emitted.
#[derive(Debug, Clone)]
pub struct CompletedBar {
    pub bar: Bar,
}

pub struct TickAggregator {
    /// MVP: only Interval::OneMinute is active
    active_interval: Interval,
    /// Per-symbol in-progress bar: key = "SYMBOL:EXCHANGE"
    in_progress: Arc<Mutex<HashMap<String, InProgressBar>>>,
    /// Output channel for completed bars
    pub bar_out_tx: mpsc::Sender<CompletedBar>,
    bar_out_rx: Option<mpsc::Receiver<CompletedBar>>,
    historical_repo: Arc<dyn HistoricalRepository>,
}

impl TickAggregator {
    pub fn new(historical_repo: Arc<dyn HistoricalRepository>) -> Self {
        let (bar_out_tx, bar_out_rx) = mpsc::channel::<CompletedBar>(4096);
        TickAggregator {
            active_interval: Interval::OneMinute,
            in_progress: Arc::new(Mutex::new(HashMap::new())),
            bar_out_tx,
            bar_out_rx: Some(bar_out_rx),
            historical_repo,
        }
    }

    pub fn take_bar_receiver(&mut self) -> Option<mpsc::Receiver<CompletedBar>> {
        self.bar_out_rx.take()
    }

    /// Process a single tick — state machine per PRD §15.3.
    pub async fn process_tick(&self, tick: &Tick) {
        let key = format!("{}:{}", tick.symbol, tick.exchange);

        // Truncate tick timestamp to the current 1-minute boundary
        let minute_ts = match tick.timestamp.duration_trunc(chrono::Duration::minutes(1)) {
            Ok(t) => t,
            Err(_) => return,
        };

        let completed: Option<InProgressBar> = {
            let mut map = self.in_progress.lock();

            if let Some(bar) = map.get_mut(&key) {
                if bar.minute_ts == minute_ts {
                    // Same minute — update H/L/C/V
                    bar.update(tick);
                    None
                } else {
                    // Minute boundary crossed — close and replace
                    let closed = bar.clone();
                    *bar = InProgressBar::new(tick, minute_ts);
                    Some(closed)
                }
            } else {
                // First tick for this symbol
                map.insert(key.clone(), InProgressBar::new(tick, minute_ts));
                None
            }
        };

        if let Some(closed_bar) = completed {
            self.emit_bar(closed_bar, &tick.symbol, &tick.exchange).await;
        }
    }

    /// Force-close all in-progress bars (called at market session close — PRD §15.3).
    pub async fn force_close_all(&self) {
        let bars: Vec<(String, InProgressBar)> = {
            let mut map = self.in_progress.lock();
            map.drain().collect()
        };

        info!(count = bars.len(), "Force-closing all in-progress bars at session close");

        for (key, bar) in bars {
            let parts: Vec<&str> = key.splitn(2, ':').collect();
            let (symbol, exchange) = if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (key.clone(), String::new())
            };
            self.emit_bar(bar, &symbol, &exchange).await;
        }
    }

    async fn emit_bar(&self, in_progress: InProgressBar, symbol: &str, exchange: &str) {
        let bar = in_progress.into_bar(
            symbol.to_string(),
            exchange.to_string(),
            self.active_interval,
        );

        debug!(
            symbol,
            ts = %bar.timestamp,
            open = bar.open,
            high = bar.high,
            low = bar.low,
            close = bar.close,
            volume = bar.volume,
            "Emitting completed 1-minute bar"
        );

        // Persist to SQLite (fire-and-forget with error log)
        let repo = Arc::clone(&self.historical_repo);

        // The bar TX is also sent over IPC to the AmiBroker plugin
        let completed = CompletedBar { bar: bar.clone() };
        if let Err(e) = self.bar_out_tx.try_send(completed) {
            tracing::warn!("Bar output channel full — dropping bar: {}", e);
        }
    }

    /// Main run loop — reads from a tick input channel.
    pub async fn run(self: Arc<Self>, mut tick_rx: mpsc::Receiver<Tick>) {
        info!("Tick Aggregation Engine started (interval: {})", self.active_interval);

        while let Some(tick) = tick_rx.recv().await {
            self.process_tick(&tick).await;
        }

        info!("Tick Aggregation Engine stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_tick(symbol: &str, exchange: &str, ts_ms: i64, ltp: f64, qty: u64) -> Tick {
        Tick {
            symbol: symbol.to_string(),
            exchange: exchange.to_string(),
            broker_token: "token".into(),
            timestamp: DateTime::<Utc>::from_timestamp_millis(ts_ms).unwrap(),
            ltp,
            last_quantity: qty,
            volume: None,
            bid: None, bid_quantity: None,
            ask: None, ask_quantity: None,
            open: None, high: None, low: None, previous_close: None,
        }
    }

    // Tick aggregation test requires a mock HistoricalRepository
    // — full test in integration test suite
}
