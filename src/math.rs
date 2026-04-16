/// Cumulative distribution function for the standard normal distribution.
pub fn norm_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x_abs = x.abs() / (2.0f64).sqrt();

    // A&S formula 7.1.26
    let t = 1.0 / (1.0 + p * x_abs);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x_abs * x_abs).exp();

    0.5 * (1.0 + sign * y)
}

/// Black-Scholes option pricing model.
/// S: Current stock price
/// K: Strike price
/// T: Time to expiration (in years)
/// r: Risk-free interest rate (e.g., 0.05 for 5%)
/// sigma: Volatility (e.g., 0.2 for 20%)
/// is_call: true for Call, false for Put
pub fn black_scholes(s: f64, k: f64, t: f64, r: f64, sigma: f64, is_call: bool) -> f64 {
    if t <= 0.0 {
        if is_call {
            return (s - k).max(0.0);
        } else {
            return (k - s).max(0.0);
        }
    }

    let d1 = (s.ln() - k.ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * t.sqrt());
    let d2 = d1 - sigma * t.sqrt();

    if is_call {
        s * norm_cdf(d1) - k * (-r * t).exp() * norm_cdf(d2)
    } else {
        k * (-r * t).exp() * norm_cdf(-d2) - s * norm_cdf(-d1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_norm_cdf() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-7);
        assert!((norm_cdf(1.96) - 0.975).abs() < 1e-3);
        assert!((norm_cdf(-1.96) - 0.025).abs() < 1e-3);
    }

    #[test]
    fn test_black_scholes_call() {
        // Sample values: S=100, K=100, T=1, r=0.05, sigma=0.2
        let price = black_scholes(100.0, 100.0, 1.0, 0.05, 0.2, true);
        assert!((price - 10.45).abs() < 0.01);
    }

    #[test]
    fn test_black_scholes_put() {
        // Sample values: S=100, K=100, T=1, r=0.05, sigma=0.2
        let price = black_scholes(100.0, 100.0, 1.0, 0.05, 0.2, false);
        assert!((price - 5.57).abs() < 0.01);
    }
}
