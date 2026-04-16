use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize};

#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self);
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum StrategyStatus {
    Idle,
    Running,
    PaperOnly,
}

pub struct StrategyManager {
    handles: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

impl StrategyManager {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_strategy(&self, strategy: Arc<dyn Strategy>) {
        let mut handles = self.handles.lock().await;
        let name = strategy.name().to_string();

        if handles.contains_key(&name) {
            return;
        }

        let handle = tokio::spawn(async move {
            strategy.run().await;
        });

        handles.insert(name, handle);
    }

    pub async fn stop_strategy(&self, name: &str) {
        let mut handles = self.handles.lock().await;
        if let Some(handle) = handles.remove(name) {
            handle.abort();
        }
    }

    pub async fn stop_all(&self) {
        let mut handles = self.handles.lock().await;
        for (_, handle) in handles.drain() {
            handle.abort();
        }
    }

    pub async fn get_running_strategies(&self) -> Vec<String> {
        let handles = self.handles.lock().await;
        handles.keys().cloned().collect()
    }
}

pub mod listing_arbitrage;
pub mod vwap_reversion;
pub mod delta_neutral;
pub mod gamma_scalping;
pub mod put_call_parity;
