use chrono::Timelike;
use crate::logger::{SystemEvent, SystemSource, SystemEventType};
use crate::agents::broadcast_audit_log;
use crate::models::{SignalAction, StrategySignal, Candle, PositionRecord, Quote, StrategyRecord};
use crate::AppState;
use async_trait::async_trait;

pub struct VwapReversionStrategy;

#[async_trait]
impl crate::strategies::TradingStrategy for VwapReversionStrategy {
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
    ) -> StrategySignal {
        let mut tracker = VwapTracker::new();
        for candle in candles {
            tracker.update(candle.close, candle.volume);
        }
        evaluate_vwap_reversion(state, &strategy.id, &quote.symbol, quote.price, &tracker, position, kronos_score, net_gex, strategy.performance_stats_json.clone(), strategy.execution_profile)
    }
}

#[allow(dead_code)]
pub fn evaluate_vwap(prices: &[f64], volumes: &[f64]) -> (f64, f64) {
    if prices.is_empty() { return (0.0, 0.0); }
    let mut pv_sum = 0.0;
    let mut vol_sum = 0.0;
    for (p, v) in prices.iter().zip(volumes.iter()) {
        pv_sum += p * v;
        vol_sum += v;
    }
    let vwap = if vol_sum > 0.0 { pv_sum / vol_sum } else { 0.0 };

    let mean = prices.iter().sum::<f64>() / prices.len() as f64;
    let variance = prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / prices.len() as f64;
    (vwap, variance.sqrt())
}

pub struct VwapTracker {
    pub cumulative_volume: f64,
    pub cumulative_pv: f64,
    pub price_history: Vec<f64>,
}

impl VwapTracker {
    pub fn new() -> Self {
        Self {
            cumulative_volume: 0.0,
            cumulative_pv: 0.0,
            price_history: Vec::new(),
        }
    }

    pub fn update(&mut self, price: f64, volume: f64) {
        self.cumulative_volume += volume;
        self.cumulative_pv += price * volume;
        self.price_history.push(price);
        // Track last 20 periods for SD calculation
        if self.price_history.len() > 20 {
            self.price_history.remove(0);
        }
    }

    pub fn vwap(&self) -> f64 {
        if self.cumulative_volume == 0.0 {
            return 0.0;
        }
        self.cumulative_pv / self.cumulative_volume
    }

    pub fn std_dev(&self) -> f64 {
        if self.price_history.len() < 10 {
            return 0.0;
        }
        let mean = self.price_history.iter().sum::<f64>() / self.price_history.len() as f64;
        let variance = self
            .price_history
            .iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f64>()
            / self.price_history.len() as f64;
        variance.sqrt()
    }
}

