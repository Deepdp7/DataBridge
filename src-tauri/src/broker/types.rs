/// Broker adapter supporting types — PRD §12.1

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::models::tick::RawTick;

/// Rate limit configuration exposed by each BrokerAdapter implementation.
/// The Historical Engine uses these values to configure its token-bucket limiter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum REST API requests per second for historical data endpoints
    pub historical_requests_per_second: f64,
    /// Maximum REST API requests per second for other endpoints
    pub general_requests_per_second: f64,
    /// Maximum number of symbols that can be subscribed in a single WebSocket message
    pub max_symbols_per_ws_message: usize,
    /// Maximum total symbols subscribable on a single WebSocket connection
    pub max_ws_subscriptions: usize,
    /// Maximum concurrent WebSocket connections (usually 1 for most brokers)
    pub max_ws_connections: usize,
    /// Whether the broker requires heartbeat/ping messages from the client
    pub requires_client_heartbeat: bool,
    /// Heartbeat interval in seconds (0 = broker manages heartbeat)
    pub heartbeat_interval_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            historical_requests_per_second: 3.0,
            general_requests_per_second: 10.0,
            max_symbols_per_ws_message: 100,
            max_ws_subscriptions: 500,
            max_ws_connections: 1,
            requires_client_heartbeat: true,
            heartbeat_interval_secs: 30,
        }
    }
}

/// Opaque handle to an active WebSocket connection managed by a BrokerAdapter.
/// The adapter creates this; callers only pass it back for subscribe/unsubscribe/disconnect.
pub struct WsHandle {
    /// Channel sender — the adapter sends `RawTick` values here for the Live Tick Engine to consume.
    pub tick_tx: mpsc::Sender<RawTick>,
    /// Internal handle used by the adapter to send control messages to the WebSocket task.
    /// Type-erased so the broker adapter can store whatever it needs.
    pub inner: Box<dyn std::any::Any + Send + Sync>,
}

impl std::fmt::Debug for WsHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WsHandle").finish_non_exhaustive()
    }
}

/// Connection state of the broker's WebSocket
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WsConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

/// Overall connection status — surfaced on the dashboard and system tray.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub broker_rest: ConnectionState,
    pub broker_websocket: WsConnectionState,
    pub amibroker_plugin: ConnectionState,
    pub session_status: crate::models::SessionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
    Reconnecting,
    Error,
    Unknown,
}
