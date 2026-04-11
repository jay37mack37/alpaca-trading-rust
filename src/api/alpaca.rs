use reqwest::{header::HeaderMap, Client};
use serde_json::Value;

use crate::error::AppResult;
use crate::models::option_chain::{OptionChainResponse, OptionEntry, StrikeData};
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

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Ok(key) = self.api_key.parse() {
            headers.insert("APCA-API-KEY-ID", key);
        }
        if let Ok(secret) = self.api_secret.parse() {
            headers.insert("APCA-API-SECRET-KEY", secret);
        }
        headers
    }

    /// Get account information
    pub async fn get_account(&self) -> AppResult<Value> {
        let url = format!("{}/account", self.base_url);
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    /// Get all open positions
    pub async fn get_positions(&self) -> AppResult<Vec<Value>> {
        let url = format!("{}/positions", self.base_url);
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    /// Get orders
    pub async fn get_orders(&self, status: Option<&str>) -> AppResult<Vec<Value>> {
        let mut url = format!("{}/orders", self.base_url);
        if let Some(s) = status {
            url = format!("{}?status={}", url, s);
        }
        let response = self
            .client
            .get(&url)
            .headers(self.build_headers())
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json().await?)
    }

    /// Create a new order
    pub async fn create_order(&self, order: OrderRequest) -> AppResult<Value> {
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

        if let Some(asset_class) = order.asset_class {
            body["asset_class"] = serde_json::json!(asset_class);
        }

        let response = self
            .client
            .post(&url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_json: Value =
                response.json().await.unwrap_or_else(|_| serde_json::json!({"message": "Unknown error"}));
            tracing::error!("Alpaca API error ({}): {:?}", status, error_json);

            // Return the error JSON as the result so the frontend can display the message from Alpaca
            return Ok(error_json);
        }

        Ok(response.json().await?)
    }

    /// Get order by ID
    pub async fn get_order_by_id(&self, order_id: &str) -> AppResult<Value> {
        let url = format!("{}/orders/{}", self.base_url, order_id);
        let response = self.client.get(&url).headers(self.build_headers()).send().await?.error_for_status()?;

        Ok(response.json().await?)
    }

    /// Cancel an order by ID
    pub async fn cancel_order(&self, order_id: &str) -> AppResult<Value> {
        let url = format!("{}/orders/{}", self.base_url, order_id);
        let response = self.client.delete(&url).headers(self.build_headers()).send().await?.error_for_status()?;

        Ok(response.json().await?)
    }

    /// Cancel all open orders
    pub async fn cancel_all_orders(&self) -> AppResult<Vec<Value>> {
        let url = format!("{}/orders", self.base_url);
        let response = self.client.delete(&url).headers(self.build_headers()).send().await?.error_for_status()?;

        Ok(response.json().await?)
    }

    /// Get current price for a symbol
    pub async fn get_current_price(&self, symbol: &str) -> AppResult<Value> {
        // Use the market data API endpoint
        let url = format!("{}/stocks/{}/quotes/latest", ALPACA_DATA_URL, symbol);
        let response = self.client.get(&url).headers(self.build_headers()).send().await?.error_for_status()?;

        Ok(response.json().await?)
    }

    /// Get option chain for a symbol (returns available strikes)
    pub async fn get_option_strikes(&self, symbol: &str, expiration: Option<&str>) -> AppResult<Value> {
        // Get snapshot data for the underlying to determine ATM strike
        let url = format!("{}/stocks/{}/quotes/latest", ALPACA_DATA_URL, symbol);
        let response = self.client.get(&url).headers(self.build_headers()).send().await?.error_for_status()?;

        let quote: Value = response.json().await?;

        // Get current stock price to determine ITM strikes
        // The quote object has 'ap' (ask price) and 'bp' (bid price)
        let quote_obj = quote.get("quote").unwrap_or(&quote);
        let ask_price = quote_obj.get("ap").and_then(|p| p.as_f64()).unwrap_or(0.0);
        let bid_price = quote_obj.get("bp").and_then(|p| p.as_f64()).unwrap_or(0.0);

        // Use ask price if available, otherwise bid price
        let current_price = if ask_price > 0.0 {
            ask_price
        } else if bid_price > 0.0 {
            bid_price
        } else {
            // If neither ask nor bid is available, check for 'price' or 'last' in case of different data format
            quote_obj.get("price").and_then(|p| p.as_f64()).unwrap_or(0.0)
        };

        // Determine strike increment based on price level
        let strike_increment = if current_price < 25.0 {
            0.5
        } else if current_price < 200.0 {
            1.0
        } else {
            5.0
        };

        // For ITM Call: closest strike below current price
        // floor(price/inc) * inc gives the strike at or below price
        // If exactly on a strike, go one below (strictly ITM)
        let strike_at_or_below = (current_price / strike_increment).floor() * strike_increment;
        let call_strike = if (strike_at_or_below - current_price).abs() < 0.001 {
            // Price is exactly on a strike, go one below for ITM
            strike_at_or_below - strike_increment
        } else {
            strike_at_or_below
        };

        // For ITM Put: closest strike above current price
        // ceil(price/inc) * inc gives the strike at or above price
        // If exactly on a strike, go one above (strictly ITM)
        let strike_at_or_above = (current_price / strike_increment).ceil() * strike_increment;
        let put_strike = if (strike_at_or_above - current_price).abs() < 0.001 {
            // Price is exactly on a strike, go one above for ITM
            strike_at_or_above + strike_increment
        } else {
            strike_at_or_above
        };

        // If expiration is provided, try to fetch options chain for that date
        // For now, return the basic strike data with expiration info
        let mut result = serde_json::json!({
            "underlying_price": current_price,
            "call_strike": call_strike,
            "put_strike": put_strike,
            "strike_increment": strike_increment
        });

        // Add expiration to result if provided
        if let Some(exp) = expiration {
            result["expiration"] = serde_json::json!(exp);
        }

        Ok(result)
    }

    /// Get real option chain for a symbol
    pub async fn get_option_chain(&self, symbol: &str) -> AppResult<OptionChainResponse> {
        // 1. Get current stock price
        let price_url = format!("{}/stocks/{}/quotes/latest", ALPACA_DATA_URL, symbol);
        let price_response =
            self.client.get(&price_url).headers(self.build_headers()).send().await?.error_for_status()?;

        let price_data: Value = price_response.json().await?;
        let quote_obj = price_data.get("quote").unwrap_or(&price_data);
        let ask_price = quote_obj.get("ap").and_then(|p| p.as_f64()).unwrap_or(0.0);
        let bid_price = quote_obj.get("bp").and_then(|p| p.as_f64()).unwrap_or(0.0);
        let underlying_price = if ask_price > 0.0 {
            ask_price
        } else if bid_price > 0.0 {
            bid_price
        } else {
            quote_obj.get("price").and_then(|p| p.as_f64()).unwrap_or(0.0)
        };

        // 2. Get option snapshots for the underlying
        // We fetch for both calls and puts to build the chain
        let mut strikes_map: std::collections::BTreeMap<String, StrikeData> = std::collections::BTreeMap::new();

        for option_type in &["call", "put"] {
            let url = format!(
                "{}/snapshots/{}?feed=indicative&type={}",
                ALPACA_OPTIONS_URL,
                symbol,
                option_type
            );

            let response = self.client.get(&url).headers(self.build_headers()).send().await?.error_for_status()?;

            let data: Value = response.json().await?;

            if let Some(snapshots) = data.get("snapshots").and_then(|s| s.as_object()) {
                for (occ_symbol, snapshot) in snapshots {
                    if let Some(quote) = snapshot.get("latestQuote") {
                        let bid = quote.get("bp").and_then(|p| p.as_f64()).unwrap_or(0.0);
                        let ask = quote.get("ap").and_then(|p| p.as_f64()).unwrap_or(0.0);
                        let size = quote.get("as").and_then(|p| p.as_i64()).unwrap_or(0);

                        // Parse strike from OCC symbol: SYMBOLYYMMDDC/PSTRIKE
                        // Strike is the last 8 digits
                        if occ_symbol.len() >= 8 {
                            let strike_part = &occ_symbol[occ_symbol.len() - 8..];
                            if let Ok(strike_val) = strike_part.parse::<f64>() {
                                let strike_price = strike_val / 1000.0;
                                let strike_key = format!("{:.3}", strike_price);

                                let entry = OptionEntry {
                                    symbol: occ_symbol.clone(),
                                    bid,
                                    ask,
                                    size,
                                };

                                let strike_data = strikes_map.entry(strike_key).or_insert_with(|| StrikeData {
                                    strike: strike_price,
                                    call: OptionEntry { symbol: "".into(), bid: 0.0, ask: 0.0, size: 0 },
                                    put: OptionEntry { symbol: "".into(), bid: 0.0, ask: 0.0, size: 0 },
                                });

                                if *option_type == "call" {
                                    strike_data.call = entry;
                                } else {
                                    strike_data.put = entry;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Filter strikes to be around the underlying price (blazing fast in Rust)
        // Let's take ±10% range around underlying price
        let min_range = underlying_price * 0.9;
        let max_range = underlying_price * 1.1;

        let filtered_strikes: Vec<StrikeData> = strikes_map
            .into_values()
            .filter(|s| s.strike >= min_range && s.strike <= max_range)
            .collect();

        Ok(OptionChainResponse {
            symbol: symbol.to_string(),
            underlying_price,
            strikes: filtered_strikes,
        })
    }

    /// Get current price for an option
    pub async fn get_option_price(&self, option_symbol: &str) -> AppResult<Value> {
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

        let response = self.client.get(&url).headers(self.build_headers()).send().await?.error_for_status()?;

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
        Self::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create default AlpacaClient: {}", e);
            Self {
                client: Client::new(),
                api_key: "".to_string(),
                api_secret: "".to_string(),
                base_url: ALPACA_PAPER_URL.to_string(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpaca_client_with_keys() {
        let result = AlpacaClient::with_keys("test_key", "test_secret");
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.api_key, "test_key");
        assert_eq!(client.api_secret, "test_secret");
    }

    #[test]
    fn test_alpaca_client_base_url_default_paper() {
        let client = AlpacaClient::with_keys("key", "secret").unwrap();
        assert_eq!(client.base_url, ALPACA_PAPER_URL);
    }

    #[test]
    fn test_alpaca_client_clone() {
        let client1 = AlpacaClient::with_keys("key123", "secret456").unwrap();
        let client2 = client1.clone();
        assert_eq!(client1.api_key, client2.api_key);
        assert_eq!(client1.api_secret, client2.api_secret);
        assert_eq!(client1.base_url, client2.base_url);
    }

    #[test]
    fn test_build_headers() {
        let client = AlpacaClient::with_keys("test_key_123", "test_secret_456").unwrap();
        let headers = client.build_headers();
        
        // Verify headers contain the API credentials
        assert!(headers.contains_key("APCA-API-KEY-ID"));
        assert!(headers.contains_key("APCA-API-SECRET-KEY"));
    }

    #[test]
    fn test_alpaca_urls_constants() {
        assert!(ALPACA_PAPER_URL.contains("paper-api"));
        assert!(ALPACA_LIVE_URL.contains("api.alpaca.markets"));
        assert!(ALPACA_DATA_URL.contains("data.alpaca.markets"));
        assert!(ALPACA_OPTIONS_URL.contains("options"));
    }

    #[test]
    fn test_order_request_fields() {
        let order = OrderRequest {
            symbol: "AAPL".to_string(),
            qty: 100.0,
            side: "buy".to_string(),
            order_type: "market".to_string(),
            time_in_force: "day".to_string(),
            limit_price: None,
            asset_class: None,
        };
        assert_eq!(order.symbol, "AAPL");
        assert_eq!(order.qty, 100.0);
        assert_eq!(order.side, "buy");
    }

    #[test]
    fn test_order_request_with_optional_fields() {
        let order = OrderRequest {
            symbol: "TSLA".to_string(),
            qty: 50.0,
            side: "sell".to_string(),
            order_type: "limit".to_string(),
            time_in_force: "gtc".to_string(),
            limit_price: Some(250.50),
            asset_class: Some("equity".to_string()),
        };
        assert_eq!(order.limit_price, Some(250.50));
        assert_eq!(order.asset_class, Some("equity".to_string()));
    }

    #[test]
    fn test_strike_increment_calculation_low_price() {
        let price = 20.0;
        let increment = if price < 25.0 { 0.5 } else if price < 200.0 { 1.0 } else { 5.0 };
        assert_eq!(increment, 0.5);
    }

    #[test]
    fn test_strike_increment_calculation_mid_price() {
        let price = 100.0;
        let increment = if price < 25.0 { 0.5 } else if price < 200.0 { 1.0 } else { 5.0 };
        assert_eq!(increment, 1.0);
    }

    #[test]
    fn test_strike_increment_calculation_high_price() {
        let price = 500.0;
        let increment = if price < 25.0 { 0.5 } else if price < 200.0 { 1.0 } else { 5.0 };
        assert_eq!(increment, 5.0);
    }

    #[test]
    fn test_strike_below_calculation() {
        let current_price: f64 = 100.5;
        let strike_increment: f64 = 1.0;
        let strike_at_or_below = (current_price / strike_increment).floor() * strike_increment;
        assert!((strike_at_or_below - 100.0).abs() < 0.0001);
    }

    #[test]
    fn test_strike_above_calculation() {
        let current_price: f64 = 100.3;
        let strike_increment: f64 = 1.0;
        let strike_at_or_above = (current_price / strike_increment).ceil() * strike_increment;
        assert!((strike_at_or_above - 101.0).abs() < 0.0001);
    }

    #[test]
    fn test_option_symbol_parsing() {
        let occ_symbol = "SPY260621C00500000";
        let underlying = if let Some(pos) = occ_symbol.find(|c: char| c.is_ascii_digit()) {
            occ_symbol[..pos].to_string()
        } else {
            "SPY".to_string()
        };
        assert_eq!(underlying, "SPY");
    }

    #[test]
    fn test_option_strike_from_occ() {
        let occ_symbol = "SPY260621C00500000";
        let strike_part = &occ_symbol[occ_symbol.len() - 8..];
        let strike_val: f64 = strike_part.parse().unwrap();
        let strike_price = strike_val / 1000.0;
        assert_eq!(strike_price, 500.0);
    }

    #[test]
    fn test_option_type_detection_call() {
        let occ_symbol = "SPY260621C00500000";
        // The actual logic: look for C to determine it's a call
        let is_call = occ_symbol.len() > 10 && occ_symbol.as_bytes()[9] as char == 'C';
        assert!(is_call, "Position 9 (after YYMMDD) should be 'C' for call");
    }

    #[test]
    fn test_option_type_detection_put() {
        let occ_symbol = "SPY260621P00500000";
        let is_put = occ_symbol.len() > 10 && occ_symbol.as_bytes()[9] as char == 'P';
        assert!(is_put, "Position 9 (after YYMMDD) should be 'P' for put");
    }

    #[test]
    fn test_price_range_filtering() {
        let underlying_price: f64 = 100.0;
        let min_range = underlying_price * 0.9;
        let max_range = underlying_price * 1.1;

        assert!((min_range - 90.0).abs() < 0.0001);
        assert!((max_range - 110.0).abs() < 0.0001);

        let strikes = vec![85.0, 90.0, 100.0, 110.0, 115.0];
        let filtered: Vec<_> = strikes.iter()
            .filter(|s| **s >= min_range && **s <= max_range)
            .collect();
        
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0], &90.0);
        assert_eq!(filtered[1], &100.0);
        assert_eq!(filtered[2], &110.0);
    }
}