/// IPC WebSocket handler — PRD §20.2
///
/// Streams tick updates, status changes, and error events to connected clients
/// (Tauri frontend and AmiBroker plugin).

use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tracing::{info, warn};

use super::IpcState;
use super::protocol::{ClientMessage, ServerMessage};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<IpcState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<IpcState>) {
    info!("IPC WebSocket client connected");

    // Send initial status
    let status_msg = serde_json::to_string(&ServerMessage::Status {
        websocket_connected: false,
        broker_connected: false,
    }).unwrap_or_default();
    let _ = socket.send(Message::Text(status_msg.into())).await;

    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Subscribe { symbols }) => {
                        info!(symbols = ?symbols, "IPC client subscribed");
                        // In full impl: register this WS client for tick pushes for these symbols
                    }
                    Ok(ClientMessage::Unsubscribe { symbols }) => {
                        info!(symbols = ?symbols, "IPC client unsubscribed");
                    }
                    Err(e) => warn!("Invalid IPC message: {}", e),
                }
            }
            Message::Ping(data) => {
                let _ = socket.send(Message::Pong(data)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    info!("IPC WebSocket client disconnected");
}
