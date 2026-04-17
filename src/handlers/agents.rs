use axum::{
    extract::{Path, State},
    Json,
};
use crate::models::{
    StrategySummary, CreateStrategyRequest, UpdateStrategyRequest,
    StrategyDetailResponse, TradeRecord, ExecutionMode,
};
use crate::error::{AppResult, ApiResponse};
use crate::{AppState, spawn_agent_loop, abort_agent_loop, run_strategy_once};
use crate::services::db::Database;

pub async fn list_strategies(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<StrategySummary>>> {
    let db = state.db.lock().await;
    let db: &Database = &*db;
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
    let created = {
        let db = state.db.lock().await;
        let db: &Database = &*db;
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
    let db: &Database = &*db;
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
        let db: &Database = &*db;
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
    let symbol_override = query.symbol.map(|value| crate::models::normalize_symbol(&value));
    let trade = run_strategy_once(&state, &strategy_id, symbol_override.as_deref()).await?;
    Ok(ApiResponse {
        success: true,
        data: Some(trade),
        error: None,
    })
}
