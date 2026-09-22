/// Market Session Manager — PRD §16
///
/// Controls the Live Tick Engine's connect/disconnect lifecycle based on
/// market open/close times, weekends, and the holiday calendar.
///
/// Session states: Closed → Opening → Open → Closing → Closed

use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;
use tokio::sync::watch;
use tracing::{debug, info};

/// The session state machine drives engine lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Outside market hours; Live Tick Engine is disconnected (or idle).
    Closed,
    /// Pre-market window (reserved for future use); WebSocket pre-warm.
    Opening,
    /// Market open; WebSocket connected and subscribed, ticks flowing.
    Open,
    /// Market about to close; final ticks being processed.
    Closing,
}

impl std::fmt::Display for SessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionState::Closed  => write!(f, "Closed"),
            SessionState::Opening => write!(f, "Opening"),
            SessionState::Open    => write!(f, "Open"),
            SessionState::Closing => write!(f, "Closing"),
        }
    }
}

/// Per-exchange session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    /// Exchange code (e.g. "NSE", "BSE")
    pub exchange: String,
    /// IANA timezone (e.g. "Asia/Kolkata")
    pub timezone: String,
    /// Market open time in exchange local time (HH:MM)
    pub market_open: String,
    /// Market close time in exchange local time (HH:MM)
    pub market_close: String,
    /// ISO 8601 date strings of exchange holidays (YYYY-MM-DD)
    pub holidays: HashSet<String>,
}

impl Default for ExchangeConfig {
    fn default() -> Self {
        ExchangeConfig {
            exchange: "NSE".into(),
            timezone: "Asia/Kolkata".into(),
            market_open: "09:15".into(),
            market_close: "15:30".into(),
            holidays: HashSet::new(),
        }
    }
}

pub struct MarketSessionManager {
    config: ExchangeConfig,
    state_tx: watch::Sender<SessionState>,
    /// Exposed for subscribers (Live Tick Engine, Dashboard, etc.)
    pub state_rx: watch::Receiver<SessionState>,
}

impl MarketSessionManager {
    pub fn new(config: ExchangeConfig) -> Self {
        let (state_tx, state_rx) = watch::channel(SessionState::Closed);
        MarketSessionManager { config, state_tx, state_rx }
    }

    /// Get the current session state.
    pub fn current_state(&self) -> SessionState {
        *self.state_rx.borrow()
    }

    /// Check if the market is currently open at the given UTC time.
    pub fn is_open(&self, now_utc: DateTime<Utc>) -> bool {
        match self.classify_time(now_utc) {
            SessionState::Open | SessionState::Closing => true,
            _ => false,
        }
    }

    /// Classify a UTC timestamp into a session state.
    pub fn classify_time(&self, now_utc: DateTime<Utc>) -> SessionState {
        let tz = match Tz::from_str(&self.config.timezone) {
            Ok(tz) => tz,
            Err(_) => {
                tracing::warn!(tz = %self.config.timezone, "Unknown timezone, defaulting to UTC");
                chrono_tz::UTC
            }
        };

        let local = now_utc.with_timezone(&tz);

        // Weekend check
        match local.weekday() {
            Weekday::Sat | Weekday::Sun => return SessionState::Closed,
            _ => {}
        }

        // Holiday check
        let date_str = local.format("%Y-%m-%d").to_string();
        if self.config.holidays.contains(&date_str) {
            debug!(date = %date_str, "Market holiday — session closed");
            return SessionState::Closed;
        }

        // Parse open/close times
        let open  = NaiveTime::parse_from_str(&self.config.market_open,  "%H:%M").unwrap_or_else(|_| NaiveTime::from_hms_opt(9, 15, 0).unwrap());
        let close = NaiveTime::parse_from_str(&self.config.market_close, "%H:%M").unwrap_or_else(|_| NaiveTime::from_hms_opt(15, 30, 0).unwrap());
        let now_time = local.time();

        // Pre-open: 5 minutes before open
        let pre_open = open - chrono::Duration::minutes(5);
        // Closing: last 1 minute of session
        let closing_start = close - chrono::Duration::minutes(1);

        if now_time < pre_open || now_time >= close {
            SessionState::Closed
        } else if now_time < open {
            SessionState::Opening
        } else if now_time >= closing_start {
            SessionState::Closing
        } else {
            SessionState::Open
        }
    }

    /// Run the session monitor in a background Tokio task.
    /// Polls every 30 seconds and sends state transitions via the watch channel.
    pub async fn run(self: std::sync::Arc<Self>) {
        let mut last_state = SessionState::Closed;
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(30));

        info!(exchange = %self.config.exchange, "Market session monitor started");

        loop {
            ticker.tick().await;
            let state = self.classify_time(Utc::now());
            if state != last_state {
                info!(
                    exchange = %self.config.exchange,
                    from = %last_state,
                    to = %state,
                    "Market session state changed"
                );
                let _ = self.state_tx.send(state);
                last_state = state;
            }
        }
    }

    /// Add a holiday date (YYYY-MM-DD format).
    pub fn add_holiday(&mut self, date: &str) {
        self.config.holidays.insert(date.to_string());
    }

    pub fn config(&self) -> &ExchangeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekday_nse_open_time_is_open() {
        let mgr = MarketSessionManager::new(ExchangeConfig::default());
        // 2026-09-22 is a Tuesday — 09:30 IST = 04:00 UTC
        let ist_open = chrono::TimeZone::with_ymd_and_hms(&chrono_tz::Asia::Kolkata, 2026, 9, 22, 9, 30, 0).unwrap();
        let utc = ist_open.with_timezone(&Utc);
        assert_eq!(mgr.classify_time(utc), SessionState::Open);
    }

    #[test]
    fn weekend_is_closed() {
        let mgr = MarketSessionManager::new(ExchangeConfig::default());
        // 2026-09-19 is a Saturday
        let saturday = chrono::TimeZone::with_ymd_and_hms(&chrono_tz::Asia::Kolkata, 2026, 9, 19, 10, 0, 0).unwrap();
        let utc = saturday.with_timezone(&Utc);
        assert_eq!(mgr.classify_time(utc), SessionState::Closed);
    }

    #[test]
    fn holiday_is_closed() {
        let mut config = ExchangeConfig::default();
        config.holidays.insert("2026-09-22".into());
        let mgr = MarketSessionManager::new(config);
        let open_time = chrono::TimeZone::with_ymd_and_hms(&chrono_tz::Asia::Kolkata, 2026, 9, 22, 10, 0, 0).unwrap();
        let utc = open_time.with_timezone(&Utc);
        assert_eq!(mgr.classify_time(utc), SessionState::Closed);
    }
}
