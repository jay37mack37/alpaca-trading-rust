use crate::models::{StrategyRecord, StrategySignal};
use crate::error::{AppResult, AppError};
use async_trait::async_trait;
use tracing::warn;
use std::sync::Arc;

#[async_trait]
pub trait RiskValidator: Send + Sync {
    async fn validate(
        &self,
        db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()>;
}

pub struct MaxPositionSizeValidator;

#[async_trait]
impl RiskValidator for MaxPositionSizeValidator {
    async fn validate(
        &self,
        _db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()> {
        let risk_params = match &strategy.risk_parameters {
            Some(rp) => rp,
            None => return Ok(()),
        };

        // If strategy.equity is 0 (new strategy), fallback to starting cash
        let equity = if strategy.equity > 0.0 { strategy.equity } else { strategy.starting_cash };
        let trade_value = equity * signal.allocation_fraction;

        if trade_value > risk_params.max_position_size && risk_params.max_position_size > 0.0 {
            return Err(AppError::Validation(format!(
                "Position size limit exceeded. Requested: ${:.2}, Limit: ${:.2}",
                trade_value, risk_params.max_position_size
            )));
        }

        Ok(())
    }
}

pub struct DailyTradeLimitValidator;

#[async_trait]
impl RiskValidator for DailyTradeLimitValidator {
    async fn validate(
        &self,
        db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        _signal: &StrategySignal,
    ) -> AppResult<()> {
        let risk_params = match &strategy.risk_parameters {
            Some(rp) => rp,
            None => return Ok(()),
        };

        if let Some(limit) = risk_params.max_daily_trades {
            if limit > 0 {
                let count = {
                    let db_lock = db.lock().await;
                    db_lock.get_count_trades_today(&strategy.id)?
                };

                if count >= limit {
                    return Err(AppError::Validation(format!(
                        "Daily trade limit reached ({} trades). Strategy {} is blocked until tomorrow.",
                        limit, strategy.name
                    )));
                }
            }
        }

        Ok(())
    }
}

pub struct HardAccountCapValidator;

#[async_trait]
impl RiskValidator for HardAccountCapValidator {
    async fn validate(
        &self,
        _db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()> {
        let equity = if strategy.equity > 0.0 { strategy.equity } else { strategy.starting_cash };
        let trade_value = equity * signal.allocation_fraction;
        
        if trade_value > 950.0 {
            return Err(AppError::Validation(format!(
                "HARD SENTRY: Order value ${:.2} exceeds the $950 safety cap.",
                trade_value
            )));
        }

        if trade_value > 900.0 {
             return Err(AppError::Validation(format!(
                "HARD SENTRY: Order value ${:.2} exceeds the $900 total account heat limit.",
                trade_value
            )));
        }

        Ok(())
    }
}

pub struct DailyDrawdownValidator;

#[async_trait]
impl RiskValidator for DailyDrawdownValidator {
    async fn validate(
        &self,
        db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        _signal: &StrategySignal,
    ) -> AppResult<()> {
        let pnl_today = {
            let db_lock = db.lock().await;
            db_lock.get_realized_pnl_today(&strategy.id)?
        };

        if pnl_today < -100.0 {
            return Err(AppError::Validation(format!(
                "HARD SENTRY: Daily realized loss (${:.2}) exceeds the $100 (10%) safety limit. Trading blocked.",
                pnl_today.abs()
            )));
        }

        Ok(())
    }
}


pub struct SpamFilterValidator;

#[async_trait]
impl RiskValidator for SpamFilterValidator {
    async fn validate(
        &self,
        db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        _signal: &StrategySignal,
    ) -> AppResult<()> {
        let count = {
            let db_lock = db.lock().await;
            db_lock.get_count_trades_today(&strategy.id)?
        };

        if count > 50 {
             return Err(AppError::Validation(format!(
                "HARD SENTRY: Daily trade count ({}) exceeded. Safety lock engaged to prevent loops.",
                count
            )));
        }

        Ok(())
    }
}

pub struct RiskEngine {
    validators: Vec<Box<dyn RiskValidator>>,
}

impl RiskEngine {
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(MaxPositionSizeValidator),
                Box::new(DailyTradeLimitValidator),
                Box::new(HardAccountCapValidator),
                Box::new(DailyDrawdownValidator),
                Box::new(SpamFilterValidator),
            ],
        }
    }

