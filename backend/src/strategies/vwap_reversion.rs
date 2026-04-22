use crate::logger::{SystemEvent, SystemSource, SystemEventType};
use crate::agents::broadcast_audit_log;
use crate::models::{SignalAction, StrategySignal};
use crate::AppState;

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
    kronos_score: Option<f64>,
) -> StrategySignal {
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
            SystemSource::Vwap,
            symbol.to_string(),
            event_type,
            math_context,
            ai_score,
            narrative,
        ),
    );

    // Entry Logic
    if abs_dev >= 2.5 && ai_score > 0.8 && adx_allows {
        let action = if dev > 0.0 { SignalAction::Sell } else { SignalAction::Buy };
        let direction = if dev > 0.0 { "Over-extended (UP)" } else { "Over-extended (DOWN)" };

        return StrategySignal {
            action,
            allocation_fraction: 0.15,
            reason: format!("{}: SNAP BACK likely ({:.2} SD)", direction, abs_dev),
            limit_price: Some(current_price),
            stop_loss: Some(current_price * (1.0 - 0.01 * dev.signum())),
            take_profit: Some(vwap),
            ..default_signal()
        };
    }

    crate::strategies::hold(format!("Monitoring | {:.2} SD", dev))
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
        source: Some("VWAP_REVERSION".to_string()),
        math_edge: None,
        ai_score: None,
        ..Default::default()
    }
}