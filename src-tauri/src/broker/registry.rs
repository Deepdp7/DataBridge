/// BrokerRegistry — maps broker_id strings to BrokerAdapter trait objects.
///
/// The registry is populated at application startup. Adding a new broker
/// requires only registering it here — zero changes to the core engine.

use std::collections::HashMap;
use std::sync::Arc;

use crate::broker::adapter::BrokerAdapter;

pub struct BrokerRegistry {
    adapters: HashMap<String, Arc<dyn BrokerAdapter>>,
}

impl BrokerRegistry {
    pub fn new() -> Self {
        BrokerRegistry {
            adapters: HashMap::new(),
        }
    }

    /// Register a broker adapter. Overwrites any existing adapter with the same id.
    pub fn register(&mut self, adapter: Arc<dyn BrokerAdapter>) {
        let id = adapter.broker_id().to_string();
        self.adapters.insert(id, adapter);
    }

    /// Retrieve an adapter by broker_id.
    pub fn get(&self, broker_id: &str) -> Option<Arc<dyn BrokerAdapter>> {
        self.adapters.get(broker_id).cloned()
    }

    /// List all registered broker IDs.
    pub fn list_ids(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }

    /// Returns the number of registered adapters.
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

impl Default for BrokerRegistry {
    fn default() -> Self { Self::new() }
}
