use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub qty: f64,
    pub side: String,          // "buy" or "sell"
    pub order_type: String,    // "market", "limit", etc.
    pub time_in_force: String, // "day", "gtc", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<String>,
}

impl OrderRequest {
    const VALID_TYPES: &'static [&'static str] = &["market", "limit", "stop", "stop_limit", "trailing_stop"];
    const VALID_TIFS: &'static [&'static str] = &["day", "gtc", "opg", "cls", "ioc", "fok"];

    pub fn validate(&self) -> Result<(), String> {
        if self.symbol.is_empty() {
            return Err("Symbol is required".to_string());
        }

        if self.qty <= 0.0 {
            return Err("Quantity must be greater than zero".to_string());
        }

        let side = self.side.to_lowercase();
        let is_option_order = self.asset_class.as_deref() == Some("us_option");
        let valid_sides: &[&str] = if is_option_order {
            &[
                "buy_to_open",
                "sell_to_open",
                "buy_to_close",
                "sell_to_close",
            ]
        } else {
            &["buy", "sell"]
        };
        if !valid_sides.contains(&side.as_str()) {
            if is_option_order {
                return Err(
                    "Option side must be one of: buy_to_open, sell_to_open, buy_to_close, sell_to_close"
                        .to_string(),
                );
            }
            return Err("Side must be 'buy' or 'sell'".to_string());
        }

        let order_type = self.order_type.to_lowercase();
        if !Self::VALID_TYPES.contains(&order_type.as_str()) {
            return Err(format!("Invalid order type. Must be one of: {:?}", Self::VALID_TYPES));
        }

        let tif = self.time_in_force.to_lowercase();
        if !Self::VALID_TIFS.contains(&tif.as_str()) {
            return Err(format!("Invalid time in force. Must be one of: {:?}", Self::VALID_TIFS));
        }

        if order_type == "limit" && self.limit_price.is_none() {
            return Err("Limit price is required for limit orders".to_string());
        }

        if let Some(price) = self.limit_price {
            if price <= 0.0 {
                return Err("Limit price must be greater than zero".to_string());
            }
        }

        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_request_serialization() {
        let order = OrderRequest {
            symbol: "AAPL".to_string(),
            qty: 10.0,
            side: "buy".to_string(),
            order_type: "market".to_string(),
            time_in_force: "gtc".to_string(),
            limit_price: None,
            asset_class: None,
        };

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("AAPL"));
        assert!(json.contains("\"qty\":10"));
        assert!(!json.contains("limit_price"));
    }

    #[test]
    fn test_order_request_with_limit_price() {
        let order = OrderRequest {
            symbol: "TSLA".to_string(),
            qty: 5.0,
            side: "buy".to_string(),
            order_type: "limit".to_string(),
            time_in_force: "day".to_string(),
            limit_price: Some(150.50),
            asset_class: None,
        };

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("limit_price"));
        assert!(json.contains("150.5"));
    }

    #[test]
    fn test_order_request_validation() {
        let mut order = OrderRequest {
            symbol: "".to_string(),
            qty: 10.0,
            side: "buy".to_string(),
            order_type: "market".to_string(),
            time_in_force: "gtc".to_string(),
            limit_price: None,
            asset_class: None,
        };
        assert!(order.validate().is_err(), "Empty symbol should fail");

        order.symbol = "AAPL".to_string();
        order.qty = -1.0;
        assert!(order.validate().is_err(), "Negative quantity should fail");

        order.qty = 10.0;
        order.side = "invalid".to_string();
        assert!(order.validate().is_err(), "Invalid side should fail");

        order.side = "buy".to_string();
        order.order_type = "invalid".to_string();
        assert!(order.validate().is_err(), "Invalid order type should fail");

        order.order_type = "limit".to_string();
        assert!(order.validate().is_err(), "Limit order without price should fail");

        order.limit_price = Some(150.0);
        assert!(order.validate().is_ok(), "Valid limit order should pass");
    }

    #[test]
    fn test_order_deserialization() {
        let json = r#"{
            "id": "test-123",
            "client_order_id": "client-456",
            "symbol": "GOOGL",
            "qty": "20",
            "side": "sell",
            "order_type": "limit",
            "time_in_force": "gtc",
            "status": "filled"
        }"#;

        let order: Order = serde_json::from_str(json).unwrap();
        assert_eq!(order.id, "test-123");
        assert_eq!(order.symbol, "GOOGL");
        assert_eq!(order.status, "filled");
    }
}
