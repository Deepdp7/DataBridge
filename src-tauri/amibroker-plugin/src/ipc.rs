use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc, TimeZone};
use parking_lot::RwLock;
use reqwest::blocking::Client;
use tungstenite::{connect, Message};
use url::Url;
use std::collections::HashMap;
use std::thread;
use tracing::{info, error, warn};
use serde::{Deserialize, Serialize};

use crate::adk::{RecentInfo, AmiDate};

// Internal model for incoming ticks/bars
#[derive(Debug, Deserialize)]
pub struct BarDto {
    pub timestamp: String,
    pub open: f32,
    pub high: f32,
    pub low: f32,
    pub close: f32,
    pub volume: f32,
    pub open_int: f32,
}

#[derive(Debug, Deserialize)]
pub struct TickDto {
    pub symbol: String,
    pub timestamp: String, // ISO8601
    pub last_price: f32,
    pub volume: f32,
    pub bid: f32,
    pub ask: f32,
}

pub struct IpcClient {
    http_client: Client,
    pub base_url: String,
    pub ws_url: String,
    pub recent_info_cache: Arc<RwLock<HashMap<String, RecentInfo>>>,
}

impl IpcClient {
    pub fn new() -> Self {
        // Defaults to the standard DataBridge IPC port.
        // In a production setup, this could be read from a config file or registry.
        let port = 7421;
        
        let client = IpcClient {
            http_client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            base_url: format!("http://127.0.0.1:{}", port),
            ws_url: format!("ws://127.0.0.1:{}/ws", port),
            recent_info_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        client.start_ws_listener();
        client
    }

    pub fn fetch_bars(&self, symbol: &str) -> Vec<BarDto> {
        let url = format!("{}/history/{}", self.base_url, symbol);
        match self.http_client.get(&url).send() {
            Ok(resp) => {
                if resp.status().is_success() {
                    match resp.json::<Vec<BarDto>>() {
                        Ok(bars) => bars,
                        Err(e) => {
                            error!("Failed to parse bars for {}: {}", symbol, e);
                            Vec::new()
                        }
                    }
                } else {
                    error!("Failed to fetch bars for {}: HTTP {}", symbol, resp.status());
                    Vec::new()
                }
            }
            Err(e) => {
                error!("HTTP error fetching bars for {}: {}", symbol, e);
                Vec::new()
            }
        }
    }

    fn start_ws_listener(&self) {
        let ws_url = self.ws_url.clone();
        let cache = self.recent_info_cache.clone();

        thread::spawn(move || {
            loop {
                info!("Connecting to DataBridge WebSocket at {}", ws_url);
                match connect(&ws_url) {
                    Ok((mut socket, _)) => {
                        info!("Connected to DataBridge Live Feed");
                        loop {
                            match socket.read() {
                                Ok(msg) => {
                                    if let Message::Text(text) = msg {
                                        if let Ok(tick) = serde_json::from_str::<TickDto>(&text) {
                                            let dt = DateTime::parse_from_rfc3339(&tick.timestamp).unwrap_or_default().with_timezone(&Utc);
                                            
                                            // Convert to AB RecentInfo
                                            let mut ri = RecentInfo::default();
                                            let sym_bytes = tick.symbol.as_bytes();
                                            let len = sym_bytes.len().min(63);
                                            for i in 0..len {
                                                ri.name[i] = sym_bytes[i] as i8;
                                            }
                                            ri.name[len] = 0;
                                            
                                            use chrono::Timelike;
                                            use chrono::Datelike;
                                            ri.date_time = AmiDate::new(
                                                dt.year() as u32,
                                                dt.month() as u32,
                                                dt.day() as u32,
                                                dt.hour() as u32,
                                                dt.minute() as u32,
                                                dt.second() as u32,
                                                (dt.nanosecond() / 1000) as u32
                                            );
                                            ri.last = tick.last_price;
                                            ri.volume = tick.volume;
                                            ri.bid = tick.bid;
                                            ri.ask = tick.ask;
                                            
                                            // Flags bit 0 means UPDATE_LAST, bit 1 means UPDATE_TRADEVOL
                                            ri.update_flags = 1 | 2; // last, volume
                                            
                                            // Update cache
                                            let mut w_cache = cache.write();
                                            w_cache.insert(tick.symbol.clone(), ri);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("WebSocket read error: {}", e);
                                    break; // reconnect
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("WebSocket connection failed: {}", e);
                    }
                }
                thread::sleep(Duration::from_secs(5));
            }
        });
    }
}
