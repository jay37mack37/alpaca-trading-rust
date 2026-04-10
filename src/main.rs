use alpaca_trading3web::api::alpaca::AlpacaClient;
use alpaca_trading3web::api::price_streamer::PriceStreamer;
use alpaca_trading3web::api::ws_manager::WsManager;
use alpaca_trading3web::{auth, build_app, AppState};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    let state = AppState {
        alpaca: alpaca_client,
        ws_manager,
    };

    let app = build_app(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server running on http://{}", addr);
    tracing::info!("Dashboard: http://localhost:3000/");
    tracing::info!("Network access: http://192.168.1.215:3000/");
    tracing::info!("Default login: admin / admin123");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
