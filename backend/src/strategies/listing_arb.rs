use crate::math::black_scholes_call;
use crate::models::{
    Candle, OptionContractSnapshot, PositionRecord, Quote, SignalAction, StrategyRecord,
    StrategySignal,
};
use crate::options::parse_expiration_from_occ;
use crate::strategies::{hold, TradingStrategy};
use async_trait::async_trait;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

const KRONOS_URL: &str = "http://localhost:8000";
const SPY_SYMBOL: &str = "SPY";

#[derive(Debug, Serialize, Deserialize)]
pub struct KronosSentiment {
    pub score: f64,
    pub latency_ms: u64,
}

/// Professional Listing Arbitrage Strategy
/// 
/// Phase 1: Selective Sniper & Drift Hunter
pub async fn evaluate_listing_arbitrage_v2(
    _strategy: &StrategyRecord,
    option: &OptionContractSnapshot,
    underlying_quote: &Quote,
    position: Option<&PositionRecord>,
) -> StrategySignal {
    // 1. Quality Filters
    if option.underlying_symbol != SPY_SYMBOL {
        return hold("Non-SPY contracts ignored");
    }

    let bid = option.bid.unwrap_or(0.0);
    let ask = option.ask.unwrap_or(0.0);
    if bid <= 0.0 || ask <= 0.0 {
        return hold("Stale or missing chain data");
    }
    let mid = (bid + ask) / 2.0;

    // 2. Intelligence Check (Kronos AI)
    let kronos = get_kronos_sentiment(&option.contract_symbol).await.unwrap_or(KronosSentiment { score: 0.5, latency_ms: 0 });
    
    // 3. Fair Value Calculation (Black-Scholes)
    let expiration = match parse_expiration_from_occ(&option.contract_symbol) {
        Some(exp) => exp,
        None => return hold("OCC parse error"),
    };
    let dte = days_until_expiration(&expiration).unwrap_or(0);
    if dte <= 0 { return hold("Expired"); }

    let iv = option.implied_volatility.unwrap_or(0.2);
    let fair_value = black_scholes_call(
        underlying_quote.price,
        option.strike,
        dte as f64 / 365.0,
        iv,
        0.05,
    );

    // 4. Alpha Calculation
    let edge = (mid - fair_value) / fair_value;
    
    // 5. Entry Logic
    if position.is_none() {
        // Snipe Condition: Edge > 2% + Kronos > 0.8
        if edge < -0.02 && kronos.score > 0.8 {
            return StrategySignal {
                action: SignalAction::Buy,
                allocation_fraction: 0.1,
                reason: format!("SNIPE: Edge {:.1}% | Kronos {:.2}", edge * 100.0, kronos.score),
                limit_price: Some(bid), // Start at Bid
                walk_to_mid: Some(true), // Walk if not filled
                stop_loss: Some(mid * 0.98), // 2% Hard Stop
                take_profit: None,
                trailing_stop: None,
                split_exit: Some(true), // 50/50 Scalp/Runner
                log_type: Some("NEW".to_string()),
            };
        }

        // Drift Condition: Kronos > 0.6 + Positive Drift
        if edge < -0.01 && kronos.score > 0.6 {
             return StrategySignal {
                action: SignalAction::Buy,
                allocation_fraction: 0.05,
                reason: format!("DRIFT: Edge {:.1}% | Kronos {:.2}", edge * 100.0, kronos.score),
                limit_price: Some(mid),
                walk_to_mid: Some(false),
                stop_loss: Some(mid * 0.98),
                take_profit: None,
                trailing_stop: None,
                split_exit: Some(true),
                log_type: Some("DRIFT".to_string()),
            };
        }
    }

    // 6. Exit Logic (Managed separately by engine via legs usually, but we define triggers)
    if let Some(pos) = position {
        // Hard Loss Exit
        let current_pnl = (mid - pos.average_price) / pos.average_price;
        if current_pnl < -0.02 {
            return StrategySignal {
                action: SignalAction::Sell,
                allocation_fraction: 1.0,
                reason: format!("SAFETY: Hard Stop 2% reached ({:.1}%)", current_pnl * 100.0),
                ..default_signal()
            };
        }

        // Sentiment Exit
        if kronos.score < 0.4 {
            return StrategySignal {
                action: SignalAction::Sell,
                allocation_fraction: 1.0,
                reason: format!("ALPHA: Kronos Sentiment Flipped ({:.2})", kronos.score),
                ..default_signal()
            };
        }

        // Price Edge Exit (Profit Taking)
        if edge > 0.01 {
             return StrategySignal {
                action: SignalAction::Sell,
                allocation_fraction: 1.0,
                reason: format!("TARGET: Fair value reached/exceeded (Edge {:.1}%)", edge * 100.0),
                ..default_signal()
            };
        }
    }

    hold(&format!("Monitoring | Edge: {:.1}% | Kronos: {:.2}", edge * 100.0, kronos.score))
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
    }
}

async fn get_kronos_sentiment(symbol: &str) -> Option<KronosSentiment> {
    // Placeholder for real bridge
    // In production, this calls the local Kronos inference engine
    Some(KronosSentiment {
        score: (symbol.len() % 10) as f64 / 10.0, // Stable mock
        latency_ms: 12,
    })
}

fn days_until_expiration(exp: &str) -> Option<i64> {
    let today = Local::now().date_naive();
    let exp_date = NaiveDate::parse_from_str(exp, "%Y-%m-%d").ok()?;
    Some(exp_date.signed_duration_since(today).num_days())
}

pub struct ListingArbitrageStrategy;

#[async_trait]
impl TradingStrategy for ListingArbitrageStrategy {
    async fn evaluate(
        &self,
        strategy: &StrategyRecord,
        candles: &[Candle],
        quote: &Quote,
        position: Option<&PositionRecord>,
    ) -> StrategySignal {
        evaluate_listing_arbitrage_wrapper(strategy, candles, quote, position).await
    }
}

// Wrapper for the engine
pub async fn evaluate_listing_arbitrage_wrapper(
    _strategy: &StrategyRecord,
    _candles: &[Candle],
    _quote: &Quote,
    _position: Option<&PositionRecord>,
) -> StrategySignal {
    // Top 20 active options sweep logic would normally go here, 
    // but the engine calls this per-symbol in its current loop.
    // For now, we evaluate the first tracked symbol.
    
    // In a real run, we'd iterate the option chain.
    // This is a simplified per-contract implementation.
    hold("Interactive chain analysis required")
}
