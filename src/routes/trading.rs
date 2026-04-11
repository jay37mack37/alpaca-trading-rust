use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::models::option_chain::OptionChainResponse;
use crate::models::order::OrderRequest;
use crate::routes::auth::get_authenticated_client;
use crate::routes::websocket::AppState;

/// Query parameters for options chain
#[derive(Debug, Deserialize)]
pub struct OptionsQuery {
    pub expiration: Option<String>,
}

/// Query parameters for orders
#[derive(Debug, Deserialize)]
pub struct OrdersQuery {
    pub status: Option<String>,
}

/// Get account information
pub async fn get_account(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    match api_client.get_account().await {
        Ok(account) => Ok(Json(account)),
        Err(e) => {
            tracing::error!("Failed to get account: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("API Error: {}", e)
            }))))
        }
    }
}

/// Get option chain for a symbol
pub async fn get_option_chain(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> Result<Json<OptionChainResponse>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    match api_client.get_option_chain(&symbol).await {
        Ok(chain) => Ok(Json(chain)),
        Err(e) => {
            tracing::error!("Failed to get option chain for {}: {}", symbol, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to get option chain: {}", e)
            }))))
        }
    }
}

/// Get all open positions
pub async fn get_positions(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

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
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<OrdersQuery>,
) -> Result<Json<Vec<Value>>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    match api_client.get_orders(query.status.clone()).await {
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
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(order): Json<OrderRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    // Get username for auditing
    let username = crate::routes::auth::get_username_from_headers(&headers).unwrap_or_else(|_| "unknown".to_string());

    // Validate order request
    if let Err(e) = order.validate() {
        tracing::warn!(user = %username, symbol = %order.symbol, "Order validation failed: {}", e);
        return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": format!("Validation Error: {}", e)
        }))));
    }

    tracing::info!(user = %username, symbol = %order.symbol, qty = %order.qty, side = %order.side, "Placing order");

    match api_client.create_order(order).await {
        Ok(order) => {
            if let Some(error) = order.get("message").and_then(|m| m.as_str()) {
                if order.get("id").is_none() {
                    return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({
                        "error": error
                    }))));
                }
            }
            tracing::info!(user = %username, order_id = ?order.get("id"), "Order placed successfully");
            Ok(Json(order))
        },
        Err(e) => {
            tracing::error!(user = %username, "Failed to create order: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("API Error: {}", e)
            }))))
        }
    }
}

/// Get current price for a symbol
pub async fn get_price(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    match api_client.get_current_price(&symbol).await {
        Ok(quote) => Ok(Json(quote)),
        Err(e) => {
            tracing::error!("Failed to get price for {}: {}", symbol, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to get price: {}", e)
            }))))
        }
    }
}

/// Get option strikes for a symbol
pub async fn get_option_strikes(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
    Query(params): Query<OptionsQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    let expiration = params.expiration.clone();
    match api_client.get_option_strikes(&symbol, expiration).await {
        Ok(strikes) => Ok(Json(strikes)),
        Err(e) => {
            tracing::error!("Failed to get option strikes for {}: {}", symbol, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to get option strikes: {}", e)
            }))))
        }
    }
}

/// Get current price for an option
pub async fn get_option_price(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    match api_client.get_option_price(&symbol).await {
        Ok(quote) => Ok(Json(quote)),
        Err(e) => {
            tracing::error!("Failed to get option price for {}: {}", symbol, e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": format!("Failed to get option price: {}", e)
            }))))
        }
    }
}