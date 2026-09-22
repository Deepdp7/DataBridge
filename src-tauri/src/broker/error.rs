/// Broker adapter error type — kept separate from AppError so broker-specific
/// error variants never leak into the core engine layer.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Session expired")]
    SessionExpired,

    #[error("API rate limit exceeded (retry after {retry_after_secs}s)")]
    RateLimited { retry_after_secs: u64 },

    #[error("HTTP error {status}: {message}")]
    Http { status: u16, message: String },

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Invalid response from broker API: {0}")]
    InvalidResponse(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Subscription limit exceeded (max: {max})")]
    SubscriptionLimitExceeded { max: usize },

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("{0}")]
    Other(String),
}

impl BrokerError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            BrokerError::RateLimited { .. }
                | BrokerError::Network(_)
                | BrokerError::Http { status: 429 | 500 | 502 | 503 | 504, .. }
        )
    }

    pub fn is_auth_error(&self) -> bool {
        matches!(self, BrokerError::AuthFailed(_) | BrokerError::SessionExpired)
    }
}
