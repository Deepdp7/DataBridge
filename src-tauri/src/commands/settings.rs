use tauri::State;
use crate::AppState;
use std::collections::HashMap;

#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    state.storage.settings.get(&key).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    state.storage.settings.set(&key, &value).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_settings(
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    state.storage.settings.get_all().await.map_err(|e| e.to_string())
}
