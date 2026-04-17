use std::{collections::HashSet, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
};
use tracing::warn;

use crate::{
    error::{AppError, AppResult},
    models::{Candle, DataProvider, Quote, RealtimeEvent, StoredCredential},
    services::providers::{fetch_alpaca_broker_sync, fetch_quote},
    AppState,
};

#[derive(Clone)]
pub struct StreamHub {
    tx: broadcast::Sender<RealtimeEvent>,
    market_subs: Arc<Mutex<HashSet<String>>>,
    broker_subs: Arc<Mutex<HashSet<String>>>,
}

impl StreamHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            tx,
            market_subs: Arc::new(Mutex::new(HashSet::new())),
            broker_subs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeEvent> {
        self.tx.subscribe()
    }

    pub async fn ensure_market_stream(
        &self,
        state: AppState,
        symbol: String,
        provider: DataProvider,
        credential: Option<StoredCredential>,
    ) -> AppResult<()> {
        let mut subs = self.market_subs.lock().await;
        let key = format!("{}:{}", provider.as_str(), symbol);
        if subs.contains(&key) {
            return Ok(());
        }

        subs.insert(key.clone());
        let tx = self.tx.clone();
        let market_subs = self.market_subs.clone();

        tokio::spawn(async move {
            loop {
                let result: AppResult<()> = match provider {
                    DataProvider::Yahoo => yahoo_market_loop(&state, &symbol, &tx).await,
                    DataProvider::Alpaca => {
                        alpaca_market_loop(&state, &symbol, credential.as_ref(), &tx).await
                    }
                };

                if let Err(err) = result {
                    warn!("Market stream {key} failed: {err}; retrying in 10s");
                    let _ = tx.send(RealtimeEvent::Status {
                        channel: "market".to_string(),
                        provider: Some(provider),
                        symbol: Some(symbol.clone()),
                        state: "reconnecting".to_string(),
                        message: err.to_string(),
                    });
                } else {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            let mut subs = market_subs.lock().await;
            subs.remove(&key);
        });

        Ok(())
    }

    pub async fn ensure_broker_stream(
        &self,
        state: AppState,
        credential: StoredCredential,
    ) -> AppResult<()> {
        let mut subs = self.broker_subs.lock().await;
        if subs.contains(&credential.id) {
            return Ok(());
        }

        subs.insert(credential.id.clone());
        let tx = self.tx.clone();
        let broker_subs = self.broker_subs.clone();

        tokio::spawn(async move {
            loop {
                if let Err(err) = alpaca_broker_loop(&state, &credential, &tx).await {
                    warn!("Broker stream {} failed: {err}; retrying in 10s", credential.id);
                    let _ = tx.send(RealtimeEvent::Status {
                        channel: "broker".to_string(),
                        provider: Some(DataProvider::Alpaca),
                        symbol: None,
                        state: "reconnecting".to_string(),
                        message: err.to_string(),
                    });
                } else {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            let mut subs = broker_subs.lock().await;
            subs.remove(&credential.id);
        });

        Ok(())
    }
}

async fn yahoo_market_loop(
    state: &AppState,
    symbol: &str,
    tx: &broadcast::Sender<RealtimeEvent>,
) -> AppResult<()> {
    // Yahoo doesn't have a public WebSocket for real-time prices anymore.
    // We'll simulate it with high-frequency polling (e.g., every 2s).
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        interval.tick().await;

        let fetched = match fetch_quote(&state.http, DataProvider::Yahoo, symbol, None).await {
            Ok(quote) => quote,
            Err(err) => {
                warn!("Yahoo market poll failed for {symbol}: {err}");
                continue;
            }
        };

        let strategies = {
            let db = state.db.lock().await;
            let db: &crate::services::db::Database = &*db;
            db.list_strategies()?
        };

        if tx
            .send(RealtimeEvent::Market {
                provider: DataProvider::Yahoo,
                symbol: symbol.to_string(),
                quote: fetched.quote,
                candle: None,
                strategies,
            })
            .is_err()
        {
            break;
        }
    }
    Ok(())
}

async fn alpaca_market_loop(
    state: &AppState,
    symbol: &str,
    credential: Option<&StoredCredential>,
    tx: &broadcast::Sender<RealtimeEvent>,
) -> AppResult<()> {
    let Some(cred) = credential else {
        return Err(AppError::Validation("Missing Alpaca credential".to_string()));
    };

    let ws_url = match cred.environment {
        crate::models::CredentialEnvironment::Paper => "wss://stream.data.alpaca.markets/v2/iex",
        crate::models::CredentialEnvironment::Live => "wss://stream.data.alpaca.markets/v2/sip",
    };

    let (mut ws, _) = connect_async(ws_url).await?;

    // Authenticate
    ws.send(Message::Text(
        json!({
            "action": "auth",
            "key": cred.key_id,
            "secret": cred.secret_key,
        })
        .to_string(),
    ))
    .await?;

    // Wait for success
    if let Some(Ok(Message::Text(text))) = ws.next().await {
        let msg: Value = serde_json::from_str(&text)?;
        if msg[0]["msg"] != "authenticated" {
            return Err(AppError::External(format!("Auth failed: {text}")));
        }
    }

    // Subscribe to trades and bars
    ws.send(Message::Text(
        json!({
            "action": "subscribe",
            "trades": [symbol],
            "bars": [symbol],
        })
        .to_string(),
    ))
    .await?;

    while let Some(Ok(msg)) = ws.next().await {
        if let Message::Text(text) = msg {
            let msgs: Vec<Value> = serde_json::from_str(&text)?;
            for m in msgs {
                match m["T"].as_str() {
                    Some("t") => {
                        // Trade message
                        let price = m["p"].as_f64().unwrap_or_default();
                        let size = m["s"].as_f64().unwrap_or_default();
                        let timestamp = m["t"].as_str().unwrap_or_default().to_string();

                        let quote = Quote {
                            symbol: symbol.to_string(),
                            provider: DataProvider::Alpaca,
                            price,
                            previous_close: None,
                            change: None,
                            change_percent: None,
                            bid: None,
                            ask: None,
                            volume: Some(size),
                            vwap: None,
                            session_high: None,
                            session_low: None,
                            timestamp,
                        };

                        broadcast_market_event(state, tx, DataProvider::Alpaca, symbol, quote, None, &json!({})).await?;
                    }
                    Some("b") => {
                        // Bar message
                        let candle = Candle {
                            timestamp: m["t"].as_str().unwrap_or_default().to_string(),
                            open: m["o"].as_f64().unwrap_or_default(),
                            high: m["h"].as_f64().unwrap_or_default(),
                            low: m["l"].as_f64().unwrap_or_default(),
                            close: m["c"].as_f64().unwrap_or_default(),
                            volume: m["v"].as_f64().unwrap_or_default(),
                            vwap: m["vw"].as_f64(),
                        };

                        let quote = Quote {
                            symbol: symbol.to_string(),
                            provider: DataProvider::Alpaca,
                            price: candle.close,
                            previous_close: None,
                            change: None,
                            change_percent: None,
                            bid: None,
                            ask: None,
                            volume: Some(candle.volume),
                            vwap: candle.vwap,
                            session_high: Some(candle.high),
                            session_low: Some(candle.low),
                            timestamp: candle.timestamp.clone(),
                        };

                        broadcast_market_event(state, tx, DataProvider::Alpaca, symbol, quote, Some(candle), &m).await?;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

async fn alpaca_broker_loop(
    state: &AppState,
    credential: &StoredCredential,
    tx: &broadcast::Sender<RealtimeEvent>,
) -> AppResult<()> {
    let ws_url = match credential.environment {
        crate::models::CredentialEnvironment::Paper => "wss://paper-api.alpaca.markets/stream",
        crate::models::CredentialEnvironment::Live => "wss://api.alpaca.markets/stream",
    };

    let (mut ws, _) = connect_async(ws_url).await?;

    // Authenticate
    ws.send(Message::Text(
        json!({
            "action": "authenticate",
            "data": {
                "key_id": credential.key_id,
                "secret_key": credential.secret_key,
            }
        })
        .to_string(),
    ))
    .await?;

    // Wait for success
    if let Some(Ok(Message::Text(text))) = ws.next().await {
        let msg: Value = serde_json::from_str(&text)?;
        if msg["data"]["status"] != "authorized" {
            return Err(AppError::External(format!("Auth failed: {text}")));
        }
    }

    // Subscribe to trade updates
    ws.send(Message::Text(
        json!({
            "action": "listen",
            "data": {
                "streams": ["trade_updates"]
            }
        })
        .to_string(),
    ))
    .await?;

    while let Some(Ok(msg)) = ws.next().await {
        if let Message::Text(text) = msg {
            let m: Value = serde_json::from_str(&text)?;
            if m["stream"] == "trade_updates" {
                let event = m["data"]["event"].as_str().map(|s| s.to_string());

                // When an order is filled, we want to trigger a full broker sync
                // to reconcile our local paper account with reality.
                let fetched = fetch_alpaca_broker_sync(&state.http, credential).await?;

                let (strategies, strategy_ids) = {
                    let db = state.db.lock().await;
                    let db: &crate::services::db::Database = &*db;
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
                    let strategies = db.list_strategies()?;
                    let ids = db.list_strategy_records()?
                        .into_iter()
                        .filter(|s| s.credential_id.as_deref() == Some(&credential.id))
                        .map(|s| s.id)
                        .collect::<Vec<_>>();
                    (strategies, ids)
                };

                let sync_state = {
                    let db = state.db.lock().await;
                    let db: &crate::services::db::Database = &*db;
                    db.broker_sync_state(&credential.id)?
                        .ok_or_else(|| AppError::Internal("Broker sync failed to persist".to_string()))?
                };

                if tx.send(RealtimeEvent::BrokerSync {
                    credential_id: credential.id.clone(),
                    strategy_ids,
                    broker_sync: sync_state,
                    strategies,
                    event,
                }).is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

async fn broadcast_market_event(
    state: &AppState,
    tx: &broadcast::Sender<RealtimeEvent>,
    provider: DataProvider,
    symbol: &str,
    quote: Quote,
    candle: Option<Candle>,
    raw_json: &Value,
) -> AppResult<()> {
    let strategies = {
        let db = state.db.lock().await;
        let db: &crate::services::db::Database = &*db;
        db.store_market_snapshot(&quote, raw_json)?;
        db.list_strategies()?
    };

    let _ = tx.send(RealtimeEvent::Market {
        provider,
        symbol: symbol.to_string(),
        quote,
        candle,
        strategies,
    });

    Ok(())
}
