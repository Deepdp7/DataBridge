use tauri::State;
use crate::AppState;

#[tauri::command]
pub async fn get_backfill_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use crate::storage::repository::BackfillJobStatus;
    let queued = state.storage.backfill
        .list_jobs_by_status(&[BackfillJobStatus::Queued]).await.map(|j| j.len()).unwrap_or(0);
    let downloading = state.storage.backfill
        .list_jobs_by_status(&[BackfillJobStatus::Downloading]).await.map(|j| j.len()).unwrap_or(0);
    let completed = state.storage.backfill
        .list_jobs_by_status(&[BackfillJobStatus::Completed]).await.map(|j| j.len()).unwrap_or(0);
    let failed = state.storage.backfill
        .list_jobs_by_status(&[BackfillJobStatus::Failed]).await.map(|j| j.len()).unwrap_or(0);

    Ok(serde_json::json!({
        "queued": queued,
        "downloading": downloading,
        "completed": completed,
        "failed": failed,
    }))
}

#[tauri::command]
pub async fn trigger_backfill(
    state: State<'_, AppState>,
    symbol_id: i64,
) -> Result<i64, String> {
    // Enqueue a high-priority backfill job
    use crate::storage::repository::NewBackfillJob;
    use chrono::Utc;

    let to   = Utc::now();
    let from = to - chrono::Duration::days(365);

    let job_id = state.storage.backfill
        .create_job(&NewBackfillJob {
            symbol_id,
            requested_from: from,
            requested_to: to,
            priority: 10, // high priority for manual trigger
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

#[tauri::command]
pub async fn cancel_backfill(
    state: State<'_, AppState>,
    job_id: i64,
) -> Result<(), String> {
    use crate::storage::repository::{BackfillJobStatus, BackfillJobUpdate};
    state.storage.backfill
        .update_job_status(job_id, &BackfillJobUpdate {
            status: BackfillJobStatus::Cancelled,
            fetched_up_to: None,
            attempt_count: None,
            last_error: None,
        })
        .await
        .map_err(|e| e.to_string())
}
