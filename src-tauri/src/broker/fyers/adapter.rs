use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::broker::{BrokerAdapter, BrokerError, WsHandle};
use crate::broker::types::RateLimitConfig;
use crate::models::{BrokerCredentials, BrokerSymbol, RawBar, Session};
use crate::models::interval::Interval;

use super::auth::FyersAuth;
use super::rest::FyersRest;
use super::ws::FyersWs;

pub struct FyersAdapter {
    active_token: Arc<RwLock<Option<String>>>,
}

impl FyersAdapter {
    pub fn new() -> Self {
        Self {
            active_token: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl BrokerAdapter for FyersAdapter {
    fn broker_id(&self) -> &str {
        "fyers_v3"
    }

    fn display_name(&self) -> &str {
        "FYERS V3 API"
    }

    async fn authenticate(&self, creds: &BrokerCredentials) -> Result<Session, BrokerError> {
        let (session, token) = FyersAuth::authenticate(creds).await?;
        *self.active_token.write().await = Some(token);
        Ok(session)
    }

    async fn refresh_session(&self, _session: &Session) -> Result<Session, BrokerError> {
        Err(BrokerError::SessionExpired) // Fyers tokens are generated daily, no automated refresh
    }

    async fn validate_session(&self, session: &Session) -> Result<Session, BrokerError> {
        let token = {
            let lock = self.active_token.read().await;
            lock.clone()
        };
        
        let token = token.ok_or_else(|| BrokerError::AuthFailed("No active token in memory".to_string()))?;
        FyersAuth::validate_session(session, &token).await
    }

    async fn get_symbol_master(&self) -> Result<Vec<BrokerSymbol>, BrokerError> {
        FyersRest::get_symbol_master().await
    }

    async fn get_historical_data(
        &self,
        token: &str,
        interval: Interval,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<RawBar>, BrokerError> {
        let auth_token = {
            let lock = self.active_token.read().await;
            lock.clone().ok_or_else(|| BrokerError::AuthFailed("No active token in memory".to_string()))?
        };
        FyersRest::fetch_historical_bars(&auth_token, token, interval, from, to).await
    }

    fn max_historical_days_per_request(&self) -> u32 {
        100 // Fyers limit is 100 days for 1min data
    }

    async fn connect_websocket(&self, session: &Session) -> Result<WsHandle, BrokerError> {
        let auth_token = {
            let lock = self.active_token.read().await;
            lock.clone().ok_or_else(|| BrokerError::AuthFailed("No active token in memory".to_string()))?
        };
        FyersWs::connect(session, &auth_token).await
    }

    async fn subscribe_symbols(&self, ws_handle: &WsHandle, tokens: &[String]) -> Result<(), BrokerError> {
        FyersWs::subscribe(ws_handle, tokens).await
    }

    async fn unsubscribe_symbols(&self, ws_handle: &WsHandle, tokens: &[String]) -> Result<(), BrokerError> {
        FyersWs::unsubscribe(ws_handle, tokens).await
    }

    async fn disconnect(&self, ws_handle: &WsHandle) -> Result<(), BrokerError> {
        FyersWs::disconnect(ws_handle).await
    }

    fn rate_limit_config(&self) -> RateLimitConfig {
        RateLimitConfig {
            historical_requests_per_second: 5.0,
            general_requests_per_second: 10.0,
            max_symbols_per_ws_message: 50,
            max_ws_subscriptions: 1000,
            max_ws_connections: 1,
            requires_client_heartbeat: true,
            heartbeat_interval_secs: 30,
        }
    }

    fn supported_intervals(&self) -> Vec<Interval> {
        vec![
            Interval::OneMinute,
            Interval::FiveMinutes,
            Interval::FifteenMinutes,
            Interval::ThirtyMinutes,
            Interval::OneHour,
            Interval::Daily,
        ]
    }
}
