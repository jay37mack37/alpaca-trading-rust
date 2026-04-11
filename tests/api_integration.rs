use axum_test::TestServer;
use axum::{
    routing::{get, post, delete},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use mockall::predicate::*;
use mockall::mock;
use async_trait::async_trait;
use tower_http::services::ServeDir;

// Import needed modules from our crate
use alpaca_trading3web::api::alpaca::{AlpacaApi, AlpacaClient};
use alpaca_trading3web::{routes, AppState};
use alpaca_trading3web::api::ws_manager::WsManager;
use alpaca_trading3web::models::order::OrderRequest;
use alpaca_trading3web::models::option_chain::OptionChainResponse;

mock! {
    pub AlpacaApi {}
    #[async_trait]
    impl AlpacaApi for AlpacaApi {
        async fn get_account(&self) -> Result<serde_json::Value, reqwest::Error>;
        async fn get_positions(&self) -> Result<Vec<serde_json::Value>, reqwest::Error>;
        async fn get_orders(&self, status: Option<String>) -> Result<Vec<serde_json::Value>, reqwest::Error>;
        async fn create_order(&self, order: OrderRequest) -> Result<serde_json::Value, reqwest::Error>;
        async fn get_order_by_id(&self, order_id: &str) -> Result<serde_json::Value, reqwest::Error>;
        async fn cancel_order(&self, order_id: &str) -> Result<serde_json::Value, reqwest::Error>;
        async fn cancel_all_orders(&self) -> Result<Vec<serde_json::Value>, reqwest::Error>;
        async fn get_current_price(&self, symbol: &str) -> Result<serde_json::Value, reqwest::Error>;
        async fn get_option_strikes(&self, symbol: &str, expiration: Option<String>) -> Result<serde_json::Value, reqwest::Error>;
        async fn get_option_chain(&self, symbol: &str) -> Result<OptionChainResponse, reqwest::Error>;
        async fn get_option_price(&self, option_symbol: &str) -> Result<serde_json::Value, reqwest::Error>;
    }
}

fn setup_test_app(mock_alpaca: Arc<dyn AlpacaApi>) -> Router {
    alpaca_trading3web::auth::init();
    let ws_manager = Arc::new(WsManager::new());
    let state = AppState {
        alpaca: Some(mock_alpaca),
        ws_manager,
    };

    Router::new()
        .route("/api/login", post(routes::auth::login))
        .route("/api/verify", get(routes::auth::verify_token))
        .route("/api/account", get(routes::trading::get_account))
        .route("/api/positions", get(routes::trading::get_positions))
        .route("/api/orders", get(routes::trading::get_orders))
        .route("/api/orders", post(routes::trading::create_order))
        .route("/api/price/{symbol}", get(routes::trading::get_price))
        .route("/api/analytics/patterns", get(routes::analytics::get_patterns))
        .with_state(state)
}

#[tokio::test]
async fn test_health_check_via_patterns() {
    let mut mock = MockAlpacaApi::new();
    let app = setup_test_app(Arc::new(mock));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/analytics/patterns").await;
    response.assert_status_ok();

    let body: serde_json::Value = response.json();
    assert!(body["patterns"].is_array());
    assert!(body["patterns"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_login_and_verify() {
    let mock = MockAlpacaApi::new();
    let app = setup_test_app(Arc::new(mock));
    let server = TestServer::new(app).unwrap();

    // Login (default admin/admin123 should work if auth::init() was called in a way that works here)
    // Note: auth::init() is typically called in main.rs. We might need to ensure it's called or mock it.
    // For now, let's assume it works or we use a helper.

    let login_response = server.post("/api/login")
        .json(&json!({
            "username": "admin",
            "password": "admin123"
        }))
        .await;

    login_response.assert_status_ok();
    let login_data = login_response.json::<serde_json::Value>();
    let token = login_data["token"].as_str().unwrap();

    let verify_response = server.get("/api/verify")
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
        .await;

    verify_response.assert_status_ok();
    assert_eq!(verify_response.json::<serde_json::Value>()["username"], "admin");
}

#[tokio::test]
async fn test_get_account_authorized() {
    let mut mock = MockAlpacaApi::new();

    mock.expect_get_account()
        .times(1)
        .returning(|| Ok(json!({
            "account_number": "PA12345",
            "status": "ACTIVE",
            "buying_power": "100000.00",
            "portfolio_value": "105000.00",
            "cash": "50000.00",
            "equity": "105000.00"
        })));

    let app = setup_test_app(Arc::new(mock));
    let server = TestServer::new(app).unwrap();

    // Login to get token
    let login_data = server.post("/api/login")
        .json(&json!({"username": "admin", "password": "admin123"}))
        .await
        .json::<serde_json::Value>();
    let token = login_data["token"].as_str().unwrap();

    // Mock the user's API keys in the auth system so get_authenticated_client works
    alpaca_trading3web::auth::set_mock_api_keys("admin", "test_key", "test_secret", "paper");

    let response = server.get("/api/account")
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
        .await;

    response.assert_status_ok();
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["account_number"], "PA12345");
}

#[tokio::test]
async fn test_create_order() {
    let mut mock = MockAlpacaApi::new();

    mock.expect_create_order()
        .times(1)
        .returning(|_| Ok(json!({
            "id": "order_123",
            "status": "accepted",
            "symbol": "AAPL"
        })));

    let app = setup_test_app(Arc::new(mock));
    let server = TestServer::new(app).unwrap();

    // Login and set keys
    let login_data = server.post("/api/login")
        .json(&json!({"username": "admin", "password": "admin123"}))
        .await
        .json::<serde_json::Value>();
    let token = login_data["token"].as_str().unwrap();
    alpaca_trading3web::auth::set_mock_api_keys("admin", "test_key", "test_secret", "paper");

    let response = server.post("/api/orders")
        .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {}", token))
        .json(&json!({
            "symbol": "AAPL",
            "qty": 10.0,
            "side": "buy",
            "order_type": "market",
            "time_in_force": "day"
        }))
        .await;

    response.assert_status_ok();
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["id"], "order_123");
}

#[tokio::test]
async fn test_get_account_unauthorized() {
    let mock = MockAlpacaApi::new();
    let app = setup_test_app(Arc::new(mock));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/api/account").await;
    response.assert_status(axum::http::StatusCode::UNAUTHORIZED);
}

// More tests would go here, but this establishes the pattern
