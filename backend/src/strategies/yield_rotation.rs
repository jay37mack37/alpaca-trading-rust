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

        // 2. Fetch system-wide buying power from the state or broker sync
        // In a real run, we'd check the cached account summary
        // For simplicity, we'll try to get it from the DB or a recent sync
        let bp = 0.0; // TODO: Pull from latest broker sync
        
        // This strategy is unique because it reacts to Account state, not Price state.
        // The implementation details will depend on the executor passing the BP.
        
        hold("Yield rotation requires system-wide BP audit (Handled by sync loop)")
    }
}
