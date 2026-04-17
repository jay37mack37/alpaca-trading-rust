use tokio::time::{self, Duration};
use dashmap::DashMap;
use serde_json::Value;
use tokio::process::Command;

use crate::strategies::StrategyTrait;
use crate::math::black_scholes;
use crate::api::alpaca::AlpacaClient;
use crate::models::order::OrderRequest;

pub struct ListingArbitrage {
    pub name: String,
    pub cache: DashMap<String, bool>,
    pub alpaca: AlpacaClient,
}

impl ListingArbitrage {
    pub fn new(alpaca: AlpacaClient) -> Self {
        Self {
            name: "Listing Arbitrage".to_string(),
            cache: DashMap::new(),
            alpaca,
        }
    }

    async fn get_kronos_sentiment(&self, symbol: &str) -> (String, f64) {
        let output = Command::new("python3")
            .arg("analytics/kronos_bridge.py")
            .arg(symbol)
            .output()
            .await;

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(v) = serde_json::from_str::<Value>(&stdout) {
                let trend = v["trend"].as_str().unwrap_or("neutral").to_string();
                let confidence = v["confidence"].as_f64().unwrap_or(0.5);
                return (trend, confidence);
            }
        }
        ("neutral".to_string(), 0.5)
    }

    fn log_decision(&self, symbol: &str, math_edge: &str, kronos_score: &str, decision: &str, reasoning: &str) {
        use std::io::Write;
        let file_res = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("strategies.log");
            
        let mut file = match file_res {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Failed to open strategies.log for writing: {}", e);
                return;
            }
        };

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = serde_json::json!({
            "time": timestamp.to_string(),
            "symbol": symbol,
            "math_edge": math_edge,
            "kronos_score": kronos_score,
            "decision": decision,
            "reasoning": reasoning
        });
        
        if let Err(e) = writeln!(file, "{}", log_entry.to_string()) {
            tracing::error!("Failed to write to strategies.log: {}", e);
        }
    }
}

impl StrategyTrait for ListingArbitrage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn run(&self) {
        tracing::info!("Starting Listing Arbitrage engine...");
        self.log_decision("SPY", "INITIALIZING", "N/A", "HEARTBEAT", "Strategy engine online. Connecting to Alpaca Market Data...");

        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            self.log_decision("SPY", "SCANNING", "N/A", "SCAN", "Polling Alpaca for $SPY option chain snapshots...");
            
            let options_chain_result = self.alpaca.get_option_chain("SPY").await;
            
            match options_chain_result {
                Ok(chain) => {
                    let s = chain.underlying_price;
                    
                    if chain.strikes.is_empty() {
                        self.log_decision("SPY", &format!("Price: ${:.2}", s), "N/A", "SCAN", "No active options strikes found for current filter range.");
                        continue;
                    }

                    // Pick an At-The-Money option from the filtered strikes
                    let target_strike = chain.strikes.iter().min_by(|a, b| {
                        (a.strike - s).abs().partial_cmp(&(b.strike - s).abs()).unwrap_or(std::cmp::Ordering::Equal)
                    });

                    if let Some(target_strike) = target_strike {
                        let live_strike_symbol = target_strike.call.symbol.clone();
                        if live_strike_symbol.is_empty() {
                            self.log_decision("SPY", "N/A", "N/A", "SCAN", "Selected strike has no valid OCC symbol.");
                            continue;
                        }
                        
                        let market_ask = target_strike.call.ask;
                        if market_ask <= 0.0 {
                            self.log_decision(&live_strike_symbol, "N/A", "N/A", "SCAN", "Symbol found but Ask price is $0.00 (Likely after-hours or illiquid)");
                            continue;
                        }

                        // 2. Calculate Fair Value via Black-Scholes using active network quote
                        let k = target_strike.strike;
                        let t = 0.5;     // Mock 6 months to expiry
                        let r = 0.05;    // Mock 5% interest rate
                        let sigma = 0.2; // Mock 20% volatility

                        let fair_value = black_scholes(s, k, t, r, sigma, true);

                        // 3. Kronos Filter
                        let (trend, confidence) = self.get_kronos_sentiment("SPY").await;
                        
                        let math_edge = format!("Fair: {:.2}, Ask: {:.2}, Edge: {:.2}%", fair_value, market_ask, ((fair_value - market_ask) / market_ask) * 100.0);
                        let kronos_score = format!("Conf: {:.2}, {}", confidence, trend);

                        // 4. Sniping Decision
                        if (fair_value - market_ask > 0.05) && (trend == "bullish" || trend == "neutral") {
                            self.log_decision(
                                &live_strike_symbol, 
                                &math_edge, 
                                &kronos_score, 
                                "BUY", 
                                "EDGE DETECTED: Executing Market Order via Alpaca Paper Trading."
                            );
                            
                            let order = OrderRequest {
                                symbol: live_strike_symbol.clone(),
                                qty: 1.0,
                                side: "buy".to_string(),
                                order_type: "market".to_string(),
                                time_in_force: "day".to_string(),
                                limit_price: None,
                                asset_class: Some("option".to_string()),
                            };

                            match self.alpaca.create_order(order).await {
                                Ok(res) => {
                                    tracing::info!("Paper order placed successfully: {:?}", res);
                                    self.log_decision(&live_strike_symbol, "FILLED", "VERIFIED", "BUY", &format!("Order Confirmed: {:?}", res.get("id").unwrap_or(&serde_json::Value::String("Unknown ID".to_string()))));
                                }
                                Err(e) => {
                                    tracing::error!("Paper order failed: {:?}", e);
                                    self.log_decision(&live_strike_symbol, "ERROR", "N/A", "SYSTEM", &format!("Order Failed: {}", e));
                                }
                            }

                        } else {
                            let reason = if fair_value - market_ask <= 0.05 {
                                format!("Skip: Low/Neg Edge (${:.2})", fair_value - market_ask)
                            } else {
                                format!("Skip: Kronos Trend mismatch ({})", trend)
                            };
                            self.log_decision(
                                &live_strike_symbol, 
                                &math_edge, 
                                &kronos_score, 
                                "SKIP", 
                                &reason
                            );
                        }
                    }
                }
                Err(e) => {
                    self.log_decision("SPY", "CONNECTION ERROR", "N/A", "SYSTEM", &format!("Alpaca API Error: {}. Retrying in 5s...", e));
                }
            }
        }
    }
}
