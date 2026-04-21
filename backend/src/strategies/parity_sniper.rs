use crate::models::{SignalAction, StrategySignal};
use crate::agents::broadcast_strategy_log;
use crate::AppState;

pub fn calculate_parity_gap(spot_price: f64, call_price: f64, put_price: f64, strike: f64) -> f64 {
    ((spot_price + put_price - call_price) - strike).abs()
}

pub fn evaluate_parity_sniper(
    state: &AppState,
    strategy_id: &str,
    symbol: &str,
    spot_price: f64,
    call_price: f64,
    put_price: f64,
    strike: f64,
) {
    let gap = calculate_parity_gap(spot_price, call_price, put_price, strike);
    let edge_pct = gap / strike;

    if edge_pct > 0.01 {
        broadcast_strategy_log(
            state,
            strategy_id,
            symbol,
            "PARITY_SNIPER",
            &format!("{:.1}%", edge_pct * 100.0),
            "0.95", // AI high confidence for arb
            "NEW",
            &format!("Arbitrage Gap detected: {:.1}%", edge_pct * 100.0),
        );
    }
}
