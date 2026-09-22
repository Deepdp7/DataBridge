/// MockBrokerAdapter — synthetic data broker for development and testing.
///
/// This adapter satisfies the `BrokerAdapter` trait using in-memory synthetic data.
/// It is the active adapter when no real broker has been configured.
/// Replace with a real adapter by adding `broker/<broker_name>/` and registering it.
///
/// Behavior:
/// - `authenticate()`: always succeeds instantly
/// - `get_symbol_master()`: returns a fixed set of well-known NSE symbols
/// - `get_historical_data()`: generates synthetic 1-minute OHLCV bars
/// - `connect_websocket()`: starts a task that emits synthetic ticks at ~10/sec
/// - `subscribe/unsubscribe`: tracked in memory, no-op otherwise
/// - `disconnect()`: stops the tick-emitting task

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, TimeZone, Utc, Weekday};
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, info};

use crate::broker::adapter::BrokerAdapter;
use crate::broker::error::BrokerError;
use crate::broker::types::{RateLimitConfig, WsHandle};
use crate::models::interval::Interval;
use crate::models::session::{BrokerCredentials, Session, SessionStatus};
use crate::models::symbol::BrokerSymbol;
use crate::models::tick::RawTick;
use crate::models::bar::RawBar;

/// Inner state for the mock WebSocket connection
struct MockWsInner {
    subscribed_tokens: Mutex<HashSet<String>>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
}

pub struct MockBrokerAdapter;

impl MockBrokerAdapter {
    pub fn new() -> Self {
        MockBrokerAdapter
    }

    /// Build a pseudo-random price walk from a seed, used for synthetic OHLCV generation.
    fn synthetic_price(seed: f64, step: u64) -> f64 {
        let noise = ((step as f64 * 2.654_435_761_859_956 + seed).sin() * 10_000.0)
            .abs()
            .rem_euclid(100.0);
        seed + noise - 50.0
    }
}

impl Default for MockBrokerAdapter {
    fn default() -> Self { Self::new() }
}

/// Symbols returned by the mock symbol master
fn mock_symbols() -> Vec<BrokerSymbol> {
    let raw = [
        ("RELIANCE", "NSE", "EQ", "2885"),
        ("TCS",      "NSE", "EQ", "2953"),
        ("INFY",     "NSE", "EQ", "1594"),
        ("HDFCBANK", "NSE", "EQ", "1333"),
        ("ICICIBANK","NSE", "EQ", "4963"),
        ("SBIN",     "NSE", "EQ", "3045"),
        ("WIPRO",    "NSE", "EQ", "3787"),
        ("AXISBANK", "NSE", "EQ", "5900"),
        ("KOTAKBANK","NSE", "EQ", "1922"),
        ("LT",       "NSE", "EQ", "11483"),
        ("NIFTY",    "NSE", "IDX", "256265"),
        ("BANKNIFTY","NSE", "IDX", "260105"),
    ];
    raw.iter().map(|(sym, exch, itype, token)| BrokerSymbol {
        broker_symbol: sym.to_string(),
        broker_token: token.to_string(),
        symbol: sym.to_string(),
        exchange: exch.to_string(),
        instrument_type: itype.to_string(),
        expiry: None,
        strike: None,
        option_type: None,
        tick_size: Some(0.05),
        lot_size: Some(1),
    }).collect()
}

#[async_trait]
impl BrokerAdapter for MockBrokerAdapter {
    fn broker_id(&self) -> &str { "mock" }
    fn display_name(&self) -> &str { "Mock Broker (Development)" }

    async fn authenticate(&self, creds: &BrokerCredentials) -> Result<Session, BrokerError> {
        info!(broker = "mock", "Mock authentication — always succeeds");
        Ok(Session {
            broker_id: creds.broker_id.clone(),
            credential_ref: format!("mock_session_{}", uuid::Uuid::new_v4()),
            status: SessionStatus::Active,
            token_expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
            last_validated_at: Some(Utc::now()),
        })
    }

    async fn refresh_session(&self, session: &Session) -> Result<Session, BrokerError> {
        Ok(Session {
            token_expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
            last_validated_at: Some(Utc::now()),
            ..session.clone()
        })
    }

    async fn validate_session(&self, session: &Session) -> Result<Session, BrokerError> {
        Ok(Session {
            last_validated_at: Some(Utc::now()),
            ..session.clone()
        })
    }

    async fn get_symbol_master(&self) -> Result<Vec<BrokerSymbol>, BrokerError> {
        info!(broker = "mock", "Returning mock symbol master ({} symbols)", mock_symbols().len());
        Ok(mock_symbols())
    }

