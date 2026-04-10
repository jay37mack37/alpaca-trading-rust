use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

use crate::routes::auth::get_authenticated_client;
use crate::routes::websocket::AppState;
use axum::extract::State;

/// Cancel an order by ID
pub async fn cancel_order(
    State(_state): State<AppState>,
    headers: axum::http::HeaderMap,
    path: axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let order_id = path.0;
    let api_client = get_authenticated_client(&headers).await?;
    let username = crate::routes::auth::get_username_from_headers(&headers)
        .unwrap_or_else(|_| "unknown".to_string());

    tracing::info!(user = %username, order_id = %order_id, "Cancelling order");

    match api_client.cancel_order(&order_id).await {
        Ok(_) => {
            tracing::info!(user = %username, order_id = %order_id, "Order cancelled successfully");
            Ok(Json(json!({
                "success": true,
                "message": format!("Order {} cancelled", order_id)
            })))
        }
        Err(e) => {
            tracing::error!(user = %username, order_id = %order_id, "Failed to cancel order: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to cancel order: {}", e)
                })),
            ))
        }
    }
}

/// Cancel all open orders
pub async fn cancel_all_orders(
    State(_state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers).await?;
    let username = crate::routes::auth::get_username_from_headers(&headers)
        .unwrap_or_else(|_| "unknown".to_string());

    tracing::info!(user = %username, "Cancelling all orders");

    match api_client.cancel_all_orders().await {
        Ok(orders) => {
            tracing::info!(user = %username, count = orders.len(), "All orders cancelled successfully");
            Ok(Json(json!({
                "success": true,
                "message": format!("Cancelled {} orders", orders.len()),
                "orders": orders
            })))
        }
        Err(e) => {
            tracing::error!(user = %username, "Failed to cancel all orders: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to cancel all orders: {}", e)
                })),
            ))
        }
    }
}

/// Get a specific order by ID
pub async fn get_order_by_id(
    State(_state): State<AppState>,
    headers: axum::http::HeaderMap,
    path: axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let order_id = path.0;
    let api_client = get_authenticated_client(&headers).await?;

    match api_client.get_order_by_id(&order_id).await {
        Ok(order) => Ok(Json(order)),
        Err(e) => {
            tracing::error!("Failed to get order: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get order: {}", e)
                })),
            ))
        }
    }
}
