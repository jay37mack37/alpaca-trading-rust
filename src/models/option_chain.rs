use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OptionChainResponse {
    pub symbol: String,
    pub underlying_price: f64,
    pub strikes: Vec<StrikeData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrikeData {
    pub strike: f64,
    pub call: OptionEntry,
    pub put: OptionEntry,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptionEntry {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub size: i64,
}
