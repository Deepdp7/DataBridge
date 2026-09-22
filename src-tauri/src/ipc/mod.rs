/// Local IPC Server — PRD §20
///
/// Hosts a loopback-only (127.0.0.1) HTTP + WebSocket server using axum.
/// Serves both the Tauri UI (via Tauri commands, not this server directly)
/// and the AmiBroker plugin (via TCP/WebSocket).
///
/// All endpoints are bound to 127.0.0.1 only — never exposed externally.

pub mod protocol;
pub mod rest;
pub mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

use crate::error::Result;
use crate::storage::AppStorage;

/// Default IPC port (configurable in Settings → AmiBroker → IPC)
pub const DEFAULT_IPC_PORT: u16 = 7421;

/// The IPC server state shared across all handlers.
#[derive(Clone)]
pub struct IpcState {
    pub storage: AppStorage,
    pub ipc_port: u16,
}

/// Start the local IPC server.
/// Returns when the server shuts down (never under normal operation).
pub async fn start_ipc_server(state: IpcState, port: u16) -> Result<()> {
    let addr: SocketAddr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| crate::error::AppError::Ipc(format!("Invalid address: {}", e)))?;

    let app = build_router(state);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| crate::error::AppError::Ipc(format!("Bind failed: {}", e)))?;

    info!(addr = %addr, "Local IPC server listening (loopback-only)");

    axum::serve(listener, app)
        .await
        .map_err(|e| crate::error::AppError::Ipc(format!("IPC server error: {}", e)))?;

    Ok(())
}

fn build_router(state: IpcState) -> Router {
    use axum::extract::State;
    use axum::response::Json;

    Router::new()
        // Health check
        .route("/health", get(|| async { Json(serde_json::json!({"status": "ok"})) }))
        // REST endpoints (PRD §20.1)
        .route("/status",              get(rest::get_status))
        .route("/symbols",             get(rest::get_symbols))
        .route("/backfill/status",     get(rest::get_backfill_status))
        .route("/live/status",         get(rest::get_live_status))
        .route("/logs",                get(rest::get_logs))
        .route("/history/:symbol",     get(rest::get_history))
        // WebSocket streaming (PRD §20.2)
        .route("/ws",                  get(ws::ws_handler))
        .with_state(Arc::new(state))
}
