use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StrategyType {
    Parity,
    Vwap,
    Listing,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub timestamp: String,
    pub strategy: StrategyType,
    pub symbol: String,
    pub edge: f64,
    pub ai_confirmation: f64,
    pub message: String,
}
