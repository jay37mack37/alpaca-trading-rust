use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum WsAction {
    Subscribe { symbols: Vec<String> },
    Unsubscribe { symbols: Vec<String> },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum WsUpdate {
    Trade(TradeUpdate),
    Quote(QuoteUpdate),
    Error { message: String },
    SubscriptionStatus { subscribed: Vec<String> },
}

#[derive(Debug, Serialize, Clone)]
pub struct TradeUpdate {
    pub symbol: String,
    pub price: f64,
    pub size: u32,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct QuoteUpdate {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub size: u32,
    pub timestamp: String,
}
