use axum::routing::{get, post};
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod models;
mod routes;

use api::alpaca::AlpacaClient;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize Alpaca client
    let alpaca_client = match AlpacaClient::new() {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("Failed to create Alpaca client: {}", e);
            tracing::info!("Running in demo mode without Alpaca API connection");
            panic!("Set ALPACA_API_KEY and ALPACA_API_SECRET in .env file");
        }
    };

    // Build CORS layer for development
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    // Build router with all routes
    let app = Router::new()
        // API routes
        .route("/api/account", get(routes::trading::get_account))
        .route("/api/positions", get(routes::trading::get_positions))
        .route("/api/orders", get(routes::trading::get_orders))
        .route("/api/orders", post(routes::trading::create_order))
        // Static files
        .nest_service("/static", ServeDir::new("static"))
        // Fallback to index.html for SPA
        .fallback_service(ServeDir::new("static"))
        .with_state(alpaca_client)
        .layer(cors);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server running on http://{}", addr);
    tracing::info!("Dashboard available at http://{}/index.html", addr);
    tracing::info!("Network access: http://192.168.1.215:3000");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}