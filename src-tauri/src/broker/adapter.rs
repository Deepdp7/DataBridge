/// BrokerAdapter trait — PRD §12.1
///
/// The ONLY abstraction layer between the DataBridge core engine and any
/// broker's REST + WebSocket APIs. Every broker-specific implementation
/// must live entirely inside `broker/<broker_name>/` and implement this trait.
///
/// No module outside `broker/<name>/` may reference broker-specific request
/// shapes, field names, rate-limit rules, or authentication details.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::models::{BrokerCredentials, BrokerSymbol, RawBar, Session};
use crate::models::interval::Interval;

use super::error::BrokerError;
use super::types::{RateLimitConfig, WsHandle};

#[async_trait]
pub trait BrokerAdapter: Send + Sync {
    /// Unique string key identifying this broker (e.g. "broker_v1", "broker_zerodha").
    /// Must match `brokers.broker_key` in the database.
    fn broker_id(&self) -> &str;

    /// Human-readable display name (e.g. "Broker V1", "Zerodha Kite").
    fn display_name(&self) -> &str;

    // ── Authentication ────────────────────────────────────────────────────────

    /// Authenticate with the broker using the provided credentials.
    /// On success, persists the access token via the OS credential store
    /// (using `credential_ref` as the key) and returns a `Session`.
    /// The raw access token MUST NOT be included in the `Session` struct —
    /// only the `credential_ref` key used to retrieve it later.
    async fn authenticate(&self, creds: &BrokerCredentials) -> Result<Session, BrokerError>;

    /// For OAuth 2.0 brokers, generate the authorization URL to open in the user's browser.
    /// Returns `None` if the broker does not use browser-based OAuth.
    fn get_auth_url(&self, creds: &BrokerCredentials, state: &str) -> Option<String> {
        None
    }

    /// For OAuth 2.0 brokers, exchange the received authorization code for a session/access token.
    async fn exchange_auth_code(
        &self,
        creds: &BrokerCredentials,
        code: &str,
        state: &str,
    ) -> Result<Session, BrokerError> {
        Err(BrokerError::AuthenticationFailed("Broker does not support OAuth".to_string()))
    }

    /// Attempt to refresh an existing session (e.g. using a refresh token).
    /// Returns an updated `Session` with a new `credential_ref` on success.
    /// Returns `BrokerError::SessionExpired` if refresh is not possible.
    async fn refresh_session(&self, session: &Session) -> Result<Session, BrokerError>;

    /// Validate that a session is still active.
    /// Returns the updated session (with refreshed `last_validated_at`) on success.
    async fn validate_session(&self, session: &Session) -> Result<Session, BrokerError>;

    // ── Symbol master ─────────────────────────────────────────────────────────

    /// Fetch the broker's instrument/symbol master.
    /// Malformed or unsupported rows must be logged and skipped —
    /// a partial failure must NOT fail the entire call.
    async fn get_symbol_master(&self) -> Result<Vec<BrokerSymbol>, BrokerError>;

    // ── Historical data ───────────────────────────────────────────────────────

    /// Fetch historical OHLCV data for a given broker token.
    ///
    /// - `token`: broker-specific instrument token
    /// - `interval`: requested candle interval
    /// - `from`: inclusive start time (UTC)
    /// - `to`: inclusive end time (UTC)
    ///
    /// The adapter is responsible for respecting the broker's maximum
    /// date-range limit per request (chunking is handled by the Historical Engine,
    /// but adapters may further sub-chunk if needed).
    ///
    /// Returns `RawBar` slices with broker-supplied timestamps.
    /// The Data Normalizer converts these to UTC `Bar` objects.
    async fn get_historical_data(
        &self,
        token: &str,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<RawBar>, BrokerError>;

    /// Returns the maximum date-range (in days) the broker allows per historical API call.
    /// Used by the Historical Engine to chunk backfill requests.
    fn max_historical_days_per_request(&self) -> u32 {
        90 // safe default; override per broker
    }

    // ── WebSocket / live data ─────────────────────────────────────────────────

    /// Establish the WebSocket connection to the broker's live feed.
    /// Returns a `WsHandle` whose `tick_tx` channel the Live Tick Engine
    /// reads from to receive normalized `RawTick` values.
    ///
    /// The adapter owns the connection lifecycle task; the `WsHandle`
    /// provides the control interface.
    async fn connect_websocket(&self, session: &Session) -> Result<WsHandle, BrokerError>;

    /// Subscribe to live tick updates for the given broker tokens.
    /// Handles batching internally (respecting `rate_limit_config.max_symbols_per_ws_message`).
    async fn subscribe_symbols(
        &self,
        ws: &WsHandle,
        tokens: &[String],
    ) -> Result<(), BrokerError>;

    /// Unsubscribe from live tick updates for the given broker tokens.
    async fn unsubscribe_symbols(
        &self,
        ws: &WsHandle,
        tokens: &[String],
    ) -> Result<(), BrokerError>;

    /// Gracefully disconnect the WebSocket.
    async fn disconnect(&self, ws: &WsHandle) -> Result<(), BrokerError>;

    // ── Configuration ─────────────────────────────────────────────────────────

    /// Rate limit configuration for this broker.
    /// The Historical Engine uses this to configure its token-bucket limiter.
    fn rate_limit_config(&self) -> RateLimitConfig;

    /// The list of intervals this broker's historical API supports.
    /// The engine will only request intervals in this list.
    fn supported_intervals(&self) -> Vec<Interval>;
}
