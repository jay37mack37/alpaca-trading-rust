use crate::agents::broadcast_strategy_log;
use crate::AppState;

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
        if self.price_history.len() < 2 {
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
    strategy_id: &str,
    symbol: &str,
    current_price: f64,
    tracker: &VwapTracker,
) {
    let vwap = tracker.vwap();
    let sd = tracker.std_dev();

    if sd > 0.0 {
        let deviations = (current_price - vwap).abs() / sd;
        if deviations > 2.0 {
            broadcast_strategy_log(
                state,
                strategy_id,
                symbol,
                "VWAP_REVERSION",
                &format!("{:.1} SD", deviations),
                "0.65", // AI confidence for reversion
                "DRIFT",
                &format!("SPY over-extended: {:.1} SD from VWAP", deviations),
            );
        }
    }
}
