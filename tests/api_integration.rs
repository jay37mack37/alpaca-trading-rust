use axum_test::TestServer;
use mockall::predicate::*;
use alpaca_trading3web::api::alpaca::{AlpacaApi};
use alpaca_trading3web::auth::{self, LoginRequest};
use alpaca_trading3web::routes::websocket::AppState;
use alpaca_trading3web::api::ws_manager::WsManager;
use alpaca_trading3web::create_router;
use std::sync::Arc;
use serde_json::json;
use async_trait::async_trait;
use alpaca_trading3web::error::AppResult;
use alpaca_trading3web::models::option_chain::OptionChainResponse;
use alpaca_trading3web::models::order::OrderRequest;

// Mock for AlpacaApi
mockall::mock! {
    pub AlpacaApi {}
    #[async_trait]
    impl AlpacaApi for AlpacaApi {
        async fn get_account(&self) -> AppResult<serde_json::Value>;
        async fn get_positions(&self) -> AppResult<Vec<serde_json::Value>>;
        async fn get_orders(&self, status: Option<String>) -> AppResult<Vec<serde_json::Value>>;
        async fn create_order(&self, order: OrderRequest) -> AppResult<serde_json::Value>;
        async fn get_order_by_id(&self, order_id: &str) -> AppResult<serde_json::Value>;
        async fn cancel_order(&self, order_id: &str) -> AppResult<serde_json::Value>;
        async fn cancel_all_orders(&self) -> AppResult<Vec<serde_json::Value>>;
        async fn get_current_price(&self, symbol: &str) -> AppResult<serde_json::Value>;
        async fn get_option_strikes(&self, symbol: &str, expiration: Option<String>) -> AppResult<serde_json::Value>;
        async fn get_option_chain(&self, symbol: &str) -> AppResult<OptionChainResponse>;
        async fn get_option_price(&self, option_symbol: &str) -> AppResult<serde_json::Value>;
    }
}

async fn setup_test_app(mock_api: MockAlpacaApi) -> TestServer {
    auth::init();
    let state = AppState {
        alpaca: Some(Arc::new(mock_api)),
        ws_manager: Arc::new(WsManager::new()),
        strategy_manager: Arc::new(alpaca_trading3web::strategies::StrategyManager::new()),
    };
    let router = create_router(state);
    TestServer::new(router)
}

#[tokio::test]
async fn test_health_check() {
    let mock = MockAlpacaApi::new();
    let server = setup_test_app(mock).await;
    let response = server.get("/api/health").await;
    response.assert_status_ok();
    assert_eq!(response.json::<serde_json::Value>()["status"], "healthy");
}

#[tokio::test]
async fn test_login_and_auth_flow() {
    let mock = MockAlpacaApi::new();
    let server = setup_test_app(mock).await;

    let login_req = LoginRequest {
        username: "admin".to_string(),
        password: "admin123".to_string(),
    };

    let response = server.post("/api/login").json(&login_req).await;
    response.assert_status_ok();
    let data = response.json::<serde_json::Value>();
    let token = data["token"].as_str().expect("Token missing in response");

    let verify_response = server
        .get("/api/verify")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
    verify_response.assert_status_ok();
}

#[tokio::test]
async fn test_get_account_mocked() {
    let mut mock = MockAlpacaApi::new();
    mock.expect_get_account()
        .returning(|| Ok(json!({
            "account_number": "123456789",
            "status": "ACTIVE",
            "portfolio_value": "100000"
        })));

    let server = setup_test_app(mock).await;
    let login_req = LoginRequest { username: "admin".to_string(), password: "admin123".to_string() };
    let login_res = server.post("/api/login").json(&login_req).await;
    let token = login_res.json::<serde_json::Value>()["token"].as_str().expect("Token missing").to_string();

    let response = server
        .get("/api/account")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status_ok();
    let data = response.json::<serde_json::Value>();
    assert_eq!(data["account_number"], "123456789");
}

#[tokio::test]
async fn test_get_positions_mocked() {
    let mut mock = MockAlpacaApi::new();
    mock.expect_get_positions()
        .returning(|| Ok(vec![json!({
            "symbol": "AAPL",
            "qty": "10",
            "current_price": "150.00"
        })]));

    let server = setup_test_app(mock).await;
    let login_req = LoginRequest { username: "admin".to_string(), password: "admin123".to_string() };
    let token = server.post("/api/login").json(&login_req).await.json::<serde_json::Value>()["token"].as_str().expect("Token missing").to_string();

    let response = server
        .get("/api/positions")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;

    response.assert_status_ok();
    let data = response.json::<Vec<serde_json::Value>>();
    assert_eq!(data[0]["symbol"], "AAPL");
}
