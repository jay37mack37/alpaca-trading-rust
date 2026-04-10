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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_serialization() {
        let position = Position {
            asset_id: "asset-123".to_string(),
            symbol: "AAPL".to_string(),
            qty: "10".to_string(),
            avg_entry_price: "150.00".to_string(),
            market_value: "1550.00".to_string(),
            current_price: "155.00".to_string(),
            unrealized_pl: "50.00".to_string(),
            unrealized_plpc: "0.0333".to_string(),
        };
        let json = serde_json::to_string(&position).unwrap();
        assert!(json.contains("AAPL"));
        assert!(json.contains("asset-123"));
        assert!(json.contains("150.00"));
    }

    #[test]
    fn test_position_deserialization() {
        let json = r#"{
            "asset_id": "asset-456",
            "symbol": "TSLA",
            "qty": "5",
            "avg_entry_price": "800.00",
            "market_value": "4100.00",
            "current_price": "820.00",
            "unrealized_pl": "100.00",
            "unrealized_plpc": "0.025"
        }"#;
        let position: Position = serde_json::from_str(json).unwrap();
        assert_eq!(position.asset_id, "asset-456");
        assert_eq!(position.symbol, "TSLA");
        assert_eq!(position.qty, "5");
        assert_eq!(position.avg_entry_price, "800.00");
    }

    #[test]
    fn test_position_debug() {
        let position = Position {
            asset_id: "test-id".to_string(),
            symbol: "GOOGL".to_string(),
            qty: "1".to_string(),
            avg_entry_price: "100".to_string(),
            market_value: "100".to_string(),
            current_price: "100".to_string(),
            unrealized_pl: "0".to_string(),
            unrealized_plpc: "0".to_string(),
        };
        let debug_str = format!("{:?}", position);
        assert!(debug_str.contains("Position"));
        assert!(debug_str.contains("GOOGL"));
    }

    #[test]
    fn test_position_with_negative_pl() {
        let position = Position {
            asset_id: "asset-789".to_string(),
            symbol: "MSFT".to_string(),
            qty: "20".to_string(),
            avg_entry_price: "300.00".to_string(),
            market_value: "5800.00".to_string(),
            current_price: "290.00".to_string(),
            unrealized_pl: "-200.00".to_string(),
            unrealized_plpc: "-0.0333".to_string(),
        };
        let json = serde_json::to_string(&position).unwrap();
        assert!(json.contains("-200.00"));
        assert!(json.contains("-0.0333"));
    }
}
