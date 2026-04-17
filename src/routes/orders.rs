use axum::{extract::Path, extract::State, Json};
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::routes::auth::{get_authenticated_client, get_username_from_headers};
use crate::routes::websocket::AppState;

/// Cancel an order by ID
pub async fn cancel_order(
    State(_state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(order_id): Path<String>,
) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers).await?;
    let username = get_username_from_headers(&headers).unwrap_or_else(|_| "unknown".to_string());

    tracing::info!(user = %username, order_id = %order_id, "Cancelling order");

    let _ = api_client.cancel_order(&order_id).await?;
    tracing::info!(user = %username, order_id = %order_id, "Order cancelled successfully");
    Ok(Json(json!({
        "success": true,
        "message": format!("Order {} cancelled", order_id)
    })))
}

/// Cancel all open orders
pub async fn cancel_all_orders(State(_state): State<AppState>, headers: axum::http::HeaderMap) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers).await?;
    let username = get_username_from_headers(&headers).unwrap_or_else(|_| "unknown".to_string());

    tracing::info!(user = %username, "Cancelling all orders");

    let orders = api_client.cancel_all_orders().await?;
    tracing::info!(user = %username, count = orders.len(), "All orders cancelled successfully");
    Ok(Json(json!({
        "success": true,
        "message": format!("Cancelled {} orders", orders.len()),
        "orders": orders
    })))
}

/// Get a specific order by ID
pub async fn get_order_by_id(
    State(_state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(order_id): Path<String>,
) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers).await?;
    let order = api_client.get_order_by_id(&order_id).await?;
    Ok(Json(order))
}
