/// IPC protocol message types — PRD §20.2

use serde::{Deserialize, Serialize};

/// Messages sent from clients (UI, AmiBroker plugin) to the IPC server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe   { symbols: Vec<String> },
    Unsubscribe { symbols: Vec<String> },
}

/// Messages pushed from the IPC server to clients.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Tick {
        symbol: String,
        exchange: String,
        timestamp: String,
        ltp: f64,
        last_quantity: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        volume: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        bid: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ask: Option<f64>,
    },
    Bar {
        symbol: String,
        exchange: String,
        interval: String,
        timestamp: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: u64,
    },
    Status {
        websocket_connected: bool,
        broker_connected: bool,
    },
    Error {
        code: String,
        message: String,
    },
}
