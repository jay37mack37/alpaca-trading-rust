use axum::{
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
};
use crate::models::{RealtimeStreamQuery, DataProvider, normalize_symbol};
use crate::error::{AppResult, AppError};
use crate::{AppState, resolve_alpaca_credential, stream_matches};
use crate::services::db::Database;
use std::convert::Infallible;
use std::time::Duration;
use serde_json::json;

pub async fn realtime_stream(
    State(state): State<AppState>,
    Query(query): Query<RealtimeStreamQuery>,
) -> AppResult<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>> {
    let symbol = normalize_symbol(query.symbol.as_deref().unwrap_or("AAPL"));
    let provider = query.provider.unwrap_or(DataProvider::Yahoo);
    let market_credential = match provider {
        DataProvider::Yahoo => None,
        DataProvider::Alpaca => resolve_alpaca_credential(&state, None, false).await?,
    };
    state
        .streams
        .ensure_market_stream(
            state.clone(),
            symbol.clone(),
            provider,
            market_credential.clone(),
        )
        .await?;

    let mut tracked_strategy_ids = Vec::new();
    let mut tracked_credential_id = None;
    if let Some(strategy_id) = query
        .strategy_id
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        let strategy = {
            let db = state.db.lock().await;
            let db: &Database = &*db;
            db.list_strategy_records()?
                .into_iter()
                .find(|candidate| candidate.id == strategy_id)
                .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?
        };

        tracked_strategy_ids.push(strategy.id.clone());
        if strategy.execution_mode.requires_external_broker() {
            let credential =
                resolve_alpaca_credential(&state, strategy.credential_id.as_deref(), false)
                    .await?
                    .ok_or_else(|| AppError::Validation("missing Alpaca credential".to_string()))?;
            tracked_credential_id = Some(credential.id.clone());
            state
                .streams
                .ensure_broker_stream(state.clone(), credential)
                .await?;
        }
    }

    let mut receiver = state.streams.subscribe();
    let filter_symbol = symbol.clone();
    let filter_strategy_ids = tracked_strategy_ids.clone();
    let filter_credential_id = tracked_credential_id.clone();
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("ready").data(json!({
            "provider": provider,
            "symbol": symbol,
            "strategy_ids": tracked_strategy_ids,
            "credential_id": tracked_credential_id,
        }).to_string()));
        loop {
            match receiver.recv().await {
                Ok(event) if stream_matches(
                    &event,
                    provider,
                    &filter_symbol,
                    &filter_strategy_ids,
                    filter_credential_id.as_deref(),
                ) => {
                    if let Ok(data) = serde_json::to_string(&event) {
                        yield Ok(Event::default().event(event.event_name()).data(data));
                    }
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    yield Ok(Event::default().event("status").data(json!({
                        "type": "status",
                        "channel": "stream",
                        "provider": provider,
                        "symbol": filter_symbol,
                        "state": "lagged",
                        "message": format!("Dropped {skipped} realtime events; continuing with latest data")
                    }).to_string()));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    ))
}
