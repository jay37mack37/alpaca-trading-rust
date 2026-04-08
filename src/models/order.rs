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
        };

        let json = serde_json::to_string(&order).unwrap();
        assert!(json.contains("limit_price"));
        assert!(json.contains("150.5"));
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