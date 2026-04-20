use crate::error::{AppError, AppResult};
use crate::models::{BrokerSyncState, DataProvider, RealtimeEvent};
use crate::services::providers::fetch_alpaca_broker_sync;
use crate::AppState;

pub async fn resolve_alpaca_credential(
    state: &AppState,
    preferred_id: Option<&str>,
    require_trading: bool,
) -> AppResult<Option<crate::models::StoredCredential>> {
    let db = state.db.lock().await;
    db.resolve_alpaca_credential(preferred_id, require_trading)
}

pub async fn sync_strategy_broker_state(
    state: &AppState,
    strategy_id: &str,
) -> AppResult<BrokerSyncState> {
    let strategy = {
        let db = state.db.lock().await;
        db.list_strategy_records()?
            .into_iter()
            .find(|strategy| strategy.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?
    };

    if !strategy.execution_mode.requires_external_broker() {
        return Err(AppError::Validation(
            "strategy is not configured for Alpaca execution".to_string(),
        ));
    }

    let credential: Option<crate::models::StoredCredential> = resolve_alpaca_credential(state, strategy.credential_id.as_deref(), false)
        .await?;
    let credential = credential.ok_or_else(|| AppError::Validation("missing Alpaca credential".to_string()))?;

    let fetched = fetch_alpaca_broker_sync(&state.http, &credential, state.config.mock_alpaca).await?;

    {
        let mut db = state.db.lock().await;
        db.store_broker_sync(
            &credential.id,
            credential.environment,
            &fetched.account,
            &fetched.positions,
            &fetched.orders,
            &fetched.raw_account,
            &fetched.raw_positions,
            &fetched.raw_orders,
        )?;
    }

    let db = state.db.lock().await;
    db.broker_sync_state(&credential.id)?
        .ok_or_else(|| AppError::Internal("broker sync did not persist".to_string()))
}

pub fn stream_matches(
    event: &RealtimeEvent,
    provider: DataProvider,
    symbol: &str,
    strategy_ids: &[String],
    credential_id: Option<&str>,
) -> bool {
    match event {
        RealtimeEvent::Market {
            provider: event_provider,
            symbol: event_symbol,
            ..
        } => *event_provider == provider && event_symbol == symbol,
        RealtimeEvent::BrokerSync {
            credential_id: event_credential_id,
            strategy_ids: event_strategy_ids,
            ..
        } => {
            credential_id == Some(event_credential_id.as_str())
                || event_strategy_ids
                    .iter()
                    .any(|event_strategy_id| strategy_ids.contains(event_strategy_id))
        }
        RealtimeEvent::Status {
            channel,
            provider: event_provider,
            symbol: event_symbol,
            ..
        } => match channel.as_str() {
            "market" => {
                event_provider == &Some(provider) && event_symbol.as_deref() == Some(symbol)
            }
            "broker" => credential_id.is_some(),
            _ => true,
        },
        RealtimeEvent::Log { strategy_id: event_strategy_id, .. } => {
            strategy_ids.contains(event_strategy_id)
        }
    }
}