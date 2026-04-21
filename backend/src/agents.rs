use std::time::Duration;

use chrono::Utc;
use futures_util::stream::{FuturesUnordered, StreamExt};

use crate::error::{AppError, AppResult};
use crate::models::{
    AssetClassTarget, DataProvider, ExecutionMode, RealtimeEvent, SignalAction,
};
use crate::services::broker::{resolve_alpaca_credential, sync_strategy_broker_state};
use crate::services::kronos::fetch_kronos_score;
use crate::services::providers::{
    fetch_candles, fetch_options, fetch_quote, poll_alpaca_order_until_filled,
    submit_alpaca_order,
};
use crate::services::trading::{prepare_trade, TradePreparationOutcome};
use crate::AppState;
use tracing::warn;

pub async fn run_strategy_once(
    state: &AppState,
    strategy_id: &str,
    symbol_override: Option<&str>,
) -> AppResult<Option<crate::models::TradeRecord>> {
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

    let mut last_trade = None;
    let mut tasks = FuturesUnordered::new();

    for symbol in symbols {
        let state = state.clone();
        let strategy_id = strategy_id.to_string();
        let trading_credential = trading_credential.clone();
        let data_credential = data_credential.clone();

        tasks.push(async move {
            let res: AppResult<Option<crate::models::TradeRecord>> = async {
                let quote =
                    fetch_quote(&state.http, provider, &symbol, data_credential.as_ref()).await?;
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
                let option_provider =
                    if latest_strategy.asset_class_target == AssetClassTarget::Options {
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
                    match fetch_options(&state.http, option_provider, &symbol, option_credential)
                        .await
                    {
                        Ok(fetched) => {
                            option_contracts = fetched.contracts;
                            let mut db = state.db.lock().await;
                            db.store_option_snapshots(&option_contracts, &fetched.raw_json)?;
                            db.refresh_option_position_quotes(
                                &strategy_id,
                                &symbol,
                                &option_contracts,
                            )?;
                        }
                        Err(err) => {
                            warn!("options fetch failed during strategy run for {symbol}: {err}");
                        }
                    }
                }
                let current_position = {
                    let db = state.db.lock().await;
                    db.get_position_for_underlying(
                        &strategy_id,
                        &symbol,
                        latest_strategy.asset_class_target,
                    )?
                };

                let kronos_result = fetch_kronos_score(&state.http, &symbol).await;
                let kronos_score = kronos_result.as_ref().map(|s| s.confidence).ok();

                let signal = crate::strategies::evaluate_strategy(
                    &latest_strategy,
                    &candles.candles,
                    &quote.quote,
                    current_position.as_ref(),
                    kronos_score,
                )
                .await;

                broadcast_strategy_log(
                    &state,
                    &strategy_id,
                    &symbol,
                    signal.source.as_deref().unwrap_or("HEARTBEAT"),
                    &format!("Price: ${:.2}", quote.quote.price),
                    &kronos_score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "N/A".to_string()),
                    signal.action.as_str(),
                    &signal.reason,
                );

                let prepared_trade = if matches!(signal.action, SignalAction::Hold) {
                    None
                } else if latest_strategy.execution_mode.requires_external_broker() {
                    let Some(credential) = trading_credential.as_ref() else {
                        let db = state.db.lock().await;
                        db.mark_strategy_run(&strategy_id, "Missing Alpaca trading credential")?;
                        RealtimeEvent::broadcast_notification(
                            &state.streams,
                            Some(strategy_id.as_str()),
                            "error",
                            "Missing Trading Credential",
                            "Alpaca trading credential is required for this execution mode.",
                        );
                        return Ok(None);
                    };

                    if matches!(latest_strategy.execution_mode, ExecutionMode::AlpacaLive)
                        && credential.environment != crate::models::CredentialEnvironment::Live
                    {
                        let db = state.db.lock().await;
                        db.mark_strategy_run(
                            &strategy_id,
                            "Live mode selected but credential is not live",
                        )?;
                        RealtimeEvent::broadcast_notification(
                            &state.streams,
                            Some(strategy_id.as_str()),
                            "error",
                            "Invalid Credential",
                            "Live mode selected but credential is not live.",
                        );
                        return Ok(None);
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
                            db.mark_strategy_run(&strategy_id, &reason)?;
                            return Ok(None);
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
                            db.mark_strategy_run(&strategy_id, &reason)?;
                            return Ok(None);
                        }
                    }
                };

                if let Some(prepared_trade) = prepared_trade.as_ref() {
                    if latest_strategy.execution_mode.requires_external_broker() {
                        let credential = trading_credential.as_ref().ok_or_else(|| {
                            AppError::Validation("missing Alpaca trading credential".to_string())
                        })?;
                        if let Some(order) = prepared_trade.broker_order.as_ref() {
                            let submitted = match submit_alpaca_order(
                                &state.http,
                                credential,
                                order,
                                state.config.mock_alpaca,
                            )
                            .await
                            {
                                Ok(order) => order,
                                Err(err) => {
                                    let db = state.db.lock().await;
                                    db.mark_strategy_run(
                                        &strategy_id,
                                        &format!("Alpaca order submission failed: {err}"),
                                    )?;
                                    RealtimeEvent::broadcast_notification(
                                        &state.streams,
                                        Some(strategy_id.as_str()),
                                        "error",
                                        "Order Submission Failed",
                                        &format!("Alpaca order submission failed for {symbol}: {err}"),
                                    );
                                    return Ok(None);
                                }
                            };

                            let fill = match poll_alpaca_order_until_filled(
                                &state.http,
                                credential,
                                &submitted.order_id,
                                Duration::from_secs(30),
                                state.config.mock_alpaca,
                                prepared_trade.local.quantity,
                                prepared_trade.local.price,
                            )
                            .await
                            {
                                Ok(fill) => fill,
                                Err(err) => {
                                    let db = state.db.lock().await;
                                    db.mark_strategy_run(
                                        &strategy_id,
                                        &format!("Alpaca fill reconciliation failed: {err}"),
                                    )?;
                                    RealtimeEvent::broadcast_notification(
                                        &state.streams,
                                        Some(strategy_id.as_str()),
                                        "error",
                                        "Fill Reconciliation Failed",
                                        &format!("Alpaca fill reconciliation failed for {symbol}: {err}"),
                                    );
                                    return Ok(None);
                                }
                            };

                            let mut reconciled_trade = prepared_trade.local.clone();
                            reconciled_trade.quantity = fill.filled_qty;
                            reconciled_trade.price = fill.filled_avg_price;

                            let trade = {
                                let mut db = state.db.lock().await;
                                db.store_market_snapshot(&quote.quote, &quote.raw_json)?;
                                db.execute_local_trade(
                                    &strategy_id,
                                    if latest_strategy.asset_class_target
                                        == AssetClassTarget::Options
                                    {
                                        option_provider
                                    } else {
                                        provider
                                    },
                                    latest_strategy.execution_mode,
                                    &signal,
                                    &reconciled_trade,
                                )?
                            };
                            return Ok(trade);
                        }
                    }
                }

                let trade = {
                    let mut db = state.db.lock().await;
                    db.store_market_snapshot(&quote.quote, &quote.raw_json)?;
                    if let Some(prepared_trade) = prepared_trade.as_ref() {
                        db.execute_local_trade(
                            &strategy_id,
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
                        db.mark_strategy_run(&strategy_id, &signal.reason)?;
                        None
                    }
                };

                Ok(trade)
            }
            .await;
            res
        });
    }

    while let Some(res) = tasks.next().await {
        if let Some(trade) = res? {
            last_trade = Some(trade);
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
    source: &str,
    math_edge: &str,
    ai_score: &str,
    decision: &str,
    narrative: &str,
) {
    let _ = state.streams.send_event(RealtimeEvent::Log {
        strategy_id: strategy_id.to_string(),
        symbol: symbol.to_string(),
        source: source.to_string(),
        math_edge: math_edge.to_string(),
        ai_score: ai_score.to_string(),
        decision: decision.to_string(),
        narrative: narrative.to_string(),
        time: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });
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
                RealtimeEvent::broadcast_notification(
                    &state_clone.streams,
                    Some(strategy_id_clone.as_str()),
                    "error",
                    "Agent Task Failed",
                    &format!("Strategy run failed: {err}"),
                );
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