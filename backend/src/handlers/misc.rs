use crate::error::{ApiResponse, AppResult};
use crate::models::{BrokerSyncState, CollectResponse, HealthResponse};
use crate::services::broker::sync_strategy_broker_state;
use crate::services::market::collect_once;
use crate::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use tracing::info;

pub async fn health() -> ApiResponse<HealthResponse> {
    ApiResponse {
        success: true,
        data: Some(HealthResponse {
            status: "ok",
            now: Utc::now().to_rfc3339(),
        }),
        error: None,
    }
}

#[derive(serde::Deserialize)]
pub struct RobinhoodIngestPayload {
    #[allow(dead_code)]
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    #[allow(dead_code)]
    pub timestamp: Option<u64>,
    pub payload: Option<serde_json::Value>,
}

pub async fn ingest_robinhood_data(
    State(_state): State<AppState>,
    Json(payload): Json<RobinhoodIngestPayload>,
) -> ApiResponse<serde_json::Value> {
    info!(
        "Received Robinhood data [{}]: {:?}",
        payload.event_type.as_deref().unwrap_or("unknown"),
        payload.payload
    );

    ApiResponse {
        success: true,
        data: Some(serde_json::json!({ "status": "accepted" })),
        error: None,
    }
}

pub async fn collect_now(State(state): State<AppState>) -> AppResult<ApiResponse<CollectResponse>> {
    let summary = collect_once(&state).await?;
    Ok(ApiResponse {
        success: true,
        data: Some(summary),
        error: None,
    })
}

pub async fn sync_strategy_broker(
    State(state): State<AppState>,
    Path(strategy_id): Path<String>,
) -> AppResult<ApiResponse<BrokerSyncState>> {
    let sync = sync_strategy_broker_state(&state, &strategy_id).await?;
    Ok(ApiResponse {
        success: true,
        data: Some(sync),
        error: None,
    })
}

#[derive(serde::Deserialize)]
pub struct SetProfileRequest {
    pub profile: crate::models::ExecutionProfile,
}

pub async fn set_global_profile(
    State(state): State<AppState>,
    Json(request): Json<SetProfileRequest>,
) -> AppResult<ApiResponse<()>> {
    let db = state.db.lock().await;
    db.set_global_execution_profile(request.profile)?;
    Ok(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}
