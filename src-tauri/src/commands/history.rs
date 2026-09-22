use tauri::State;
use crate::AppState;
use crate::models::Interval;
use chrono::{DateTime, Utc};

#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
    symbol_id: i64,
    interval: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<serde_json::Value, String> {
    let iv = interval.as_deref().unwrap_or("1m").parse::<Interval>()
        .map_err(|e| e.to_string())?;
    let from_dt = from.and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let to_dt = to.and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);

    let bars = state.storage.historical
        .query_bars(symbol_id, iv, from_dt, to_dt)
        .await
        .map_err(|e| e.to_string())?;

    let first = bars.first().map(|b| b.timestamp.to_rfc3339());
    let last  = bars.last().map(|b| b.timestamp.to_rfc3339());
    let count = bars.len();

    Ok(serde_json::json!({
        "symbol_id": symbol_id,
        "interval": iv.as_str(),
        "first_timestamp": first,
        "last_timestamp": last,
        "record_count": count,
        "bars": bars,
    }))
}

#[tauri::command]
pub async fn get_bar_count(
    state: State<'_, AppState>,
    symbol_id: i64,
    interval: Option<String>,
) -> Result<i64, String> {
    let iv = interval.as_deref().unwrap_or("1m").parse::<Interval>()
        .map_err(|e| e.to_string())?;
    state.storage.historical
        .count_bars(symbol_id, iv)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_gaps(
    state: State<'_, AppState>,
    symbol_id: i64,
) -> Result<serde_json::Value, String> {
    let gaps = state.storage.backfill
        .list_open_gaps(symbol_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!(gaps))
}

#[tauri::command]
pub async fn repair_gaps(
    state: State<'_, AppState>,
    symbol_id: i64,
) -> Result<serde_json::Value, String> {
    // Queue repair jobs for all open gaps
    let gaps = state.storage.backfill
        .list_open_gaps(symbol_id)
        .await
        .map_err(|e| e.to_string())?;

    let mut repair_count = 0usize;
    for gap in &gaps {
        let _ = state.storage.backfill.create_job(
            &crate::storage::repository::NewBackfillJob {
                symbol_id,
                requested_from: gap.gap_start,
                requested_to: gap.gap_end,
                priority: 5,
            }
        ).await;
        repair_count += 1;
    }

    Ok(serde_json::json!({
        "gaps_found": gaps.len(),
        "repair_jobs_created": repair_count,
    }))
}