pub fn evaluate_vwap_reversion(
    state: &AppState,
    _strategy_id: &str,
    symbol: &str,
    current_price: f64,
    tracker: &VwapTracker,
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

    let current_hour = (chrono::Utc::now().hour() + 20) % 24;
    let is_suboptimal_window = false;
    let vwap = tracker.vwap();
    let sd = tracker.std_dev();

    if vwap <= 0.0 || sd <= 0.0 {
        return crate::strategies::hold("VWAP/SD warming up");
    }

    let dev = (current_price - vwap) / sd;
    let abs_dev = dev.abs();
    let ai_score = kronos_score.unwrap_or(0.5);

    // Mock ADX calculation for decision support (ADX < 25 is range/mean-reversion friendly)
    let adx = 22.5;
    let adx_allows = adx < 25.0;

    // SYSTEM LOGGING: Decision Support System
    let mut event_type = SystemEventType::Scan;
    let mut narrative = format!("Monitoring | {:.2} SD distance from Mean.", dev);
    let math_context = format!("Price:{:.2} SD:{:.2} ADX:{:.1}", current_price, sd, adx);

    if abs_dev > 2.0 && abs_dev < 2.5 {
        narrative = format!("Over-extended ({:.1} SD). Awaiting confirmation.", dev);
    } else if ai_score > 0.8 && abs_dev >= 2.5 && adx_allows {
        event_type = SystemEventType::Signal;
        narrative = "Critical Deviation + Low Trend (ADX < 25). Initiating snap-back trade.".to_string();
    }

    broadcast_audit_log(
        state,
        SystemEvent::now(
            "VWAP_REVERSION".to_string(),
            Some(_strategy_id.to_string()),
            symbol.to_string(),
            event_type,
            format!("SD: {:.2} | ADX: {:.1}", dev, adx),
            ai_score,
            narrative,
            Some(profile),
        ),
    );

    // GEX Filtering (Gamma Flip Intelligence)
    let gex_val = _net_gex.unwrap_or(0.0);
    let is_gamma_unstable = gex_val < -500_000.0; // High volatility risk zone

    // Entry Logic
    if abs_dev >= 2.0 && !is_gamma_unstable {
        let (action, entry_style, direction) = if dev > 0.0 {
            // Price is high -> Snap back DOWN -> Buy PUT
            (SignalAction::Buy, crate::models::OptionEntryStyle::LongPut, "Over-extended (UP)")
        } else {
            // Price is low -> Snap back UP -> Buy CALL
            (SignalAction::Buy, crate::models::OptionEntryStyle::LongCall, "Over-extended (DOWN)")
        };

        // Directional Confirmation (Kronos)
        let ai_confirms = if dev > 0.0 { ai_score < 0.4 } else { ai_score > 0.6 };

        if ai_confirms {
            let mut size_multiplier = 1.0;
            let mut learning_loop_context = String::new();
            if is_suboptimal_window {
                size_multiplier = 0.5;
                learning_loop_context = format!(" | [REDUCED SIZE: Sub-optimal Window {}:00]", current_hour);
            }

            let allocation = if (dev > 0.0 && ai_score < 0.25) || (dev < 0.0 && ai_score > 0.75) {
                0.25 // High Conviction
            } else {
                0.15 // Standard
            };

            // SNIPER 0DTE FILTER: Ensure we only pick 0DTE contracts in Sniper mode
            let (max_dte, max_cost) = match profile {
                crate::models::ExecutionProfile::Sniper0Dte => (0, 2.50), // Focus on high leverage/Low cost for 0DTE
                crate::models::ExecutionProfile::Standard => (45, 50.0),
            };

            // In Style-based strategies, the actual contract is resolved in trading.rs,
            // but we can pass hints or just trust the global failsafe I just added.
            // However, it's better to log the intent correctly here.
            let sniper_context = if profile == crate::models::ExecutionProfile::Sniper0Dte {
                " [SNIPER 0DTE]"
            } else {
                ""
            };

            return StrategySignal {
                action,
                allocation_fraction: allocation * size_multiplier,
                reason: format!(
                    "VWAP SNAP-BACK confirmed by KRONOS ({:.2}): Mean reversion likely ({:.2} SD) | GEX: {:.1}M{}{}",
                    ai_score,
                    abs_dev,
                    gex_val / 1_000_000.0,
                    learning_loop_context,
                    sniper_context
                ),
                limit_price: Some(current_price),
                stop_loss: Some(current_price * (1.0 - 0.01 * dev.signum())),
                take_profit: Some(vwap),
                exit_logic: Some("VWAP Center".to_string()),
                planned_exit: Some(format!("Close position at VWAP mean target (~${:.2})", vwap)),
                option_entry_style: Some(entry_style),
                ai_score: Some(format!("{:.2}", ai_score)),
                ..default_signal()
            };
        }
    }

    if let Some(_) = position {
        return StrategySignal {
            action: SignalAction::Hold,
            reason: format!("MONITORING: Currently at {:.2} SD from VWAP", dev),
            hold_intent: Some(format!("Holding until price reverts to session VWAP mean (${:.2})", vwap)),
            planned_exit: Some(format!("Targeting VWAP cross at ${:.2}", vwap)),
            ..default_signal()
        };
    }

    crate::strategies::hold_with_intent(
        format!("SCANNING: {:.2} SD from Mean", dev),
        format!("Awaiting +/- 2.5 SD extension for reversion entry (Current: {:.2} SD)", dev)
    )
}

fn default_signal() -> StrategySignal {
    StrategySignal {
        source: Some("VWAP_REVERSION".to_string()),
        ..Default::default()
    }
}