use axum::{
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use std::sync::Arc;
use crate::api::alpaca::{AlpacaApi, AlpacaClient};
use crate::auth;
use crate::auth::{LoginRequest, PasswordRequest, ApiKeyRequest};

/// Login endpoint
pub async fn login(
    Json(payload): Json<LoginRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match auth::login(&payload.username, &payload.password) {
        Some(response) => Ok(Json(json!({
            "token": response.token,
            "username": response.username
        }))),
        None => Err((StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid username or password"
        })))),
    }
}

/// Verify token endpoint
pub async fn verify_token(
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_token(&headers)?;

    match auth::verify_token(&token) {
        Some(username) => Ok(Json(json!({
            "valid": true,
            "username": username
        }))),
        None => Err((StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid or expired token"
        })))),
    }
}

/// Logout endpoint
pub async fn logout(
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let token = extract_token(&headers)?;
    auth::logout(&token);
    Ok(Json(json!({ "success": true })))
}

/// Get API key status
pub async fn get_api_key_status(
    headers: axum::http::HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let username = get_username_from_headers(&headers)?;
    let (configured, environment) = auth::get_api_key_status(&username);

    Ok(Json(json!({
        "configured": configured,
        "environment": environment
    })))
}

/// Save API keys
pub async fn save_api_keys(
    headers: axum::http::HeaderMap,
    Json(payload): Json<ApiKeyRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let username = get_username_from_headers(&headers)?;

    match auth::save_api_keys(&username, &payload.api_key, &payload.api_secret, &payload.environment) {
        Ok(_) => Ok(Json(json!({ "success": true }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
            "error": e
        })))),
    }
}

/// Change password
pub async fn change_password(
    headers: axum::http::HeaderMap,
    Json(payload): Json<PasswordRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let username = get_username_from_headers(&headers)?;

    match auth::change_password(&username, &payload.current_password, &payload.new_password) {
        Ok(_) => Ok(Json(json!({ "success": true }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json(json!({
            "error": e
        })))),
    }
}

/// Helper to extract token from Authorization header
fn extract_token(headers: &axum::http::HeaderMap) -> Result<String, (StatusCode, Json<Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Missing Authorization header"
        }))))?;

    if !auth_header.starts_with("Bearer ") {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid Authorization header format"
        }))));
    }

    Ok(auth_header[7..].to_string())
}

/// Helper to get username from auth headers
pub fn get_username_from_headers(headers: &axum::http::HeaderMap) -> Result<String, (StatusCode, Json<Value>)> {
    let token = extract_token(headers)?;
    auth::verify_token(&token)
        .ok_or((StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid or expired token"
        }))))
}

/// Middleware helper for authenticated routes with Alpaca client.
/// Strictly requires user-specific API keys.
pub async fn get_authenticated_client(
    headers: &axum::http::HeaderMap,
    state: &crate::AppState,
) -> Result<Arc<dyn AlpacaApi>, (StatusCode, Json<Value>)> {
    let username = get_username_from_headers(headers)?;

    if let Some(alpaca) = &state.alpaca {
        return Ok(alpaca.clone());
    }

    match auth::get_api_keys(&username) {
        Some((api_key, api_secret, _environment)) => {
            // Create client with user's keys
            AlpacaClient::with_keys(&api_key, &api_secret)
                .map(|c| Arc::new(c) as Arc<dyn AlpacaApi>)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                    "error": e
                }))))
        }
        None => {
            // No user-specific keys found
            Err((StatusCode::FORBIDDEN, Json(json!({
                "error": "No API keys configured. Please configure your Alpaca API keys in Settings."
            }))))
        }
    }
}