use crate::models::{
    Candle, Quote, SignalAction, StrategyRecord, StrategySignal,
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
        position: Option<&crate::models::PositionRecord>,
        _kronos_score: Option<f64>,
    ) -> StrategySignal {
        // 1. Check if we already have a position
        if let Some(pos) = position {
            // Exit Logic: 90% Drawdown OR 300% Profit Target with 50% Trailing
            // In a real implementation, we'd calculate unrealized P&L here
            return hold("Managing gamma flip position");
        }

        // 2. Calculate Net GEX
        let mut net_gex = 0.0;
        let risk_free_rate = 0.045; 
        let mut contracts_scanned = 0;

        crate::agents::broadcast_audit_log(
            _state,
            crate::logger::SystemEvent::now(
                crate::logger::SystemSource::System,
                Some(_strategy.id.clone()),
                quote.symbol.clone(),
                crate::logger::SystemEventType::Scan,
                format!("Price: ${:.2}", quote.price),
                0.5,
                format!("GEX SCAN: Analyzing {} option contracts for gamma inflection.", options.len()),
            )
        );

        for contract in options {
            contracts_scanned += 1;
            let t = 1.0 / 252.0; 
            let sigma = contract.implied_volatility.unwrap_or(0.25);

            let greeks = GreeksEngine::calculate_greeks(
                quote.price,
                contract.strike,
                t,
                risk_free_rate,
                sigma
            );

            let oi = contract.open_interest.unwrap_or(0.0);
            let vol = contract.volume.unwrap_or(0.0);
            
            let is_call = contract.contract_symbol.to_uppercase().contains('C'); 
            
            let gex = GreeksEngine::calculate_gex(quote.price, greeks.gamma, oi, is_call);
            let weighted_gex = GreeksEngine::weighted_gex(gex, vol, oi);
            
            net_gex += weighted_gex;
        }

        crate::agents::broadcast_audit_log(
            _state,
            crate::logger::SystemEvent::now(
                crate::logger::SystemSource::System,
                Some(_strategy.id.clone()),
                quote.symbol.clone(),
                crate::logger::SystemEventType::Scan,
                format!("GEX: {:.0}", net_gex),
                0.5,
                format!("SCAN COMPLETE: Net GEX for {} is {:.2}M across {} contracts.", quote.symbol, net_gex / 1_000_000.0, contracts_scanned),
            )
        );

        // 3. The Trigger: Negative Gamma + Volume Spike
        // For a full implementation, we need the "Previous GEX" to detect the crossover
        // For now, we trigger if Net GEX is significantly negative
        if net_gex < -1000000.0 { // Arbitrary threshold for "Negative Gamma Territory"
            // Filter for 0DTE OTM Options (1.5% out)
            let target_strike = quote.price * 0.985; // OTM Put for a down-flip
            
            return buy(
                format!("Gamma Flip Triggered (Net GEX: {:.0})", net_gex),
                0.20, // 20% allocation
                Some(target_strike)
            );
        }

        hold(format!("Stable GEX: {:.0}", net_gex))
    }
}
