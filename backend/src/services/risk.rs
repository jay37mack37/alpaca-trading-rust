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

pub struct RiskEngine {
    validators: Vec<Box<dyn RiskValidator>>,
}

impl RiskEngine {
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(MaxPositionSizeValidator),
                Box::new(DailyTradeLimitValidator),
            ],
        }
    }

    pub async fn validate(
        &self,
        db: Arc<tokio::sync::Mutex<crate::services::db::Database>>,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()> {
        if strategy.risk_parameters.is_none() {
            warn!("Risk parameters not set for strategy {}, bypassing risk checks.", strategy.id);
            return Ok(());
        }

        for validator in &self.validators {
            validator.validate(db.clone(), strategy, signal).await?;
        }

        Ok(())
    }
}