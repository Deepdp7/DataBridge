use crate::broker::BrokerError;
use crate::models::BrokerCredentials;
use crate::models::session::{Session, SessionStatus};
use chrono::Utc;

pub struct FyersAuth {}

impl FyersAuth {
    pub async fn authenticate(creds: &BrokerCredentials) -> Result<(Session, String), BrokerError> {
        let app_id = creds.api_key.clone().unwrap_or_default();
        let access_token = creds.api_secret.clone().unwrap_or_default();
        
        if app_id.is_empty() || access_token.is_empty() {
            return Err(BrokerError::AuthFailed("Missing app_id (api_key) or access_token (api_secret) in credentials".to_string()));
        }
        
        let token = format!("{}:{}", app_id, access_token);
        
        let client = reqwest::Client::new();
        let resp = client.get("https://api.fyers.in/api/v3/profile")
            .header("Authorization", &token)
            .send()
            .await
            .map_err(|e| BrokerError::Network(e.to_string()))?;
            
        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(BrokerError::AuthFailed(format!("Fyers Auth failed: {}", error_text)));
        }
        
        let session = Session {
            broker_id: creds.broker_id.clone(),
            credential_ref: format!("databridge_{}_credentials", creds.broker_id),
            status: SessionStatus::Active,
            token_expires_at: None,
            last_validated_at: Some(Utc::now()),
        };
        
        Ok((session, token))
    }

    pub async fn validate_session(session: &Session, token: &str) -> Result<Session, BrokerError> {
        let client = reqwest::Client::new();
        let resp = client.get("https://api.fyers.in/api/v3/profile")
            .header("Authorization", token)
            .send()
            .await
            .map_err(|e| BrokerError::Network(e.to_string()))?;
            
        if resp.status().is_success() {
            let mut updated = session.clone();
            updated.last_validated_at = Some(Utc::now());
            Ok(updated)
        } else {
            Err(BrokerError::SessionExpired)
        }
    }
}
