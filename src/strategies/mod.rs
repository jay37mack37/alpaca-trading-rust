pub mod listing_arbitrage;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Represents the state of a single strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StrategyState {
    #[serde(rename = "Idle")]
    Idle,
    #[serde(rename = "Running")]
    Running,
    #[serde(rename = "Error")]
    Error,
}

/// Represents a single strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub state: StrategyState,
}

/// Global Strategy Manager - maintains task handles and state for all strategies
pub struct StrategyManager {
    tasks: Arc<RwLock<HashMap<u32, JoinHandle<()>>>>,
    states: Arc<RwLock<HashMap<u32, StrategyState>>>,
}

impl StrategyManager {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        
        // Initialize all 5 strategies as Idle
        states.insert(1, StrategyState::Idle);
        states.insert(2, StrategyState::Idle);
        states.insert(3, StrategyState::Idle);
        states.insert(4, StrategyState::Idle);
        states.insert(5, StrategyState::Idle);

        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(states)),
        }
    }

    /// Get the current state of a strategy
    pub async fn get_state(&self, strategy_id: u32) -> Option<StrategyState> {
        self.states.read().await.get(&strategy_id).copied()
    }

    /// Start a strategy by spawning a background task
    pub async fn start_strategy(&self, strategy_id: u32) -> Result<(), String> {
        // Check if already running
        if let Some(state) = self.get_state(strategy_id).await {
            if state == StrategyState::Running {
                return Err("Strategy is already running".to_string());
            }
        } else {
            return Err("Strategy not found".to_string());
        }

        // Update state to Running
        self.states.write().await.insert(strategy_id, StrategyState::Running);

        // Spawn background task for this strategy
        let states = Arc::clone(&self.states);
        let task = tokio::spawn(async move {
            match strategy_id {
                1 => run_listing_arbitrage(&states).await,
                2 => run_vwap_mean_reversion(&states).await,
                3 => run_0dte_delta_neutral(&states).await,
                4 => run_gamma_scalping(&states).await,
                5 => run_put_call_parity(&states).await,
                _ => {
                    tracing::error!("Unknown strategy ID: {}", strategy_id);
                    states.write().await.insert(strategy_id, StrategyState::Error);
                }
            }
        });

        self.tasks.write().await.insert(strategy_id, task);
        tracing::info!("Strategy {} started", strategy_id);
        Ok(())
    }

    /// Stop a strategy by aborting its task
    pub async fn stop_strategy(&self, strategy_id: u32) -> Result<(), String> {
        // Check if running
        if let Some(state) = self.get_state(strategy_id).await {
            if state != StrategyState::Running {
                return Err("Strategy is not running".to_string());
            }
        } else {
            return Err("Strategy not found".to_string());
        }

        // Abort the task if it exists
        if let Some(task) = self.tasks.write().await.remove(&strategy_id) {
            task.abort();
        }

        // Update state to Idle
        self.states.write().await.insert(strategy_id, StrategyState::Idle);
        tracing::info!("Strategy {} stopped", strategy_id);
        Ok(())
    }

    /// Stop all running strategies
    pub async fn stop_all_strategies(&self) -> Vec<(u32, String)> {
        let mut results = Vec::new();
        let states = self.states.read().await;
        let running_ids: Vec<u32> = states
            .iter()
            .filter(|(_, state)| **state == StrategyState::Running)
            .map(|(id, _)| *id)
            .collect();
        drop(states);

        for strategy_id in running_ids {
            match self.stop_strategy(strategy_id).await {
                Ok(_) => results.push((strategy_id, "stopped".to_string())),
                Err(e) => results.push((strategy_id, e)),
            }
        }
        results
    }

    /// Get all strategies with their current state
    pub async fn get_all_strategies(&self) -> Vec<Strategy> {
        let strategies_data = vec![
            (1, "Listing Arbitrage", "Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering."),
            (2, "VWAP Mean Reversion", "Automated entries on standard deviation price extensions from the VWAP."),
            (3, "0DTE Delta-Neutral", "Harvests theta decay on same-day expiry options via automated spreads."),
            (4, "Gamma Scalping", "Dynamic delta hedging to profit from realized volatility."),
            (5, "Put-Call Parity", "Arbitrages discrepancies between synthesized and market option prices."),
        ];

        let states = self.states.read().await;
        
        strategies_data
            .iter()
            .map(|(id, name, desc)| Strategy {
                id: *id,
                name: name.to_string(),
                description: desc.to_string(),
                state: *states.get(id).unwrap_or(&StrategyState::Idle),
            })
            .collect()
    }
}

