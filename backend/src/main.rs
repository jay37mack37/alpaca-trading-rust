mod agents;
mod auth;
mod logger;
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
use chrono::Utc;
use std::time::Duration;
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
    pub api_token: Arc<String>,
    pub risk_engine: Arc<services::risk::RiskEngine>,
    pub analytics: Arc<services::analytics::AnalyticsService>,
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

    dotenvy::dotenv().ok();

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

    let db_shared = Arc::new(Mutex::new(db));
    let state = AppState {
        db: db_shared.clone(),
        http: Client::builder()
            .user_agent("AutoStonksAlgoSuite/0.1")
            .build()?,
        config,
        streams: StreamHub::new(),
        agent_tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        api_token: api_token.token().clone(),
        risk_engine: Arc::new(services::risk::RiskEngine::new()),
        analytics: Arc::new(services::analytics::AnalyticsService::new(db_shared.clone())),
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

    // Global Heartbeat and Connectivity Audit Loop
    let state_hb = state.clone();
    tokio::spawn(async move {
        use crate::models::RealtimeEvent;
        use crate::logger::{SystemEvent, SystemSource, SystemEventType};
        use crate::agents::broadcast_audit_log;
        use crate::services::providers::fetch_alpaca_broker_sync;
        use crate::services::kronos::fetch_kronos_score;
        use crate::services::broker::resolve_alpaca_credential;

        loop {
            // 1. Audit Kronos
            let kronos_status = match fetch_kronos_score(&state_hb.http, "SPY").await {
                Ok(score) => format!("CONNECTED | Signal: {} ({:.1}%)", score.trend, score.confidence * 100.0),
                Err(_) => "OFFLINE | Using fallback internal probability".to_string(),
            };

            // 2. Audit Alpaca (Fetch system-wide buying power)
            let mut buying_power = 0.0;
            let mut alpaca_status = "STBY".to_string();
            
            if state_hb.config.mock_alpaca {
                buying_power = 100000.0;
                alpaca_status = "LIVE (MOCK)".to_string();
            } else {
                // Try to get buying power using ANY available alpaca credential
                if let Ok(Some(cred)) = resolve_alpaca_credential(&state_hb, None, false).await {
                    if let Ok(sync) = fetch_alpaca_broker_sync(&state_hb.http, &cred, false).await {
                        buying_power = sync.account.buying_power.unwrap_or(0.0);
                        alpaca_status = "LIVE".to_string();
                    } else {
                        alpaca_status = "ERROR | Connectivity".to_string();
                    }
                }
            }

            // 3. Audit Options Chain (Fetch SPY options for sanity check)
            let mut options_active = false;
            if let Ok(strats) = state_hb.db.lock().await.list_strategy_records() {
                if let Some(_strat) = strats.into_iter().find(|s| s.enabled) {
                match crate::services::providers::fetch_options(
                    &state_hb.http,
                    crate::models::DataProvider::Yahoo,
                    "SPY",
                    None
                ).await {
                    Ok(_) => options_active = true,
                    Err(e) => {
                        // Only log if it was previously active to avoid noise
                        tracing::warn!("Options heartbeat check failed for SPY: {:?}", e);
                    }
                }
                }
            }

            // Fallback for Options connectivity in Mock Mode
            if state_hb.config.mock_alpaca && !options_active {
               options_active = true; 
            }

            let execution_profile = {
                let db = state_hb.db.lock().await;
                db.get_global_execution_profile()
            };

            // 4. Broadcast Heartbeat (for UI indicator)
            let _ = state_hb.streams.send_event(RealtimeEvent::Heartbeat { 
                timestamp: Utc::now().timestamp_millis() as u64,
                buying_power,
                kronos_active: kronos_status.contains("CONNECTED") || kronos_status.contains("SIM"),
                alpaca_active: alpaca_status.contains("LIVE"),
                options_active,
                execution_profile,
            });

            // 5. Audit Log
            broadcast_audit_log(
                &state_hb,
                SystemEvent::now(
                    "SYSTEM".to_string(),
                    None,
                    "SYS".to_string(),
                    SystemEventType::Scan,
                    format!("BP:${:.0}", buying_power),
                    0.0,
                    format!("Kronos: {} | Alpaca: {} | Options: {}", kronos_status, alpaca_status, if options_active { "LINKED" } else { "ERROR" }),
                    None,
                )
            );

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // One-time reconciliation for "old" positions to move to history
    {
        let db = state.db.lock().await;
        if let Err(e) = db.reconcile_all_to_history() {
            tracing::error!("Bulk reconciliation failed: {:?}", e);
        }
    }

    // High-Frequency Position Update Loop (10Hz)
    let state_pos = state.clone();
    tokio::spawn(async move {
        loop {
            let (positions, broker_positions) = {
                let db = state_pos.db.lock().await;
                (
                    db.list_all_open_positions().unwrap_or_default(),
                    db.list_all_broker_positions()
                        .unwrap_or_default()
                        .into_iter()
                        .flat_map(|(_, positions)| positions)
                        .collect::<Vec<_>>()
                )
            };
            
            if !positions.is_empty() || !broker_positions.is_empty() {
                let _ = state_pos.streams.send_event(crate::models::RealtimeEvent::Positions { 
                    positions,
                    broker_positions,
                });
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let app = Router::new()
        .route("/api/health", get(handlers::misc::health))
        .route("/api/setup/status", get(handlers::setup::setup_status))
        .route("/api/setup/env", post(handlers::setup::write_env))
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
            "/api/strategies/:strategy_id/logs",
            get(handlers::agents::get_strategy_logs),
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
        .route(
            "/api/strategies/:strategy_id/positions/:symbol/flatten",
            post(handlers::agents::flatten_strategy_position),
        )
        .route(
            "/api/strategies/trades/:trade_id/hide",
            delete(handlers::agents::hide_trade),
        )
        .route("/api/strategies/panic", post(handlers::agents::panic_all))
        .route("/api/broker/liquidate-all", post(handlers::agents::liquidate_all_broker_positions))
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
            "/api/analytics/performance",
            get(handlers::analytics::get_strategy_performance),
        )
        .route(
            "/api/analytics/patterns",
            get(handlers::analytics::run_pattern_analysis),
        )
        .route("/api/config/profile", post(handlers::misc::set_global_profile))
        .layer(middleware::from_fn_with_state(
            api_token.clone(),
            require_token,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state.clone());

    let address = SocketAddr::new(state.config.host.parse()?, state.config.port);
    println!("🚀 AutoStonks Trade Engine is initializing...");
    println!("📡 API Server listening on: http://{}", address);
    
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("✅ Web server is READY for requests!");
    axum::serve(listener, app).await?;
    Ok(())
}
