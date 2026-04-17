use axum::routing::{delete, get, post};
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod math;
mod api;
mod auth;
mod error;
mod models;
mod routes;
mod strategies;

use api::alpaca::AlpacaClient;
use api::price_streamer::PriceStreamer;
use api::ws_manager::WsManager;
use routes::websocket::AppState;
use strategies::StrategyManager;
use std::sync::Arc;
use strategies::StrategyManager;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize auth system
    auth::init();

    // Initialize Alpaca client (for demo/fallback)
    let (alpaca_client, api_key, api_secret) = match AlpacaClient::new() {
        Ok(client) => {
            tracing::info!("Alpaca client initialized from environment variables");
            let key = std::env::var("ALPACA_API_KEY").ok();
            let secret = std::env::var("ALPACA_API_SECRET").ok();
            (Some(client), key, secret)
        }
        Err(_) => {
            tracing::info!("No Alpaca API keys in environment. Configure in Settings.");
            (None, None, None)
        }
    };

    // Initialize WebSocket Manager and Streamer
    let ws_manager = Arc::new(WsManager::new());
    let streamer = PriceStreamer::new(ws_manager.clone(), api_key, api_secret);
    streamer.start().await;

    // Initialize Strategy Manager
    let strategy_manager = Arc::new(StrategyManager::new());

    let state = AppState {
        alpaca: alpaca_client,
        ws_manager,
        strategy_manager,
    };

    // Build CORS layer for development
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // Build router with all routes
    let app = Router::new()
        // Auth routes (public)
        .route("/api/login", post(routes::auth::login))
        .route("/api/verify", get(routes::auth::verify_token))
        .route("/api/logout", post(routes::auth::logout))

        // Config routes (authenticated)
        .route("/api/config/status", get(routes::auth::get_api_key_status))
        .route("/api/config/api-keys", post(routes::auth::save_api_keys))
        .route("/api/config/password", post(routes::auth::change_password))

        // Trading routes (authenticated)
        .route("/api/account", get(routes::trading::get_account))
        .route("/api/positions", get(routes::trading::get_positions))
        .route("/api/orders", get(routes::trading::get_orders))
        .route("/api/orders", post(routes::trading::create_order))
        .route("/api/price/{symbol}", get(routes::trading::get_price))
        .route("/api/option-strikes/{symbol}", get(routes::trading::get_option_strikes))
        .route("/api/option-quote/{symbol}", get(routes::trading::get_option_price))
        .route("/api/option-chain/{symbol}", get(routes::trading::get_option_chain))

        // Order management routes (authenticated)
        .route("/api/orders/{id}", get(routes::orders::get_order_by_id))
        .route("/api/orders/{id}", delete(routes::orders::cancel_order))
        .route("/api/orders/cancel-all", post(routes::orders::cancel_all_orders))

        // Analytics routes (authenticated)
        .route("/api/analytics/watchlist", get(routes::analytics::get_watchlist))
        .route("/api/analytics/watchlist", post(routes::analytics::update_watchlist))
        .route("/api/analytics/fetch", post(routes::analytics::fetch_data))
        .route("/api/analytics/summary", get(routes::analytics::get_summary))
        .route("/api/analytics/analyze", post(routes::analytics::run_analysis))
        .route("/api/analytics/patterns", get(routes::analytics::get_patterns))

        // Strategies routes (authenticated)
        .route("/api/strategies", get(routes::strategies::list_strategies))
        .route("/api/strategies/status", get(routes::strategies::get_strategies_status))
        .route("/api/strategies/{id}/start", post(routes::strategies::start_strategy))
        .route("/api/strategies/{id}/stop", post(routes::strategies::stop_strategy))

        // WebSocket route
        .route("/api/ws/prices", get(routes::websocket::ws_handler))
        .with_state(state)
        .fallback_service(ServeDir::new("static"))
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server running on http://{}", addr);
    tracing::info!("Dashboard: http://localhost:3000/");
    tracing::info!("Network access: http://192.168.1.215:3000/");
    tracing::info!("Default login: admin / admin123");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}