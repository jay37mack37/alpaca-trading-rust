use std::f64::consts::PI;

/// High-performance Black-Scholes Greeks Engine
pub struct GreeksEngine;

impl GreeksEngine {
    /// Standard Normal Cumulative Distribution Function (numerical approximation)
    fn normal_cdf(x: f64) -> f64 {
        let b1 = 0.319381530;
        let b2 = -0.356563782;
        let b3 = 1.781477937;
        let b4 = -1.821255978;
        let b5 = 1.330274429;
        let p = 0.2316419;
        let c = 0.39894228;

        if x >= 0.0 {
            let t = 1.0 / (1.0 + p * x);
            1.0 - c * (-x * x / 2.0).exp() * t * (t * (t * (t * (t * b5 + b4) + b3) + b2) + b1)
        } else {
            1.0 - Self::normal_cdf(-x)
        }
    }

    /// Standard Normal Probability Density Function
    fn normal_pdf(x: f64) -> f64 {
        (1.0 / (2.0 * PI).sqrt()) * (-0.5 * x * x).exp()
    }

    /// Calculates Black-Scholes price and greeks
    /// S: Spot price, K: Strike price, T: Time to maturity (years), r: Risk-free rate, sigma: Volatility (annualized)
    pub fn calculate_greeks(s: f64, k: f64, t: f64, r: f64, sigma: f64) -> GreeksResult {
        if t <= 0.0 {
            return GreeksResult::default();
        }

        let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
        let d2 = d1 - sigma * t.sqrt();

        let call_price = s * Self::normal_cdf(d1) - k * (-r * t).exp() * Self::normal_cdf(d2);
        let put_price = k * (-r * t).exp() * Self::normal_cdf(-d2) - s * Self::normal_cdf(-d1);

        let call_delta = Self::normal_cdf(d1);
        let put_delta = call_delta - 1.0;

        let gamma = Self::normal_pdf(d1) / (s * sigma * t.sqrt());

        GreeksResult {
            call_price,
            put_price,
            call_delta,
            put_delta,
            gamma,
            d1,
            d2,
        }
    }

    /// Calculates Gamma Exposure (GEX) for a single strike
    pub fn calculate_gex(s: f64, gamma: f64, oi: f64, is_call: bool) -> f64 {
        // Simple GEX = Gamma * OI * 100 * Spot * Spot (Standard representation)
        // Some use Spot^2 directly, we'll use a normalized version
        let multiplier = if is_call { 1.0 } else { -1.0 };
        gamma * oi * 100.0 * s * s * 0.01 * multiplier
    }

    /// Applies Volume-Weighted Adjustment to GEX
    /// If intraday volume is much higher than daily OI, we increase the GEX weight
    pub fn weighted_gex(gex: f64, volume: f64, oi: f64) -> f64 {
        if oi <= 0.0 {
            return 0.0;
        }
        let weight = (1.0 + (volume / oi).min(5.0)).sqrt(); // Max 5x adjustment
        gex * weight
    }
}

#[derive(Debug, Default)]
pub struct GreeksResult {
    pub call_price: f64,
    pub put_price: f64,
    pub call_delta: f64,
    pub put_delta: f64,
    pub gamma: f64,
    pub d1: f64,
    pub d2: f64,
}
