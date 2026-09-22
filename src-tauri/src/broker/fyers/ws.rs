use std::collections::HashSet;
use std::sync::Arc;
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, debug};

use crate::broker::{BrokerError, WsHandle};
use crate::models::session::Session;
use crate::models::tick::RawTick;

struct FyersWsInner {
    write_tx: mpsc::Sender<Message>,
}

pub struct FyersWs {}

impl FyersWs {
    pub async fn connect(session: &Session, auth_token: &str) -> Result<WsHandle, BrokerError> {
        let url = "wss://api.fyers.in/socket/v2/data/";
        
        let mut request = url.into_client_request()
            .map_err(|e| BrokerError::Network(format!("Invalid WS URL: {}", e)))?;
        
        let headers = request.headers_mut();
        if let Ok(hv) = HeaderValue::from_str(auth_token) {
            headers.insert(AUTHORIZATION, hv);
        }

        let (ws_stream, _) = connect_async(request)
            .await
            .map_err(|e| BrokerError::Network(format!("WS Connect failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();
        let (tick_tx, tick_rx) = mpsc::channel::<RawTick>(1024);
        let (write_tx, mut write_rx) = mpsc::channel::<Message>(128);

        // Writer task
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if let Err(e) = write.send(msg).await {
                    error!("Fyers WS write error: {}", e);
                    break;
                }
            }
        });

        // Reader task
        let tick_tx_clone = tick_tx.clone();
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        Self::handle_message(&text, &tick_tx_clone).await;
                    }
                    Ok(Message::Binary(bin)) => {
                        // Sometimes Fyers sends binary, try to parse as JSON if it's text-encoded
                        if let Ok(text) = String::from_utf8(bin) {
                            Self::handle_message(&text, &tick_tx_clone).await;
                        }
                    }
                    Ok(Message::Ping(p)) => {
                        // Tungstenite handles ping/pong automatically in most cases, but just in case
                        debug!("Received Ping from Fyers");
                    }
                    Ok(Message::Close(_)) => {
                        info!("Fyers WS closed");
                        break;
                    }
                    Err(e) => {
                        error!("Fyers WS read error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        let inner = FyersWsInner { write_tx };

        Ok(WsHandle {
            tick_tx,
            inner: Box::new(inner),
        })
    }

    async fn handle_message(text: &str, tick_tx: &mpsc::Sender<RawTick>) {
        if let Ok(val) = serde_json::from_str::<Value>(text) {
            // Handle dictionary/JSON payload based on fyers API v3 spec
            // Example: {"symbol": "NSE:TCS-EQ", "ltp": 3730.5, "timestamp": 1729912302, "type": "symbolUpdate"}
            // Or {"type": "sf", "symbol": "...", ...}
            
            // Fyers payloads can be nested inside a "data" array or sent directly.
            // Let's assume standard Symbol Update JSON as per research.
            let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
            
            if msg_type == "symbolUpdate" || msg_type == "sf" {
                if let Some(symbol) = val.get("symbol").and_then(|v| v.as_str()) {
                    if let Some(ltp) = val.get("ltp").and_then(|v| v.as_f64()) {
                        let timestamp = val.get("timestamp")
                            .and_then(|v| v.as_i64())
                            .unwrap_or_else(|| chrono::Utc::now().timestamp());
                            
                        let volume = val.get("v").and_then(|v| v.as_u64()); // Fyers usually sends volume in 'v' or 'vol'
                        
                        let tick = RawTick {
                            broker_token: symbol.to_string(),
                            broker_timestamp_ms: timestamp * 1000,
                            ltp,
                            last_quantity: 0,
                            volume,
                            bid: None,
                            bid_quantity: None,
                            ask: None,
                            ask_quantity: None,
                            open: val.get("o").and_then(|v| v.as_f64()),
                            high: val.get("h").and_then(|v| v.as_f64()),
                            low: val.get("l").and_then(|v| v.as_f64()),
                            previous_close: val.get("c").and_then(|v| v.as_f64()),
                        };
                        
                        let _ = tick_tx.send(tick).await;
                    }
                }
            }
        }
    }

    pub async fn subscribe(ws_handle: &WsHandle, tokens: &[String]) -> Result<(), BrokerError> {
        let payload = json!({
            "T": "SUB_DATA",
            "L2list": tokens,
            "type": "symbolData"
        });

        if let Some(inner) = ws_handle.inner.downcast_ref::<FyersWsInner>() {
            inner.write_tx.send(Message::Text(payload.to_string()))
                .await
                .map_err(|e| BrokerError::Network(format!("WS Subscribe failed: {}", e)))?;
        }
        Ok(())
    }

    pub async fn unsubscribe(ws_handle: &WsHandle, tokens: &[String]) -> Result<(), BrokerError> {
        let payload = json!({
            "T": "UNSUB_DATA",
            "L2list": tokens,
            "type": "symbolData"
        });

        if let Some(inner) = ws_handle.inner.downcast_ref::<FyersWsInner>() {
            inner.write_tx.send(Message::Text(payload.to_string()))
                .await
                .map_err(|e| BrokerError::Network(format!("WS Unsubscribe failed: {}", e)))?;
        }
        Ok(())
    }

    pub async fn disconnect(ws_handle: &WsHandle) -> Result<(), BrokerError> {
        if let Some(inner) = ws_handle.inner.downcast_ref::<FyersWsInner>() {
            let _ = inner.write_tx.send(Message::Close(None)).await;
        }
        Ok(())
    }
}
