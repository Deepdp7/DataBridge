/// Auth commands — save/load credentials, session status, trigger re-auth

use tauri::State;
use crate::AppState;

/// Save broker credentials to the OS secure store.
/// The raw secret is passed from the Settings UI and immediately stored —
/// it is NEVER written to logs, the database, or any intermediate buffer.
#[tauri::command]
pub async fn save_broker_credentials(
    state: State<'_, AppState>,
    broker_id: String,
    api_key: String,
    api_secret: String,
    extra: std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let target = format!("databridge_{}_credentials", broker_id);
    let payload = serde_json::json!({
        "api_key": api_key,
        "api_secret": api_secret,
        "extra": extra,
    });
    let payload_str = payload.to_string();

    state.credential_store
        .save(&target, &broker_id, &payload_str)
        .map_err(|e| e.to_string())?;

    tracing::info!(broker_id = %broker_id, "Broker credentials saved to secure store");
    Ok(())
}

/// Get the current session status for a broker.
#[tauri::command]
pub async fn get_session_status(
    state: State<'_, AppState>,
    broker_id: String,
) -> Result<serde_json::Value, String> {
    let broker = state.storage.brokers
        .get_broker_by_key(&broker_id)
        .await
        .map_err(|e| e.to_string())?;

    match broker {
        None => Ok(serde_json::json!({ "status": "unknown", "broker_found": false })),
        Some(b) => {
            let session = state.storage.brokers
                .get_active_session(b.id)
                .await
                .map_err(|e| e.to_string())?;

            match session {
                None => Ok(serde_json::json!({ "status": "unknown", "broker_found": true })),
                Some(s) => Ok(serde_json::json!({
                    "status": s.session_status,
                    "broker_found": true,
                    "token_expires_at": s.token_expires_at,
                    "last_validated_at": s.last_validated_at,
                })),
            }
        }
    }
}

/// Trigger a fresh authentication attempt using stored credentials.
#[tauri::command]
pub async fn trigger_authentication(
    state: State<'_, AppState>,
    broker_id: String,
) -> Result<serde_json::Value, String> {
    let target = format!("databridge_{}_credentials", broker_id);

    // Load credentials from secure store
    let creds_json = state.credential_store
        .load(&target)
        .map_err(|_| "No credentials stored for this broker. Please enter them in Settings → Broker.".to_string())?;

    let creds_val: serde_json::Value = serde_json::from_str(&creds_json)
        .map_err(|_| "Stored credentials are malformed".to_string())?;

    let creds = crate::models::BrokerCredentials {
        broker_id: broker_id.clone(),
        api_key: creds_val["api_key"].as_str().map(String::from),
        api_secret: creds_val["api_secret"].as_str().map(String::from),
        redirect_uri: None,
        totp_secret: None,
        extra: Default::default(),
    };

    // Get the adapter for this broker
    let adapter = state.broker_registry
        .read()
        .await
        .get(&broker_id)
        .ok_or_else(|| format!("No adapter registered for broker '{}'", broker_id))?;

    // Authenticate
    let session = adapter
        .authenticate(&creds)
        .await
        .map_err(|e| e.to_string())?;

    // Persist session metadata to DB (NOT the raw token — only the credential_ref)
    let broker = state.storage.brokers
        .get_broker_by_key(&broker_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Broker '{}' not found in DB", broker_id))?;

    state.storage.brokers
        .upsert_session(&crate::storage::repository::NewSessionRow {
            broker_id: broker.id,
            credential_ref: session.credential_ref.clone(),
            session_status: session.status.to_string(),
            token_expires_at: session.token_expires_at,
            last_validated_at: session.last_validated_at,
        })
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "success": true,
        "status": session.status,
        "expires_at": session.token_expires_at,
    }))
}
