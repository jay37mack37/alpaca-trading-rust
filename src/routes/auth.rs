use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::api::alpaca::{AlpacaApi, AlpacaClient};
use crate::auth;
use crate::auth::{ApiKeyRequest, LoginRequest, PasswordRequest};
use crate::error::{AppError, AppResult};

/// Login endpoint
pub async fn login(Json(payload): Json<LoginRequest>) -> AppResult<Json<Value>> {
    match auth::login(&payload.username, &payload.password) {
        Some(response) => Ok(Json(json!({
            "token": response.token,
            "username": response.username
        }))),
        None => Err(AppError::Unauthorized("Invalid username or password".to_string())),
    }
}

/// Verify token endpoint
pub async fn verify_token(headers: axum::http::HeaderMap) -> AppResult<Json<Value>> {
    let token = extract_token(&headers)?;

    match auth::verify_token(&token) {
        Some(username) => Ok(Json(json!({
            "valid": true,
            "username": username
        }))),
        None => Err(AppError::Unauthorized("Invalid or expired token".to_string())),
    }
}

/// Logout endpoint
pub async fn logout(headers: axum::http::HeaderMap) -> AppResult<Json<Value>> {
    let token = extract_token(&headers)?;
    auth::logout(&token);
    Ok(Json(json!({ "success": true })))
}

/// Get API key status
pub async fn get_api_key_status(headers: axum::http::HeaderMap) -> AppResult<Json<Value>> {
    let username = get_username_from_headers(&headers)?;
    let (configured, environment) = auth::get_api_key_status(&username);

    Ok(Json(json!({
        "configured": configured,
        "environment": environment
    })))
}

/// Save API keys
pub async fn save_api_keys(headers: axum::http::HeaderMap, Json(payload): Json<ApiKeyRequest>) -> AppResult<Json<Value>> {
    let username = get_username_from_headers(&headers)?;

    auth::save_api_keys(&username, &payload.api_key, &payload.api_secret, &payload.environment)
        .map_err(AppError::Internal)?;
    Ok(Json(json!({ "success": true })))
}

/// Change password
pub async fn change_password(
    headers: axum::http::HeaderMap,
    Json(payload): Json<PasswordRequest>,
) -> AppResult<Json<Value>> {
    let username = get_username_from_headers(&headers)?;

    auth::change_password(&username, &payload.current_password, &payload.new_password)
        .map_err(AppError::ValidationError)?;
    Ok(Json(json!({ "success": true })))
}

/// Helper to extract token from Authorization header
fn extract_token(headers: &axum::http::HeaderMap) -> AppResult<String> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized("Invalid Authorization header format".to_string()));
    }

    Ok(auth_header[7..].to_string())
}

/// Helper to get username from auth headers
pub fn get_username_from_headers(headers: &axum::http::HeaderMap) -> AppResult<String> {
    let token = extract_token(headers)?;
    auth::verify_token(&token).ok_or_else(|| AppError::Unauthorized("Invalid or expired token".to_string()))
}

/// Middleware helper for authenticated routes with Alpaca client.
/// Strictly requires user-specific API keys.
pub async fn get_authenticated_client(headers: &axum::http::HeaderMap, state: &crate::routes::websocket::AppState) -> AppResult<Arc<dyn AlpacaApi>> {
    let username = get_username_from_headers(headers)?;

    if let Some(alpaca) = &state.alpaca {
        return Ok(alpaca.clone());
    }

    match auth::get_api_keys(&username) {
        Some((api_key, api_secret, _environment)) => {
            // Create client with user's keys
            AlpacaClient::with_keys(&api_key, &api_secret).map(|c| Arc::new(c) as Arc<dyn AlpacaApi>).map_err(|e| AppError::Internal(e.to_string()))
        }
        None => {
            // No user-specific keys found
            Err(AppError::Forbidden(
                "No API keys configured. Please configure your Alpaca API keys in Settings.".to_string(),
            ))
        }
    }
}