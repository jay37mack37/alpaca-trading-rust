use crate::models::{
    Candle, PositionRecord, Quote, SignalAction, StrategyRecord, StrategySignal,
};
use async_trait::async_trait;
use crate::strategies::{TradingStrategy, hold, buy};
use crate::AppState;
use crate::services::greeks::GreeksEngine;

pub struct GammaFlipStrategy;

#[async_trait]
impl TradingStrategy for GammaFlipStrategy {
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
        // 1. Position Management (Regime Flip Detection)
        if let Some(pos) = position {
            let gex_val = net_gex.unwrap_or(0.0);
            let is_call = pos.option_type.as_deref().map(|s| s.to_lowercase() == "call").unwrap_or(false);
            
            // If we are long a CALL but GEX has flipped negative -> EXIT
            if is_call && gex_val < 0.0 {
                return StrategySignal {
                    action: SignalAction::Sell,
                    allocation_fraction: 1.0,
                    reason: format!("GAMMA REGIME FLIP: GEX turned negative ({:.2}M) while holding Calls. Exiting.", gex_val / 1_000_000.0),
                    exit_logic: Some("Regime Flip (Negative GEX)".to_string()),
                    ..Default::default()
                };
            }
            
            // If we are long a PUT but GEX has flipped positive -> EXIT
            if !is_call && gex_val > 0.0 {
                return StrategySignal {
                    action: SignalAction::Sell,
                    allocation_fraction: 1.0,
                    reason: format!("GAMMA REGIME FLIP: GEX turned positive ({:.2}M) while holding Puts. Exiting.", gex_val / 1_000_000.0),
                    exit_logic: Some("Regime Flip (Positive GEX)".to_string()),
                    ..Default::default()
                };
            }

            return hold("Managing gamma flip position | Regime stable");
        }

        // 2. Use the provided net_gex (passed from agents.rs)
        let gex_val = net_gex.unwrap_or(0.0);

        // 3. Bidirectional Gamma Flip Trigger
        // Threshold: +/- $500k Net GEX (Normalized for high-convexity setup)
        let threshold = 500_000.0;
        let ai_val = kronos_score.unwrap_or(0.5);
        
        if gex_val < -threshold && ai_val < 0.4 {
            // Negative Gamma Territory + Bearish AI conviction
            let target_strike = quote.price * 0.985; // 1.5% OTM Put
            let mut signal = buy(
                format!("GAMMA FLIP (DOWN) confirmed by KRONOS ({:.2}): Extreme negative GEX ({:.2}M)", ai_val, gex_val / 1_000_000.0),
                0.15,
                Some(target_strike)
            );
            signal.ai_score = Some(format!("{:.2}", ai_val));
            return signal;
        } else if gex_val > threshold && ai_val > 0.6 {
            // Positive Gamma Territory + Bullish AI conviction
            let target_strike = quote.price * 1.015; // 1.5% OTM Call
            let mut signal = buy(
                format!("GAMMA FLIP (UP) confirmed by KRONOS ({:.2}): Extreme positive GEX ({:.2}M)", ai_val, gex_val / 1_000_000.0),
                0.15,
                Some(target_strike)
            );
            signal.ai_score = Some(format!("{:.2}", ai_val));
            return signal;
        }

        hold(format!("Stable GEX Zone: {:.2}M", gex_val / 1_000_000.0))
    }
}

pub fn calculate_net_gex(spot_price: f64, options: &[crate::models::OptionContractSnapshot]) -> f64 {
    let mut net_gex = 0.0;
    let risk_free_rate = 0.045; 

    for contract in options {
        let t = 1.0 / 252.0; 
        let sigma = contract.implied_volatility.unwrap_or(0.25);

        let greeks = GreeksEngine::calculate_greeks(
            spot_price,
            contract.strike,
            t,
            risk_free_rate,
            sigma
        );

        let oi = contract.open_interest.unwrap_or(0.0);
        let vol = contract.volume.unwrap_or(0.0);
        
        let is_call = contract.contract_symbol.to_uppercase().contains('C'); 
        
        let gex = GreeksEngine::calculate_gex(spot_price, greeks.gamma, oi, is_call);
        let weighted_gex = GreeksEngine::weighted_gex(gex, vol, oi);
        
        net_gex += weighted_gex;
    }
    net_gex
}
