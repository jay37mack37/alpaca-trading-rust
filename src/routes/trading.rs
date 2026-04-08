use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde_json::Value;

use crate::api::alpaca::AlpacaClient;
use crate::models::order::OrderRequest;
use crate::routes::auth::{get_username_from_headers, get_authenticated_client};

/// Get account information
pub async fn get_account(
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // First try to get user's specific API keys
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            // Fall back to environment client
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured. Please configure in Settings."
            }))))?
        }
    };

    match api_client.get_account().await {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            tracing::error!("Failed to get account: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "message": format!("API Error: {}", e)
            }))))
        }
    }
}

/// Get all open positions
pub async fn get_positions(
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured"
            }))))?
        }
    };

    match api_client.get_positions().await {
        Ok(positions) => Ok(Json(positions)),
        Err(e) => {
            tracing::error!("Failed to get positions: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("API Error: {}", e)
            }))))
        }
    }
}

/// Get orders
pub async fn get_orders(
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured"
            }))))?
        }
    };

    match api_client.get_orders().await {
        Ok(orders) => Ok(Json(orders)),
        Err(e) => {
            tracing::error!("Failed to get orders: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("API Error: {}", e)
            }))))
        }
    }
}

/// Create a new order
pub async fn create_order(
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
    Json(order): Json<OrderRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured"
            }))))?
        }
    };

    match api_client.create_order(order).await {
        Ok(order) => Ok(Json(order)),
        Err(e) => {
            tracing::error!("Failed to create order: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("API Error: {}", e)
            }))))
        }
    }
}