    async fn get_historical_data(
        &self,
        token: &str,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<RawBar>, BrokerError> {
        let interval_secs = interval.duration_secs().unwrap_or(60) as i64;
        let seed = token.bytes().fold(1500.0_f64, |acc, b| acc + b as f64);

        let mut bars = Vec::new();
        let mut t = from.timestamp();
        let end = to.timestamp();
        let mut step: u64 = 0;

        while t < end {
            // Skip weekends (mock market hours: Mon–Fri)
            let dt = Utc.timestamp_opt(t, 0).unwrap();
            let weekday = dt.weekday();
            if weekday != Weekday::Sat && weekday != Weekday::Sun {
                let open = Self::synthetic_price(seed, step).abs() + 100.0;
                let close = Self::synthetic_price(seed, step + 1).abs() + 100.0;
                let high = open.max(close) + (step % 5) as f64 * 0.1;
                let low  = open.min(close) - (step % 3) as f64 * 0.1;
                let vol  = 1000 + (step % 500) * 10;

                bars.push(RawBar {
                    broker_timestamp_ms: t * 1000,
                    open,
                    high,
                    low,
                    close,
                    volume: vol,
                });
            }
            t += interval_secs;
            step += 1;
        }

        debug!(
            broker = "mock",
            token,
            from = %from,
            to = %to,
            bars = bars.len(),
            "Generated synthetic historical bars"
        );
        // Simulate a small network delay
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok(bars)
    }

    fn max_historical_days_per_request(&self) -> u32 { 90 }

    async fn connect_websocket(&self, session: &Session) -> Result<WsHandle, BrokerError> {
        info!(broker = "mock", session_id = %session.credential_ref, "Mock WebSocket connecting");

        let (tick_tx, _tick_rx) = mpsc::channel::<RawTick>(1024);
        let inner_state = Arc::new(MockWsInner {
            subscribed_tokens: Mutex::new(HashSet::new()),
            task_handle: Mutex::new(None),
        });

        // Clone handles for the spawned task
        let tick_tx_clone = tick_tx.clone();
        let state_clone = Arc::clone(&inner_state);

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            let mut step: u64 = 0;
            loop {
                interval.tick().await;
                let tokens: Vec<String> = {
                    state_clone.subscribed_tokens.lock().iter().cloned().collect()
                };
                for token in &tokens {
                    let seed = token.bytes().fold(1500.0_f64, |acc, b| acc + b as f64);
                    let ltp = (MockBrokerAdapter::synthetic_price(seed, step).abs() + 100.0 * ((step as f64 * 0.001).sin() + 1.0)).max(1.0);
                    let tick = RawTick {
                        broker_token: token.clone(),
                        broker_timestamp_ms: Utc::now().timestamp_millis(),
                        ltp,
                        last_quantity: 10 + (step % 50),
                        volume: Some(100_000 + step * 10),
                        bid: Some(ltp - 0.05),
                        bid_quantity: Some(50 + step % 100),
                        ask: Some(ltp + 0.05),
                        ask_quantity: Some(50 + step % 100),
                        open: None,
                        high: None,
                        low: None,
                        previous_close: None,
                    };
                    if tick_tx_clone.send(tick).await.is_err() {
                        break; // Receiver dropped — exit task
                    }
                }
                step += 1;
            }
        });

        *inner_state.task_handle.lock() = Some(handle);

        Ok(WsHandle {
            tick_tx,
            inner: Box::new(inner_state),
        })
    }

    async fn subscribe_symbols(
        &self,
        ws: &WsHandle,
        tokens: &[String],
    ) -> Result<(), BrokerError> {
        if let Some(inner) = ws.inner.downcast_ref::<Arc<MockWsInner>>() {
            let mut subscribed = inner.subscribed_tokens.lock();
            for token in tokens {
                subscribed.insert(token.clone());
            }
            info!(broker = "mock", count = tokens.len(), "Subscribed to mock tokens");
        }
        Ok(())
    }

    async fn unsubscribe_symbols(
        &self,
        ws: &WsHandle,
        tokens: &[String],
    ) -> Result<(), BrokerError> {
        if let Some(inner) = ws.inner.downcast_ref::<Arc<MockWsInner>>() {
            let mut subscribed = inner.subscribed_tokens.lock();
            for token in tokens {
                subscribed.remove(token);
            }
            info!(broker = "mock", count = tokens.len(), "Unsubscribed from mock tokens");
        }
        Ok(())
    }

    async fn disconnect(&self, ws: &WsHandle) -> Result<(), BrokerError> {
        if let Some(inner) = ws.inner.downcast_ref::<Arc<MockWsInner>>() {
            if let Some(handle) = inner.task_handle.lock().take() {
                handle.abort();
            }
            info!(broker = "mock", "Mock WebSocket disconnected");
        }
        Ok(())
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            historical_requests_per_second: 10.0, // generous for mock
            general_requests_per_second: 20.0,
            max_symbols_per_ws_message: 200,
            max_ws_subscriptions: 2000,
            max_ws_connections: 1,
            requires_client_heartbeat: false,
            heartbeat_interval_secs: 0,
        }
    }

    fn supported_intervals(&self) -> Vec<Interval> {
        vec![
            Interval::OneMinute,
            Interval::FiveMinutes,
            Interval::FifteenMinutes,
            Interval::ThirtyMinutes,
            Interval::OneHour,
            Interval::Daily,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_authenticate_succeeds() {
        let adapter = MockBrokerAdapter::new();
        let creds = BrokerCredentials {
            broker_id: "mock".into(),
            api_key: None,
            api_secret: None,
            redirect_uri: None,
            totp_secret: None,
            extra: Default::default(),
        };
        let session = adapter.authenticate(&creds).await.unwrap();
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.token_expires_at.is_some());
    }

    #[tokio::test]
    async fn mock_symbol_master_not_empty() {
        let adapter = MockBrokerAdapter::new();
        let creds = BrokerCredentials {
            broker_id: "mock".into(),
            api_key: None,
            api_secret: None,
            redirect_uri: None,
            totp_secret: None,
            extra: Default::default(),
        };
        let session = adapter.authenticate(&creds).await.unwrap();
        let _ = session; // satisfy unused warning
        let symbols = adapter.get_symbol_master().await.unwrap();
        assert!(!symbols.is_empty());
    }

    #[tokio::test]
    async fn mock_historical_data_generates_bars() {
        let adapter = MockBrokerAdapter::new();
        let from = Utc.with_ymd_and_hms(2026, 9, 1, 3, 45, 0).unwrap();
        let to   = Utc.with_ymd_and_hms(2026, 9, 5, 10, 0, 0).unwrap();
        let bars = adapter.get_historical_data("2885", Interval::OneMinute, from, to).await.unwrap();
        assert!(!bars.is_empty(), "Expected synthetic bars to be generated");
    }
}
