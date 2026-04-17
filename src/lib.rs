use axum::routing::{delete, get, post};
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod api;
pub mod auth;
pub mod error;
pub mod math;
pub mod models;
pub mod routes;
pub mod strategies;

use api::alpaca::{AlpacaApi, AlpacaClient};
use api::price_streamer::PriceStreamer;
use api::ws_manager::WsManager;
pub use routes::websocket::AppState;
use strategies::StrategyManager;
use std::sync::Arc;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/api/login", post(routes::auth::login))
        .route("/api/verify", get(routes::auth::verify_token))
        .route("/api/logout", post(routes::auth::logout))
        .route("/api/config/status", get(routes::auth::get_api_key_status))
        .route("/api/config/api-keys", post(routes::auth::save_api_keys))
        .route("/api/config/password", post(routes::auth::change_password))
        .route("/api/account", get(routes::trading::get_account))
        .route("/api/positions", get(routes::trading::get_positions))
        .route("/api/orders", get(routes::trading::get_orders))
        .route("/api/orders", post(routes::trading::create_order))
        .route("/api/price/{symbol}", get(routes::trading::get_price))
        .route("/api/option-strikes/{symbol}", get(routes::trading::get_option_strikes))
        .route("/api/option-quote/{symbol}", get(routes::trading::get_option_price))
        .route("/api/option-chain/{symbol}", get(routes::trading::get_option_chain))
        .route("/api/orders/{id}", get(routes::orders::get_order_by_id))
        .route("/api/orders/{id}", delete(routes::orders::cancel_order))
        .route("/api/orders/cancel-all", post(routes::orders::cancel_all_orders))
        .route("/api/analytics/watchlist", get(routes::analytics::get_watchlist))
        .route("/api/analytics/watchlist", post(routes::analytics::update_watchlist))
        .route("/api/analytics/fetch", post(routes::analytics::fetch_data))
        .route("/api/analytics/summary", get(routes::analytics::get_summary))
        .route("/api/analytics/analyze", post(routes::analytics::run_analysis))
        .route("/api/analytics/patterns", get(routes::analytics::get_patterns))

        // Strategies routes (authenticated)
        .route("/api/strategies", get(routes::strategies::list_strategies))
        .route("/api/strategies/status", get(routes::strategies::get_strategies_status))
        .route("/api/strategies/stop-all", post(routes::strategies::stop_all_strategies))
        .route("/api/strategies/{id}/start", post(routes::strategies::start_strategy))
        .route("/api/strategies/{id}/stop", post(routes::strategies::stop_strategy))

        .route("/api/ws/prices", get(routes::websocket::ws_handler))
        .with_state(state)
        .fallback_service(ServeDir::new("static"))
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
}

pub async fn run_app() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    auth::init();

    let (alpaca_client, api_key, api_secret) = match AlpacaClient::new() {
        Ok(client) => {
            tracing::info!("Alpaca client initialized from environment variables");
            (Some(Arc::new(client) as Arc<dyn AlpacaApi>), std::env::var("ALPACA_API_KEY").ok(), std::env::var("ALPACA_API_SECRET").ok())
        }
        Err(_) => {
            tracing::info!("No Alpaca API keys in environment. Configure in Settings.");
            (None, None, None)
        }
    };

    let ws_manager = Arc::new(WsManager::new());
    let streamer = PriceStreamer::new(ws_manager.clone(), api_key, api_secret);
    streamer.start().await;

    let strategy_manager = Arc::new(StrategyManager::new());

    let state = AppState {
        alpaca: alpaca_client,
        ws_manager,
        strategy_manager,
    };

    let app = create_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
