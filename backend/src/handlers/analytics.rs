use crate::error::{ApiResponse, AppResult};
use crate::AppState;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PatternAnalysisQuery {
    pub symbols: Option<String>,
    pub provider: Option<String>,
    pub min_confidence: Option<f64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PatternSignal {
    pub symbol: String,
    pub pattern: String,
    pub direction: String,
    pub confidence: f64,
    pub details: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PatternAnalysisResponse {
    pub timestamp: String,
    pub symbols: Vec<String>,
    pub signals: Vec<PatternSignal>,
}

pub async fn run_pattern_analysis(
    State(state): State<AppState>,
    Query(query): Query<PatternAnalysisQuery>,
) -> AppResult<ApiResponse<PatternAnalysisResponse>> {
    let symbols_str = query.symbols.unwrap_or_default();
    let symbols: Vec<String> = symbols_str
        .split(',')
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();

    let symbols = if symbols.is_empty() {
        let db = state.db.lock().await;
        db.list_strategy_records()
            .unwrap_or_default()
            .iter()
            .flat_map(|s| s.tracked_symbols.clone())
            .chain(
                db.list_watchlists()
                    .unwrap_or_default()
                    .into_iter()
                    .flat_map(|w| w.symbols),
            )
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(10)
            .collect()
    } else {
        symbols
    };

    let min_confidence = query.min_confidence.unwrap_or(0.3);
    let mut all_signals = Vec::new();

    for symbol in &symbols {
        let provider = query.provider.as_deref().unwrap_or("yahoo");
        let data_provider = if provider == "alpaca" {
            crate::models::DataProvider::Alpaca
        } else {
            crate::models::DataProvider::Yahoo
        };

        let credential = if data_provider == crate::models::DataProvider::Alpaca {
            crate::services::broker::resolve_alpaca_credential(&state, None, false)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let candles = crate::services::providers::fetch_candles(
            &state.http,
            data_provider,
            symbol,
            "1d",
            "1m",
            credential.as_ref(),
        )
        .await;

        let candle_data = match candles {
            Ok(data) if data.candles.len() >= 20 => data,
            _ => continue,
        };

        let close_prices: Vec<f64> = candle_data.candles.iter().map(|c| c.close).collect();

        // VWAP deviation signal - use only candles with volume data
        let vol_candles: Vec<(&crate::models::Candle, f64)> = candle_data
            .candles
            .iter()
            .filter(|c| c.volume > 0.0)
            .map(|c| (c, c.volume))
            .collect();

        if !vol_candles.is_empty() {
            let total_volume: f64 = vol_candles.iter().map(|(_, v)| *v).sum();
            let vwap: f64 = vol_candles
                .iter()
                .map(|(c, v)| ((c.high + c.low + c.close) / 3.0) * v)
                .sum::<f64>()
                / total_volume;
            let current_price = vol_candles.last().map(|(c, _)| c.close).unwrap_or(0.0);

            if current_price > 0.0 && vwap > 0.0 {
                let deviation_pct = (current_price - vwap) / vwap * 100.0;

                if deviation_pct.abs() > 0.5 {
                    let confidence = (deviation_pct.abs() / 3.0).min(1.0);
                    if confidence >= min_confidence {
                        all_signals.push(PatternSignal {
                            symbol: symbol.clone(),
                            pattern: "vwap_deviation".to_string(),
                            direction: if deviation_pct < 0.0 {
                                "bullish".to_string()
                            } else {
                                "bearish".to_string()
                            },
                            confidence: (confidence * 100.0).round() / 100.0,
                            details: serde_json::json!({
                                "vwap": (vwap * 100.0).round() / 100.0,
                                "price": (current_price * 100.0).round() / 100.0,
                                "deviation_pct": (deviation_pct * 100.0).round() / 100.0,
                            }),
                        });
                    }
                }
            }
        }

        // Volume spike signal - use candles with actual volume data
        let vol_with_prices: Vec<(f64, f64, f64)> = candle_data
            .candles
            .iter()
            .filter(|c| c.volume > 0.0)
            .map(|c| (c.close, c.open, c.volume))
            .collect();

        if vol_with_prices.len() >= 20 {
            let window = 20.min(vol_with_prices.len());
            let recent: Vec<f64> = vol_with_prices[vol_with_prices.len() - window..]
                .iter()
                .map(|(_, _, v)| *v)
                .collect();
            let avg: f64 = recent.iter().sum::<f64>() / window as f64;
            let std_dev =
                (recent.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / window as f64).sqrt();
            let (last_close, last_open, current_vol) = vol_with_prices[vol_with_prices.len() - 1];

            if std_dev > 0.0 && avg > 0.0 {
                let z_score = (current_vol - avg) / std_dev;
                if z_score > 2.0 {
                    let confidence = (z_score / 5.0).min(1.0);
                    if confidence >= min_confidence {
                        all_signals.push(PatternSignal {
                            symbol: symbol.clone(),
                            pattern: "unusual_volume".to_string(),
                            direction: if last_close >= last_open {
                                "bullish".to_string()
                            } else {
                                "bearish".to_string()
                            },
                            confidence: (confidence * 100.0).round() / 100.0,
                            details: serde_json::json!({
                                "volume": current_vol as i64,
                                "avg_volume": avg as i64,
                                "z_score": (z_score * 100.0).round() / 100.0,
                            }),
                        });
                    }
                }
            }
        }

        // Momentum / rate of change signal
        let roc_periods = [5, 10, 20];
        for period in roc_periods {
            if close_prices.len() > period {
                let current = close_prices[close_prices.len() - 1];
                let past = close_prices[close_prices.len() - 1 - period];
                if past > 0.0 {
                    let roc_pct = (current - past) / past * 100.0;
                    if roc_pct.abs() > 1.0 {
                        let confidence = (roc_pct.abs() / 5.0).min(1.0);
                        if confidence >= min_confidence {
                            all_signals.push(PatternSignal {
                                symbol: symbol.clone(),
                                pattern: format!("momentum_{period}d"),
                                direction: if roc_pct > 0.0 {
                                    "bullish".to_string()
                                } else {
                                    "bearish".to_string()
                                },
                                confidence: (confidence * 100.0).round() / 100.0,
                                details: serde_json::json!({
                                    "roc_pct": (roc_pct * 100.0).round() / 100.0,
                                    "period": period,
                                }),
                            });
                        }
                    }
                }
            }
        }
    }

    all_signals.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(ApiResponse {
        success: true,
        data: Some(PatternAnalysisResponse {
            timestamp: chrono::Utc::now().to_rfc3339(),
            symbols,
            signals: all_signals,
        }),
        error: None,
    })
}
