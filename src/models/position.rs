use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Position {
    pub asset_id: String,
    pub symbol: String,
    pub qty: String,
    pub avg_entry_price: String,
    pub market_value: String,
    pub current_price: String,
    pub unrealized_pl: String,
    pub unrealized_plpc: String,
}