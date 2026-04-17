use axum::{
    extract::{Path, State},
    Json,
};
use crate::models::{
    Watchlist, CreateWatchlistRequest, UpdateWatchlistRequest, WatchlistAddRequest,
};
use crate::error::{AppResult, ApiResponse};
use crate::AppState;
use crate::services::db::Database;
use serde_json::json;

pub async fn list_watchlists(State(state): State<AppState>) -> AppResult<ApiResponse<Vec<Watchlist>>> {
    let watchlists = {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.list_watchlists()?
    };
    Ok(ApiResponse {
        success: true,
        data: Some(watchlists),
        error: None,
    })
}

pub async fn create_watchlist(
    State(state): State<AppState>,
    Json(payload): Json<CreateWatchlistRequest>,
) -> AppResult<ApiResponse<Watchlist>> {
    let id = uuid::Uuid::new_v4().to_string();
    {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.insert_watchlist(&id, &payload.name, &payload.symbols)?;
    }

    let watchlists = {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.list_watchlists()?
    };
    let created = watchlists.into_iter().find(|w| w.id == id).unwrap();
    Ok(ApiResponse {
        success: true,
        data: Some(created),
        error: None,
    })
}

pub async fn update_watchlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateWatchlistRequest>,
) -> AppResult<ApiResponse<Watchlist>> {
    {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.update_watchlist(&id, &payload)?;
    }

    let watchlists = {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.list_watchlists()?
    };
    let updated = watchlists.into_iter().find(|w| w.id == id).unwrap();
    Ok(ApiResponse {
        success: true,
        data: Some(updated),
        error: None,
    })
}

pub async fn delete_watchlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.delete_watchlist(&id)?;
    }
    Ok(ApiResponse {
        success: true,
        data: Some(json!({ "status": "ok" })),
        error: None,
    })
}

pub async fn add_watchlist_symbol(
    State(state): State<AppState>,
    Json(request): Json<WatchlistAddRequest>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let db = state.db.lock().await;
    let db: &Database = &*db;
    db.insert_watchlist_symbol(&request.symbol)?;
    Ok(ApiResponse {
        success: true,
        data: Some(json!({
            "status": "ok",
            "symbol": request.symbol.trim().to_uppercase(),
        })),
        error: None,
    })
}

pub async fn remove_watchlist_symbol(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
) -> AppResult<ApiResponse<serde_json::Value>> {
    let db = state.db.lock().await;
    let db: &Database = &*db;
    db.delete_watchlist_symbol(&symbol)?;
    Ok(ApiResponse {
        success: true,
        data: Some(json!({
            "status": "ok",
            "symbol": symbol.trim().to_uppercase(),
        })),
        error: None,
    })
}
