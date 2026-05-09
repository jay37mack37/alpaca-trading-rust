use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::error::{AppError, AppResult};
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
            warn!("Kronos Bridge OFFLINE: {}. Real intelligence required for execution.", err);
            return Err(AppError::External(format!("Kronos Bridge disconnected: {}", err)));
        }
    };

    if !response.status().is_success() {
        warn!("Kronos Bridge error status: {}. Blocking trade.", response.status());
        return Err(AppError::External(format!("Kronos Bridge returned error: {}", response.status())));
    }

    let score = response.json::<KronosScore>().await
        .map_err(|e| AppError::External(format!("Failed to parse Kronos score: {}", e)))?;
    
    Ok(score)
}
