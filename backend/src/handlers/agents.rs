use crate::agents::{abort_agent_loop, run_strategy_once, spawn_agent_loop};
use crate::error::{ApiResponse, AppError, AppResult};
use crate::models::{
    CreateStrategyRequest, ExecutionMode, StrategyDetailResponse, StrategySummary, TradeRecord,
    UpdateStrategyRequest,
};
use crate::services::broker::resolve_alpaca_credential;
use crate::services::db::Database;
use crate::services::providers::cancel_all_alpaca_orders;
use crate::AppState;
use axum::{
    extract::{Path, State},
    Json,
};

pub async fn list_strategies(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<StrategySummary>>> {
    let db = state.db.lock().await;
    let db: &Database = &db;
    Ok(ApiResponse {
        success: true,
        data: Some(db.list_strategies()?),
        error: None,
    })
}

pub async fn create_strategy(
    State(state): State<AppState>,
    Json(request): Json<CreateStrategyRequest>,
) -> AppResult<ApiResponse<StrategySummary>> {
    // If execution mode is AlpacaLive, require confirmation phrase
    if let Some(mode) = request.execution_mode {
        if mode == ExecutionMode::AlpacaLive {
            let confirmed = request.live_confirmation.as_deref().unwrap_or("");
            if confirmed != "TRADE REAL MONEY" {
                return Err(AppError::Validation(
                    "Live confirmation phrase must be 'TRADE REAL MONEY'".to_string(),
                ));
            }
        }
    }

    let created = {
        let db = state.db.lock().await;
        let db: &Database = &db;
        db.insert_strategy(request)?
    };
    if created.enabled {
        spawn_agent_loop(state.clone(), created.id.clone()).await;
    }
    Ok(ApiResponse {
        success: true,
        data: Some(created),
        error: None,
    })
}

pub async fn strategy_detail(
    State(state): State<AppState>,
    Path(strategy_id): Path<String>,
) -> AppResult<ApiResponse<StrategyDetailResponse>> {
    let db = state.db.lock().await;
    let db: &Database = &db;
    Ok(ApiResponse {
        success: true,
        data: Some(db.strategy_detail(&strategy_id)?),
        error: None,
    })
}

pub async fn update_strategy(
    State(state): State<AppState>,
    Path(strategy_id): Path<String>,
    Json(request): Json<UpdateStrategyRequest>,
) -> AppResult<ApiResponse<StrategySummary>> {
    if matches!(request.execution_mode, Some(ExecutionMode::AlpacaLive))
        && request.live_confirmation.as_deref() != Some("TRADE REAL MONEY")
    {
        return Err(crate::error::AppError::Validation(
            "live trading requires the confirmation phrase TRADE REAL MONEY".to_string(),
        ));
    }

    let updated = {
        let db = state.db.lock().await;
        let db: &Database = &db;
        db.update_strategy(&strategy_id, request)?
    };

    abort_agent_loop(&state, &strategy_id).await;
    if updated.enabled {
        spawn_agent_loop(state.clone(), strategy_id).await;
    }

    Ok(ApiResponse {
        success: true,
        data: Some(updated),
        error: None,
    })
}

pub async fn run_strategy(
    State(state): State<AppState>,
    Path(strategy_id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<crate::RunQuery>,
) -> AppResult<ApiResponse<Option<TradeRecord>>> {
    let symbol_override = query
        .symbol
        .map(|value| crate::models::normalize_symbol(&value));
    let trade = run_strategy_once(&state, &strategy_id, symbol_override.as_deref()).await?;
    Ok(ApiResponse {
        success: true,
        data: Some(trade),
        error: None,
    })
}

pub async fn start_strategy(
    State(state): State<AppState>,
    Path(strategy_id): Path<String>,
) -> AppResult<ApiResponse<StrategySummary>> {
    let updated = {
        let db = state.db.lock().await;
        db.set_strategy_enabled(&strategy_id, true)?
    };
    spawn_agent_loop(state, strategy_id).await;
    Ok(ApiResponse {
        success: true,
        data: Some(updated),
        error: None,
    })
}

pub async fn stop_strategy(
    State(state): State<AppState>,
    Path(strategy_id): Path<String>,
) -> AppResult<ApiResponse<StrategySummary>> {
    abort_agent_loop(&state, &strategy_id).await;
    let updated = {
        let db = state.db.lock().await;
        db.set_strategy_enabled(&strategy_id, false)?
    };
    Ok(ApiResponse {
        success: true,
        data: Some(updated),
        error: None,
    })
}

pub async fn panic_all(State(state): State<AppState>) -> ApiResponse<()> {
    let ids: Vec<String> = {
        let tasks = state.agent_tasks.lock().await;
        tasks.keys().cloned().collect()
    };
    for id in &ids {
        abort_agent_loop(&state, id).await;
    }
    // mark all enabled strategies as disabled in the DB
    {
        let db = state.db.lock().await;
        if let Ok(strategies) = db.list_strategy_records() {
            for s in strategies.into_iter().filter(|s| s.enabled) {
                let _ = db.set_strategy_enabled(&s.id, false);
            }
        }
    }
    // cancel all open orders on Alpaca
    if let Ok(Some(credential)) = resolve_alpaca_credential(&state, None, true).await {
        let _ = cancel_all_alpaca_orders(&state.http, &credential).await;
    }
    ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    }
}
