use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use crate::models::websocket::WsUpdate;

pub struct WsManager {
    // Channel to broadcast updates to all connected clients
    pub tx: broadcast::Sender<WsUpdate>,
    // Track global subscriptions to optimize backend requests
    pub active_symbols: Arc<Mutex<HashSet<String>>>,
}

impl WsManager {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            tx,
            active_symbols: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn get_active_symbols(&self) -> Vec<String> {
        let symbols = self.active_symbols.lock().unwrap();
        symbols.iter().cloned().collect()
    }

    pub fn add_symbols(&self, symbols: &[String]) {
        let mut active = self.active_symbols.lock().unwrap();
        for symbol in symbols {
            active.insert(symbol.clone());
        }
    }

    pub fn remove_symbols(&self, _symbols: &[String]) {
        // In a real app, we'd need to count how many clients are subscribed to each symbol
        // For simplicity here, we'll just keep them active or implement reference counting if needed.
        // For now, let's keep them active to avoid frequent re-subscribing to Alpaca.
    }
}
