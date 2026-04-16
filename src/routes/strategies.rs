use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::routes::websocket::AppState;
use crate::strategies::{Strategy, StrategyState};

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<StrategyData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyData {
    pub strategy: Strategy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategiesListResponse {
    pub success: bool,
    pub strategies: Vec<Strategy>,
}

/// GET /api/strategies - List all strategies with their current states
pub async fn list_strategies(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // For now, return empty since we don't have a global StrategyManager yet
    // This will be enhanced when we integrate with the app state
    
    let strategies = vec![
        Strategy {
            id: 1,
            name: "Listing Arbitrage".to_string(),
            description: "Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering.".to_string(),
            state: StrategyState::Idle,
        },
        Strategy {
            id: 2,
            name: "VWAP Mean Reversion".to_string(),
            description: "Automated entries on standard deviation price extensions from the VWAP.".to_string(),
            state: StrategyState::Idle,
        },
        Strategy {
            id: 3,
            name: "0DTE Delta-Neutral".to_string(),
            description: "Harvests theta decay on same-day expiry options via automated spreads.".to_string(),
            state: StrategyState::Idle,
        },
        Strategy {
            id: 4,
            name: "Gamma Scalping".to_string(),
            description: "Dynamic delta hedging to profit from realized volatility.".to_string(),
            state: StrategyState::Idle,
        },
        Strategy {
            id: 5,
            name: "Put-Call Parity".to_string(),
            description: "Arbitrages discrepancies between synthesized and market option prices.".to_string(),
            state: StrategyState::Idle,
        },
    ];

    Json(StrategiesListResponse {
        success: true,
        strategies,
    })
}

/// POST /api/strategies/:id/start - Start a strategy
pub async fn start_strategy(
    Path(id): Path<u32>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // TODO: Integrate with global StrategyManager when implemented
    tracing::info!("Received request to start strategy {}", id);

    // Validate strategy ID
    if id < 1 || id > 5 {
        return (
            StatusCode::NOT_FOUND,
            Json(StrategyResponse {
                success: false,
                message: format!("Strategy {} not found", id),
                data: None,
            }),
        );
    }

    // For now, just return success
    // When StrategyManager is integrated into AppState, this will actually spawn the task
    (
        StatusCode::OK,
        Json(StrategyResponse {
            success: true,
            message: format!("Strategy {} started", id),
            data: Some(StrategyData {
                strategy: create_strategy(id),
            }),
        }),
    )
}

/// POST /api/strategies/:id/stop - Stop a strategy
pub async fn stop_strategy(
    Path(id): Path<u32>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // TODO: Integrate with global StrategyManager when implemented
    tracing::info!("Received request to stop strategy {}", id);

    // Validate strategy ID
    if id < 1 || id > 5 {
        return (
            StatusCode::NOT_FOUND,
            Json(StrategyResponse {
                success: false,
                message: format!("Strategy {} not found", id),
                data: None,
            }),
        );
    }

    // For now, just return success
    // When StrategyManager is integrated into AppState, this will actually abort the task
    (
        StatusCode::OK,
        Json(StrategyResponse {
            success: true,
            message: format!("Strategy {} stopped", id),
            data: Some(StrategyData {
                strategy: create_strategy_with_state(id, StrategyState::Idle),
            }),
        }),
    )
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

fn create_strategy(id: u32) -> Strategy {
    let strategies = vec![
        ("Listing Arbitrage", "Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering."),
        ("VWAP Mean Reversion", "Automated entries on standard deviation price extensions from the VWAP."),
        ("0DTE Delta-Neutral", "Harvests theta decay on same-day expiry options via automated spreads."),
        ("Gamma Scalping", "Dynamic delta hedging to profit from realized volatility."),
        ("Put-Call Parity", "Arbitrages discrepancies between synthesized and market option prices."),
    ];

    let (name, desc) = if id as usize <= strategies.len() {
        strategies[id as usize - 1]
    } else {
        ("Unknown", "")
    };

    Strategy {
        id,
        name: name.to_string(),
        description: desc.to_string(),
        state: StrategyState::Running,
    }
}

fn create_strategy_with_state(id: u32, state: StrategyState) -> Strategy {
    let strategies = vec![
        ("Listing Arbitrage", "Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering."),
        ("VWAP Mean Reversion", "Automated entries on standard deviation price extensions from the VWAP."),
        ("0DTE Delta-Neutral", "Harvests theta decay on same-day expiry options via automated spreads."),
        ("Gamma Scalping", "Dynamic delta hedging to profit from realized volatility."),
        ("Put-Call Parity", "Arbitrages discrepancies between synthesized and market option prices."),
    ];

    let (name, desc) = if id as usize <= strategies.len() {
        strategies[id as usize - 1]
    } else {
        ("Unknown", "")
    };

    Strategy {
        id,
        name: name.to_string(),
        description: desc.to_string(),
        state,
    }
}
