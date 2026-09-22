use tauri::State;
use crate::AppState;

#[tauri::command]
pub async fn get_live_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "ws_connected": false,
        "ticks_per_sec": 0.0,
        "active_subscriptions": 0,
        "last_tick_at": null,
    }))
}

#[tauri::command]
pub async fn get_tick_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "ticks_per_sec": 0.0,
        "total_ticks": 0,
        "duplicate_ticks": 0,
        "dropped_ticks": 0,
        "active_subscriptions": 0,
        "ws_state": "disconnected",
        "last_tick_at": null,
    }))
}
