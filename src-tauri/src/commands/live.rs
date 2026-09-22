use std::sync::Arc;
use tauri::State;
use crate::AppState;
use crate::websocket::{LiveTickEngine, LiveTickCommand};
use crate::models::session::Session;

#[tauri::command]
pub async fn start_live_feed(state: State<'_, AppState>) -> Result<(), String> {
    let mut engine_lock = state.live_engine.lock().await;
    if engine_lock.is_some() {
        return Err("Live feed is already running".to_string());
    }

    let broker_id = &state.active_broker_id;
    let broker_row = state.storage.brokers.get_broker_by_key(broker_id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Broker not found".to_string())?;

    let session_row = state.storage.brokers.get_active_session(broker_row.id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "No active session. Please authenticate first.".to_string())?;

    if session_row.session_status != "active" {
        return Err("Session is not active".to_string());
    }

    let target = format!("databridge_{}_credentials", broker_id);
    let creds_json = state.credential_store.load(&target)
        .map_err(|_| "Credentials missing".to_string())?;
    
    let creds_val: serde_json::Value = serde_json::from_str(&creds_json)
        .map_err(|_| "Malformed credentials".to_string())?;

    let session = Session {
        broker_id: broker_id.clone(),
        credential_ref: session_row.credential_ref,
        status: crate::models::session::SessionStatus::Active,
        token_expires_at: session_row.token_expires_at,
        last_validated_at: session_row.last_validated_at,
    };

    let adapter = state.broker_registry.read().await.get(broker_id).map(|a| a.clone())
        .ok_or_else(|| "Adapter not found".to_string())?;

    let engine = Arc::new(LiveTickEngine::new(adapter));

    // Get tokens to subscribe to
    let mappings = state.storage.symbols.list_mappings_for_broker(broker_row.id).await.map_err(|e| e.to_string())?;
    let tokens: Vec<String> = mappings.into_iter().map(|m| m.broker_token).collect();

    // Start background task
    let engine_clone = engine.clone();
    tokio::spawn(async move {
        engine_clone.run(session).await;
    });

    if !tokens.is_empty() {
        engine.subscribe(tokens).await;
    }

    *engine_lock = Some(engine);
    Ok(())
}

#[tauri::command]
pub async fn stop_live_feed(state: State<'_, AppState>) -> Result<(), String> {
    let mut engine_lock = state.live_engine.lock().await;
    if let Some(engine) = engine_lock.take() {
        let _ = engine.command_sender().send(LiveTickCommand::Shutdown).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_live_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let engine_lock = state.live_engine.lock().await;
    if let Some(engine) = engine_lock.as_ref() {
        let stats = engine.stats.lock().clone();
        Ok(serde_json::json!({
            "ws_connected": stats.ws_state == "connected",
            "ticks_per_sec": stats.ticks_per_sec,
            "active_subscriptions": stats.active_subscriptions,
            "last_tick_at": stats.last_tick_at,
        }))
    } else {
        Ok(serde_json::json!({
            "ws_connected": false,
            "ticks_per_sec": 0.0,
            "active_subscriptions": 0,
            "last_tick_at": null,
        }))
    }
}

#[tauri::command]
pub async fn get_tick_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let engine_lock = state.live_engine.lock().await;
    if let Some(engine) = engine_lock.as_ref() {
        let stats = engine.stats.lock().clone();
        Ok(serde_json::to_value(&stats).unwrap())
    } else {
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
}
