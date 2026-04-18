mod auth;
mod error;
mod models;
mod strategies;
mod services;
mod handlers;
mod config;
mod math;

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use auth::{require_token, ApiToken};
use futures_util::future::join_all;
use axum::{
    http::{HeaderValue, Method},
    middleware,
    routing::{delete, get, post},
    Router,
};
use chrono::Utc;
use crate::models::{
    AssetClassTarget, BrokerSyncState, CollectResponse, DataProvider, ExecutionMode,
    OptionContractSnapshot, OptionEntryStyle, OptionStructurePreset, RealtimeEvent,
    SignalAction, StrategyRecord, StrategySignal, TradeLeg, TradeSide, AppConfig
};
use crate::services::providers::{
    fetch_alpaca_broker_sync, fetch_candles, fetch_options, fetch_quote, submit_alpaca_order,
    AlpacaOrderLeg, AlpacaOrderRequest, AlpacaOrderType,
    poll_alpaca_order_until_filled,
};
use crate::services::db::{Database, LocalTradeInput};
use crate::services::streaming::StreamHub;
use crate::error::{AppError, AppResult};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::Mutex;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, warn};

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

#[derive(Debug, Clone)]
pub struct PreparedTrade {
    pub local: LocalTradeInput,
    pub broker_order: Option<AlpacaOrderRequest>,
}

#[derive(Debug, Clone)]
pub struct ResolvedOptionContract {
    pub contract_symbol: String,
    pub option_type: String,
    pub expiration: String,
    pub strike: f64,
    pub bid: f64,
    pub ask: f64,
    pub mark_price: f64,
    pub marketable_limit_price: f64,
}

