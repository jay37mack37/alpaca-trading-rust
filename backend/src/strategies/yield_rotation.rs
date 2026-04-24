use crate::models::{
    Candle, Quote, SignalAction, StrategyRecord, StrategySignal,
};
use async_trait::async_trait;
use crate::strategies::{TradingStrategy, hold, buy};
use crate::AppState;

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
            )
        );

        // 3. Fetch system-wide buying power from the state or broker sync
        let bp = 0.0; // TODO: Pull from latest broker sync
        
        hold("Yield rotation requires system-wide BP audit (Handled by sync loop)")
    }
}
