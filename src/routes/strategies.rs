use axum::{extract::State, http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::AppResult;
use crate::routes::auth::get_username_from_headers;
use crate::routes::websocket::AppState;
use crate::strategies::listing_arbitrage::ListingArbitrage;
use crate::strategies::vwap_reversion::VwapReversion;
use crate::strategies::delta_neutral::DeltaNeutral;
use crate::strategies::gamma_scalping::GammaScalping;
use crate::strategies::put_call_parity::PutCallParity;
use crate::strategies::Strategy;

#[derive(Deserialize)]
pub struct StrategyRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct StrategyInfo {
    pub name: String,
    pub status: String,
}

pub async fn start_strategy(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<StrategyRequest>,
) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let strategy: Arc<dyn Strategy> = match body.name.as_str() {
        "Listing Arbitrage" => Arc::new(ListingArbitrage::new()),
        "VWAP Mean Reversion" => Arc::new(VwapReversion::new()),
        "0DTE Delta-Neutral" => Arc::new(DeltaNeutral::new()),
        "Gamma Scalping" => Arc::new(GammaScalping::new()),
        "Put-Call Parity" => Arc::new(PutCallParity::new()),
        _ => return Ok(Json(json!({"error": "Unknown strategy"}))),
    };

    state.strategy_manager.start_strategy(strategy).await;

    Ok(Json(json!({"status": "started", "name": body.name})))
}

pub async fn stop_strategy(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(body): Json<StrategyRequest>,
) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    state.strategy_manager.stop_strategy(&body.name).await;

    Ok(Json(json!({"status": "stopped", "name": body.name})))
}

pub async fn get_strategy_status(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let running = state.strategy_manager.get_running_strategies().await;

    let strategies = vec![
        "Listing Arbitrage",
        "VWAP Mean Reversion",
        "0DTE Delta-Neutral",
        "Gamma Scalping",
        "Put-Call Parity",
    ];

    let info: Vec<StrategyInfo> = strategies
        .into_iter()
        .map(|name| StrategyInfo {
            name: name.to_string(),
            status: if running.contains(&name.to_string()) {
                "Running".to_string()
            } else {
                "Idle".to_string()
            },
        })
        .collect();

    Ok(Json(json!({ "strategies": info })))
}

pub async fn panic_button(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    // 1. Stop all strategies
    state.strategy_manager.stop_all().await;

    // 2. Liquidate all paper positions (if Alpaca is configured)
    if let Some(alpaca) = &state.alpaca {
        match alpaca.cancel_all_orders().await {
            Ok(_) => tracing::info!("Global Panic: All orders cancelled"),
            Err(e) => tracing::error!("Global Panic: Failed to cancel orders: {}", e),
        }

        // Note: Real liquidation of positions requires closing each one
        // For now we log it as a safety measure.
        tracing::warn!("Global Panic: Strategies halted and orders cancelled.");
    }

    Ok(Json(json!({"status": "panic_executed", "message": "All strategies stopped and orders cancelled."})))
}
