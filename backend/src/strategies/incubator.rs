use crate::models::{
    SignalAction, StrategySignal, Candle, PositionRecord, Quote, StrategyRecord
};
use crate::AppState;
use async_trait::async_trait;
use crate::strategies::{TradingStrategy, hold, closes, intraday_vwap, rsi, sma};

pub struct VwapReflexiveStrategy;

#[async_trait]
impl TradingStrategy for VwapReflexiveStrategy {
    async fn evaluate(
        &self,
        _state: &AppState,
        _strategy: &StrategyRecord,
        candles: &[Candle],
        quote: &Quote,
        _options: &[crate::models::OptionContractSnapshot],
        position: Option<&PositionRecord>,
        kronos_score: Option<f64>,
        _net_gex: Option<f64>,
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
                allocation_fraction: 0.05,
                reason: format!("Price is {:.2}% above session VWAP", d * 100.0),
                ..Default::default()
            },
            (Some(_), d) if d < -0.001 => StrategySignal {
                action: SignalAction::Sell,
                allocation_fraction: 1.0,
                reason: format!("Price fell {:.2}% below session VWAP", d * 100.0),
                ..Default::default()
            },
            _ => hold("Waiting for VWAP displacement"),
        }
    }
}

pub struct RsiMeanReversionStrategy;

#[async_trait]
impl TradingStrategy for RsiMeanReversionStrategy {
    async fn evaluate(
        &self,
        _state: &AppState,
        _strategy: &StrategyRecord,
        candles: &[Candle],
        _quote: &Quote,
        _options: &[crate::models::OptionContractSnapshot],
        position: Option<&PositionRecord>,
        _kronos_score: Option<f64>,
        _net_gex: Option<f64>,
    ) -> StrategySignal {
        let closes = closes(candles);
        let Some(rsi_val) = rsi(&closes, 14) else {
            return hold("RSI unavailable");
        };

        match (position, rsi_val) {
            (None, value) if value < 30.0 => StrategySignal {
                action: SignalAction::Buy,
                allocation_fraction: 0.05,
                reason: format!("RSI mean reversion entry at {:.1}", value),
                ..Default::default()
            },
            (Some(_), value) if value > 62.0 => StrategySignal {
                action: SignalAction::Sell,
                allocation_fraction: 1.0,
                reason: format!("RSI exit at {:.1}", value),
                ..Default::default()
            },
            _ => hold("RSI within neutral zone"),
        }
    }
}
