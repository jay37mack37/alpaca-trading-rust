use chrono::{Utc, Timelike};
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
        net_gex: Option<f64>,
    ) -> StrategySignal {
        evaluate_parity_sniper(state, &strategy.id, &quote.symbol, quote.price, options, position, kronos_score, net_gex, strategy.performance_stats_json.clone(), strategy.execution_profile)
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
    _net_gex: Option<f64>,
    performance_stats: Option<String>,
    profile: crate::models::ExecutionProfile,
) -> StrategySignal {
    // 0. Learning Loop Feedback
    let best_hours = performance_stats.and_then(|json| {
        let stats: serde_json::Value = serde_json::from_str(&json).ok()?;
        stats.get("best_hours").and_then(|h| h.as_array()).map(|arr| {
            arr.iter().filter_map(|v| v.as_u64().map(|u| u as u32)).collect::<Vec<u32>>()
        })
    });

    // 0. Timezone Correction (Force Eastern Time for Trading Windows)
    // UTC 15:00 -> EDT 11:00
    let current_hour = (Utc::now().hour() + 20) % 24; 
    
    let is_suboptimal_window = false; // Force optimal during observation phase
    // 1. Position Management Logic
    if let Some(pos) = position {
        // Cool-down: Don't exit within the first 60 seconds unless profit is significant
        let entry_time = chrono::DateTime::parse_from_rfc3339(pos.entry_time.as_deref().unwrap_or("")).ok();
        let hold_seconds = entry_time.map(|t| Utc::now().signed_duration_since(t.with_timezone(&Utc)).num_seconds()).unwrap_or(0);
        
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
            let exp_date = chrono::DateTime::parse_from_rfc3339(&call.expiration).map(|d| d.date_naive()).unwrap_or(today);
            let dte = exp_date.signed_duration_since(today).num_days() as f64;
            
            let r = 0.05;
            let t = dte / 365.0;
            let pv_k = strike * (-r * t).exp();
            
            let current_gap_val = (spot_price + pp - cp) - pv_k;
            let current_edge = if strike > 0.0 { current_gap_val / strike } else { 0.0 };

            // EXIT CONDITION: Gap has closed (Edge < 0.25%) AND we've held for at least 60s
            // Or if we have a significant profit despite the hold time
            let is_stop_loss = current_edge < -0.01; // Exit if gap flips hard against us
            
            if (current_edge < 0.0025 && hold_seconds > 60) || is_stop_loss {
                return StrategySignal {
                    action: SignalAction::Sell,
                    allocation_fraction: 1.0,
                    reason: format!("PROACTIVE EXIT: Parity gap has converged (Edge: {:.2}%, Hold: {}s)", current_edge * 100.0, hold_seconds),
                    source: Some("PARITY_SNIPER".to_string()),
                    exit_logic: Some(if is_stop_loss { "Gap Inversion".to_string() } else { "Gap Convergence".to_string() }),
                    ..default_signal()
                };
            }

            return StrategySignal {
                action: SignalAction::Hold,
                reason: format!("MONITORING: Parity gap is {:.2}% (Target: <0.25%)", current_edge * 100.0),
                hold_intent: Some(format!("Waiting for gap convergence or inversion ({:.2}% current)", current_edge * 100.0)),
                planned_exit: Some(format!("Sell at strike {:.0} once gap converges to <0.25%", strike)),
                ..default_signal()
            };
        }
        return crate::strategies::hold("Monitoring parity gap for convergence");
    }

    // 2. Entry Scan Logic
    let mut best_gap = 0.0;
    let mut best_strike = 0.0;
    let mut best_target_symbol: Option<String> = None;
    let mut best_target_type: Option<String> = None;
    let mut best_context = String::new();

    // Market Regime Filter
    let gex_val = _net_gex.unwrap_or(0.0);
    let is_volatile = gex_val.abs() > 1_000_000.0; // High GEX extension

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

            if let (Some(put), Some(kronos)) = (matching_put, kronos_score) {
                // EXCEPTIONALLY IMPORTANT: DTE Filter based on Profile
                let expiration_dt = chrono::DateTime::parse_from_rfc3339(&contract.expiration).ok();
                let dte = expiration_dt.map(|dt| dt.date_naive().signed_duration_since(chrono::Utc::now().date_naive()).num_days()).unwrap_or(999);
                
                let (max_dte, max_cost) = match profile {
                    crate::models::ExecutionProfile::Sniper0Dte => (0, 2.50), // 0DTE only, $250 max cost per contract
                    crate::models::ExecutionProfile::Standard => (45, 50.0), // Standard ranges
                };

                if dte > max_dte {
                    continue; // Skip if beyond profile expiration limit
                }
                let call_price = contract.ask.unwrap_or(0.0);
                let put_price = put.ask.unwrap_or(0.0);

                if call_price > 0.0 && put_price > 0.0 {
                    let expiration = chrono::DateTime::parse_from_rfc3339(&contract.expiration)
                        .ok()
                        .map(|dt| dt.date_naive());
                    
                    if let Some(exp_date) = expiration {
                        let call_price = contract.ask.unwrap_or(0.0);
                        if call_price > max_cost {
                            continue; // Skip contracts that exceed profile budget
                        }
                        let today = chrono::Utc::now().date_naive();
                        let dte = exp_date.signed_duration_since(today).num_days();
                        let r = 0.05f64;
                        let t = dte as f64 / 365.0;
                        let pv_k = (contract.strike as f64) * f64::exp(-r * t);
                        
                        // Gap Calculation
                        // Call side undervalued: Spot + Put > Call + PV(K)
                        let call_gap = (spot_price + put_price) - (call_price + pv_k);
                        // Put side undervalued: Call + PV(K) > Spot + Put
                        let put_gap = (call_price + pv_k) - (spot_price + put_price);
                        
                        let call_edge = call_gap / contract.strike;
                        let put_edge = put_gap / contract.strike;

                        if call_edge > best_gap && kronos > 0.6 {
                            best_gap = call_edge;
                            best_strike = contract.strike;
                            best_target_symbol = Some(contract.contract_symbol.clone());
                            best_target_type = Some("CALL".to_string());
                            best_context = format!("CALL ARB | Gap:{:.2} S:{:.2} K:{:.0} K:{:.2}", call_edge * 100.0, spot_price, contract.strike, kronos);
                        } else if put_edge > best_gap && kronos < 0.4 {
                            best_gap = put_edge;
                            best_strike = contract.strike;
                            best_target_symbol = Some(put.contract_symbol.clone());
                            best_target_type = Some("PUT".to_string());
                            best_context = format!("PUT ARB | Gap:{:.2} S:{:.2} K:{:.0}", put_gap, spot_price, contract.strike);
                        }
                    }
                }
            }
        }
    }

    let edge_pct = best_gap;

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
            "PARITY_SNIPER".to_string(),
            Some(_strategy_id.to_string()),
            symbol.to_string(),
            event_type,
            format!("Gap: {:.2}%", edge_pct * 100.0),
            kronos_score.unwrap_or(0.5),
            narrative,
            Some(profile),
        ),
    );

    if edge_pct > 0.005 && !is_volatile {
        let mut size_multiplier = 1.0;
        let mut learning_loop_context = String::new();
        if is_suboptimal_window {
            size_multiplier = 0.5;
            learning_loop_context = format!(" | [REDUCED SIZE: Sub-optimal Window {}:00]", current_hour);
        }

        let reason = format!(
            "PARITY SNIPE confirmed by KRONOS ({:.2}): {} target at strike {:.0} with {:.2}% gap edge.{}",
            kronos_score.unwrap_or(0.5),
            best_target_type.as_deref().unwrap_or(""),
            best_strike,
            best_gap * 100.0,
            learning_loop_context
        );

        return StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.30 * size_multiplier,
            reason: reason.clone(),
            limit_price: None,
            stop_loss: None,
            take_profit: None,
            source: Some("PARITY_SNIPER".to_string()),
            exit_logic: Some("Gap Convergence".to_string()),
            option_entry_style: Some(if best_target_type.as_deref() == Some("CALL") {
                crate::models::OptionEntryStyle::LongCall
            } else {
                crate::models::OptionEntryStyle::LongPut
            }),
            contract_symbol: best_target_symbol,
            math_edge: Some(format!("{:.4}", best_gap)),
            ai_score: kronos_score.map(|s| format!("{:.2}", s)),
            ..default_signal()
        };
    }

    crate::strategies::hold_with_intent(
        format!("SCANNING: Parity Gaps across {} options", options.len()),
        format!("Monitoring for {}% parity arbitrage edge. Best found: {:.2}%", 1.0, edge_pct * 100.0)
    )
}

fn default_signal() -> StrategySignal {
    StrategySignal {
        source: Some("PARITY_SNIPER".to_string()),
        ..Default::default()
    }
}