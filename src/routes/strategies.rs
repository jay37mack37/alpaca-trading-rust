use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::routes::websocket::AppState;
use crate::strategies::Strategy;

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

/// GET /api/strategies/status - Get current status of all strategies
pub async fn get_strategies_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let strategies = state.strategy_manager.get_all_strategies().await;
    Json(StrategiesListResponse {
        success: true,
        strategies,
    })
}

/// GET /api/strategies - List all strategies with their current states
pub async fn list_strategies(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let strategies = state.strategy_manager.get_all_strategies().await;
    Json(StrategiesListResponse {
        success: true,
        strategies,
    })
}

/// POST /api/strategies/{id}/start - Start a specific strategy
pub async fn start_strategy(
    Path(strategy_id): Path<u32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.strategy_manager.start_strategy(strategy_id).await {
        Ok(_) => Json(StrategyResponse {
            success: true,
            message: format!("Strategy {} started successfully", strategy_id),
            data: None,
        }),
        Err(e) => Json(StrategyResponse {
            success: false,
            message: e,
            data: None,
        }),
    }
}

/// POST /api/strategies/{id}/stop - Stop a specific strategy
pub async fn stop_strategy(
    Path(strategy_id): Path<u32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.strategy_manager.stop_strategy(strategy_id).await {
        Ok(_) => Json(StrategyResponse {
            success: true,
            message: format!("Strategy {} stopped successfully", strategy_id),
            data: None,
        }),
        Err(e) => Json(StrategyResponse {
            success: false,
            message: e,
            data: None,
        }),
    }
}

/// GET /api/strategies/logs - Get the last 20 strategy logs
pub async fn get_strategy_logs() -> impl IntoResponse {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let mut logs = Vec::new();
    if let Ok(file) = File::open("strategies.log") {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(l) = line {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&l) {
                    logs.push(json);
                }
            }
        }
    }
    
    let len = logs.len();
    let start = if len > 20 { len - 20 } else { 0 };
    let last_20 = &logs[start..];
    
    Json(serde_json::json!({
        "success": true,
        "logs": last_20
    }))
}

/// POST /api/strategies/logs - Add a log entry from the frontend
pub async fn append_strategy_log(Json(entry): Json<serde_json::Value>) -> impl IntoResponse {
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("strategies.log") {
        let _ = writeln!(file, "{}", entry.to_string());
    }
    Json(serde_json::json!({"success": true}))
}

/// POST /api/strategies/stop-all - Stop all running strategies (PANIC button)
pub async fn stop_all_strategies(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let results = state.strategy_manager.stop_all_strategies().await;
    let stopped_count = results.iter().filter(|(_, status)| status == "stopped").count();
    Json(StrategyResponse {
        success: true,
        message: format!("Stopped {} strategies", stopped_count),
        data: None,
    })
}
