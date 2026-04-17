pub mod listing_arb;

use crate::models::{
    Candle, PositionRecord, Quote, SignalAction, StrategyKind, StrategyRecord, StrategySignal,
};

pub fn evaluate_strategy(
    strategy: &StrategyRecord,
    candles: &[Candle],
    quote: &Quote,
    position: Option<&PositionRecord>,
) -> StrategySignal {
    match strategy.kind {
        StrategyKind::VwapReflexive => evaluate_vwap_reflexive(candles, quote, position),
        StrategyKind::RsiMeanReversion => evaluate_rsi_mean_reversion(candles, quote, position),
        StrategyKind::SmaTrend => evaluate_sma_trend(candles, quote, position),
        StrategyKind::ListingArbitrage => {
            // ListingArbitrage strategy
            // This would be called with option-specific data in a real implementation
            hold("ListingArbitrage strategy requires option contract data")
        }
    }
}

fn evaluate_vwap_reflexive(
    candles: &[Candle],
    quote: &Quote,
    position: Option<&PositionRecord>,
) -> StrategySignal {
    let session_vwap = quote.vwap.or_else(|| intraday_vwap(candles));
    let Some(vwap) = session_vwap else {
        return hold("VWAP unavailable");
    };

    if vwap <= 0.0 {
        return hold("VWAP invalid");
    }

    let distance = (quote.price - vwap) / vwap;
    match (position, distance) {
        (None, d) if d > 0.002 => StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.18,
            reason: format!("Price is {:.2}% above session VWAP", d * 100.0),
        },
        (Some(_), d) if d < -0.001 => StrategySignal {
            action: SignalAction::Sell,
            allocation_fraction: 1.0,
            reason: format!("Price fell {:.2}% below session VWAP", d * 100.0),
        },
        _ => hold("Waiting for VWAP displacement"),
    }
}

fn evaluate_rsi_mean_reversion(
    candles: &[Candle],
    _quote: &Quote,
    position: Option<&PositionRecord>,
) -> StrategySignal {
    let closes = closes(candles);
    let Some(rsi) = rsi(&closes, 14) else {
        return hold("RSI unavailable");
    };

    match (position, rsi) {
        (None, value) if value < 30.0 => StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.12,
            reason: format!("RSI mean reversion entry at {:.1}", value),
        },
        (Some(_), value) if value > 62.0 => StrategySignal {
            action: SignalAction::Sell,
            allocation_fraction: 1.0,
            reason: format!("RSI exit at {:.1}", value),
        },
        _ => hold("RSI within neutral zone"),
    }
}

fn evaluate_sma_trend(
    candles: &[Candle],
    _quote: &Quote,
    position: Option<&PositionRecord>,
) -> StrategySignal {
    let closes = closes(candles);
    let Some(fast) = sma(&closes, 20) else {
        return hold("20 period SMA unavailable");
    };
    let Some(slow) = sma(&closes, 50) else {
        return hold("50 period SMA unavailable");
    };

    match (position, fast > slow) {
        (None, true) => StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.15,
            reason: format!("Fast SMA {:.2} crossed above slow SMA {:.2}", fast, slow),
        },
        (Some(_), false) => StrategySignal {
            action: SignalAction::Sell,
            allocation_fraction: 1.0,
            reason: format!("Fast SMA {:.2} dropped below slow SMA {:.2}", fast, slow),
        },
        _ => hold("Trend regime unchanged"),
    }
}

fn hold(reason: impl Into<String>) -> StrategySignal {
    StrategySignal {
        action: SignalAction::Hold,
        allocation_fraction: 0.0,
        reason: reason.into(),
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
    use crate::models::{
        AssetClassTarget, Candle, DataProvider, ExecutionMode, OptionEntryStyle,
        OptionStructurePreset, PositionRecord, Quote, SignalAction, StrategyKind, StrategyRecord,
    };
    use super::*;

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

    fn make_position() -> PositionRecord {
        PositionRecord {
            underlying_symbol: "AAPL".to_string(),
            instrument_symbol: "AAPL".to_string(),
            asset_type: "equity".to_string(),
            quantity: 10.0,
            average_price: 100.0,
            market_price: 100.0,
            multiplier: 1.0,
            option_structure_preset: None,
            option_type: None,
            expiration: None,
            strike: None,
            stale_quote: false,
            legs: Vec::new(),
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

    #[test]
    fn test_evaluate_vwap_reflexive_basic() {
        let candles = vec![];
        let quote = make_quote(100.5, Some(100.0));
        let signal = evaluate_vwap_reflexive(&candles, &quote, None);
        assert_eq!(signal.action, SignalAction::Buy);
    }

    #[test]
    fn test_evaluate_rsi_mean_reversion() {
        let mut candles = vec![];
        for i in 0..15 {
            candles.push(make_candle(100.0 - i as f64));
        }
        let quote = make_quote(100.0, None);
        let signal = evaluate_rsi_mean_reversion(&candles, &quote, None);
        assert_eq!(signal.action, SignalAction::Buy);
    }

    #[test]
    fn test_evaluate_sma_trend() {
        let mut candles = vec![];
        for i in 0..50 {
            candles.push(make_candle(100.0 + i as f64));
        }
        let quote = make_quote(100.0, None);
        let signal = evaluate_sma_trend(&candles, &quote, None);
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
        let signal = evaluate_vwap_reflexive(&[], &quote, None);
        assert_eq!(signal.action, SignalAction::Hold);
        assert_eq!(signal.reason, "VWAP unavailable");
    }
}
