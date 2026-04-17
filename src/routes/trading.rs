use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::models::option_chain::OptionChainResponse;
use crate::models::order::OrderRequest;
use crate::routes::auth::{get_authenticated_client, get_username_from_headers};
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
pub async fn get_account(State(state): State<AppState>, headers: axum::http::HeaderMap) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let account = api_client.get_account().await?;
    Ok(Json(account))
}

/// Get option chain for a symbol
pub async fn get_option_chain(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> AppResult<Json<OptionChainResponse>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let chain = api_client.get_option_chain(&symbol).await?;
    Ok(Json(chain))
}

/// Get all open positions
pub async fn get_positions(State(state): State<AppState>, headers: axum::http::HeaderMap) -> AppResult<Json<Vec<Value>>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let positions = api_client.get_positions().await?;
    Ok(Json(positions))
}

/// Get orders
pub async fn get_orders(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<OrdersQuery>,
) -> AppResult<Json<Vec<Value>>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let orders = api_client.get_orders(query.status).await?;
    Ok(Json(orders))
}

/// Create a new order
pub async fn create_order(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(order): Json<OrderRequest>,
) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers, &state).await?;

    // Get username for auditing
    let username = get_username_from_headers(&headers).unwrap_or_else(|_| "unknown".to_string());

    // Validate order request
    if let Err(e) = order.validate() {
        tracing::warn!(user = %username, symbol = %order.symbol, "Order validation failed: {}", e);
        return Err(AppError::ValidationError(e));
    }

    tracing::info!(user = %username, symbol = %order.symbol, qty = %order.qty, side = %order.side, "Placing order");

    let result = api_client.create_order(order).await?;

    if let Some(error) = result.get("message").and_then(|m| m.as_str()) {
        if result.get("id").is_none() {
            return Err(AppError::ValidationError(error.to_string()));
        }
    }

    tracing::info!(user = %username, order_id = ?result.get("id"), "Order placed successfully");
    Ok(Json(result))
}

/// Get current price for a symbol
pub async fn get_price(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let quote = api_client.get_current_price(&symbol).await?;
    Ok(Json(quote))
}

/// Get option strikes for a symbol
pub async fn get_option_strikes(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
    Query(params): Query<OptionsQuery>,
) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let strikes = api_client.get_option_strikes(&symbol, params.expiration).await?;
    Ok(Json(strikes))
}

/// Get current price for an option
pub async fn get_option_price(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> AppResult<Json<Value>> {
    let api_client = get_authenticated_client(&headers, &state).await?;
    let quote = api_client.get_option_price(&symbol).await?;
    Ok(Json(quote))
}