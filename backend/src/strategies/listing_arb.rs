use crate::models::{OptionContractSnapshot, PositionRecord, Quote, SignalAction, StrategySignal};
use chrono::{Local, NaiveDate};

/// Listing Arbitrage Strategy
/// 
/// Uses Black-Scholes fair value calculation to identify options listed 
/// at fair value discrepancies. Parses the OCC symbol format (YYMMDD) to 
/// extract expiration and calculates theoretical fair value using Black-Scholes.
pub fn evaluate_listing_arbitrage(
    option: &OptionContractSnapshot,
    underlying_quote: &Quote,
    position: Option<&PositionRecord>,
) -> StrategySignal {
    // Parse expiration from OCC symbol
    let expiration = match parse_expiration_from_occ(&option.contract_symbol) {
        Some(exp) => exp,
        None => return hold("Unable to parse expiration from OCC symbol"),
    };

    // Validate we have required market data
    let mid_price = match (option.bid, option.ask) {
        (Some(bid), Some(ask)) if bid > 0.0 && ask > 0.0 => (bid + ask) / 2.0,
        _ => return hold("Missing bid/ask quotes"),
    };

    let iv = match option.implied_volatility {
        Some(v) if v > 0.0 => v,
        _ => return hold("Implied volatility unavailable"),
    };

    // Calculate days to expiration
    let dte = match days_until_expiration(&expiration) {
        Some(d) if d > 0 => d as f64,
        _ => return hold("Invalid expiration date or expired"),
    };

    // Calculate Black-Scholes fair value
    let fair_value = black_scholes_call(
        underlying_quote.price,
        option.strike,
        dte / 365.0,
        iv,
        0.05, // risk-free rate
    );

    // Generate signal based on fair value
    let spread = mid_price - fair_value;
    let spread_pct = if fair_value != 0.0 {
        spread / fair_value
    } else {
        0.0
    };

    match (position, spread_pct) {
        // Buy signal: option is underpriced
        (None, sp) if sp < -0.02 => StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.15,
            reason: format!(
                "Listing arbitrage: {:.2}% under fair value (${:.2} vs ${:.2})",
                spread_pct * 100.0, mid_price, fair_value
            ),
        },
        // Sell signal: position exists AND option is overpriced
        (Some(_), sp) if sp > 0.02 => StrategySignal {
            action: SignalAction::Sell,
            allocation_fraction: 1.0,
            reason: format!(
                "Listing arbitrage: {:.2}% over fair value, closing position",
                spread_pct * 100.0,
            ),
        },
        // Hold
        _ => hold(&format!(
            "Awaiting setup: spread={:.2}%, fair_value=${:.2}",
            spread_pct * 100.0, fair_value
        )),
    }
}

/// Parse OCC option symbol to extract expiration date
/// Format: [Underlying][YY][MM][DD][C/P][Strike]
/// Example: AAPL250419C00150000 -> Some("2025-04-19")
fn parse_expiration_from_occ(symbol: &str) -> Option<String> {
    if symbol.len() < 21 {
        return None;
    }

    // Find the C or P that marks the option type
    let option_type_pos = symbol.find(|c| c == 'C' || c == 'P')?;

    // The expiration should be 6 characters before the option type
    if option_type_pos < 6 {
        return None;
    }

    let exp_start = option_type_pos - 6;
    let exp_str = &symbol[exp_start..option_type_pos];

    if exp_str.len() != 6 {
        return None;
    }

    // Parse YY, MM, DD
    let yy: u32 = exp_str[0..2].parse().ok()?;
    let mm: u32 = exp_str[2..4].parse().ok()?;
    let dd: u32 = exp_str[4..6].parse().ok()?;

    // Convert YY to full year (00-30 = 2000-2030, 31-99 = 1931-1999)
    let year = if yy <= 30 { 2000 + yy } else { 1900 + yy };

    // Validate month and day
    if mm < 1 || mm > 12 || dd < 1 || dd > 31 {
        return None;
    }

    Some(format!("{:04}-{:02}-{:02}", year, mm, dd))
}

/// Calculate days until expiration date
fn days_until_expiration(expiration_str: &str) -> Option<i64> {
    let today = Local::now().date_naive();
    let exp_date = NaiveDate::parse_from_str(expiration_str, "%Y-%m-%d").ok()?;
    let duration = exp_date.signed_duration_since(today);
    Some(duration.num_days())
}

/// Black-Scholes formula for European call option
fn black_scholes_call(spot: f64, strike: f64, time: f64, volatility: f64, rate: f64) -> f64 {
    if time <= 0.0 || volatility <= 0.0 {
        return (spot - strike).max(0.0);
    }

    let d1 = ((spot / strike).ln() + (rate + 0.5 * volatility.powi(2)) * time)
        / (volatility * time.sqrt());
    let d2 = d1 - volatility * time.sqrt();

    let n_d1 = norm_cdf(d1);
    let n_d2 = norm_cdf(d2);

    spot * n_d1 - strike * (-rate * time).exp() * n_d2
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

fn hold(reason: &str) -> StrategySignal {
    StrategySignal {
        action: SignalAction::Hold,
        allocation_fraction: 0.0,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_expiration_from_occ() {
        let symbol = "AAPL250419C00150000";
        assert_eq!(
            parse_expiration_from_occ(symbol),
            Some("2025-04-19".to_string())
        );

        let symbol = "SPY251231P05000000";
        assert_eq!(
            parse_expiration_from_occ(symbol),
            Some("2025-12-31".to_string())
        );

        assert_eq!(parse_expiration_from_occ("INVALID"), None);
        assert_eq!(parse_expiration_from_occ("AAPL"), None);
    }

    #[test]
    fn test_black_scholes_call() {
        let value = black_scholes_call(100.0, 100.0, 1.0, 0.2, 0.05);
        assert!(value > 10.0 && value < 11.0);

        let itm_value = black_scholes_call(110.0, 100.0, 1.0, 0.2, 0.05);
        assert!(itm_value > value);

        let otm_value = black_scholes_call(90.0, 100.0, 1.0, 0.2, 0.05);
        assert!(otm_value < value);

        let intrinsic = black_scholes_call(100.0, 90.0, 0.0, 0.2, 0.05);
        assert!((intrinsic - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_norm_cdf() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 0.01);
        assert!(norm_cdf(1.0) > 0.5 && norm_cdf(1.0) < 1.0);
        assert!(norm_cdf(-1.0) < 0.5 && norm_cdf(-1.0) > 0.0);
    }
}
