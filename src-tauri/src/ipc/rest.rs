/// IPC REST handlers — PRD §20.1

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::storage::repository::{LogFilter, SymbolFilter};

use super::IpcState;

pub async fn get_status(State(state): State<Arc<IpcState>>) -> Json<Value> {
    let total_symbols = state.storage.symbols
        .count_symbols(None).await.unwrap_or(0);
    let active_symbols = state.storage.symbols
        .count_symbols(Some(crate::models::SymbolStatus::Enabled)).await.unwrap_or(0);
    let errors = state.storage.logs
        .count_errors_last_hour().await.unwrap_or(0);

    Json(json!({
        "broker_connected": false,      // updated by engine state in full impl
        "websocket_connected": false,
        "amibroker_connected": false,
        "symbols_total": total_symbols,
        "symbols_active": active_symbols,
        "backfill_progress_pct": 0,
        "ticks_per_sec": 0,
        "errors_last_hour": errors
    }))
}

pub async fn get_symbols(State(state): State<Arc<IpcState>>) -> Json<Value> {
    let symbols = state.storage.symbols
        .list_symbols(&SymbolFilter::default())
        .await
        .unwrap_or_default();
    Json(json!(symbols))
}

pub async fn get_backfill_status(State(state): State<Arc<IpcState>>) -> Json<Value> {
    Json(json!({
        "queued": 0,
        "downloading": 0,
        "completed": 0,
        "failed": 0
    }))
}

pub async fn get_live_status(State(state): State<Arc<IpcState>>) -> Json<Value> {
    Json(json!({
        "ws_connected": false,
        "ticks_per_sec": 0,
        "active_subscriptions": 0,
        "last_tick_at": null
    }))
}

#[derive(Deserialize)]
pub struct LogQuery {
    pub level: Option<String>,
    pub module: Option<String>,
    pub limit: Option<i64>,
}

pub async fn get_logs(
    State(state): State<Arc<IpcState>>,
    Query(q): Query<LogQuery>,
) -> Json<Value> {
    let filter = LogFilter {
        level: q.level,
        module: q.module,
        limit: q.limit.or(Some(100)),
        ..Default::default()
    };
    let logs = state.storage.logs.query_logs(&filter).await.unwrap_or_default();
    Json(json!(logs))
}

pub async fn get_history(
    State(state): State<Arc<IpcState>>,
    axum::extract::Path(symbol): axum::extract::Path<String>,
) -> Json<Value> {
    use crate::models::Interval;
    use chrono::{Utc, TimeZone};
    
    // Look up the symbol id
    let symbols = state.storage.symbols.list_symbols(&SymbolFilter::default()).await.unwrap_or_default();
    if let Some(sym) = symbols.into_iter().find(|s| s.symbol == symbol) {
        // Fetch bars
        let from = Utc.timestamp_opt(0, 0).unwrap();
        let to = Utc::now();
        if let Ok(bars) = state.storage.historical.query_bars(sym.id, Interval::OneMinute, from, to).await {
            // Map to JSON
            let mut result = Vec::new();
            for b in bars {
                result.push(json!({
                    "timestamp": b.timestamp.to_rfc3339(),
                    "open": b.open,
                    "high": b.high,
                    "low": b.low,
                    "close": b.close,
                    "volume": b.volume,
                    "open_int": b.open_int
                }));
            }
            return Json(json!(result));
        }
    }
    
    Json(json!([]))
}