    pub async fn validate(
        &self,
        db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()> {
        for validator in &self.validators {
            validator.validate(db.clone(), strategy, signal).await?;
        }

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{StrategyKind, ExecutionMode, AssetClassTarget, OptionEntryStyle, OptionStructurePreset, SignalAction, RiskParameters};
    use crate::services::db::Database;
    use tempfile::tempdir;
    use rusqlite::params;
    use std::path::Path;

    fn make_test_strategy() -> StrategyRecord {
        StrategyRecord {
            id: "test-strat".into(),
            name: "Test Strategy".into(),
            kind: StrategyKind::VwapReversion,
            enabled: true,
            execution_mode: ExecutionMode::LocalPaper,
            asset_class_target: AssetClassTarget::Equity,
            option_entry_style: OptionEntryStyle::LongCall,
            option_structure_preset: OptionStructurePreset::Single,
            option_spread_width: 0.0,
            option_target_delta: 0.0,
            option_dte_min: 0,
            option_dte_max: 0,
            option_max_spread_pct: 0.0,
            option_limit_buffer_pct: 0.0,
            credential_id: None,
            starting_cash: 1000.0,
            cash_balance: 1000.0,
            equity: 1000.0,
            tracked_symbols: vec!["SPY".into()],
            total_trades: 0,
            wins: 0,
            losses: 0,
            last_signal: None,
            last_run_at: None,
            run_interval_ms: 0,
            state_json: serde_json::json!({}),
            risk_parameters: Some(RiskParameters {
                max_position_size: 250.0,
                max_daily_loss: 50.0,
                blacklisted_symbols: vec![],
                max_daily_trades: Some(50),
            }),
            performance_stats_json: None,
            shared_cash: None,
            use_shared_cash: false,
        }
    }

    #[tokio::test]
    async fn test_hard_account_cap_validator() {
        let validator = HardAccountCapValidator;
        let strategy = make_test_strategy();
        let db_dir = tempdir().unwrap();
        let db_path = db_dir.path().join("test.db");
        let db = Arc::new(tokio::sync::Mutex::new(Database::open(&db_path, &[], "test-master-key-strong-enough").unwrap()));

        // Case 1: Under limit
        let signal = StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.2, 
            ..Default::default()
        };
        assert!(validator.validate(db.clone(), &strategy, &signal).await.is_ok());

        // Case 2: Over limit
        let signal = StrategySignal {
            action: SignalAction::Buy,
            allocation_fraction: 0.3, 
            ..Default::default()
        };
        let result = validator.validate(db.clone(), &strategy, &signal).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HARD SENTRY"));
    }

    #[tokio::test]
    async fn test_daily_drawdown_validator() {
        let validator = DailyDrawdownValidator;
        let strategy = make_test_strategy();
        let db_dir = tempdir().unwrap();
        let db_path = db_dir.path().join("test.db");
        let db = Database::open(&db_path, &[], "test-master-key-strong-enough").unwrap();
        
        // Mock a loss
        db.get_conn().execute(
            "INSERT INTO strategies (id, name, kind, enabled, execution_mode, asset_class_target, starting_cash, cash_balance, equity, tracked_symbols, total_trades, wins, losses, state_json) 
             VALUES (?, 'Test', 'vwap_reversion', 1, 'LocalPaper', 'equity', 1000.0, 1000.0, 1000.0, '[]', 0, 0, 0, '{}')",
            params![strategy.id],
        ).unwrap();

        db.get_conn().execute(
            "INSERT INTO trade_log (id, strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, side, quantity, price, multiplier, provider, reason, execution_mode, realized_pnl, executed_at, hidden, signal_price, slippage_pnl) 
             VALUES (?, ?, 'SPY', 'SPY', 'SPY', 'Equity', 'buy', 10, 100.0, 1, 'Alpaca', 'test', 'LocalPaper', -60.0, datetime('now'), 0, 100.0, 0.0)",
            params![uuid::Uuid::new_v4().to_string(), strategy.id],
        ).unwrap();

        let db_arc = Arc::new(tokio::sync::Mutex::new(db));
        let signal = StrategySignal::default();
        
        let result = validator.validate(db_arc, &strategy, &signal).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HARD SENTRY"));
    }
}
