use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{Utc, DateTime, Duration};
use serde::{Serialize, Deserialize};

use crate::error::AppResult;
use crate::services::db::Database;
use crate::models::{TradeRecord, StrategyRecord};

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_pnl: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub max_drawdown_pct: f64,
    pub avg_slippage_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub trades_today: u32,
    pub daily_pnl: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EquityPoint {
    pub timestamp: String,
    pub equity: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub metrics: PerformanceMetrics,
    pub equity_curve: Vec<EquityPoint>,
    pub recent_leakage: Vec<SlippageAudit>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlippageAudit {
    pub symbol: String,
    pub slippage_pct: f64,
    pub executed_at: String,
}

pub struct AnalyticsService {
    db: Arc<Mutex<Database>>,
}

impl AnalyticsService {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    pub async fn get_strategy_performance(&self, strategy_id: &str) -> AppResult<PerformanceReport> {
        let db = self.db.lock().await;
        
        let strategy = db.list_strategy_records()?.into_iter()
            .find(|s| s.id == strategy_id)
            .ok_or_else(|| crate::error::AppError::NotFound(format!("Strategy {}", strategy_id)))?;

        let trades = db.list_trades(Some(strategy_id), 1000)?;
        
        let metrics = self.calculate_metrics(&strategy, &trades)?;
        let equity_curve = self.reconstruct_equity_curve(&strategy, &trades)?;
        let recent_leakage = self.audit_slippage(&trades)?;

        Ok(PerformanceReport {
            metrics,
            equity_curve,
            recent_leakage,
        })
    }

    fn calculate_metrics(&self, strategy: &StrategyRecord, trades: &[TradeRecord]) -> AppResult<PerformanceMetrics> {
        let mut total_wins = 0.0;
        let mut total_losses = 0.0;
        let mut win_count = 0;
        let mut loss_count = 0;
        let mut total_slippage = 0.0;
        let mut slippage_count = 0;

        for trade in trades {
            if let Some(pnl) = trade.realized_pnl {
                if pnl > 0.0 {
                    total_wins += pnl;
                    win_count += 1;
                } else if pnl < 0.0 {
                    total_losses += pnl.abs();
                    loss_count += 1;
                }
            }

            // Slippage calculation
            if let Some(sig_price) = trade.signal_price {
                if sig_price > 0.0 {
                    let slip = (trade.price - sig_price).abs() / sig_price;
                    total_slippage += slip;
                    slippage_count += 1;
                }
            }
        }

        let total_trades = win_count + loss_count;
        let win_rate = if total_trades > 0 { win_count as f64 / total_trades as f64 } else { 0.0 };
        let profit_factor = if total_losses > 0.0 { total_wins / total_losses } else { total_wins };
        let avg_win = if win_count > 0 { total_wins / win_count as f64 } else { 0.0 };
        let avg_loss = if loss_count > 0 { total_losses / loss_count as f64 } else { 0.0 };
        let avg_slippage = if slippage_count > 0 { total_slippage / slippage_count as f64 } else { 0.0 };

        // Sharpe Ratio Calculation (Basic)
        let returns: Vec<f64> = trades.iter().filter_map(|t| t.realized_pnl).collect();
        let sharpe = if returns.len() > 1 {
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
            if variance > 0.0 { mean / variance.sqrt() * (252.0_f64).sqrt() } else { 0.0 }
        } else { 0.0 };

        Ok(PerformanceMetrics {
            total_pnl: strategy.equity - strategy.starting_cash,
            win_rate,
            profit_factor,
            avg_win,
            avg_loss,
            max_drawdown_pct: 0.0, 
            avg_slippage_pct: avg_slippage * 100.0,
            sharpe_ratio: sharpe,
            sortino_ratio: sharpe * 1.1, // Mock sortino for now
            trades_today: 0, 
            daily_pnl: 0.0,
        })
    }

    fn reconstruct_equity_curve(&self, strategy: &StrategyRecord, trades: &[TradeRecord]) -> AppResult<Vec<EquityPoint>> {
        let mut curve = Vec::new();
        let mut current_equity = strategy.starting_cash;

        // Start point
        curve.push(EquityPoint {
            timestamp: "Start".into(), // Better to use actual creation time
            equity: current_equity,
        });

        // Add points for each realized PnL
        for trade in trades {
            if let Some(pnl) = trade.realized_pnl {
                current_equity += pnl;
                curve.push(EquityPoint {
                    timestamp: trade.executed_at.clone(),
                    equity: current_equity,
                });
            }
        }

        Ok(curve)
    }

    fn audit_slippage(&self, trades: &[TradeRecord]) -> AppResult<Vec<SlippageAudit>> {
        Ok(trades.iter()
            .filter_map(|t| {
                t.signal_price.map(|sig| SlippageAudit {
                    symbol: t.instrument_symbol.clone(),
                    slippage_pct: ((t.price - sig).abs() / sig) * 100.0,
                    executed_at: t.executed_at.clone(),
                })
            })
            .take(10)
            .collect())
    }
}
