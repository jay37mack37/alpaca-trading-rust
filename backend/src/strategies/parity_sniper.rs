use crate::logger::{SystemEvent, SystemSource, SystemEventType};
use crate::agents::broadcast_audit_log;
use crate::models::{SignalAction, StrategySignal, OptionContractSnapshot};
use crate::AppState;

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
    kronos_score: Option<f64>,
) -> StrategySignal {
    let mut best_gap = 0.0;
    let mut best_strike = 0.0;
    let mut best_context = String::new();

    for contract in options {
        if (contract.strike - spot_price).abs() / spot_price > 0.05 {
            continue;
        }

        if contract.option_type.to_lowercase() == "call" {
            let matching_put = options.iter().find(|o| {
                o.strike == contract.strike &&
                o.expiration == contract.expiration &&
                o.option_type.to_lowercase() == "put"
            });

            if let Some(put) = matching_put {
                let call_price = contract.ask.unwrap_or(0.0);
                let put_price = put.ask.unwrap_or(0.0);

                if call_price > 0.0 && put_price > 0.0 {
                    let today = chrono::Local::now().date_naive();
                    let exp_date = chrono::NaiveDate::parse_from_str(&contract.expiration, "%Y-%m-%d").unwrap_or(today);
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
            ..default_signal()
        };
    }

    crate::strategies::hold("Monitoring for Parity Gaps")
}

fn default_signal() -> StrategySignal {
    StrategySignal {
        action: SignalAction::Hold,
        allocation_fraction: 0.0,
        reason: "".to_string(),
        limit_price: None,
        stop_loss: None,
        take_profit: None,
        trailing_stop: None,
        walk_to_mid: None,
        split_exit: None,
        log_type: None,
        new_state: None,
        source: Some("PARITY_SNIPER".to_string()),
        math_edge: None,
        ai_score: None,
        ..Default::default()
    }
}