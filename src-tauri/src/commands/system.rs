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
            Some(0),
            KEY_SET_VALUE,
            &mut hkey,
        ).ok().map_err(|e| format!("RegOpenKeyExW failed: {}", e))?;

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
                Some(0),
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    exe_str.as_ptr() as *const u8,
                    exe_str.len() * 2,
                )),
            ).ok().map_err(|e| format!("RegSetValueExW failed: {}", e))?;
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

#[tauri::command]
pub async fn run_health_check(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let sqlite_ok = state.storage.settings.get("ipc_port").await.is_ok();
    
    let broker_id = &state.active_broker_id;
    let mut rest_ok = false;
    
    if let Ok(Some(broker)) = state.storage.brokers.get_broker_by_key(broker_id).await {
        if let Ok(Some(session)) = state.storage.brokers.get_active_session(broker.id).await {
            if session.session_status == "active" {
                if let Ok(creds_json) = state.credential_store.load(&format!("databridge_{}_credentials", broker_id)) {
                    if let Ok(creds_val) = serde_json::from_str::<serde_json::Value>(&creds_json) {
                        let adapter = state.broker_registry.read().await.get(broker_id).map(|a| a.clone());
                        if let Some(adapter) = adapter {
                            let auth = crate::models::BrokerCredentials {
                                broker_id: broker_id.clone(),
                                api_key: creds_val["api_key"].as_str().map(String::from),
                                api_secret: creds_val["api_secret"].as_str().map(String::from),
                                redirect_uri: None,
                                totp_secret: None,
                                extra: Default::default(),
                            };
                            if let Ok(s) = adapter.authenticate(&auth).await {
                                if s.status == crate::models::session::SessionStatus::Active {
                                    rest_ok = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut ws_ok = false;
    let mut live_ok = false;
    {
        let engine_lock = state.live_engine.lock().await;
        if let Some(engine) = engine_lock.as_ref() {
            let stats = engine.stats.lock().clone();
            ws_ok = stats.ws_state == "connected";
            live_ok = true;
        }
    }

    Ok(serde_json::json!({
        "sqlite": sqlite_ok,
        "rest": rest_ok,
        "historical": rest_ok,
        "websocket": ws_ok,
        "live": live_ok,
        "ipc": true
    }))
}

#[tauri::command]
pub async fn install_plugin(path: String) -> Result<(), String> {
    use std::path::PathBuf;
    
    // The DLL is built in target/release/DataBridge.dll or target/release/amibroker_plugin.dll
    // Wait, the lib name in Cargo.toml is DataBridge.
    // On Windows, a cdylib named "DataBridge" becomes "DataBridge.dll"
    
    let mut source_path = std::env::current_dir().map_err(|e| e.to_string())?;
    source_path.push("target");
    source_path.push("release");
    source_path.push("DataBridge.dll");
    
    if !source_path.exists() {
        return Err(format!("Plugin DLL not found at {:?}. Please build the project first.", source_path));
    }
    
    let mut target_path = PathBuf::from(&path);
    if target_path.is_dir() {
        target_path.push("DataBridge.dll");
    } else {
        return Err("Provided path is not a valid directory.".into());
    }
    
    std::fs::copy(&source_path, &target_path).map_err(|e| format!("Failed to copy DLL: {}", e))?;
    
    Ok(())
}
