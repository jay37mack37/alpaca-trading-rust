use crate::error::AppResult;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KronosScore {
    pub symbol: String,
    pub trend: String,
    pub confidence: f64,
}

pub async fn fetch_kronos_score(client: &Client, symbol: &str) -> AppResult<KronosScore> {
    let base_url =
        std::env::var("KRONOS_BRIDGE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let url = format!("{}/score/{}", base_url, symbol);

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(crate::error::AppError::External(format!(
            "Kronos Bridge returned error: {}",
            response.status()
        )));
    }

    let score = response.json::<KronosScore>().await?;
    Ok(score)
}
