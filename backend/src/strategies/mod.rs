use chrono::{DateTime, Datelike, Timelike, Utc};
pub mod listing_arb;
pub mod parity_sniper;
pub mod vwap_reversion;
pub mod jarrod_vwap;
pub mod yield_rotation;
pub mod gamma_flip;
pub mod incubator;
pub mod zero_dte_neutral;
pub mod distribution_sniper;

use crate::models::{
    Candle, PositionRecord, Quote, SignalAction, StrategyKind, StrategyRecord, StrategySignal,
};
use async_trait::async_trait;
use crate::AppState;
use std::collections::HashMap;
use std::sync::OnceLock;

#[async_trait]
pub trait TradingStrategy: Send + Sync {
    async fn evaluate(
        &self,
        state: &AppState,
        strategy: &StrategyRecord,
        candles: &[Candle],
        quote: &Quote,
        options: &[crate::models::OptionContractSnapshot],
        position: Option<&PositionRecord>,
        kronos_score: Option<f64>,
        net_gex: Option<f64>,
    ) -> StrategySignal;
}

static STRATEGY_REGISTRY: OnceLock<HashMap<StrategyKind, Box<dyn TradingStrategy + Send + Sync>>> =
    OnceLock::new();

fn get_strategy_registry() -> &'static HashMap<StrategyKind, Box<dyn TradingStrategy + Send + Sync>> {
    STRATEGY_REGISTRY.get_or_init(|| {
        let mut m: HashMap<StrategyKind, Box<dyn TradingStrategy + Send + Sync>> = HashMap::new();
        m.insert(StrategyKind::VwapReflexive, Box::new(incubator::VwapReflexiveStrategy));
        m.insert(
            StrategyKind::RsiMeanReversion,
            Box::new(incubator::RsiMeanReversionStrategy),
        );
        m.insert(StrategyKind::SmaTrend, Box::new(zero_dte_neutral::ZeroDteNeutralStrategy));
        m.insert(
            StrategyKind::ListingArbitrage,
            Box::new(listing_arb::ListingArbitrageStrategy),
        );
        m.insert(StrategyKind::PutCallParity, Box::new(parity_sniper::ParitySniperStrategy));
        m.insert(StrategyKind::ParitySniper, Box::new(parity_sniper::ParitySniperStrategy));
        m.insert(StrategyKind::VwapReversion, Box::new(vwap_reversion::VwapReversionStrategy));
        m.insert(StrategyKind::JarrodVwap, Box::new(jarrod_vwap::JarrodVwapStrategy));
        m.insert(StrategyKind::YieldRotation, Box::new(yield_rotation::YieldRotationStrategy));
        m.insert(StrategyKind::GammaFlip, Box::new(gamma_flip::GammaFlipStrategy));
        m.insert(StrategyKind::ZeroDteNeutral, Box::new(zero_dte_neutral::ZeroDteNeutralStrategy));
        m.insert(StrategyKind::DistributionSniper, Box::new(distribution_sniper::DistributionSniperStrategy));
        m.insert(StrategyKind::SmaTrend, Box::new(incubator::VwapReflexiveStrategy));
        m
    })
}

