use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use alpaca_trading3web::api::ws_manager::WsManager;
use alpaca_trading3web::{auth, build_app, AppState};
use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use lazy_static::lazy_static;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tower::util::ServiceExt;

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

struct TestPaths {
    config_path: PathBuf,
    env_path: PathBuf,
}

impl TestPaths {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let base = env::temp_dir().join(format!("alpaca-trading-rust-tests-{unique}"));
        fs::create_dir_all(&base).expect("temp dir should be created");
        Self {
            config_path: base.join("config.json"),
            env_path: base.join(".env"),
        }
    }

    fn apply(&self) {
        env::set_var("ALPACA_CONFIG_FILE", &self.config_path);
        env::set_var("ALPACA_ENV_FILE", &self.env_path);
    }
}

impl Drop for TestPaths {
    fn drop(&mut self) {
        env::remove_var("ALPACA_CONFIG_FILE");
        env::remove_var("ALPACA_ENV_FILE");
        env::remove_var("ALPACA_PAPER_URL_OVERRIDE");
        env::remove_var("ALPACA_LIVE_URL_OVERRIDE");
        env::remove_var("ALPACA_DATA_URL_OVERRIDE");
        env::remove_var("ALPACA_OPTIONS_URL_OVERRIDE");

        if let Some(parent) = self.config_path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

#[derive(Clone)]
struct MockOrderState {
    last_order: Arc<Mutex<Option<Value>>>,
}

async fn login(app: &Router) -> String {
    let response = send_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/login")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "username": "admin",
                    "password": "admin123"
                })
                .to_string(),
            ))
            .expect("login request should build"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    response
        .body
        .get("token")
        .and_then(Value::as_str)
        .expect("login response should contain a token")
        .to_string()
}

async fn save_api_keys(app: &Router, token: &str, environment: &str) {
    let response = send_json(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/config/api-keys")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(
                json!({
                    "api_key": "test-key",
                    "api_secret": "test-secret",
                    "environment": environment
                })
                .to_string(),
            ))
            .expect("save api keys request should build"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
}

fn test_app() -> Router {
    auth::init();
    build_app(AppState {
        alpaca: None,
        ws_manager: Arc::new(WsManager::new()),
    })
}

async fn send_json(app: &Router, request: Request<Body>) -> TestResponse {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("request should succeed");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = if bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&bytes).expect("response body should be valid json")
    };

    TestResponse { status, body }
}

struct TestResponse {
    status: StatusCode,
    body: Value,
}

async fn spawn_server(router: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("listener should bind");
    let addr = listener
        .local_addr()
        .expect("listener should have a local address");

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock server should serve");
    });

    addr
}

fn base_url(addr: SocketAddr, suffix: &str) -> String {
    format!("http://{addr}{suffix}")
}

#[tokio::test]
async fn option_orders_accept_option_sides_and_reach_upstream() {
    let _guard = TEST_MUTEX.lock().await;
    let paths = TestPaths::new();
    paths.apply();

    let order_state = MockOrderState {
        last_order: Arc::new(Mutex::new(None)),
    };

    let mock_router = Router::new()
        .route(
            "/v2/orders",
            post({
                move |State(order_state): State<MockOrderState>, Json(payload): Json<Value>| async move {
                    *order_state.last_order.lock().await = Some(payload.clone());
                    Json(json!({
                        "id": "order-123",
                        "status": "accepted",
                        "symbol": payload["symbol"],
                        "side": payload["side"]
                    }))
                }
            }),
        )
        .with_state(order_state.clone());

    let addr = spawn_server(mock_router).await;
    env::set_var("ALPACA_PAPER_URL_OVERRIDE", base_url(addr, "/v2"));

    let app = test_app();
    let token = login(&app).await;
    save_api_keys(&app, &token, "paper").await;

    let response = send_json(
        &app,
        Request::builder()
            .method("POST")
            .uri("/api/orders")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(
                json!({
                    "symbol": "SPY260619C00500000",
                    "qty": 1,
                    "side": "buy_to_open",
                    "order_type": "limit",
                    "time_in_force": "day",
                    "limit_price": 5.25,
                    "asset_class": "us_option"
                })
                .to_string(),
            ))
            .expect("option order request should build"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["id"], "order-123");

    let captured = order_state.last_order.lock().await.clone().unwrap();
    assert_eq!(captured["side"], "buy_to_open");
    assert_eq!(captured["asset_class"], "us_option");
}

