/// broker/mod.rs — Broker adapter subsystem

pub mod adapter;
pub mod error;
pub mod mock;
pub mod fyers;
pub mod registry;
pub mod types;

pub use adapter::BrokerAdapter;
pub use error::BrokerError;
pub use mock::MockBrokerAdapter;
pub use registry::BrokerRegistry;
pub use types::{ConnectionState, ConnectionStatus, RateLimitConfig, WsConnectionState, WsHandle};
