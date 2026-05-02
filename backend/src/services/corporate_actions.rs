use crate::error::AppResult;
use crate::models::StoredCredential;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DividendInfo {
    pub symbol: String,
    pub ex_dividend_date: DateTime<Utc>,
    pub amount: f64,
}

pub struct CorporateActionsService;

impl CorporateActionsService {
    /// Primarily fetches dividend info from Yahoo Finance
    pub async fn fetch_dividend_info(
        client: &Client,
        symbol: &str,
    ) -> AppResult<Option<DividendInfo>> {
        let url = format!(
            "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{}?modules=summaryDetail",
            symbol
        );
        let response = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let json: serde_json::Value = response.json().await?;
        let summary = &json["quoteSummary"]["result"][0]["summaryDetail"];

        let ex_date_raw = summary["exDividendDate"]["raw"].as_i64();
        let amount = summary["dividendRate"]["raw"].as_f64();

        if let (Some(ts), Some(amt)) = (ex_date_raw, amount) {
            if amt > 0.0 {
                return Ok(Some(DividendInfo {
                    symbol: symbol.to_string(),
                    ex_dividend_date: Utc.timestamp_opt(ts, 0).single().unwrap_or(Utc::now()),
                    amount: amt,
                }));
            }
        }

        Ok(None)
    }

    /// Backup fetcher using Alpaca's Corporate Actions API
    pub async fn fetch_alpaca_dividends(
        client: &Client,
        symbol: &str,
        credential: &StoredCredential,
    ) -> AppResult<Vec<DividendInfo>> {
        // Alpaca Corporate Actions endpoint (v2/corporate-actions)
        let url = format!(
            "{}/v2/corporate-actions?symbols={}",
            credential.environment.base_trading_url(),
            symbol
        );
        let response = client
            .get(url)
            .header("APCA-API-KEY-ID", credential.key_id.as_str())
            .header("APCA-API-SECRET-KEY", credential.secret_key.as_str())
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value = response.json().await?;
        let mut dividends = Vec::new();

        if let Some(actions) = json.as_array() {
            for action in actions {
                if action["ca_type"].as_str() == Some("dividend") {
                    if let Some(ex_date_str) = action["ex_date"].as_str() {
                        // Assuming format YYYY-MM-DD
                        if let Ok(ex_date) =
                            DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", ex_date_str))
                        {
                            dividends.push(DividendInfo {
                                symbol: symbol.to_string(),
                                ex_dividend_date: ex_date.with_timezone(&Utc),
                                amount: action["cash_rate"].as_f64().unwrap_or(0.0),
                            });
                        }
                    }
                }
            }
        }

        Ok(dividends)
    }
}
