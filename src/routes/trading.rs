use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use crate::api::alpaca::AlpacaClient;
use crate::models::order::OrderRequest;
use crate::routes::auth::get_authenticated_client;

/// Query parameters for options chain
#[derive(Debug, Deserialize)]
pub struct OptionsQuery {
    pub expiration: Option<String>,
}

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

/// Get current price for a symbol
pub async fn get_price(
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured"
            }))))?
        }
    };

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
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
    Query(params): Query<OptionsQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured"
            }))))?
        }
    };

    let expiration = params.expiration.as_deref();
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
    State(client): State<Option<AlpacaClient>>,
    headers: axum::http::HeaderMap,
    Path(symbol): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_client = match get_authenticated_client(headers).await {
        Ok(c) => c,
        Err(_) => {
            client.ok_or((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
                "error": "No API keys configured"
            }))))?
        }
    };

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