pub async fn evaluate_strategy(
    state: &AppState,
    strategy: &StrategyRecord,
    candles: &[Candle],
    quote: &Quote,
    options: &[crate::models::OptionContractSnapshot],
    position: Option<&PositionRecord>,
    kronos_score: Option<f64>,
    net_gex: Option<f64>,
) -> StrategySignal {
    // 1. GLOBAL 0DTE SAFETY KILL-SWITCH
    // Ensures options expiring today are closed before market close.
    if let Some(pos) = position {
        if let Some(ref expiry_str) = pos.expiration {
            if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
                let today = chrono::Utc::now().date_naive();
                if expiry.date_naive() == today {
                    // Check if time is past 15:50 EST/EDT (19:50 UTC in Summer)
                    let now = chrono::Utc::now();
                    let trigger_time_reached = (now.hour() == 19 && now.minute() >= 50) || now.hour() >= 20;

                    if trigger_time_reached {
                         return StrategySignal {
                            action: SignalAction::Sell,
                            allocation_fraction: 1.0,
                            reason: "0DTE SAFETY KILL-SWITCH: Closing expiring option before market close".to_string(),
                            exit_logic: Some("Automatic 0DTE Expiry Protection".to_string()),
                            ..Default::default()
                        };
                    }
                }
            }
        }
    }

    let registry = get_strategy_registry();
    let mut signal = if let Some(trading_strategy) = registry.get(&strategy.kind) {
        trading_strategy
            .evaluate(state, strategy, candles, quote, options, position, kronos_score, net_gex)
            .await
    } else {
        hold(format!("Strategy implementation for {:?} not found", strategy.kind))
    };

    // 2. PROFIT GUARD: Dynamic Protection for 0DTE and High-Profit Trades
    if let Some(pos) = position {
        let current_price = quote.price;
        let avg_price = pos.average_price;
        let profit_pct = if pos.quantity > 0.0 {
            (current_price - avg_price) / avg_price
        } else {
            0.0
        };

        // STAGE 3: Trailing Stop at +30% profit
        if profit_pct >= 0.30 {
            signal.trailing_stop = Some(0.05); // 5% trailing stop
            signal.reason = format!("PROFIT GUARD [STAGE 3]: +30% reached ({:.1}%). Locking in gains with 5% Trailing Stop.", profit_pct * 100.0);
        }
        // STAGE 1 & 2: Breakeven Anchor at +15%
        else if profit_pct >= 0.15 {
             // If we don't have a stop loss or it's below breakeven, move it up.
             let breakeven_plus = avg_price * 1.005; // Entry + 0.5% to cover fees
             if signal.stop_loss.is_none() || signal.stop_loss.unwrap() < breakeven_plus {
                 signal.stop_loss = Some(breakeven_plus);
                 signal.reason = format!("PROFIT GUARD [STAGE 1/2]: +15% reached ({:.1}%). Anchoring Stop-Loss to Breakeven.", profit_pct * 100.0);
             }
        }

        // --- EXPLICIT TARGET MONITORING ---
        // 1. Take Profit Check
        if let Some(tp_target) = pos.take_profit {
            let is_long = pos.quantity > 0.0;
            let tp_hit = if is_long { current_price >= tp_target } else { current_price <= tp_target };
            if tp_hit {
                signal.action = SignalAction::Sell;
                signal.reason = format!("GLOBAL TARGET HIT: Take-Profit reached at ${:.2} (Target: ${:.2})", current_price, tp_target);
                signal.exit_logic = Some("Take Profit".to_string());
            }
        }

        // 2. Stop Loss Check (Razor Stop)
        if let Some(sl_target) = pos.razor_stop {
            let is_long = pos.quantity > 0.0;
            let sl_hit = if is_long { current_price <= sl_target } else { current_price >= sl_target };
            if sl_hit {
                signal.action = SignalAction::Sell;
                signal.reason = format!("GLOBAL PROTECTION: Stop-Loss reached at ${:.2} (Limit: ${:.2})", current_price, sl_target);
                signal.exit_logic = Some("Stop Loss".to_string());
            }
        }
        
        // THETA KILL: If 0DTE and holding > 45 mins without profit, exit.
        // (Requires entry_time in PositionRecord)
        if let Some(ref entry_time_str) = pos.entry_time {
            if let Ok(entry_time) = chrono::DateTime::parse_from_rfc3339(entry_time_str) {
                let hold_duration = chrono::Utc::now() - entry_time.with_timezone(&chrono::Utc);
                if hold_duration.num_minutes() >= 45 && profit_pct < 0.05 {
                    signal.action = SignalAction::Sell;
                    signal.reason = "PROFIT GUARD [THETA KILL]: 0DTE held > 45 mins without 5% profit breakout. Exiting to preserve premium.".to_string();
                }
            }
        }
    }

    // 3. ENRICH EXITS: Ensure every buy signal has a planned exit strategy
    if matches!(signal.action, SignalAction::Buy) && signal.planned_exit.is_none() {
        signal.planned_exit = Some("Standard Risk Exit: 25% TP / 15% SL".to_string());
    }

    signal
}


pub fn hold(reason: impl Into<String>) -> StrategySignal {
    StrategySignal {
        action: SignalAction::Hold,
        allocation_fraction: 0.0,
        reason: reason.into(),
        limit_price: None,
        stop_loss: None,
        take_profit: None,
        trailing_stop: None,
        walk_to_mid: None,
        split_exit: None,
        log_type: None,
        new_state: None,
        source: None,
        math_edge: None,
        ai_score: None,
        ..Default::default()
    }
}

