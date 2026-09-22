/// Session / authentication models — PRD §9.1
///
/// `BrokerCredentials` carries what the user enters in Settings.
/// `Session` carries the live auth state returned by `BrokerAdapter::authenticate()`.
/// Raw token values are stored in Windows Credential Manager — NEVER in SQLite or logs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Credentials entered by the user in Settings → Broker.
///
/// Fields are generic to support any broker. Broker-specific adapters
/// read only the fields they need. Unused fields are `None`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerCredentials {
    /// Broker identifier matching `brokers.broker_key` (e.g. "broker_v1")
    pub broker_id: String,
    /// API key / client ID / user ID depending on broker
    pub api_key: Option<String>,
    /// API secret / password depending on broker
    pub api_secret: Option<String>,
    /// Redirect URI (for OAuth flows)
    pub redirect_uri: Option<String>,
    /// TOTP secret (for brokers using TOTP-based 2FA)
    pub totp_secret: Option<String>,
    /// Any additional broker-specific key-value pairs
    pub extra: std::collections::HashMap<String, String>,
}

/// An active broker session returned from `BrokerAdapter::authenticate()`.
///
/// Access/session tokens are stored via the OS credential store — this struct
/// holds only the metadata (status, expiry) needed for session management.
/// The opaque `credential_ref` is the key used to retrieve the actual token
/// from the credential store without storing the raw value here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub broker_id: String,
    /// Opaque reference key into Windows Credential Manager
    pub credential_ref: String,
    pub status: SessionStatus,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub last_validated_at: Option<DateTime<Utc>>,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        match self.status {
            SessionStatus::Active => {
                // If the broker gave us an expiry, check it; otherwise assume valid.
                if let Some(expiry) = self.token_expires_at {
                    expiry > Utc::now()
                } else {
                    true
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Expired,
    Invalid,
    Unknown,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Active   => write!(f, "active"),
            SessionStatus::Expired  => write!(f, "expired"),
            SessionStatus::Invalid  => write!(f, "invalid"),
            SessionStatus::Unknown  => write!(f, "unknown"),
        }
    }
}
