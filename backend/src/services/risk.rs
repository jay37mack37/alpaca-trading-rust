use crate::models::{StrategyRecord, StrategySignal};
use crate::error::{AppResult, AppError};
use async_trait::async_trait;
use tracing::warn;

#[async_trait]
pub trait RiskValidator: Send + Sync {
    async fn validate(
        &self,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()>;
}

pub struct MaxPositionSizeValidator;

#[async_trait]
impl RiskValidator for MaxPositionSizeValidator {
    async fn validate(
        &self,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()> {
        let risk_params = match &strategy.risk_parameters {
            Some(rp) => rp,
            None => return Ok(()),
        };

        let trade_value = strategy.equity * signal.allocation_fraction;

        if trade_value > risk_params.max_position_size {
            return Err(AppError::Validation(format!(
                "Position size limit exceeded. Requested: ${:.2}, Limit: ${:.2}",
                trade_value, risk_params.max_position_size
            )));
        }

        Ok(())
    }
}

// TODO: MaxDailyLossValidator — requires daily watermark tracking (start_of_day_equity)
// and trade_log queries. Omitted from MVP to avoid misleading users with a no-op check.

pub struct RiskEngine {
    validators: Vec<Box<dyn RiskValidator>>,
}

impl RiskEngine {
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(MaxPositionSizeValidator),
            ],
        }
    }

    pub async fn validate(
        &self,
        strategy: &StrategyRecord,
        signal: &StrategySignal,
    ) -> AppResult<()> {
        if strategy.risk_parameters.is_none() {
            warn!("Risk parameters not set for strategy {}, bypassing risk checks.", strategy.id);
            return Ok(());
        }

        for validator in &self.validators {
            validator.validate(strategy, signal).await?;
        }

        Ok(())
    }
}