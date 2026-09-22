use tauri::State;
use crate::AppState;

#[tauri::command]
pub async fn get_system_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let total  = state.storage.symbols.count_symbols(None).await.unwrap_or(0);
    let active = state.storage.symbols.count_symbols(Some(crate::models::SymbolStatus::Enabled)).await.unwrap_or(0);
    let errors = state.storage.logs.count_errors_last_hour().await.unwrap_or(0);
    let settings = state.storage.settings.get_all().await.unwrap_or_default();

    Ok(serde_json::json!({
        "broker_connected": false,
        "websocket_connected": false,
        "amibroker_connected": false,
        "session_status": "unknown",
        "symbols_total": total,
        "symbols_active": active,
        "backfill_progress_pct": 0,
        "ticks_per_sec": 0.0,
        "errors_last_hour": errors,
        "ipc_port": settings.get("ipc_port").cloned().unwrap_or_else(|| "7421".into()),
        "active_broker": state.active_broker_id,
    }))
}

#[tauri::command]
pub async fn set_start_with_windows(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    state.storage.settings
        .set("start_with_windows", if enabled { "true" } else { "false" })
        .await
        .map_err(|e| e.to_string())?;

    // Write or remove the Windows startup registry key
    #[cfg(target_os = "windows")]
    {
        set_windows_startup(enabled)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_windows_startup(enabled: bool) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegOpenKeyExW, RegSetValueExW, RegDeleteValueW,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    let run_key: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16().collect();
    let value_name: Vec<u16> = "DataBridge\0".encode_utf16().collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(run_key.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        ).map_err(|e| format!("RegOpenKeyExW failed: {}", e))?;

        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| format!("Cannot get exe path: {}", e))?;
            let exe_str: Vec<u16> = exe_path
                .to_string_lossy()
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            RegSetValueExW(
                hkey,
                PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    exe_str.as_ptr() as *const u8,
                    exe_str.len() * 2,
                )),
            ).map_err(|e| format!("RegSetValueExW failed: {}", e))?;
        } else {
            let _ = RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()));
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_ipc_port(state: State<'_, AppState>) -> Result<u16, String> {
    let port = state.storage.settings
        .get("ipc_port").await
        .map_err(|e| e.to_string())?
        .and_then(|s| s.parse().ok())
        .unwrap_or(7421u16);
    Ok(port)
}
