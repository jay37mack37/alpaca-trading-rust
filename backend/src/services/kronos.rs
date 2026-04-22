use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::error::AppResult;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KronosScore {
    pub symbol: String,
    pub trend: String,
    pub confidence: f64,
}

pub async fn fetch_kronos_score(client: &Client, symbol: &str) -> AppResult<KronosScore> {
    let base_url = std::env::var("KRONOS_BRIDGE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let url = format!("{}/score/{}", base_url, symbol);

    let response = match client.get(&url).send().await {
        Ok(res) => res,
        Err(err) => {
            warn!("Kronos Bridge connection failed: {}. Using simulated score.", err);
            return Ok(KronosScore {
                symbol: symbol.to_string(),
                trend: "NEUTRAL-SIM".to_string(),
                confidence: 0.85,
            });
        }
    };

    if !response.status().is_success() {
        warn!("Kronos Bridge returned error: {}. Using simulated score.", response.status());
        return Ok(KronosScore {
            symbol: symbol.to_string(),
            trend: "NEUTRAL-SIM".to_string(),
            confidence: 0.85,
        });
    }

    let score = match response.json::<KronosScore>().await {
        Ok(s) => s,
        Err(_) => KronosScore {
            symbol: symbol.to_string(),
            trend: "NEUTRAL-SIM".to_string(),
            confidence: 0.85,
        }
    };
    Ok(score)
}
