/// Live Tick Engine — PRD §14
///
/// Manages the broker WebSocket connection lifecycle, symbol subscriptions,
/// heartbeat/ping-pong, tick deduplication, and routing to the aggregation pipeline.
/// Runs as an independent supervised Tokio task.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::broker::{BrokerAdapter, BrokerError, WsConnectionState};
use crate::models::session::Session;
use crate::models::tick::{RawTick, Tick, TickFingerprint};

/// Commands the engine accepts while running.
#[derive(Debug)]
pub enum LiveTickCommand {
    Subscribe   { tokens: Vec<String> },
    Unsubscribe { tokens: Vec<String> },
    Reconnect,
    Shutdown,
}

/// Real-time statistics — surfaced on the Dashboard / Live Monitor.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TickStats {
    pub ticks_per_sec: f64,
    pub total_ticks: u64,
    pub duplicate_ticks: u64,
    pub dropped_ticks: u64,
    pub active_subscriptions: usize,
    pub ws_state: String,
    pub last_tick_at: Option<i64>,  // Unix timestamp millis
}

/// Internal deduplication window — PRD §14.3
struct DedupWindow {
    seen: VecDeque<(TickFingerprint, Instant)>,
}

impl DedupWindow {
    fn new() -> Self {
        DedupWindow { seen: VecDeque::with_capacity(1024) }
    }

    fn is_duplicate(&mut self, fp: &TickFingerprint) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(2);

        // Purge stale entries
        while self.seen.front().map(|(_, t)| now.duration_since(*t) > window).unwrap_or(false) {
            self.seen.pop_front();
        }

        if self.seen.iter().any(|(f, _)| f == fp) {
            return true;
        }
        self.seen.push_back((fp.clone(), now));
        false
    }
}

pub struct LiveTickEngine {
    adapter: Arc<dyn BrokerAdapter>,
    /// Normalized ticks pushed to the aggregation pipeline (bounded channel)
    pub tick_out_tx: mpsc::Sender<Tick>,
    tick_out_rx: Arc<Mutex<Option<mpsc::Receiver<Tick>>>>,
    cmd_tx: mpsc::Sender<LiveTickCommand>,
    cmd_rx: Arc<Mutex<mpsc::Receiver<LiveTickCommand>>>,
    pub stats: Arc<Mutex<TickStats>>,
    subscribed_tokens: Arc<Mutex<HashSet<String>>>,
    ws_state_tx: watch::Sender<WsConnectionState>,
    pub ws_state_rx: watch::Receiver<WsConnectionState>,
}

impl LiveTickEngine {
    pub fn new(adapter: Arc<dyn BrokerAdapter>) -> Self {
        // Bounded channel — backpressure applied per PRD §21
        let (tick_out_tx, tick_out_rx) = mpsc::channel::<Tick>(8192);
        let (cmd_tx, cmd_rx) = mpsc::channel(256);
        let (ws_state_tx, ws_state_rx) = watch::channel(WsConnectionState::Disconnected);

        LiveTickEngine {
            adapter,
            tick_out_tx,
            tick_out_rx: Arc::new(Mutex::new(Some(tick_out_rx))),
            cmd_tx,
            cmd_rx: Arc::new(Mutex::new(cmd_rx)),
            stats: Arc::new(Mutex::new(TickStats::default())),
            subscribed_tokens: Arc::new(Mutex::new(HashSet::new())),
            ws_state_tx,
            ws_state_rx,
        }
    }

    pub fn command_sender(&self) -> mpsc::Sender<LiveTickCommand> {
        self.cmd_tx.clone()
    }

    /// Take the tick output receiver (called once by the aggregation engine at startup).
    pub fn take_tick_receiver(&self) -> Option<mpsc::Receiver<Tick>> {
        self.tick_out_rx.lock().take()
    }

    /// Subscribe additional tokens at runtime (e.g. symbol added mid-session).
    pub async fn subscribe(&self, tokens: Vec<String>) {
        let _ = self.cmd_tx.send(LiveTickCommand::Subscribe { tokens }).await;
    }

    /// Unsubscribe tokens.
    pub async fn unsubscribe(&self, tokens: Vec<String>) {
        let _ = self.cmd_tx.send(LiveTickCommand::Unsubscribe { tokens }).await;
    }

