use crate::error::{AppError, AppResult};
use crate::models::{
    OptionContractSnapshot, OptionEntryStyle, OptionStructurePreset, SignalAction, StrategyRecord,
    StrategySignal, TradeLeg, TradeSide,
};
use crate::services::db::LocalTradeInput;
use crate::services::providers::{AlpacaOrderLeg, AlpacaOrderRequest, AlpacaOrderType};
use chrono::Utc;

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

pub fn prepare_trade(
    strategy: &StrategyRecord,
    position: Option<&crate::models::PositionRecord>,
    symbol: &str,
    quote: &crate::models::Quote,
    signal: &StrategySignal,
    option_contracts: &[OptionContractSnapshot],
    needs_broker_order: bool,
) -> AppResult<TradePreparationOutcome> {
    match strategy.asset_class_target {
        crate::models::AssetClassTarget::Equity => Ok(prepare_equity_trade(
            strategy,
            position,
            symbol,
            signal,
            quote.price,
            needs_broker_order,
        )),
        crate::models::AssetClassTarget::Options => prepare_option_trade(
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
    position: Option<&crate::models::PositionRecord>,
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
        return TradePreparationOutcome::Skip(
            "Signal skipped: quantity rounded to zero".to_string(),
        );
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
        take_profit: signal.take_profit,
        exit_logic: signal.exit_logic.clone(),
        buy_logic: Some(signal.reason.clone()),
        entry_math: signal.math_edge.clone(),
        entry_ai: signal.ai_score.as_ref().and_then(|s| s.parse::<f64>().ok()),
        hold_intent: signal.hold_intent.clone(),
        planned_exit: signal.planned_exit.clone(),
    };
    let broker_order = needs_broker_order.then_some(AlpacaOrderRequest::Single {
        symbol: symbol.to_string(),
        side,
        quantity,
        order_type: AlpacaOrderType::Market,
    });

    TradePreparationOutcome::Ready(PreparedTrade {
        local,
        broker_order,
    })
}

pub fn prepare_option_trade(
    strategy: &StrategyRecord,
    position: Option<&crate::models::PositionRecord>,
    symbol: &str,
    quote: &crate::models::Quote,
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
            let Some(contract) = resolve_option_contract(strategy, signal, quote, option_contracts)
            else {
                return Ok(TradePreparationOutcome::Skip(
                    "Signal skipped: no tradable option contract matched the selector".to_string(),
                ));
            };
            let option_structure_preset = Some(strategy.option_structure_preset);
            let (instrument_symbol, net_limit_price, net_mark_price, legs) = match strategy
                .option_structure_preset
            {
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
                take_profit: signal.take_profit,
                exit_logic: signal.exit_logic.clone(),
                buy_logic: Some(signal.reason.clone()),
                entry_math: signal.math_edge.clone(),
                entry_ai: signal.ai_score.as_ref().and_then(|s| s.parse::<f64>().ok()),
                hold_intent: signal.hold_intent.clone(),
                planned_exit: signal.planned_exit.clone(),
            };
            let broker_order =
                needs_broker_order.then_some(match strategy.option_structure_preset {
                    OptionStructurePreset::Single => AlpacaOrderRequest::Single {
                        symbol: contract.contract_symbol.clone(),
                        side,
                        quantity,
                        order_type: AlpacaOrderType::Limit {
                            limit_price: net_limit_price,
                        },
                    },
                    OptionStructurePreset::BullCallSpread
                    | OptionStructurePreset::BearPutSpread => AlpacaOrderRequest::MultiLeg {
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
                    },
                });
            Ok(TradePreparationOutcome::Ready(PreparedTrade {
                local,
                broker_order,
            }))
        }
        SignalAction::Sell => {
            let Some(position) = position else {
                return Ok(TradePreparationOutcome::Skip(
                    "Sell signal skipped: no open option position".to_string(),
                ));
            };
            let (market_price, legs): (f64, Vec<TradeLeg>) = if position.asset_type
                == "option_spread"
            {
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
                    let (leg_side, position_intent, leg_price, sign) = if leg.position_side
                        == "short"
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
                        price: snapshot.last.unwrap_or(
                            (snapshot.bid.unwrap_or_default() + snapshot.ask.unwrap_or_default())
                                / 2.0,
                        ),
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
                    .and_then(|contract| {
                        option_mark_price(contract, side, strategy.option_limit_buffer_pct)
                    })
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
                take_profit: signal.take_profit,
                exit_logic: signal.exit_logic.clone(),
                buy_logic: None,
                entry_math: None,
                entry_ai: None,
                hold_intent: None,
                planned_exit: None,
            };
            let broker_order = needs_broker_order.then_some(
                match position
                    .option_structure_preset
                    .unwrap_or(OptionStructurePreset::Single)
                {
                    OptionStructurePreset::Single => AlpacaOrderRequest::Single {
                        symbol: position.instrument_symbol.clone(),
                        side,
                        quantity,
                        order_type: AlpacaOrderType::Limit {
                            limit_price: market_price.max(0.01),
                        },
                    },
                    OptionStructurePreset::BullCallSpread
                    | OptionStructurePreset::BearPutSpread => AlpacaOrderRequest::MultiLeg {
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
                    },
                },
            );
            Ok(TradePreparationOutcome::Ready(PreparedTrade {
                local,
                broker_order,
            }))
        }
        SignalAction::Hold => unreachable!(),
    }
}

pub fn resolve_option_contract(
    strategy: &StrategyRecord,
    signal: &StrategySignal,
    quote: &crate::models::Quote,
    option_contracts: &[OptionContractSnapshot],
) -> Option<ResolvedOptionContract> {
    let entry_style = signal
        .option_entry_style
        .unwrap_or(strategy.option_entry_style);
    let target_type = match strategy.option_structure_preset {
        OptionStructurePreset::BullCallSpread => "call",
        OptionStructurePreset::BearPutSpread => "put",
        OptionStructurePreset::Single => match entry_style {
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
            let dte_midpoint =
                (strategy.option_dte_min as f64 + strategy.option_dte_max as f64) / 2.0;
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
            .then_with(|| {
                left.1
                    .partial_cmp(&right.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                left.2
                    .partial_cmp(&right.2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    candidates
        .into_iter()
        .map(|(_, _, _, contract)| contract)
        .next()
}

pub fn resolve_spread_short_leg(
    strategy: &StrategyRecord,
    long_leg: &ResolvedOptionContract,
    option_contracts: &[OptionContractSnapshot],
) -> Option<ResolvedOptionContract> {
    let mut candidates = option_contracts
        .iter()
        .filter_map(|contract| {
            if contract.option_type != long_leg.option_type
                || contract.expiration != long_leg.expiration
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

pub fn top_option_contracts(
    mut contracts: Vec<OptionContractSnapshot>,
) -> Vec<OptionContractSnapshot> {
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

pub fn option_structure_label(preset: OptionStructurePreset) -> &'static str {
    match preset {
        OptionStructurePreset::Single => "single",
        OptionStructurePreset::BullCallSpread => "bull_call_spread",
        OptionStructurePreset::BearPutSpread => "bear_put_spread",
    }
}

pub fn fallback_delta_score(
    contract: &OptionContractSnapshot,
    underlying_price: f64,
    target_type: &str,
) -> f64 {
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
