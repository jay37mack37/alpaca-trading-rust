use reqwest::Client;
use serde_json::Value;

use crate::models::order::OrderRequest;

const ALPACA_PAPER_URL: &str = "https://paper-api.alpaca.markets/v2";
const ALPACA_LIVE_URL: &str = "https://api.alpaca.markets/v2";
const ALPACA_DATA_URL: &str = "https://data.alpaca.markets/v2";
const ALPACA_OPTIONS_URL: &str = "https://data.alpaca.markets/v1beta1/options";

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

    /// Get order by ID
    pub async fn get_order_by_id(&self, order_id: &str) -> Result<Value, reqwest::Error> {
        let url = format!("{}/orders/{}", self.base_url, order_id);
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Cancel an order by ID
    pub async fn cancel_order(&self, order_id: &str) -> Result<Value, reqwest::Error> {
        let url = format!("{}/orders/{}", self.base_url, order_id);
        let response = self.client
            .delete(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Cancel all open orders
    pub async fn cancel_all_orders(&self) -> Result<Vec<Value>, reqwest::Error> {
        let url = format!("{}/orders", self.base_url);
        let response = self.client
            .delete(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Get current price for a symbol
    pub async fn get_current_price(&self, symbol: &str) -> Result<Value, reqwest::Error> {
        // Use the market data API endpoint
        let url = format!("{}/stocks/{}/quotes/latest", ALPACA_DATA_URL, symbol);
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        response.json().await
    }

    /// Get option chain for a symbol (returns available strikes)
    pub async fn get_option_strikes(&self, symbol: &str) -> Result<Value, reqwest::Error> {
        // Get snapshot data for the underlying to determine ATM strike
        let url = format!("{}/stocks/{}/quotes/latest", ALPACA_DATA_URL, symbol);
        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        let quote: Value = response.json().await?;

        // Get current stock price to determine ITM strikes
        // The quote object has 'ap' (ask price) and 'bp' (bid price)
        let quote_obj = quote.get("quote").unwrap_or(&quote);
        let ask_price = quote_obj.get("ap").and_then(|p| p.as_f64()).unwrap_or(0.0);
        let bid_price = quote_obj.get("bp").and_then(|p| p.as_f64()).unwrap_or(0.0);

        // Use ask price if available, otherwise bid price
        let current_price = if ask_price > 0.0 { ask_price } else { bid_price };

        // Determine strike increment based on price level
        let strike_increment = if current_price < 25.0 { 0.5 } else if current_price < 200.0 { 1.0 } else { 5.0 };

        // For ITM Call: find highest strike below current price (nearest ITM)
        // Round down to nearest strike increment for call
        let call_strike = ((current_price / strike_increment).floor() - 1.0) * strike_increment;
        let call_strike = if call_strike < strike_increment { strike_increment } else { call_strike };

        // For ITM Put: find lowest strike above current price (nearest ITM)
        // Round up to nearest strike increment for put
        let put_strike = ((current_price / strike_increment).ceil() + 1.0) * strike_increment;

        // Return ITM strikes for call and put
        Ok(serde_json::json!({
            "underlying_price": current_price,
            "call_strike": call_strike,
            "put_strike": put_strike,
            "strike_increment": strike_increment
        }))
    }

    /// Get current price for an option
    pub async fn get_option_price(&self, option_symbol: &str) -> Result<Value, reqwest::Error> {
        // Extract underlying symbol from OCC format
        // OCC format: SYMBOL + YYMMDD + C/P + STRIKE (8 chars)
        // For short symbols (like SPY), no padding: SPY260408C00670000
        // For longer symbols (like GOOGL), no padding: GOOGL260408C00670000

        // Find where the date starts (6 digits for YYMMDD)
        let underlying = if let Some(pos) = option_symbol.find(|c: char| c.is_ascii_digit()) {
            let date_part: String = option_symbol[pos..].chars().take_while(|c| c.is_ascii_digit()).collect();
            if date_part.len() >= 6 {
                option_symbol[..pos].to_string()
            } else {
                option_symbol.chars().take(6).collect::<String>().trim().to_string()
            }
        } else {
            option_symbol.chars().take(6).collect::<String>().trim().to_string()
        };

        // Determine if call or put from the option symbol
        // Format: SYMBOL + YYMMDD + C/P + STRIKE
        // After 6-digit date, next char is C (call) or P (put)
        let normalized = option_symbol.replace(" ", "");
        let option_type = if let Some(pos) = normalized.find(|c: char| c.is_ascii_digit()) {
            // pos is start of date, date is 6 chars, then C/P
            let after_date = pos + 6;
            if after_date < normalized.len() {
                let type_char = normalized.chars().nth(after_date);
                if type_char == Some('P') {
                    "put"
                } else {
                    "call"
                }
            } else {
                "call"
            }
        } else {
            "call"
        };

        // Use the options snapshots API with type filter
        let url = format!(
            "{}/snapshots/{}?feed=indicative&type={}",
            ALPACA_OPTIONS_URL,
            underlying,
            option_type
        );

        let response = self.client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?;

        let data: Value = response.json().await?;

        // Find the specific option in the snapshots
        let normalized_search = normalized;

        if let Some(snapshots) = data.get("snapshots") {
            if let Some(obj) = snapshots.as_object() {
                for key in obj.keys() {
                    if key.replace(" ", "") == normalized_search {
                        if let Some(snapshot) = snapshots.get(key) {
                            if let Some(quote) = snapshot.get("latestQuote") {
                                return Ok(serde_json::json!({
                                    "quote": quote,
                                    "symbol": key
                                }));
                            }
                        }
                    }
                }
            }
        }

        // Return error if not found
        Ok(serde_json::json!({
            "error": format!("Option {} not found in chain", option_symbol),
            "underlying": underlying,
            "type": option_type
        }))
    }
}

impl Default for AlpacaClient {
    fn default() -> Self {
        Self::new().expect("Failed to create AlpacaClient")
    }
}