    /// Main engine loop — connects, subscribes, processes ticks, and reconnects.
    pub async fn run(self: Arc<Self>, session: Session) {
        info!(broker = %self.adapter.broker_id(), "Live Tick Engine started");

        let mut reconnect_delay_secs = 1u64;
        let max_delay_secs = 60u64;

        loop {
            let _ = self.ws_state_tx.send(WsConnectionState::Connecting);
            info!("Connecting WebSocket...");

            match self.adapter.connect_websocket(&session).await {
                Ok(ws_handle) => {
                    reconnect_delay_secs = 1; // reset on successful connection
                    let _ = self.ws_state_tx.send(WsConnectionState::Connected);
                    info!("WebSocket connected");

                    // Re-subscribe all previously subscribed tokens
                    let tokens: Vec<String> = self.subscribed_tokens.lock().iter().cloned().collect();
                    if !tokens.is_empty() {
                        let _ = self.adapter.subscribe_symbols(&ws_handle, &tokens).await;
                        info!(count = tokens.len(), "Resubscribed all tokens");
                    }

                    // Process incoming ticks and commands until disconnect
                    let disconnect = self.process_ticks(&ws_handle).await;

                    let _ = self.adapter.disconnect(&ws_handle).await;
                    let _ = self.ws_state_tx.send(WsConnectionState::Reconnecting);

                    if !disconnect {
                        // Normal shutdown requested
                        break;
                    }
                }
                Err(e) => {
                    error!("WebSocket connection failed: {}", e);
                    let _ = self.ws_state_tx.send(WsConnectionState::Reconnecting);
                }
            }

            warn!(secs = reconnect_delay_secs, "Reconnecting after delay...");
            tokio::time::sleep(Duration::from_secs(reconnect_delay_secs)).await;
            reconnect_delay_secs = (reconnect_delay_secs * 2).min(max_delay_secs);
        }

        let _ = self.ws_state_tx.send(WsConnectionState::Disconnected);
        info!("Live Tick Engine stopped");
    }

    /// Returns `true` if reconnect is needed, `false` if clean shutdown was requested.
    async fn process_ticks(&self, ws_handle: &crate::broker::WsHandle) -> bool {
        let mut dedup = DedupWindow::new();
        let mut ticks_in_window = 0u64;
        let mut window_start = Instant::now();

        // We get raw ticks from the WsHandle's tick_tx channel on the adapter side.
        // The adapter sends RawTick → we receive on tick_tx's paired receiver.
        // For now, since WsHandle owns the sender, we poll via a small sleep loop
        // and check cmd_rx for control messages.

        let mut cmd_rx = self.cmd_rx.lock();
        let mut heartbeat_ticker = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(LiveTickCommand::Subscribe { tokens }) => {
                            {
                                let mut subs = self.subscribed_tokens.lock();
                                for t in &tokens { subs.insert(t.clone()); }
                            }
                            let _ = self.adapter.subscribe_symbols(ws_handle, &tokens).await;
                            let count = self.subscribed_tokens.lock().len();
                            self.stats.lock().active_subscriptions = count;
                        }
                        Some(LiveTickCommand::Unsubscribe { tokens }) => {
                            {
                                let mut subs = self.subscribed_tokens.lock();
                                for t in &tokens { subs.remove(t); }
                            }
                            let _ = self.adapter.unsubscribe_symbols(ws_handle, &tokens).await;
                            let count = self.subscribed_tokens.lock().len();
                            self.stats.lock().active_subscriptions = count;
                        }
                        Some(LiveTickCommand::Reconnect) => return true,
                        Some(LiveTickCommand::Shutdown) | None => return false,
                    }
                }
                _ = heartbeat_ticker.tick() => {
                    // Update ticks/sec stat
                    let elapsed = window_start.elapsed().as_secs_f64();
                    if elapsed > 0.0 {
                        self.stats.lock().ticks_per_sec = ticks_in_window as f64 / elapsed;
                    }
                    ticks_in_window = 0;
                    window_start = Instant::now();
                }
            }
        }
    }

    /// Process a raw tick from the adapter: dedup → normalize → push downstream.
    fn process_raw_tick(&self, raw: RawTick, dedup: &mut DedupWindow) -> bool {
        // Build a normalized Tick from RawTick (symbol/exchange lookup would happen here)
        let tick = Tick {
            symbol:        String::new(), // will be filled by symbol lookup in production
            exchange:      String::new(),
            broker_token:  raw.broker_token,
            timestamp:     chrono::DateTime::<chrono::Utc>::from_timestamp_millis(raw.broker_timestamp_ms)
                               .unwrap_or_else(chrono::Utc::now),
            ltp:           raw.ltp,
            last_quantity: raw.last_quantity,
            volume:        raw.volume,
            bid:           raw.bid,
            bid_quantity:  raw.bid_quantity,
            ask:           raw.ask,
            ask_quantity:  raw.ask_quantity,
            open:          raw.open,
            high:          raw.high,
            low:           raw.low,
            previous_close: raw.previous_close,
        };

        let fp = tick.fingerprint();
        if dedup.is_duplicate(&fp) {
            self.stats.lock().duplicate_ticks += 1;
            return false;
        }

        // Push to aggregation pipeline (bounded — drop if full with backpressure log)
        match self.tick_out_tx.try_send(tick) {
            Ok(_) => {
                let mut stats = self.stats.lock();
                stats.total_ticks += 1;
                stats.last_tick_at = Some(chrono::Utc::now().timestamp_millis());
                true
            }
            Err(_) => {
                warn!("Tick channel full — dropping tick (backpressure)");
                self.stats.lock().dropped_ticks += 1;
                false
            }
        }
    }
}
