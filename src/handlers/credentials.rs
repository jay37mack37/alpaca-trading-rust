use axum::{
    extract::State,
    Json,
};
use crate::models::{
    CredentialSummary, CreateCredentialRequest,
};
use crate::error::{AppResult, ApiResponse};
use crate::AppState;
use crate::services::db::Database;

pub async fn list_credentials(
    State(state): State<AppState>,
) -> AppResult<ApiResponse<Vec<CredentialSummary>>> {
    let db = state.db.lock().await;
    let db: &Database = &*db;
    Ok(ApiResponse {
        success: true,
        data: Some(db.list_credentials()?),
        error: None,
    })
}

pub async fn create_credential(
    State(state): State<AppState>,
    Json(request): Json<CreateCredentialRequest>,
) -> AppResult<ApiResponse<CredentialSummary>> {
    if request.label.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "credential label is required".to_string(),
        ));
    }

    let created = {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.insert_credential(request)?
    };

    Ok(ApiResponse {
        success: true,
        data: Some(created),
        error: None,
    })
}
