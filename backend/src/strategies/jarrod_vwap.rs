use crate::models::{SignalAction, StrategySignal, Candle, PositionRecord, Quote, StrategyRecord, OptionEntryStyle, AssetClassTarget};
use crate::AppState;
use async_trait::async_trait;

pub struct JarrodVwapStrategy;

#[async_trait]
impl crate::strategies::TradingStrategy for JarrodVwapStrategy {
    async fn evaluate(
        &self,
        state: &AppState,
        strategy: &StrategyRecord,
        candles: &[Candle],
        _quote: &Quote,
        _options: &[crate::models::OptionContractSnapshot],
        position: Option<&PositionRecord>,
        kronos_score: Option<f64>,
    ) -> StrategySignal {
        evaluate_jarrod_vwap(state, strategy, candles, position, kronos_score)
    }
}

pub struct VwapCalculator {
    cumulative_pv: f64,
    cumulative_vol: f64,
}

impl VwapCalculator {
    pub fn new() -> Self {
        Self {
            cumulative_pv: 0.0,
            cumulative_vol: 0.0,
        }
    }

    pub fn update(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let typical_price = (high + low + close) / 3.0;
        self.cumulative_pv += typical_price * volume;
        self.cumulative_vol += volume;
        if self.cumulative_vol > 0.0 {
            self.cumulative_pv / self.cumulative_vol
        } else {
            0.0
        }
    }

    pub fn reset(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_vol = 0.0;
    }
}

fn calculate_atr(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period + 1 {
        return None;
    }
    let mut trs = Vec::with_capacity(period);
    for i in (candles.len() - period)..candles.len() {
        let current = &candles[i];
        let previous = &candles[i - 1];
        let hl = current.high - current.low;
        let hc = (current.high - previous.close).abs();
        let lc = (current.low - previous.close).abs();
        let tr = hl.max(hc).max(lc);
        trs.push(tr);
    }
    Some(trs.iter().sum::<f64>() / period as f64)
}

fn avg_volume(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period { return None; }
    let sum: f64 = candles[candles.len() - period..].iter().map(|c| c.volume).sum();
    Some(sum / period as f64)
}

fn sma(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period || period == 0 {
        return None;
    }
    let slice = &values[values.len() - period..];
    Some(slice.iter().sum::<f64>() / period as f64)
}

fn default_signal() -> StrategySignal {
    StrategySignal {
        source: Some("JARROD_VWAP".to_string()),
        ..Default::default()
    }
}