// ============================================================
// STRATEGY IMPLEMENTATIONS
// ============================================================

pub trait StrategyTrait {
    fn name(&self) -> &str;
    #[allow(async_fn_in_trait)]
    async fn run(&self);
}

/// Strategy 1: Listing Arbitrage
async fn run_listing_arbitrage(states: &Arc<RwLock<HashMap<u32, StrategyState>>>) {
    tracing::info!("Starting Listing Arbitrage strategy...");
    
    if let Ok(alpaca) = crate::api::alpaca::AlpacaClient::new() {
        let strategy = listing_arbitrage::ListingArbitrage::new(alpaca);
        strategy.run().await;
    } else {
        tracing::error!("Failed to initialize AlpacaClient. Check ENV variables.");
        states.write().await.insert(1, StrategyState::Error);
    }

    tracing::info!("Listing Arbitrage strategy stopped");
}

/// Strategy 2: VWAP Mean Reversion
/// Automated entries on standard deviation price extensions from the VWAP.
async fn run_vwap_mean_reversion(states: &Arc<RwLock<HashMap<u32, StrategyState>>>) {
    tracing::info!("Starting VWAP Mean Reversion strategy...");
    
    loop {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                // Check if we should stop
                if let Some(state) = states.read().await.get(&2) {
                    if *state != StrategyState::Running {
                        break;
                    }
                }
                
                // VWAP mean reversion logic
                // - Calculate VWAP
                // - Detect std dev extensions
                // - Place reverting trades
                tracing::debug!("VWAP Mean Reversion: Analyzing price deviations...");
            }
        }
    }

    tracing::info!("VWAP Mean Reversion strategy stopped");
}

/// Strategy 3: 0DTE Delta-Neutral
/// Harvests theta decay on same-day expiry options via automated spreads.
async fn run_0dte_delta_neutral(states: &Arc<RwLock<HashMap<u32, StrategyState>>>) {
    tracing::info!("Starting 0DTE Delta-Neutral strategy...");
    
    loop {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                // Check if we should stop
                if let Some(state) = states.read().await.get(&3) {
                    if *state != StrategyState::Running {
                        break;
                    }
                }
                
                // 0DTE delta-neutral logic
                // - Monitor same-day expiry options
                // - Build delta-neutral spreads
                // - Harvest theta decay
                tracing::debug!("0DTE Delta-Neutral: Harvesting theta...");
            }
        }
    }

    tracing::info!("0DTE Delta-Neutral strategy stopped");
}

/// Strategy 4: Gamma Scalping
/// Dynamic delta hedging to profit from realized volatility.
async fn run_gamma_scalping(states: &Arc<RwLock<HashMap<u32, StrategyState>>>) {
    tracing::info!("Starting Gamma Scalping strategy...");
    
    loop {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                // Check if we should stop
                if let Some(state) = states.read().await.get(&4) {
                    if *state != StrategyState::Running {
                        break;
                    }
                }
                
                // Gamma scalping logic
                // - Monitor gamma and delta
                // - Dynamic rehedging
                // - Profit from realized volatility
                tracing::debug!("Gamma Scalping: Rehedging positions...");
            }
        }
    }

    tracing::info!("Gamma Scalping strategy stopped");
}

/// Strategy 5: Put-Call Parity
/// Arbitrages discrepancies between synthesized and market option prices.
async fn run_put_call_parity(states: &Arc<RwLock<HashMap<u32, StrategyState>>>) {
    tracing::info!("Starting Put-Call Parity strategy...");
    
    loop {
        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {
                // Check if we should stop
                if let Some(state) = states.read().await.get(&5) {
                    if *state != StrategyState::Running {
                        break;
                    }
                }
                
                // Put-call parity arbitrage logic
                // - Calculate synthetic prices
                // - Monitor market prices
                // - Execute arbitrage trades
                tracing::debug!("Put-Call Parity: Scanning for discrepancies...");
            }
        }
    }

    tracing::info!("Put-Call Parity strategy stopped");
}

// ============================================================
// KRONOS AI BRIDGE INTEGRATION
// ============================================================

/// Connect to Kronos AI bridge on localhost:8000
async fn connect_to_kronos_bridge() -> Result<(), String> {
    let client = reqwest::Client::new();
    
    match client
        .get("http://localhost:8000/health")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                Ok(())
            } else {
                Err(format!("Kronos bridge returned status: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to connect to Kronos bridge: {}", e)),
    }
}

impl Default for StrategyManager {
    fn default() -> Self {
        Self::new()
    }
}
