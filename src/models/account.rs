use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub account_number: String,
    pub status: String,
    pub currency: String,
    pub buying_power: String,
    pub cash: String,
    pub portfolio_value: String,
    pub equity: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_serialization() {
        let account = Account {
            id: "acc-123".to_string(),
            account_number: "123456789".to_string(),
            status: "ACTIVE".to_string(),
            currency: "USD".to_string(),
            buying_power: "100000.00".to_string(),
            cash: "50000.00".to_string(),
            portfolio_value: "105000.00".to_string(),
            equity: "105000.00".to_string(),
        };
        let json = serde_json::to_string(&account).unwrap();
        assert!(json.contains("acc-123"));
        assert!(json.contains("ACTIVE"));
        assert!(json.contains("USD"));
    }

    #[test]
    fn test_account_deserialization() {
        let json = r#"{ 
            "id": "acc-456",
            "account_number": "987654321",
            "status": "INACTIVE",
            "currency": "EUR",
            "buying_power": "50000.00",
            "cash": "25000.00",
            "portfolio_value": "75000.00",
            "equity": "75000.00"
        }"#;
        let account: Account = serde_json::from_str(json).unwrap();
        assert_eq!(account.id, "acc-456");
        assert_eq!(account.account_number, "987654321");
        assert_eq!(account.status, "INACTIVE");
        assert_eq!(account.currency, "EUR");
    }

    #[test]
    fn test_account_debug() {
        let account = Account {
            id: "test".to_string(),
            account_number: "123".to_string(),
            status: "ACTIVE".to_string(),
            currency: "USD".to_string(),
            buying_power: "0".to_string(),
            cash: "0".to_string(),
            portfolio_value: "0".to_string(),
            equity: "0".to_string(),
        };
        let debug_str = format!("{:?}", account);
        assert!(debug_str.contains("Account"));
        assert!(debug_str.contains("test"));
    }
}