fn profit_target_pct(strategy: &StrategyRecord) -> f64 {
    strategy.state_json.get("profit_target_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.05)
}

fn get_entry_state(prev_below: bool, curr_above: bool, prev_above: bool, curr_below: bool) -> Option<String> {
    if prev_below && curr_above {
        Some("reclaim".to_string())
    } else if prev_above && curr_below {
        Some("breakdown".to_string())
    } else {
        None
    }
}

pub fn evaluate_jarrod_vwap(
    _state: &AppState,
    strategy: &StrategyRecord,
    candles: &[Candle],
    position: Option<&PositionRecord>,
    _kronos_score: Option<f64>,
) -> StrategySignal {
    if candles.len() < 2 {
        return crate::strategies::hold("Not enough candles");
    }

    let symbol = strategy.tracked_symbols.first()
        .map(|s| s.as_str())
        .unwrap_or("SPY");

    let current = &candles[candles.len() - 1];
    let previous = &candles[candles.len() - 2];

    let mut calc = VwapCalculator::new();
    let mut prev_vwap = 0.0;
    let mut curr_vwap = 0.0;
    let mut last_date = "";

    for (i, c) in candles.iter().enumerate() {
        if c.timestamp.len() >= 10 {
            let date = &c.timestamp[..10];
            if date != last_date {
                calc.reset();
                last_date = date;
            }
        }
        let vwap = calc.update(c.high, c.low, c.close, c.volume);
        if i == candles.len() - 2 {
            prev_vwap = vwap;
        }
        if i == candles.len() - 1 {
            curr_vwap = vwap;
        }
    }

    if curr_vwap == 0.0 || prev_vwap == 0.0 {
        return crate::strategies::hold("VWAP warming up");
    }

    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let sma_200 = sma(&closes, 200).unwrap_or(0.0);

    let is_bullish = if sma_200 > 0.0 { current.close > sma_200 } else { true };
    let is_bearish = if sma_200 > 0.0 { current.close < sma_200 } else { true };

    let prev_below = previous.close < prev_vwap;
    let curr_above = current.close > curr_vwap;
    let prev_above = previous.close > prev_vwap;
    let curr_below = current.close < curr_vwap;

    let vol_20 = avg_volume(candles, 20).unwrap_or(0.0);
    let rvol = if vol_20 > 0.0 { current.volume / vol_20 } else { 0.0 };

    let target_pct = profit_target_pct(strategy);

    // --- Position Management: Exit Logic ---
    if let Some(pos) = position {
        if pos.average_price > 0.0 && pos.market_price > 0.0 {
            let pnl_pct = (pos.market_price - pos.average_price) / pos.average_price;
            if pnl_pct >= target_pct {
                return StrategySignal {
                    action: SignalAction::Sell,
                    allocation_fraction: 1.0,
                    reason: format!(
                        "Jarrod VWAP profit target hit: {:.1}% >= {:.1}% | RVOL: {:.2}",
                        pnl_pct * 100.0,
                        target_pct * 100.0,
                        rvol
                    ),
                    limit_price: Some(pos.market_price),
                    source: Some("JARROD_VWAP".to_string()),
                    ..default_signal()
                };
            }
        }
        return crate::strategies::hold(format!(
            "In position | Unrealized: {:.1}% | Target: {:.1}% | RVOL: {:.2}",
            ((pos.market_price - pos.average_price) / pos.average_price) * 100.0,
            target_pct * 100.0,
            rvol
        ));
    }

    // --- Entry Logic ---
    let _is_options = strategy.asset_class_target == AssetClassTarget::Options;

    let (triggered, direction, option_style) = if is_bullish && prev_below && curr_above && rvol > 1.5 {
        (true, "VWAP Reclaim", Some(OptionEntryStyle::LongCall))
    } else if is_bearish && prev_above && curr_below && rvol > 1.5 {
        (true, "VWAP Breakdown", Some(OptionEntryStyle::LongPut))
    } else {
        (false, "", None)
    };

    if !triggered {
        return crate::strategies::hold(format!(
            "Monitoring VWAP (RVOL: {:.2} | SMA filter: {})",
            rvol,
            if sma_200 > 0.0 {
                if current.close > sma_200 { "bullish" } else { "bearish" }
            } else { "warming" }
        ));
    }

    let atr = calculate_atr(candles, 14).unwrap_or(current.close * 0.01);
    let stop_loss = (current.close - 1.5 * atr).max(curr_vwap - 0.01);
    let risk_per_share = current.close - stop_loss;
    let allocation = if risk_per_share > 0.0 {
        let units_frac = 0.01 / risk_per_share * current.close;
        units_frac.min(1.0)
    } else {
        0.05
    };

    let entry_state = get_entry_state(prev_below, curr_above, prev_above, curr_below)
        .unwrap_or_else(|| direction.to_string());

    let mut state_json = strategy.state_json.clone();
    if let Some(obj) = state_json.as_object_mut() {
        obj.insert("last_entry_state".to_string(), serde_json::Value::String(entry_state.clone()));
    }

    StrategySignal {
        action: SignalAction::Buy,
        allocation_fraction: allocation,
        reason: format!(
            "Jarrod VWAP {}! {} | RVOL: {:.2}",
            direction,
            symbol,
            rvol
        ),
        limit_price: Some(current.close),
        stop_loss: Some(stop_loss),
        take_profit: Some(current.close + 2.0 * risk_per_share),
        exit_logic: Some(format!("VWAP {} target", direction)),
        new_state: Some(state_json),
        source: Some("JARROD_VWAP".to_string()),
        option_entry_style: option_style,
        math_edge: Some(format!("RVOL:{:.2}", rvol)),
        ..default_signal()
    }
}
