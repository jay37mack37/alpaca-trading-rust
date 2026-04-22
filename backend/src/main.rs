mod agents;
mod auth;
mod config;
mod error;
mod handlers;
mod math;
mod models;
mod options;
mod services;
mod strategies;

use std::{env, net::SocketAddr, sync::Arc};

use auth::{require_token, ApiToken};
use axum::{
    http::{HeaderValue, Method},
    middleware,
    routing::{delete, get, post},
    Router,
};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;

use crate::agents::spawn_agent_loop;
use crate::models::AppConfig;
use crate::services::db::Database;
use crate::services::market::collector_loop;
use crate::services::streaming::StreamHub;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub http: Client,
    pub config: AppConfig,
    pub streams: StreamHub,
    pub agent_tasks: Arc<Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>>,
}

#[derive(Debug, Deserialize)]
pub struct RunQuery {
    pub symbol: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "autostonks_backend=info,backend=info,tower_http=info".into()),
        )
        .init();

    let config = AppConfig::from_env();
    let master_key = env::var("AUTO_STONKS_MASTER_KEY").unwrap_or_default();
    let db = Database::open(
        &config.database_path,
        &config.default_watchlist,
        &master_key,
    )?;
    drop(master_key);

    let api_token = ApiToken::load_or_generate(
        &config.database_path,
        env::var("AUTO_STONKS_API_TOKEN").ok().as_deref(),
    )?;

    let cors_origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .filter_map(|origin: &String| origin.parse::<HeaderValue>().ok())
        .collect();
    if cors_origins.is_empty() {
        return Err(anyhow::anyhow!(
            "AUTO_STONKS_ALLOWED_ORIGINS did not yield any parsable origins; set it to a comma-separated list of frontend origins"
        ));
    }
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]);

    let state = AppState {
        db: Arc::new(Mutex::new(db)),
        http: Client::builder()
            .user_agent("AutoStonksAlgoSuite/0.1")
            .build()?,
        config,
        streams: StreamHub::new(),
        agent_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
    };

    let active_strategies: Vec<String> = {
        let db = state.db.lock().await;
        db.list_strategy_records()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| s.id)
            .collect()
    };
    for strategy_id in active_strategies {
        spawn_agent_loop(state.clone(), strategy_id).await;
    }

    if state.config.polling_seconds > 0 {
        tokio::spawn(collector_loop(state.clone()));
    }

    let app = Router::new()
        .route("/api/health", get(handlers::misc::health))
        .route("/api/dashboard", get(handlers::market::dashboard))
        .route("/api/stream", get(handlers::stream::realtime_stream))
        .route(
            "/api/market/quote/:symbol",
            get(handlers::market::market_quote),
        )
        .route(
            "/api/market/candles/:symbol",
            get(handlers::market::market_candles),
        )
        .route("/api/options/:symbol", get(handlers::market::options_chain))
        .route(
            "/api/watchlists",
            get(handlers::watchlist::list_watchlists).post(handlers::watchlist::create_watchlist),
        )
        .route(
            "/api/watchlists/:id",
            axum::routing::put(handlers::watchlist::update_watchlist)
                .delete(handlers::watchlist::delete_watchlist),
        )
        .route(
            "/api/credentials",
            get(handlers::credentials::list_credentials)
                .post(handlers::credentials::create_credential),
        )
        .route(
            "/api/strategies",
            get(handlers::agents::list_strategies).post(handlers::agents::create_strategy),
        )
        .route(
            "/api/strategies/:strategy_id",
            get(handlers::agents::strategy_detail).patch(handlers::agents::update_strategy),
        )
        .route(
            "/api/strategies/:strategy_id/alpaca-sync",
            post(handlers::misc::sync_strategy_broker),
        )
        .route(
            "/api/strategies/:strategy_id/run",
            post(handlers::agents::run_strategy),
        )
        .route(
            "/api/strategies/:strategy_id/start",
            post(handlers::agents::start_strategy),
        )
        .route(
            "/api/strategies/:strategy_id/stop",
            post(handlers::agents::stop_strategy),
        )
        .route("/api/panic", post(handlers::agents::panic_all))
        .route(
            "/api/watchlist",
            post(handlers::watchlist::add_watchlist_symbol),
        )
        .route(
            "/api/watchlist/:symbol",
            delete(handlers::watchlist::remove_watchlist_symbol),
        )
        .route("/api/collect", post(handlers::misc::collect_now))
        .route(
            "/api/robinhood/ingest",
            post(handlers::misc::ingest_robinhood_data),
        )
        .route(
            "/api/analytics/patterns",
            get(handlers::analytics::run_pattern_analysis),
        )
        .layer(middleware::from_fn_with_state(
            api_token.clone(),
            require_token,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state.clone());

    let address = SocketAddr::new(state.config.host.parse()?, state.config.port);
    info!("backend listening on http://{address}");

    let listener = tokio::net::TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
