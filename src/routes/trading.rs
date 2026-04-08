use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;

use crate::api::alpaca::AlpacaClient;
use crate::models::order::OrderRequest;

/// Get account information
pub async fn get_account(State(client): State<AlpacaClient>) -> Result<Json<Value>, StatusCode> {
    match client.get_account().await {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            tracing::error!("Failed to get account: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get all open positions
pub async fn get_positions(State(client): State<AlpacaClient>) -> Result<Json<Vec<Value>>, StatusCode> {
    match client.get_positions().await {
        Ok(positions) => Ok(Json(positions)),
        Err(e) => {
            tracing::error!("Failed to get positions: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get orders
pub async fn get_orders(State(client): State<AlpacaClient>) -> Result<Json<Vec<Value>>, StatusCode> {
    match client.get_orders().await {
        Ok(orders) => Ok(Json(orders)),
        Err(e) => {
            tracing::error!("Failed to get orders: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create a new order
pub async fn create_order(
    State(client): State<AlpacaClient>,
    Json(order): Json<OrderRequest>,
) -> Result<Json<Value>, StatusCode> {
    match client.create_order(order).await {
        Ok(order) => Ok(Json(order)),
        Err(e) => {
            tracing::error!("Failed to create order: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}