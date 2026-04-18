/// Black-Scholes formula for European options
pub fn black_scholes(spot: f64, strike: f64, time: f64, rate: f64, volatility: f64, is_call: bool) -> f64 {
    if time <= 0.0 || volatility <= 0.0 {
        return if is_call {
            (spot - strike).max(0.0)
        } else {
            (strike - spot).max(0.0)
        };
    }

    let d1 = ((spot / strike).ln() + (rate + 0.5 * volatility.powi(2)) * time)
        / (volatility * time.sqrt());
    let d2 = d1 - volatility * time.sqrt();

    let n_d1 = norm_cdf(d1);
    let n_d2 = norm_cdf(d2);

    if is_call {
        spot * n_d1 - strike * (-rate * time).exp() * n_d2
    } else {
        strike * (-rate * time).exp() * (1.0 - n_d2) - spot * (1.0 - n_d1)
    }
}

/// Cumulative normal distribution function
fn norm_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Error function approximation
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}
