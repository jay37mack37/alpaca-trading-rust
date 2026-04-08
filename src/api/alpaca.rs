use reqwest::Client;
use serde_json::Value;

use crate::models::order::OrderRequest;

const ALPACA_PAPER_URL: &str = "https://paper-api.alpaca.markets/v2";
const ALPACA_LIVE_URL: &str = "https://api.alpaca.markets/v2";

#[derive(Clone)]
pub struct AlpacaClient {
    client: Client,
    api_key: String,
    api_secret: String,
    base_url: String,
}

impl AlpacaClient {
    pub fn new() -> Result<Self, &'static str> {
        let api_key = std::env::var("ALPACA_API_KEY")
            .map_err(|_| "ALPACA_API_KEY not set")?;
        let api_secret = std::env::var("ALPACA_API_SECRET")
            .map_err(|_| "ALPACA_API_SECRET not set")?;
        let environment = std::env::var("ALPACA_ENV").unwrap_or_else(|_| "paper".to_string());

        let base_url = if environment == "live" {
            ALPACA_LIVE_URL.to_string()
        } else {
            ALPACA_PAPER_URL.to_string()
        };

        Ok(Self {
            client: Client::new(),
            api_key,
            api_secret,
            base_url,
        })
    }

    pub fn with_keys(api_key: &str, api_secret: &str) -> Result<Self, &'static str> {
        Ok(Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            base_url: ALPACA_PAPER_URL.to_string(), // Default to paper
        })
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("APCA-API-KEY-ID", self.api_key.parse().unwrap());
        headers.insert("APCA-API-SECRET-KEY", self.api_secret.parse().unwrap());
        headers
    }

    /// Get account information
    pub async fn get_account(&self) -> Result<Value, reqwest::Error> {
        let url = format!("{}/account", self.base_url);
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Get all open positions
    pub async fn get_positions(&self) -> Result<Vec<Value>, reqwest::Error> {
        let url = format!("{}/positions", self.base_url);
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Get orders
    pub async fn get_orders(&self) -> Result<Vec<Value>, reqwest::Error> {
        let url = format!("{}/orders", self.base_url);
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Create a new order
    pub async fn create_order(&self, order: OrderRequest) -> Result<Value, reqwest::Error> {
        let url = format!("{}/orders", self.base_url);

        let mut body = serde_json::json!({
            "symbol": order.symbol,
            "qty": order.qty.to_string(),
            "side": order.side,
            "type": order.order_type,
            "time_in_force": order.time_in_force,
        });

        if let Some(price) = order.limit_price {
            body["limit_price"] = serde_json::json!(price.to_string());
        }

        let response = self.client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await?;

        response.json().await
    }
}

impl Default for AlpacaClient {
    fn default() -> Self {
        Self::new().expect("Failed to create AlpacaClient")
    }
}