pub fn hold_with_intent(reason: impl Into<String>, intent: impl Into<String>) -> StrategySignal {
    StrategySignal {
        action: SignalAction::Hold,
        reason: reason.into(),
        hold_intent: Some(intent.into()),
        ..Default::default()
    }
}

pub fn buy(reason: impl Into<String>, allocation: f64, price: Option<f64>) -> StrategySignal {
    StrategySignal {
        action: SignalAction::Buy,
        allocation_fraction: allocation,
        reason: reason.into(),
        limit_price: price,
        ..Default::default()
    }
}

fn closes(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|candle| candle.close).collect()
}

fn intraday_vwap(candles: &[Candle]) -> Option<f64> {
    let mut cumulative_price_volume = 0.0;
    let mut cumulative_volume = 0.0;

    for candle in candles {
        if candle.volume <= 0.0 {
            continue;
        }
        let typical_price = (candle.high + candle.low + candle.close) / 3.0;
        cumulative_price_volume += typical_price * candle.volume;
        cumulative_volume += candle.volume;
    }

    if cumulative_volume > 0.0 {
        Some(cumulative_price_volume / cumulative_volume)
    } else {
        None
    }
}

fn sma(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period || period == 0 {
        return None;
    }

    let slice = &values[values.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

fn rsi(values: &[f64], period: usize) -> Option<f64> {
    if values.len() <= period || period == 0 {
        return None;
    }

    let mut gains = 0.0;
    let mut losses = 0.0;

    for window in values[values.len() - (period + 1)..].windows(2) {
        let delta = window[1] - window[0];
        if delta >= 0.0 {
            gains += delta;
        } else {
            losses += delta.abs();
        }
    }

    if losses == 0.0 {
        return Some(100.0);
    }

    let rs = gains / losses;
    Some(100.0 - (100.0 / (1.0 + rs)))
}

#[cfg(test)]
mod tests {
    use crate::models::{DataProvider, ExecutionMode, AssetClassTarget, OptionEntryStyle, OptionStructurePreset};
    use crate::services::streaming::StreamHub;
    use super::*;

    fn make_test_app_state() -> AppState {
        let db = crate::services::db::Database::open(
            std::path::Path::new(":memory:"),
            &[],
            "test-master-key-for-unit-tests",
        ).unwrap();
        AppState {
            api_token: std::sync::Arc::new("test-token".to_string()),
            db: std::sync::Arc::new(tokio::sync::Mutex::new(db)),
            http: reqwest::Client::new(),
            config: crate::models::AppConfig {
                host: "localhost".into(),
                port: 0,
                database_path: std::path::PathBuf::from(":memory:"),
                default_watchlist: vec![],
                polling_seconds: 60,
                allowed_origins: vec![],
                mock_alpaca: false,
            },
            streams: StreamHub::new(),
            agent_tasks: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            risk_engine: std::sync::Arc::new(crate::services::risk::RiskEngine::new()),
        }
    }

    fn make_quote(price: f64, vwap: Option<f64>) -> Quote {
        Quote {
            symbol: "AAPL".to_string(),
            provider: DataProvider::Yahoo,
            price,
            previous_close: None,
            change: None,
            change_percent: None,
            bid: None,
            ask: None,
            volume: None,
            vwap,
            session_high: None,
            session_low: None,
            timestamp: "2021-01-01T00:00:00Z".to_string(),
        }
    }

    fn make_candle(close: f64) -> Candle {
        Candle {
            timestamp: "2021-01-01T00:00:00Z".to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 100.0,
            vwap: None,
        }
    }

    #[test]
    fn test_rsi_not_enough_data() {
        assert_eq!(rsi(&[10.0, 11.0], 2), None);
        assert_eq!(rsi(&[10.0], 2), None);
        assert_eq!(rsi(&[], 2), None);
    }

    #[test]
    fn test_rsi_period_zero() {
        assert_eq!(rsi(&[10.0, 11.0, 12.0], 0), None);
    }

    #[test]
    fn test_rsi_all_gains() {
        assert_eq!(rsi(&[10.0, 11.0, 12.0, 13.0], 3), Some(100.0));
    }

    #[test]
    fn test_rsi_all_losses() {
        assert_eq!(rsi(&[13.0, 12.0, 11.0, 10.0], 3), Some(0.0));
    }

    fn make_test_strategy(kind: StrategyKind) -> StrategyRecord {
        StrategyRecord {
            id: "test".into(),
            name: "test".into(),
            kind,
            enabled: true,
            execution_mode: ExecutionMode::LocalPaper,
            asset_class_target: AssetClassTarget::Equity,
            option_entry_style: OptionEntryStyle::LongCall,
            option_structure_preset: OptionStructurePreset::Single,
            option_spread_width: 0.0,
            option_target_delta: 0.0,
            option_dte_min: 0,
            option_dte_max: 0,
            option_max_spread_pct: 0.0,
            option_limit_buffer_pct: 0.0,
            credential_id: None,
            starting_cash: 0.0,
            cash_balance: 0.0,
            equity: 0.0,
            tracked_symbols: vec![],
            total_trades: 0,
            wins: 0,
            losses: 0,
            last_signal: None,
            last_run_at: None,
            run_interval_ms: 0,
            state_json: serde_json::json!({}),
            risk_parameters: None,
            performance_stats_json: None,
            shared_cash: None,
            use_shared_cash: false,
            execution_profile: crate::models::ExecutionProfile::Standard,
        }
    }

    #[test]
    fn test_evaluate_vwap_reflexive_basic() {
        let candles = vec![];
        let quote = make_quote(100.5, Some(100.0));
        let state = make_test_app_state();
        let strategy = make_test_strategy(StrategyKind::VwapReflexive);
        let signal = tokio_test::block_on(evaluate_strategy(&state, &strategy, &candles, &quote, &[], None, None, None));
        assert_eq!(signal.action, SignalAction::Buy);
    }

    #[test]
    fn test_evaluate_rsi_mean_reversion() {
        let mut candles = vec![];
        for i in 0..15 {
            candles.push(make_candle(100.0 - i as f64));
        }
        let quote = make_quote(100.0, None);
        let state = make_test_app_state();
        let strategy = make_test_strategy(StrategyKind::RsiMeanReversion);
        let signal = tokio_test::block_on(evaluate_strategy(&state, &strategy, &candles, &quote, &[], None, None, None));
        assert_eq!(signal.action, SignalAction::Buy);
    }

    #[test]
    fn test_evaluate_sma_trend() {
        let mut candles = vec![];
        for i in 0..50 {
            candles.push(make_candle(100.0 + i as f64));
        }
        let quote = make_quote(100.0, None);
        let state = make_test_app_state();
        let strategy = make_test_strategy(StrategyKind::SmaTrend);
        let signal = tokio_test::block_on(evaluate_strategy(&state, &strategy, &candles, &quote, &[], None, None, None));
        assert_eq!(signal.action, SignalAction::Buy);
    }

    #[test]
    fn test_intraday_vwap() {
        let candles = vec![
            Candle { timestamp: "".into(), open: 10.0, high: 12.0, low: 8.0, close: 10.0, volume: 100.0, vwap: None },
            Candle { timestamp: "".into(), open: 20.0, high: 22.0, low: 18.0, close: 20.0, volume: 200.0, vwap: None },
        ];
        let vwap = intraday_vwap(&candles).unwrap();
        assert!((vwap - 16.666666666666668).abs() < 1e-9);
    }

    #[test]
    fn test_sma() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(sma(&values, 3), Some(4.0));
        assert_eq!(sma(&values, 5), Some(3.0));
        assert_eq!(sma(&values, 6), None);
    }

    #[test]
    fn test_rsi() {
        let values = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        assert_eq!(rsi(&values, 4), Some(100.0));

        let values = vec![10.0, 9.0, 8.0, 7.0, 6.0];
        assert_eq!(rsi(&values, 4), Some(0.0));
    }

    #[test]
    fn test_evaluate_vwap_reflexive_unavailable() {
        let quote = make_quote(150.0, None);
        let strategy = make_test_strategy(StrategyKind::VwapReflexive);
        let state = make_test_app_state();
        let signal = tokio_test::block_on(evaluate_strategy(&state, &strategy, &[], &quote, &[], None, None, None));
        assert_eq!(signal.action, SignalAction::Hold);
        assert_eq!(signal.reason, "VWAP unavailable");
    }
}