#[tokio::test]
async fn account_route_uses_saved_live_environment() {
    let _guard = TEST_MUTEX.lock().await;
    let paths = TestPaths::new();
    paths.apply();

    let paper_addr = spawn_server(Router::new().route(
        "/v2/account",
        get(|| async { Json(json!({ "account_number": "PAPER-ACCOUNT" })) }),
    ))
    .await;
    let live_addr = spawn_server(Router::new().route(
        "/v2/account",
        get(|| async { Json(json!({ "account_number": "LIVE-ACCOUNT" })) }),
    ))
    .await;

    env::set_var("ALPACA_PAPER_URL_OVERRIDE", base_url(paper_addr, "/v2"));
    env::set_var("ALPACA_LIVE_URL_OVERRIDE", base_url(live_addr, "/v2"));

    let app = test_app();
    let token = login(&app).await;
    save_api_keys(&app, &token, "live").await;

    let response = send_json(
        &app,
        Request::builder()
            .method("GET")
            .uri("/api/account")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("account request should build"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["account_number"], "LIVE-ACCOUNT");
}

#[tokio::test]
async fn option_chain_route_filters_by_requested_expiration() {
    let _guard = TEST_MUTEX.lock().await;
    let paths = TestPaths::new();
    paths.apply();

    let mock_router = Router::new()
        .route(
            "/v2/stocks/{symbol}/quotes/latest",
            get(|| async { Json(json!({ "quote": { "ap": 500.0, "bp": 499.5 } })) }),
        )
        .route(
            "/v1beta1/options/snapshots/{symbol}",
            get(|request: axum::extract::Request| async move {
                let query = request.uri().query().unwrap_or_default();
                let snapshots = if query.contains("type=call") {
                    json!({
                        "SPY260619C00500000": {
                            "latestQuote": { "bp": 5.0, "ap": 5.2, "as": 10 }
                        },
                        "SPY260626C00500000": {
                            "latestQuote": { "bp": 6.1, "ap": 6.4, "as": 12 }
                        }
                    })
                } else {
                    json!({
                        "SPY260619P00500000": {
                            "latestQuote": { "bp": 4.8, "ap": 5.0, "as": 11 }
                        },
                        "SPY260626P00500000": {
                            "latestQuote": { "bp": 5.9, "ap": 6.2, "as": 13 }
                        }
                    })
                };

                Json(json!({ "snapshots": snapshots }))
            }),
        );

    let addr = spawn_server(mock_router).await;
    env::set_var("ALPACA_PAPER_URL_OVERRIDE", base_url(addr, "/v2"));
    env::set_var("ALPACA_DATA_URL_OVERRIDE", base_url(addr, "/v2"));
    env::set_var(
        "ALPACA_OPTIONS_URL_OVERRIDE",
        base_url(addr, "/v1beta1/options"),
    );

    let app = test_app();
    let token = login(&app).await;
    save_api_keys(&app, &token, "paper").await;

    let response = send_json(
        &app,
        Request::builder()
            .method("GET")
            .uri("/api/option-chain/SPY?expiration=2026-06-19")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("option chain request should build"),
    )
    .await;

    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["symbol"], "SPY");
    let strikes = response.body["strikes"]
        .as_array()
        .expect("option chain strikes should be an array");
    assert_eq!(strikes.len(), 1);
    assert_eq!(strikes[0]["call"]["symbol"], "SPY260619C00500000");
    assert_eq!(strikes[0]["put"]["symbol"], "SPY260619P00500000");
}

#[tokio::test]
async fn cancel_order_route_surfaces_upstream_errors() {
    let _guard = TEST_MUTEX.lock().await;
    let paths = TestPaths::new();
    paths.apply();

    let mock_router = Router::new().route(
        "/v2/orders/{id}",
        delete(|| async {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "message": "order cannot be cancelled" })),
            )
                .into_response()
        }),
    );

    let addr = spawn_server(mock_router).await;
    env::set_var("ALPACA_PAPER_URL_OVERRIDE", base_url(addr, "/v2"));

    let app = test_app();
    let token = login(&app).await;
    save_api_keys(&app, &token, "paper").await;

    let response = send_json(
        &app,
        Request::builder()
            .method("DELETE")
            .uri("/api/orders/bad-order")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("cancel request should build"),
    )
    .await;

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.body["error"]
        .as_str()
        .expect("error should be a string")
        .contains("422"));
}
