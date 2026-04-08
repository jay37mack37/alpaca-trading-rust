use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub qty: f64,
    pub side: String,        // "buy" or "sell"
    pub order_type: String,  // "market", "limit", etc.
    pub time_in_force: String, // "day", "gtc", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub qty: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: String,
    pub status: String,
}