use chrono::Utc;
use crate::logger::{SystemEvent, SystemSource, SystemEventType};
use crate::agents::broadcast_audit_log;
use crate::models::{SignalAction, StrategySignal, OptionContractSnapshot, Candle, PositionRecord, Quote, StrategyRecord};
use crate::AppState;
use async_trait::async_trait;

pub struct ParitySniperStrategy;

#[async_trait]
impl crate::strategies::TradingStrategy for ParitySniperStrategy {
    async fn evaluate(
        &self,
        state: &AppState,
        strategy: &StrategyRecord,
        _candles: &[Candle],
        quote: &Quote,
        options: &[crate::models::OptionContractSnapshot],
        position: Option<&PositionRecord>,
        kronos_score: Option<f64>,
    ) -> StrategySignal {
        evaluate_parity_sniper(state, &strategy.id, &quote.symbol, quote.price, options, position, kronos_score)
    }
}

#[allow(dead_code)]
pub fn evaluate_parity(spot: f64, call: f64, put: f64, strike: f64) -> f64 {
    ((spot + put - call) - strike).abs() / strike
}

pub fn calculate_parity_gap(spot_price: f64, call_price: f64, put_price: f64, strike: f64, dte: f64) -> f64 {
    // Interest Rate constant (e.g. 5% = 0.05)
    let r = 0.05;
    let t = dte / 365.0;
    let pv_k = strike * (-r * t).exp();
    ((spot_price + put_price - call_price) - pv_k).abs()
}

pub fn evaluate_parity_sniper(
    state: &AppState,
    _strategy_id: &str,
    symbol: &str,
    spot_price: f64,
    options: &[OptionContractSnapshot],
    position: Option<&PositionRecord>,
    kronos_score: Option<f64>,
) -> StrategySignal {
    // 1. Position Management Logic
    if let Some(pos) = position {
        // Find current parity gap for the held strike/expiry
        let matching_call = options.iter().find(|o| o.contract_symbol == pos.instrument_symbol);
        let matching_put = options.iter().find(|o| {
            o.strike == pos.strike.unwrap_or(0.0) &&
            o.expiration == pos.expiration.as_deref().unwrap_or("") &&
            o.option_type.to_lowercase() == "put"
        });

        if let (Some(call), Some(put)) = (matching_call, matching_put) {
            let cp = (call.bid.unwrap_or(0.0) + call.ask.unwrap_or(0.0)) / 2.0;
            let pp = (put.bid.unwrap_or(0.0) + put.ask.unwrap_or(0.0)) / 2.0;
            let strike = pos.strike.unwrap_or(0.0);
            
            let today = Utc::now().date_naive();
            let exp_date = chrono::NaiveDate::parse_from_str(pos.expiration.as_deref().unwrap_or(""), "%Y-%m-%d").unwrap_or(today);
            let dte = exp_date.signed_duration_since(today).num_days() as f64;
            
            let current_gap = calculate_parity_gap(spot_price, cp, pp, strike, dte);
            let current_edge = if strike > 0.0 { current_gap / strike } else { 0.0 };

            // EXIT CONDITION: Gap has closed (Edge < 0.5%)
            if current_edge < 0.005 {
                return StrategySignal {
                    action: SignalAction::Sell,
                    allocation_fraction: 1.0,
                    reason: format!("PROACTIVE EXIT: Parity gap has converged (Edge: {:.2}%)", current_edge * 100.0),
                    source: Some("PARITY_SNIPER".to_string()),
                    exit_logic: Some("Gap Convergence".to_string()),
                    ..default_signal()
                };
            }
        }
        return crate::strategies::hold("Monitoring parity gap for convergence");
    }

    // 2. Entry Scan Logic
    let mut best_gap = 0.0;
    let mut best_strike = 0.0;
    let mut best_context = String::new();

    // Pre-index puts by (strike, expiration) for O(1) lookups
    let mut puts_map = std::collections::HashMap::new();
    for contract in options {
        if (contract.strike - spot_price).abs() / spot_price > 0.05 {
            continue;
        }
        if contract.option_type.eq_ignore_ascii_case("put") {
            puts_map.insert((contract.strike.to_bits(), &contract.expiration), contract);
        }
    }

    for contract in options {
        if (contract.strike - spot_price).abs() / spot_price > 0.05 {
            continue;
        }

        if contract.option_type.eq_ignore_ascii_case("call") {
            if let Some(put) = puts_map.get(&(contract.strike.to_bits(), &contract.expiration)) {
                let call_price = contract.ask.unwrap_or(0.0);
                let put_price = put.ask.unwrap_or(0.0);

                if call_price > 0.0 && put_price > 0.0 {
                    let today = Utc::now().date_naive();
                    let exp_date = chrono::DateTime::parse_from_rfc3339(&contract.expiration).map(|d| d.date_naive()).unwrap_or(today);
                    let dte = exp_date.signed_duration_since(today).num_days() as f64;

                    let gap = calculate_parity_gap(spot_price, call_price, put_price, contract.strike, dte);
                    if gap > best_gap {
                        best_gap = gap;
                        best_strike = contract.strike;
                        best_context = format!("S:{:.2} C:{:.2} P:{:.2} K:{:.0}", spot_price, call_price, put_price, contract.strike);
                    }
                }
            }
        }
    }

    let edge_pct = if best_strike > 0.0 { best_gap / best_strike } else { 0.0 };

    // SYSTEM LOGGING: Decision Support System
    let event_type = if edge_pct >= 0.01 { SystemEventType::Signal } else { SystemEventType::Scan };
    let narrative = if edge_pct >= 0.01 {
        format!("CRITICAL: {:.2}% Parity Gap Found. Verified arbitrage path at strike {:.0}.", edge_pct * 100.0, best_strike)
    } else {
        format!("Scanning Parity: {:.2}% Gap - No Signal", edge_pct * 100.0)
    };

    broadcast_audit_log(
        state,
        SystemEvent::now(
            SystemSource::Parity,
            Some(_strategy_id.to_string()),
            symbol.to_string(),
            event_type,
            best_context,
            kronos_score.unwrap_or(0.5),
            narrative,
        ),
    );

    if edge_pct > 0.01 {
        return StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.2,
            reason: format!("ARBITRAGE: Parity Gap of {:.1}% detected at strike {:.0}", edge_pct * 100.0, best_strike),
            source: Some("PARITY_SNIPER".to_string()),
            math_edge: Some(format!("{:.2}%", edge_pct * 100.0)),
            exit_logic: Some("Parity Mean".to_string()),
            planned_exit: Some("Proactive selling upon gap convergence (<0.5%)".to_string()),
            ..default_signal()
        };
    }

    crate::strategies::hold("Monitoring for Parity Gaps")
}

fn default_signal() -> StrategySignal {
    StrategySignal {
        source: Some("PARITY_SNIPER".to_string()),
        ..Default::default()
    }
}