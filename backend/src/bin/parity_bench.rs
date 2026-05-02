use std::time::Instant;

#[derive(Clone)]
pub struct OptionContractSnapshot {
    pub contract_symbol: String,
    pub option_type: String,
    pub expiration: String,
    pub strike: f64,
    pub ask: Option<f64>,
}

pub fn calculate_parity_gap(spot_price: f64, call_price: f64, put_price: f64, strike: f64, dte: f64) -> f64 {
    let r = 0.05;
    let t = dte / 365.0;
    let pv_k = strike * (-r * t).exp();
    ((spot_price + put_price - call_price) - pv_k).abs()
}

pub fn evaluate_parity_sniper_benchmark(
    spot_price: f64,
    options: &[OptionContractSnapshot],
) {
    let mut best_gap = 0.0;
    let mut best_strike = 0.0;

    for contract in options {
        if (contract.strike - spot_price).abs() / spot_price > 0.05 {
            continue;
        }

        if contract.option_type.to_lowercase() == "call" {
            let matching_put = options.iter().find(|o| {
                o.strike == contract.strike &&
                o.expiration == contract.expiration &&
                o.option_type.to_lowercase() == "put"
            });

            if let Some(put) = matching_put {
                let call_price = contract.ask.unwrap_or(0.0);
                let put_price = put.ask.unwrap_or(0.0);

                if call_price > 0.0 && put_price > 0.0 {
                    let dte = 30.0;
                    let gap = calculate_parity_gap(spot_price, call_price, put_price, contract.strike, dte);
                    if gap > best_gap {
                        best_gap = gap;
                        best_strike = contract.strike;
                    }
                }
            }
        }
    }

    std::hint::black_box(best_gap);
    std::hint::black_box(best_strike);
}

pub fn evaluate_parity_sniper_optimized(
    spot_price: f64,
    options: &[OptionContractSnapshot],
) {
    use std::collections::HashMap;

    let mut best_gap = 0.0;
    let mut best_strike = 0.0;

    let mut puts_map = HashMap::new();
    for contract in options {
        if (contract.strike - spot_price).abs() / spot_price > 0.05 {
            continue;
        }
        if contract.option_type.eq_ignore_ascii_case("put") {
            puts_map.insert((contract.strike.to_bits(), &contract.expiration), contract);
        }
    }

    for contract in options {
        if (contract.strike - spot_price).abs() / spot_price > 0.05 {
            continue;
        }

        if contract.option_type.eq_ignore_ascii_case("call") {
            if let Some(put) = puts_map.get(&(contract.strike.to_bits(), &contract.expiration)) {
                let call_price = contract.ask.unwrap_or(0.0);
                let put_price = put.ask.unwrap_or(0.0);

                if call_price > 0.0 && put_price > 0.0 {
                    let dte = 30.0;
                    let gap = calculate_parity_gap(spot_price, call_price, put_price, contract.strike, dte);
                    if gap > best_gap {
                        best_gap = gap;
                        best_strike = contract.strike;
                    }
                }
            }
        }
    }

    std::hint::black_box(best_gap);
    std::hint::black_box(best_strike);
}

fn main() {
    let mut options = Vec::new();
    let spot_price = 100.0;

    // Create 2500 options within the 5% threshold (95.0 to 105.0)
    for i in 0..1250 {
        let strike = 95.0 + (i as f64) * 0.008;

        options.push(OptionContractSnapshot {
            contract_symbol: format!("CALL{}", i),
            option_type: "call".to_string(),
            expiration: "2025-01-01".to_string(),
            strike,
            ask: Some(2.1),
        });
        options.push(OptionContractSnapshot {
            contract_symbol: format!("PUT{}", i),
            option_type: "put".to_string(),
            expiration: "2025-01-01".to_string(),
            strike,
            ask: Some(2.1),
        });
    }

    // Add 2000 options outside the threshold to simulate real world
    for i in 0..1000 {
        let strike = 110.0 + (i as f64) * 0.04;

        options.push(OptionContractSnapshot {
            contract_symbol: format!("OUT_CALL{}", i),
            option_type: "call".to_string(),
            expiration: "2025-01-01".to_string(),
            strike,
            ask: Some(2.1),
        });
        options.push(OptionContractSnapshot {
            contract_symbol: format!("OUT_PUT{}", i),
            option_type: "put".to_string(),
            expiration: "2025-01-01".to_string(),
            strike,
            ask: Some(2.1),
        });
    }

    // Warmup
    for _ in 0..100 {
        evaluate_parity_sniper_benchmark(spot_price, &options);
        evaluate_parity_sniper_optimized(spot_price, &options);
    }

    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        evaluate_parity_sniper_benchmark(spot_price, &options);
    }
    let orig_duration = start.elapsed();

    let start = Instant::now();
    for _ in 0..iterations {
        evaluate_parity_sniper_optimized(spot_price, &options);
    }
    let opt_duration = start.elapsed();

    println!("Baseline duration ({} iters): {:?}", iterations, orig_duration);
    println!("Optimized duration ({} iters): {:?}", iterations, opt_duration);
    println!("Speedup: {:.2}x", orig_duration.as_secs_f64() / opt_duration.as_secs_f64());
}
