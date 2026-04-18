use axum::{
    extract::{Path, State},
    Json,
};
use crate::models::{
    HealthResponse, CollectResponse, BrokerSyncState,
};
use crate::error::{AppResult, ApiResponse};
use crate::{AppState, collect_once, sync_strategy_broker_state};
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
    pub source: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
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
