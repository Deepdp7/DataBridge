use tauri::State;
use crate::AppState;
use crate::storage::repository::{LogFilter, NewLogEntry};
use chrono::Utc;

#[tauri::command]
pub async fn get_logs(
    state: State<'_, AppState>,
    level: Option<String>,
    module: Option<String>,
    symbol: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let logs = state.storage.logs
        .query_logs(&LogFilter {
            level,
            module,
            symbol,
            since: None,
            limit: limit.or(Some(200)),
            offset,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!(logs))
}

#[tauri::command]
pub async fn clear_logs(state: State<'_, AppState>) -> Result<u64, String> {
    // Keep last 1 day of logs when clearing
    state.storage.logs.purge_old_logs(0).await.map_err(|e| e.to_string())
}