pub enum TradePreparationOutcome {
    Ready(PreparedTrade),
    Skip(String),
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
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
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
        .route("/api/market/quote/:symbol", get(handlers::market::market_quote))
        .route("/api/market/candles/:symbol", get(handlers::market::market_candles))
        .route("/api/options/:symbol", get(handlers::market::options_chain))
        .route(
            "/api/watchlists",
            get(handlers::watchlist::list_watchlists).post(handlers::watchlist::create_watchlist),
        )
        .route(
            "/api/watchlists/:id",
            axum::routing::put(handlers::watchlist::update_watchlist).delete(handlers::watchlist::delete_watchlist),
        )
        .route(
            "/api/credentials",
            get(handlers::credentials::list_credentials).post(handlers::credentials::create_credential),
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
        .route("/api/strategies/:strategy_id/run", post(handlers::agents::run_strategy))
        .route("/api/strategies/:strategy_id/start", post(handlers::agents::start_strategy))
        .route("/api/strategies/:strategy_id/stop", post(handlers::agents::stop_strategy))
        .route("/api/panic", post(handlers::agents::panic_all))
        .route("/api/watchlist", post(handlers::watchlist::add_watchlist_symbol))
        .route("/api/watchlist/:symbol", delete(handlers::watchlist::remove_watchlist_symbol))
        .route("/api/collect", post(handlers::misc::collect_now))
        .route("/api/robinhood/ingest", post(handlers::misc::ingest_robinhood_data))
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

async fn collector_loop(state: AppState) {
    let interval = Duration::from_secs(state.config.polling_seconds);
    loop {
        tokio::time::sleep(interval).await;
        match collect_once(&state).await {
            Ok(summary) => info!(
                "collector run complete: {} market data symbols refreshed",
                summary.symbols_collected
            ),
            Err(err) => warn!("collector run failed: {err}"),
        }
    }
}

pub async fn collect_once(state: &AppState) -> AppResult<CollectResponse> {
    let tracked_symbols = {
        let db = state.db.lock().await;
        let mut symbols = db.tracked_symbols_union(&state.config.default_watchlist)?;
        symbols.extend(db.watchlist_symbols_union()?);

        let mut set = std::collections::BTreeSet::new();
        for s in symbols {
            set.insert(s);
        }
        set.into_iter().collect::<Vec<_>>()
    };

    let fetch_futures = tracked_symbols.iter().map(|symbol| async move {
        let fetched = fetch_quote(&state.http, DataProvider::Yahoo, symbol, None).await;
        (symbol, fetched)
    });

    let results = join_all(fetch_futures).await;

    for (symbol, fetched) in results {
        match fetched {
            Ok(snapshot) => {
                let db = state.db.lock().await;
                db.store_market_snapshot(&snapshot.quote, &snapshot.raw_json)?;
                db.mark_symbol_price(symbol, snapshot.quote.price)?;
            }
            Err(err) => warn!("snapshot failed for {symbol}: {err}"),
        }
    }

    Ok(CollectResponse {
        symbols_collected: tracked_symbols.len(),
        strategies_evaluated: 0,
        trades_executed: 0,
        collected_at: Utc::now().to_rfc3339(),
    })
}

pub async fn run_strategy_once(
    state: &AppState,
    strategy_id: &str,
    symbol_override: Option<&str>,
) -> AppResult<Option<models::TradeRecord>> {
    let strategy = {
        let db = state.db.lock().await;
        db.list_strategy_records()?
            .into_iter()
            .find(|strategy| strategy.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?
    };

    let symbols = if let Some(symbol) = symbol_override {
        vec![symbol.to_string()]
    } else {
        strategy.tracked_symbols.clone()
    };

    let mut last_trade = None;

    for symbol in symbols {
        let trading_credential = if strategy.execution_mode.requires_external_broker() {
            resolve_alpaca_credential(state, strategy.credential_id.as_deref(), true).await?
        } else {
            None
        };
        let data_credential = if strategy.execution_mode.requires_external_broker() {
            resolve_alpaca_credential(state, strategy.credential_id.as_deref(), false).await?
        } else {
            None
        };

        let provider = if data_credential.is_some() {
            DataProvider::Alpaca
        } else {
            DataProvider::Yahoo
        };
        let quote = fetch_quote(&state.http, provider, &symbol, data_credential.as_ref()).await?;
        let candles = fetch_candles(
            &state.http,
            provider,
            &symbol,
            "1d",
            "1m",
            data_credential.as_ref(),
        )
        .await?;

        let latest_strategy = {
            let db = state.db.lock().await;
            db.list_strategy_records()?
                .into_iter()
                .find(|candidate| candidate.id == strategy_id)
                .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?
        };
        let mut option_contracts = Vec::new();
        let option_provider = if latest_strategy.asset_class_target == AssetClassTarget::Options {
            if latest_strategy.execution_mode.requires_external_broker() {
                DataProvider::Alpaca
            } else {
                provider
            }
        } else {
            provider
        };
        if latest_strategy.asset_class_target == AssetClassTarget::Options {
            let option_credential = if option_provider == DataProvider::Alpaca {
                trading_credential.as_ref().or(data_credential.as_ref())
            } else {
                None
            };
            match fetch_options(&state.http, option_provider, &symbol, option_credential).await {
                Ok(fetched) => {
                    option_contracts = fetched.contracts;
                    let db = state.db.lock().await;
                    db.store_option_snapshots(&option_contracts, &fetched.raw_json)?;
                    db.refresh_option_position_quotes(strategy_id, &symbol, &option_contracts)?;
                }
                Err(err) => {
                    warn!("options fetch failed during strategy run for {symbol}: {err}");
                }
            }
        }
        let current_position = {
            let db = state.db.lock().await;
            db.get_position_for_underlying(
                strategy_id,
                &symbol,
                latest_strategy.asset_class_target,
            )?
        };

        let signal = strategies::evaluate_strategy(
            &latest_strategy,
            &candles.candles,
            &quote.quote,
            current_position.as_ref(),
        ).await;

        broadcast_strategy_log(
            state,
            strategy_id,
            &symbol,
            &format!("Price: ${:.2}", quote.quote.price),
            "N/A", // Score could be added later
            signal.action.as_str(),
            &signal.reason,
        );

        let prepared_trade = if matches!(signal.action, SignalAction::Hold) {
            None
        } else if latest_strategy.execution_mode.requires_external_broker() {
            let Some(credential) = trading_credential.as_ref() else {
                let db = state.db.lock().await;
                db.mark_strategy_run(strategy_id, "Missing Alpaca trading credential")?;
                continue;
            };

            if matches!(latest_strategy.execution_mode, ExecutionMode::AlpacaLive)
                && credential.environment != models::CredentialEnvironment::Live
            {
                let db = state.db.lock().await;
                db.mark_strategy_run(
                    strategy_id,
                    "Live mode selected but credential is not live",
                )?;
                continue;
            }

            match prepare_trade(
                &latest_strategy,
                current_position.as_ref(),
                &symbol,
                &quote.quote,
                &signal,
                &option_contracts,
                true,
            )? {
                TradePreparationOutcome::Ready(trade) => Some(trade),
                TradePreparationOutcome::Skip(reason) => {
                    let db = state.db.lock().await;
                    db.mark_strategy_run(strategy_id, &reason)?;
                    continue;
                }
            }
        } else {
            match prepare_trade(
                &latest_strategy,
                current_position.as_ref(),
                &symbol,
                &quote.quote,
                &signal,
                &option_contracts,
                false,
            )? {
                TradePreparationOutcome::Ready(trade) => Some(trade),
                TradePreparationOutcome::Skip(reason) => {
                    let db = state.db.lock().await;
                    db.mark_strategy_run(strategy_id, &reason)?;
                    continue;
                }
            }
        };

        if let Some(prepared_trade) = prepared_trade.as_ref() {
            if latest_strategy.execution_mode.requires_external_broker() {
                let credential = trading_credential
                    .as_ref()
                    .ok_or_else(|| AppError::Validation("missing Alpaca trading credential".to_string()))?;
                if let Some(order) = prepared_trade.broker_order.as_ref() {
                    let submitted = match submit_alpaca_order(&state.http, credential, order).await {
                        Ok(order) => order,
                        Err(err) => {
                            let db = state.db.lock().await;
                            db.mark_strategy_run(
                                strategy_id,
                                &format!("Alpaca order submission failed: {err}"),
                            )?;
                            continue;
                        }
                    };

                    let fill = match poll_alpaca_order_until_filled(
                        &state.http,
                        credential,
                        &submitted.order_id,
                        Duration::from_secs(30),
                    )
                    .await
                    {
                        Ok(fill) => fill,
                        Err(err) => {
                            let db = state.db.lock().await;
                            db.mark_strategy_run(
                                strategy_id,
                                &format!("Alpaca fill reconciliation failed: {err}"),
                            )?;
                            continue;
                        }
                    };

                    let mut reconciled_trade = prepared_trade.local.clone();
                    reconciled_trade.quantity = fill.filled_qty;
                    reconciled_trade.price = fill.filled_avg_price;

                    let trade = {
                        let db = state.db.lock().await;
                        db.store_market_snapshot(&quote.quote, &quote.raw_json)?;
                        db.execute_local_trade(
                            strategy_id,
                            if latest_strategy.asset_class_target == AssetClassTarget::Options {
                                option_provider
                            } else {
                                provider
                            },
                            latest_strategy.execution_mode,
                            &signal,
                            &reconciled_trade,
                        )?
                    };
                    if trade.is_some() {
                        last_trade = trade;
                    }
                    continue;
                }
            }
        }

        let trade = {
            let db = state.db.lock().await;
            db.store_market_snapshot(&quote.quote, &quote.raw_json)?;
            if let Some(prepared_trade) = prepared_trade.as_ref() {
                db.execute_local_trade(
                    strategy_id,
                    if latest_strategy.asset_class_target == AssetClassTarget::Options {
                        option_provider
                    } else {
                        provider
                    },
                    latest_strategy.execution_mode,
                    &signal,
                    &prepared_trade.local,
                )?
            } else {
                db.mark_strategy_run(strategy_id, &signal.reason)?;
                None
            }
        };

        if trade.is_some() {
            last_trade = trade;
        }
    }

    if strategy.execution_mode.requires_external_broker() {
        if let Err(err) = sync_strategy_broker_state(state, strategy_id).await {
            warn!("post-trade broker sync failed for {strategy_id}: {err}");
        }
    }

    Ok(last_trade)
}

pub fn broadcast_strategy_log(
    state: &AppState,
    strategy_id: &str,
    symbol: &str,
    math_edge: &str,
    kronos_score: &str,
    decision: &str,
    reasoning: &str,
) {
    let _ = state.streams.send_event(RealtimeEvent::Log {
        strategy_id: strategy_id.to_string(),
        symbol: symbol.to_string(),
        math_edge: math_edge.to_string(),
        kronos_score: kronos_score.to_string(),
        decision: decision.to_string(),
        reasoning: reasoning.to_string(),
        time: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });
}

pub async fn resolve_alpaca_credential(
    state: &AppState,
    preferred_id: Option<&str>,
    require_trading: bool,
) -> AppResult<Option<models::StoredCredential>> {
    let db = state.db.lock().await;
    db.resolve_alpaca_credential(preferred_id, require_trading)
}

pub async fn sync_strategy_broker_state(
    state: &AppState,
    strategy_id: &str,
) -> AppResult<BrokerSyncState> {
    let strategy = {
        let db = state.db.lock().await;
        db.list_strategy_records()?
            .into_iter()
            .find(|strategy| strategy.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?
    };

    if !strategy.execution_mode.requires_external_broker() {
        return Err(AppError::Validation(
            "strategy is not configured for Alpaca execution".to_string(),
        ));
    }

    let credential: Option<models::StoredCredential> = resolve_alpaca_credential(state, strategy.credential_id.as_deref(), false)
        .await?;
    let credential = credential.ok_or_else(|| AppError::Validation("missing Alpaca credential".to_string()))?;

    let fetched = fetch_alpaca_broker_sync(&state.http, &credential).await?;

    {
        let db = state.db.lock().await;
        db.store_broker_sync(
            &credential.id,
            credential.environment,
            &fetched.account,
            &fetched.positions,
            &fetched.orders,
            &fetched.raw_account,
            &fetched.raw_positions,
            &fetched.raw_orders,
        )?;
    }

    let db = state.db.lock().await;
    db.broker_sync_state(&credential.id)?
        .ok_or_else(|| AppError::Internal("broker sync did not persist".to_string()))
}

pub fn stream_matches(
    event: &RealtimeEvent,
    provider: DataProvider,
    symbol: &str,
    strategy_ids: &[String],
    credential_id: Option<&str>,
) -> bool {
    match event {
        RealtimeEvent::Market {
            provider: event_provider,
            symbol: event_symbol,
            ..
        } => *event_provider == provider && event_symbol == symbol,
        RealtimeEvent::BrokerSync {
            credential_id: event_credential_id,
            strategy_ids: event_strategy_ids,
            ..
        } => {
            credential_id == Some(event_credential_id.as_str())
                || event_strategy_ids
                    .iter()
                    .any(|event_strategy_id| strategy_ids.contains(event_strategy_id))
        }
        RealtimeEvent::Status {
            channel,
            provider: event_provider,
            symbol: event_symbol,
            ..
        } => match channel.as_str() {
            "market" => {
                event_provider == &Some(provider) && event_symbol.as_deref() == Some(symbol)
            }
            "broker" => credential_id.is_some(),
            _ => true,
        },
        RealtimeEvent::Log { strategy_id: event_strategy_id, .. } => {
            strategy_ids.contains(event_strategy_id)
        }
    }
}

pub fn prepare_trade(
    strategy: &StrategyRecord,
    position: Option<&models::PositionRecord>,
    symbol: &str,
    quote: &models::Quote,
    signal: &StrategySignal,
    option_contracts: &[OptionContractSnapshot],
    needs_broker_order: bool,
) -> AppResult<TradePreparationOutcome> {
    match strategy.asset_class_target {
        AssetClassTarget::Equity => Ok(prepare_equity_trade(
            strategy,
            position,
            symbol,
            signal,
            quote.price,
            needs_broker_order,
        )),
        AssetClassTarget::Options => prepare_option_trade(
            strategy,
            position,
            symbol,
            quote,
            signal,
            option_contracts,
            needs_broker_order,
        ),
    }
}

pub fn prepare_equity_trade(
    strategy: &StrategyRecord,
    position: Option<&models::PositionRecord>,
    symbol: &str,
    signal: &StrategySignal,
    price: f64,
    needs_broker_order: bool,
) -> TradePreparationOutcome {
    let quantity = match signal.action {
        SignalAction::Buy => {
            let notional = strategy.cash_balance * signal.allocation_fraction.clamp(0.0, 1.0);
            round_quantity(notional / price)
        }
        SignalAction::Sell => {
            let Some(position) = position else {
                return TradePreparationOutcome::Skip(
                    "Sell signal skipped: no open position".to_string(),
                );
            };
            let mut quantity = position.quantity * signal.allocation_fraction.clamp(0.0, 1.0);
            if signal.allocation_fraction >= 0.99 {
                quantity = position.quantity;
            }
            round_quantity(quantity)
        }
        SignalAction::Hold => 0.0,
    };

    if quantity <= 0.0 {
        return TradePreparationOutcome::Skip("Signal skipped: quantity rounded to zero".to_string());
    }

    let side = match signal.action {
        SignalAction::Buy => TradeSide::Buy,
        SignalAction::Sell => TradeSide::Sell,
        SignalAction::Hold => unreachable!(),
    };

    let local = LocalTradeInput {
        underlying_symbol: symbol.to_string(),
        instrument_symbol: symbol.to_string(),
        asset_type: "equity".to_string(),
        side,
        quantity,
        price,
        multiplier: 1.0,
        option_structure_preset: None,
        option_type: None,
        expiration: None,
        strike: None,
        legs: Vec::new(),
    };
    let broker_order = needs_broker_order.then_some(AlpacaOrderRequest::Single {
        symbol: symbol.to_string(),
        side,
        quantity,
        order_type: AlpacaOrderType::Market,
    });

    TradePreparationOutcome::Ready(PreparedTrade { local, broker_order })
}

pub fn top_option_contracts(
    mut contracts: Vec<models::OptionContractSnapshot>,
) -> Vec<models::OptionContractSnapshot> {
    contracts.sort_by(|left, right| {
        let left_volume = left.volume.unwrap_or_default();
        let right_volume = right.volume.unwrap_or_default();
        right_volume
            .partial_cmp(&left_volume)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    contracts.into_iter().take(20).collect()
}

pub fn round_quantity(value: f64) -> f64 {
    (value * 1000.0).floor() / 1000.0
}

pub fn prepare_option_trade(
    strategy: &StrategyRecord,
    position: Option<&models::PositionRecord>,
    symbol: &str,
    quote: &models::Quote,
    signal: &StrategySignal,
    option_contracts: &[OptionContractSnapshot],
    needs_broker_order: bool,
) -> AppResult<TradePreparationOutcome> {
    let side = match signal.action {
        SignalAction::Buy => TradeSide::Buy,
        SignalAction::Sell => TradeSide::Sell,
        SignalAction::Hold => unreachable!(),
    };

    match signal.action {
        SignalAction::Buy => {
            let Some(contract) = resolve_option_contract(strategy, quote, option_contracts) else {
                return Ok(TradePreparationOutcome::Skip(
                    "Signal skipped: no tradable option contract matched the selector"
                        .to_string(),
                ));
            };
            let option_structure_preset = Some(strategy.option_structure_preset);
            let (instrument_symbol, net_limit_price, net_mark_price, legs) =
                match strategy.option_structure_preset {
                    OptionStructurePreset::Single => (
                        contract.contract_symbol.clone(),
                        contract.marketable_limit_price,
                        contract.mark_price,
                        vec![TradeLeg {
                            instrument_symbol: contract.contract_symbol.clone(),
                            side: TradeSide::Buy,
                            ratio_quantity: 1,
                            position_intent: Some("buy_to_open".to_string()),
                            price: contract.mark_price,
                            multiplier: 100.0,
                            option_type: Some(contract.option_type.clone()),
                            expiration: Some(contract.expiration.clone()),
                            strike: Some(contract.strike),
                        }],
                    ),
                    OptionStructurePreset::BullCallSpread | OptionStructurePreset::BearPutSpread => {
                        let Some(short_leg) =
                            resolve_spread_short_leg(strategy, &contract, option_contracts)
                        else {
                            return Ok(TradePreparationOutcome::Skip(
                                "Signal skipped: no spread wing matched the configured width"
                                    .to_string(),
                            ));
                        };
                        let net_mark = (contract.mark_price - short_leg.mark_price).max(0.01);
                        let net_limit = (contract.ask * (1.0 + strategy.option_limit_buffer_pct)
                            - short_leg.bid * (1.0 - strategy.option_limit_buffer_pct))
                            .max(0.01);
                        let spread_symbol = format!(
                            "{}:{}:{}:{:.2}:{:.2}",
                            option_structure_label(strategy.option_structure_preset),
                            symbol,
                            contract.expiration,
                            contract.strike,
                            short_leg.strike
                        );
                        let legs = vec![
                            TradeLeg {
                                instrument_symbol: contract.contract_symbol.clone(),
                                side: TradeSide::Buy,
                                ratio_quantity: 1,
                                position_intent: Some("buy_to_open".to_string()),
                                price: contract.mark_price,
                                multiplier: 100.0,
                                option_type: Some(contract.option_type.clone()),
                                expiration: Some(contract.expiration.clone()),
                                strike: Some(contract.strike),
                            },
                            TradeLeg {
                                instrument_symbol: short_leg.contract_symbol.clone(),
                                side: TradeSide::Sell,
                                ratio_quantity: 1,
                                position_intent: Some("sell_to_open".to_string()),
                                price: short_leg.mark_price,
                                multiplier: 100.0,
                                option_type: Some(short_leg.option_type.clone()),
                                expiration: Some(short_leg.expiration.clone()),
                                strike: Some(short_leg.strike),
                            },
                        ];
                        (spread_symbol, net_limit, net_mark, legs)
                    }
                };
            let budget = strategy.cash_balance * signal.allocation_fraction.clamp(0.0, 1.0);
            let contract_cost = net_limit_price * 100.0;
            let quantity = if contract_cost > 0.0 {
                (budget / contract_cost).floor()
            } else {
                0.0
            };
            if quantity < 1.0 {
                return Ok(TradePreparationOutcome::Skip(
                    "Signal skipped: insufficient cash for one options contract".to_string(),
                ));
            }

            let local = LocalTradeInput {
                underlying_symbol: symbol.to_string(),
                instrument_symbol,
                asset_type: if strategy.option_structure_preset == OptionStructurePreset::Single {
                    "option".to_string()
                } else {
                    "option_spread".to_string()
                },
                side,
                quantity,
                price: net_mark_price,
                multiplier: 100.0,
                option_structure_preset,
                option_type: Some(contract.option_type.clone()),
                expiration: Some(contract.expiration.clone()),
                strike: Some(contract.strike),
                legs: legs.clone(),
            };
            let broker_order = needs_broker_order.then_some(match strategy.option_structure_preset {
                OptionStructurePreset::Single => AlpacaOrderRequest::Single {
                    symbol: contract.contract_symbol.clone(),
                    side,
                    quantity,
                    order_type: AlpacaOrderType::Limit {
                        limit_price: net_limit_price,
                    },
                },
                OptionStructurePreset::BullCallSpread | OptionStructurePreset::BearPutSpread => {
                    AlpacaOrderRequest::MultiLeg {
                        quantity: quantity as u32,
                        limit_price: net_limit_price,
                        legs: legs
                            .iter()
                            .map(|leg| AlpacaOrderLeg {
                                symbol: leg.instrument_symbol.clone(),
                                ratio_qty: leg.ratio_quantity,
                                side: leg.side,
                                position_intent: leg.position_intent.clone().unwrap_or_default(),
                            })
                            .collect(),
                    }
                }
            });
            Ok(TradePreparationOutcome::Ready(PreparedTrade { local, broker_order }))
        }
        SignalAction::Sell => {
            let Some(position) = position else {
                return Ok(TradePreparationOutcome::Skip(
                    "Sell signal skipped: no open option position".to_string(),
                ));
            };
            let (market_price, legs): (f64, Vec<TradeLeg>) = if position.asset_type == "option_spread" {
                let mut net_credit: f64 = 0.0;
                let mut close_legs = Vec::with_capacity(position.legs.len());
                for leg in &position.legs {
                    let snapshot = option_contracts
                        .iter()
                        .find(|contract| contract.contract_symbol == leg.instrument_symbol);
                    let Some(snapshot) = snapshot else {
                        return Ok(TradePreparationOutcome::Skip(
                            "Sell signal skipped: spread leg quote unavailable".to_string(),
                        ));
                    };
                    let (leg_side, position_intent, leg_price, sign) = if leg.position_side == "short"
                    {
                        let ask = snapshot.ask.ok_or_else(|| {
                            AppError::Validation(
                                "Sell signal skipped: spread short leg ask unavailable".to_string(),
                            )
                        })?;
                        (
                            TradeSide::Buy,
                            "buy_to_close".to_string(),
                            ask * (1.0 + strategy.option_limit_buffer_pct),
                            -1.0,
                        )
                    } else {
                        let bid = snapshot.bid.ok_or_else(|| {
                            AppError::Validation(
                                "Sell signal skipped: spread long leg bid unavailable".to_string(),
                            )
                        })?;
                        (
                            TradeSide::Sell,
                            "sell_to_close".to_string(),
                            bid * (1.0 - strategy.option_limit_buffer_pct),
                            1.0,
                        )
                    };
                    net_credit += sign * leg_price;
                    close_legs.push(TradeLeg {
                        instrument_symbol: leg.instrument_symbol.clone(),
                        side: leg_side,
                        ratio_quantity: leg.ratio_quantity,
                        position_intent: Some(position_intent),
                        price: snapshot
                            .last
                            .unwrap_or((snapshot.bid.unwrap_or_default() + snapshot.ask.unwrap_or_default()) / 2.0),
                        multiplier: leg.multiplier,
                        option_type: leg.option_type.clone(),
                        expiration: leg.expiration.clone(),
                        strike: leg.strike,
                    });
                }
                (net_credit.max(0.01), close_legs)
            } else {
                let market_price = option_contracts
                    .iter()
                    .find(|contract| contract.contract_symbol == position.instrument_symbol)
                    .and_then(|contract| option_mark_price(contract, side, strategy.option_limit_buffer_pct))
                    .unwrap_or(position.market_price);
                (
                    market_price,
                    vec![TradeLeg {
                        instrument_symbol: position.instrument_symbol.clone(),
                        side,
                        ratio_quantity: 1,
                        position_intent: Some("sell_to_close".to_string()),
                        price: market_price,
                        multiplier: position.multiplier,
                        option_type: position.option_type.clone(),
                        expiration: position.expiration.clone(),
                        strike: position.strike,
                    }],
                )
            };
            let mut quantity = position.quantity * signal.allocation_fraction.clamp(0.0, 1.0);
            if signal.allocation_fraction >= 0.99 {
                quantity = position.quantity;
            }
            quantity = quantity.floor();
            if quantity < 1.0 {
                return Ok(TradePreparationOutcome::Skip(
                    "Sell signal skipped: quantity rounded below one contract".to_string(),
                ));
            }

            let local = LocalTradeInput {
                underlying_symbol: position.underlying_symbol.clone(),
                instrument_symbol: position.instrument_symbol.clone(),
                asset_type: position.asset_type.clone(),
                side,
                quantity,
                price: market_price,
                multiplier: position.multiplier.max(100.0),
                option_structure_preset: position.option_structure_preset,
                option_type: position.option_type.clone(),
                expiration: position.expiration.clone(),
                strike: position.strike,
                legs: legs.clone(),
            };
            let broker_order =
                needs_broker_order.then_some(match position.option_structure_preset.unwrap_or(
                    OptionStructurePreset::Single,
                ) {
                    OptionStructurePreset::Single => AlpacaOrderRequest::Single {
                        symbol: position.instrument_symbol.clone(),
                        side,
                        quantity,
                        order_type: AlpacaOrderType::Limit {
                            limit_price: market_price.max(0.01),
                        },
                    },
                    OptionStructurePreset::BullCallSpread | OptionStructurePreset::BearPutSpread => {
                        AlpacaOrderRequest::MultiLeg {
                            quantity: quantity as u32,
                            limit_price: market_price.max(0.01),
                            legs: legs
                                .iter()
                                .map(|leg| AlpacaOrderLeg {
                                    symbol: leg.instrument_symbol.clone(),
                                    ratio_qty: leg.ratio_quantity,
                                    side: leg.side,
                                    position_intent: leg.position_intent.clone().unwrap_or_default(),
                                })
                                .collect(),
                        }
                    }
                });
            Ok(TradePreparationOutcome::Ready(PreparedTrade { local, broker_order }))
        }
        SignalAction::Hold => unreachable!(),
    }
}

pub fn resolve_option_contract(
    strategy: &StrategyRecord,
    quote: &models::Quote,
    option_contracts: &[OptionContractSnapshot],
) -> Option<ResolvedOptionContract> {
    let target_type = match strategy.option_structure_preset {
        OptionStructurePreset::BullCallSpread => "call",
        OptionStructurePreset::BearPutSpread => "put",
        OptionStructurePreset::Single => match strategy.option_entry_style {
            OptionEntryStyle::LongCall => "call",
            OptionEntryStyle::LongPut => "put",
        },
    };
    let today = Utc::now().date_naive();
    let mut candidates = option_contracts
        .iter()
        .filter_map(|contract| {
            if contract.option_type != target_type {
                return None;
            }
            let expiration = chrono::DateTime::parse_from_rfc3339(&contract.expiration)
                .ok()?
                .date_naive();
            let dte = expiration.signed_duration_since(today).num_days();
            if dte < strategy.option_dte_min as i64 || dte > strategy.option_dte_max as i64 {
                return None;
            }
            let bid = contract.bid?;
            let ask = contract.ask?;
            if bid <= 0.0 || ask <= 0.0 || ask < bid {
                return None;
            }
            let mid = (bid + ask) / 2.0;
            if mid <= 0.0 {
                return None;
            }
            let spread_pct = (ask - bid) / mid;
            if spread_pct > strategy.option_max_spread_pct {
                return None;
            }
            let delta_score = contract
                .delta
                .map(|delta: f64| (delta.abs() - strategy.option_target_delta).abs())
                .unwrap_or_else(|| fallback_delta_score(contract, quote.price, target_type));
            let dte_midpoint = (strategy.option_dte_min as f64 + strategy.option_dte_max as f64) / 2.0;
            let dte_score = ((dte as f64) - dte_midpoint).abs();

            Some((
                delta_score,
                spread_pct,
                dte_score,
                ResolvedOptionContract {
                    contract_symbol: contract.contract_symbol.clone(),
                    option_type: contract.option_type.clone(),
                    expiration: contract.expiration.clone(),
                    strike: contract.strike,
                    bid,
                    ask,
                    mark_price: mid,
                    marketable_limit_price: (ask * (1.0 + strategy.option_limit_buffer_pct)).max(0.01),
                },
            ))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1.partial_cmp(&right.1).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| left.2.partial_cmp(&right.2).unwrap_or(std::cmp::Ordering::Equal))
    });
    candidates.into_iter().map(|(_, _, _, contract)| contract).next()
}

pub fn resolve_spread_short_leg(
    strategy: &StrategyRecord,
    long_leg: &ResolvedOptionContract,
    option_contracts: &[OptionContractSnapshot],
) -> Option<ResolvedOptionContract> {
    let mut candidates = option_contracts
        .iter()
        .filter_map(|contract| {
            if contract.option_type != long_leg.option_type || contract.expiration != long_leg.expiration
            {
                return None;
            }
            let valid_strike = match strategy.option_structure_preset {
                OptionStructurePreset::BullCallSpread => contract.strike > long_leg.strike,
                OptionStructurePreset::BearPutSpread => contract.strike < long_leg.strike,
                OptionStructurePreset::Single => false,
            };
            if !valid_strike {
                return None;
            }
            let bid = contract.bid?;
            let ask = contract.ask?;
            if bid <= 0.0 || ask <= 0.0 || ask < bid {
                return None;
            }
            let mid = (bid + ask) / 2.0;
            if mid <= 0.0 {
                return None;
            }
            let width_error = match strategy.option_structure_preset {
                OptionStructurePreset::BullCallSpread => {
                    ((contract.strike - long_leg.strike) - strategy.option_spread_width).abs()
                }
                OptionStructurePreset::BearPutSpread => {
                    ((long_leg.strike - contract.strike) - strategy.option_spread_width).abs()
                }
                OptionStructurePreset::Single => f64::MAX,
            };
            Some((
                width_error,
                ResolvedOptionContract {
                    contract_symbol: contract.contract_symbol.clone(),
                    option_type: contract.option_type.clone(),
                    expiration: contract.expiration.clone(),
                    strike: contract.strike,
                    bid,
                    ask,
                    mark_price: mid,
                    marketable_limit_price: (ask * (1.0 + strategy.option_limit_buffer_pct))
                        .max(0.01),
                },
            ))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.into_iter().map(|(_, contract)| contract).next()
}

pub fn option_structure_label(preset: OptionStructurePreset) -> &'static str {
    match preset {
        OptionStructurePreset::Single => "single",
        OptionStructurePreset::BullCallSpread => "bull_call_spread",
        OptionStructurePreset::BearPutSpread => "bear_put_spread",
    }
}

pub fn fallback_delta_score(contract: &OptionContractSnapshot, underlying_price: f64, target_type: &str) -> f64 {
    if underlying_price <= 0.0 {
        return 1.0;
    }
    let desired_strike = if target_type == "call" {
        contract.strike.max(underlying_price)
    } else {
        contract.strike.min(underlying_price)
    };
    ((desired_strike - underlying_price) / underlying_price).abs()
}

pub fn option_mark_price(
    contract: &OptionContractSnapshot,
    side: TradeSide,
    buffer_pct: f64,
) -> Option<f64> {
    let bid = contract.bid?;
    let ask = contract.ask?;
    if bid <= 0.0 || ask <= 0.0 || ask < bid {
        return None;
    }
    Some(match side {
        TradeSide::Buy => (ask * (1.0 + buffer_pct)).max(0.01),
        TradeSide::Sell => (bid * (1.0 - buffer_pct)).max(0.01),
    })
}

pub async fn spawn_agent_loop(state: AppState, strategy_id: String) {
    let mut tasks = state.agent_tasks.lock().await;
    if let Some(handle) = tasks.remove(&strategy_id) {
        handle.abort();
    }

    let state_clone = state.clone();
    let strategy_id_clone = strategy_id.clone();

    let handle = tokio::spawn(async move {
        loop {
            let interval_ms = {
                let db = state_clone.db.lock().await;
                if let Ok(records) = db.list_strategy_records() {
                    if let Some(strat) = records.into_iter().find(|s| s.id == strategy_id_clone) {
                        if !strat.enabled {
                            break;
                        }
                        strat.run_interval_ms as u64
                    } else {
                        break;
                    }
                } else {
                    30000
                }
            };

            if let Err(err) = run_strategy_once(&state_clone, &strategy_id_clone, None).await {
                tracing::error!("Agent {strategy_id_clone} failed its task run: {err}");
            }

            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    });

    tasks.insert(strategy_id, handle);
}

pub async fn abort_agent_loop(state: &AppState, strategy_id: &str) {
    let mut tasks = state.agent_tasks.lock().await;
    if let Some(handle) = tasks.remove(strategy_id) {
        handle.abort();
    }
}
