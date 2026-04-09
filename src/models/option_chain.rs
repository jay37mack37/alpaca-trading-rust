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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_entry_serialization() {
        let entry = OptionEntry {
            symbol: "SPY240621C00500000".to_string(),
            bid: 5.20,
            ask: 5.30,
            size: 10,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("SPY240621C00500000"));
        assert!(json.contains("5.2"));
        assert!(json.contains("5.3"));
    }

    #[test]
    fn test_option_entry_deserialization() {
        let json = r#"{
            "symbol": "AAPL240621P00180000",
            "bid": 2.50,
            "ask": 2.60,
            "size": 25
        }"#;
        let entry: OptionEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.symbol, "AAPL240621P00180000");
        assert_eq!(entry.bid, 2.50);
        assert_eq!(entry.ask, 2.60);
        assert_eq!(entry.size, 25);
    }

    #[test]
    fn test_strike_data_serialization() {
        let strike = StrikeData {
            strike: 500.0,
            call: OptionEntry {
                symbol: "SPY240621C00500000".to_string(),
                bid: 5.0,
                ask: 5.1,
                size: 5,
            },
            put: OptionEntry {
                symbol: "SPY240621P00500000".to_string(),
                bid: 5.2,
                ask: 5.3,
                size: 5,
            },
        };
        let json = serde_json::to_string(&strike).unwrap();
        assert!(json.contains("500.0"));
        assert!(json.contains("call"));
        assert!(json.contains("put"));
    }

    #[test]
    fn test_strike_data_deserialization() {
        let json = r#"{
            "strike": 480.5,
            "call": {
                "symbol": "TEST240621C00480500",
                "bid": 10.0,
                "ask": 10.5,
                "size": 100
            },
            "put": {
                "symbol": "TEST240621P00480500",
                "bid": 1.0,
                "ask": 1.1,
                "size": 100
            }
        }"#;
        let strike: StrikeData = serde_json::from_str(json).unwrap();
        assert_eq!(strike.strike, 480.5);
        assert_eq!(strike.call.symbol, "TEST240621C00480500");
        assert_eq!(strike.put.symbol, "TEST240621P00480500");
    }

    #[test]
    fn test_option_chain_response_serialization() {
        let response = OptionChainResponse {
            symbol: "SPY".to_string(),
            underlying_price: 500.0,
            strikes: vec![
                StrikeData {
                    strike: 500.0,
                    call: OptionEntry {
                        symbol: "SPY240621C00500000".to_string(),
                        bid: 5.0,
                        ask: 5.1,
                        size: 5,
                    },
                    put: OptionEntry {
                        symbol: "SPY240621P00500000".to_string(),
                        bid: 5.2,
                        ask: 5.3,
                        size: 5,
                    },
                },
            ],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("SPY"));
        assert!(json.contains("500.0"));
        assert!(json.contains("strikes"));
    }

    #[test]
    fn test_option_chain_response_deserialization() {
        let json = r#"{
            "symbol": "AAPL",
            "underlying_price": 180.50,
            "strikes": [
                {
                    "strike": 180.0,
                    "call": {
                        "symbol": "AAPL240621C00180000",
                        "bid": 3.0,
                        "ask": 3.1,
                        "size": 50
                    },
                    "put": {
                        "symbol": "AAPL240621P00180000",
                        "bid": 2.5,
                        "ask": 2.6,
                        "size": 50
                    }
                },
                {
                    "strike": 185.0,
                    "call": {
                        "symbol": "AAPL240621C00185000",
                        "bid": 1.0,
                        "ask": 1.1,
                        "size": 50
                    },
                    "put": {
                        "symbol": "AAPL240621P00185000",
                        "bid": 5.0,
                        "ask": 5.1,
                        "size": 50
                    }
                }
            ]
        }"#;
        let response: OptionChainResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.symbol, "AAPL");
        assert_eq!(response.underlying_price, 180.50);
        assert_eq!(response.strikes.len(), 2);
        assert_eq!(response.strikes[0].strike, 180.0);
        assert_eq!(response.strikes[1].strike, 185.0);
    }

    #[test]
    fn test_option_chain_empty_strikes() {
        let response = OptionChainResponse {
            symbol: "TEST".to_string(),
            underlying_price: 100.0,
            strikes: vec![],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TEST"));
        assert!(json.contains("\"strikes\":[]"));
    }

    #[test]
    fn test_option_entry_zero_values() {
        let entry = OptionEntry {
            symbol: "TEST".to_string(),
            bid: 0.0,
            ask: 0.0,
            size: 0,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: OptionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.bid, 0.0);
        assert_eq!(deserialized.ask, 0.0);
        assert_eq!(deserialized.size, 0);
    }
}
