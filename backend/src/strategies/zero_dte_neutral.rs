use crate::models::{
    SignalAction, StrategySignal, Candle, PositionRecord, Quote, StrategyRecord, OptionEntryStyle, TradeLeg, TradeSide
};
use crate::AppState;
use async_trait::async_trait;
use crate::strategies::{TradingStrategy, hold};
use chrono::{Utc, Timelike};

pub struct ZeroDteNeutralStrategy;

#[async_trait]
impl TradingStrategy for ZeroDteNeutralStrategy {
    async fn evaluate(
        &self,
        _state: &AppState,
        _strategy: &StrategyRecord,
        _candles: &[Candle],
        quote: &Quote,
        options: &[crate::models::OptionContractSnapshot],
        position: Option<&PositionRecord>,
        kronos_score: Option<f64>,
        net_gex: Option<f64>,
    ) -> StrategySignal {
        let gex_val = net_gex.unwrap_or(0.0);
        let ai_score = kronos_score.unwrap_or(0.5);
        let current_time = Utc::now();

        // 1. Position Management
        if let Some(pos) = position {
            let entry_strike = pos.strike.unwrap_or(quote.price);
            let drift_pct = (quote.price - entry_strike).abs() / entry_strike;
            
            // Exit if GEX falls (instability), AI detects a breakout, or price drifts > 1% from strike
            if gex_val < 500_000.0 || ai_score > 0.85 || ai_score < 0.15 || drift_pct > 0.01 {
                let reason = if drift_pct > 0.01 {
                    format!("0DTE NEUTRAL EXIT: Price drift too high ({:.2}%) from strike {:.0}", drift_pct * 100.0, entry_strike)
                } else {
                    format!("0DTE NEUTRAL EXIT: KRONOS ({:.2}) detected trend breakout | GEX: {:.0}", ai_score, gex_val)
                };
                
                return StrategySignal {
                    action: SignalAction::Sell,
                    allocation_fraction: 1.0,
                    reason,
                    exit_logic: Some("Pin Breakdown".to_string()),
                    ai_score: Some(format!("{:.2}", ai_score)),
                    ..Default::default()
                };
            }
            return hold(format!("Harvesting theta (Drift: {:.2}%)", drift_pct * 100.0));
        }

        // 2. Time Filter: Only enter between 10:30 AM and 2:30 PM EST
        // (14:30 to 18:30 UTC during Daylight Savings)
        let hour = current_time.hour();
        if hour < 14 || hour > 18 {
             return hold(format!("Outside 0DTE Trading Window ({} UTC)", hour));
        }

        // 3. Entry Logic (0DTE Delta-Neutral Straddle)
        // We look for "Market Pinned" conditions: High Positive GEX and Neutral AI
        let is_pinned = gex_val > 1_500_000.0;
        let is_neutral_ai = ai_score > 0.45 && ai_score < 0.55;

        if is_pinned && is_neutral_ai {
            // Find At-The-Money Call and Put
            let atm_strike = quote.price.round();
            let call = options.iter().find(|o| o.strike == atm_strike && o.option_type.to_lowercase() == "call");
            let put = options.iter().find(|o| o.strike == atm_strike && o.option_type.to_lowercase() == "put");

            if let (Some(c), Some(p)) = (call, put) {
                let mut legs = Vec::new();
                legs.push(TradeLeg {
                    instrument_symbol: c.contract_symbol.clone(),
                    side: TradeSide::Buy,
                    ratio_quantity: 1,
                    multiplier: 100.0,
                    price: (c.bid.unwrap_or(0.0) + c.ask.unwrap_or(0.0)) / 2.0,
                    option_type: Some("call".to_string()),
                    expiration: Some(c.expiration.clone()),
                    strike: Some(c.strike),
                    position_intent: Some("buy_to_open".to_string()),
                });
                legs.push(TradeLeg {
                    instrument_symbol: p.contract_symbol.clone(),
                    side: TradeSide::Buy,
                    ratio_quantity: 1,
                    multiplier: 100.0,
                    price: (p.bid.unwrap_or(0.0) + p.ask.unwrap_or(0.0)) / 2.0,
                    option_type: Some("put".to_string()),
                    expiration: Some(p.expiration.clone()),
                    strike: Some(p.strike),
                    position_intent: Some("buy_to_open".to_string()),
                });

                return StrategySignal {
                    action: SignalAction::Buy,
                    allocation_fraction: 0.15,
                    reason: format!("0DTE NEUTRAL confirmed by KRONOS ({:.2}): Pinned regime detected (GEX: {:.2}M)", ai_score, gex_val / 1_000_000.0),
                    legs: Some(legs),
                    planned_exit: Some("Theta harvest or GEX breakdown".to_string()),
                    ai_score: Some(format!("{:.2}", ai_score)),
                    ..Default::default()
                };
            }
        }

        hold(format!("Scanning for Pin (GEX: {:.1}M, AI: {:.2})", gex_val / 1_000_000.0, ai_score))
    }
}
