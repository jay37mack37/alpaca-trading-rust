use crate::models::{Candle, Quote, StrategyRecord, StrategySignal};
use crate::strategies::{buy, hold, TradingStrategy};
use crate::AppState;
use async_trait::async_trait;

/// Treasury Yield Rotation Strategy
/// Rotates idle cash into $SGOV if BP > $1000 for prolonged periods.
pub struct YieldRotationStrategy;

#[async_trait]
impl TradingStrategy for YieldRotationStrategy {
    async fn evaluate(
        &self,
        state: &AppState,
        _strategy: &StrategyRecord,
        _candles: &[Candle],
        _quote: &Quote,
        _options: &[crate::models::OptionContractSnapshot],
        position: Option<&crate::models::PositionRecord>,
        _kronos_score: Option<f64>,
    ) -> StrategySignal {
        // 1. If we already have a position, we just hold (The manager handles exits)
        if position.is_some() {
            return hold("Currently harvesting yield in SGOV");
        }

        // 2. Logging for console visibility
        crate::agents::broadcast_audit_log(
            state,
            crate::logger::SystemEvent::now(
                crate::logger::SystemSource::System,
                Some(_strategy.id.clone()),
                "SGOV".to_string(),
                crate::logger::SystemEventType::Scan,
                "Audit".to_string(),
                0.5,
                "YIELD AUDIT: Checking idle cash thresholds for SGOV rotation.".to_string(),
            ),
        );

        // 3. Fetch current Buying Power from the strategy's linked account
        let db = state.db.lock().await;
        let bp = db
            .strategy_broker_sync(&_strategy.id)
            .ok()
            .flatten()
            .and_then(|s| s.account)
            .and_then(|a| a.buying_power)
            .unwrap_or(0.0);

        // 4. The Trigger: Idle Cash > $2000
        if bp > 2000.0 {
            return buy(
                format!("Rotating Idle Cash (${:.2}) to SGOV", bp),
                0.95, // Use 95% of BP for SGOV
                None,
            );
        }

        hold(format!(
            "Insufficient idle cash for yield rotation (${:.2})",
            bp
        ))
    }
}
