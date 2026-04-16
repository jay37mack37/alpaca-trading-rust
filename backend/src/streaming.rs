use std::{collections::HashSet, sync::Arc, time::Duration};

use axum::http::HeaderValue;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{TimeZone, Timelike, Utc};
use futures_util::{SinkExt, StreamExt};
use prost::Message as _;
use serde_json::{json, Value};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, Message},
};
use tracing::warn;

use crate::{
    error::{AppError, AppResult},
    models::{Candle, DataProvider, Quote, RealtimeEvent, StoredCredential, StrategySummary},
    providers::{fetch_alpaca_broker_sync, fetch_quote},
    AppState,
};

#[derive(Clone)]
pub struct StreamHub {
    sender: broadcast::Sender<RealtimeEvent>,
    market_workers: Arc<Mutex<HashSet<String>>>,
    broker_workers: Arc<Mutex<HashSet<String>>>,
}

impl StreamHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(512);
        Self {
            sender,
            market_workers: Arc::new(Mutex::new(HashSet::new())),
            broker_workers: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeEvent> {
        self.sender.subscribe()
    }

    pub async fn ensure_market_stream(
        &self,
        state: AppState,
        symbol: String,
        provider: DataProvider,
        credential: Option<StoredCredential>,
    ) -> AppResult<()> {
        let key = match (provider, credential.as_ref()) {
            (DataProvider::Yahoo, _) => format!("yahoo:{symbol}"),
            (DataProvider::Alpaca, Some(credential)) => {
                format!("alpaca:{}:{symbol}", credential.id)
            }
            (DataProvider::Alpaca, None) => {
                return Err(AppError::Validation(
                    "Alpaca streaming requires a configured data credential".to_string(),
                ));
            }
        };

        let mut workers = self.market_workers.lock().await;
        if !workers.insert(key.clone()) {
            return Ok(());
        }
        drop(workers);

        let hub = self.clone();
        tokio::spawn(async move {
            let result = match provider {
                DataProvider::Yahoo => {
                    run_yahoo_market_stream(state, hub.clone(), symbol.clone()).await
                }
                DataProvider::Alpaca => {
                    run_alpaca_market_stream(
                        state,
                        hub.clone(),
                        symbol.clone(),
                        credential.expect("credential checked above"),
                    )
                    .await
                }
            };

            if let Err(err) = result {
                warn!("market stream worker failed for {key}: {err}");
                hub.emit(RealtimeEvent::Status {
                    channel: "market".to_string(),
                    provider: Some(provider),
                    symbol: Some(symbol.clone()),
                    state: "failed".to_string(),
                    message: err.to_string(),
                });
            }
            hub.market_workers.lock().await.remove(&key);
        });

        Ok(())
    }

    pub async fn ensure_broker_stream(
        &self,
        state: AppState,
        credential: StoredCredential,
    ) -> AppResult<()> {
        let key = format!("broker:{}", credential.id);
        let mut workers = self.broker_workers.lock().await;
        if !workers.insert(key.clone()) {
            return Ok(());
        }
        drop(workers);

        let hub = self.clone();
        tokio::spawn(async move {
            let credential_id = credential.id.clone();
            if let Err(err) = run_alpaca_broker_stream(state, hub.clone(), credential).await {
                warn!("broker stream worker failed for {credential_id}: {err}");
                hub.emit(RealtimeEvent::Status {
                    channel: "broker".to_string(),
                    provider: Some(DataProvider::Alpaca),
                    symbol: None,
                    state: "failed".to_string(),
                    message: err.to_string(),
                });
            }
            hub.broker_workers.lock().await.remove(&key);
        });

        Ok(())
    }

    fn emit(&self, event: RealtimeEvent) {
        let _ = self.sender.send(event);
    }
}

#[derive(Clone, PartialEq, prost::Message)]
struct YahooTicker {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(float, tag = "2")]
    price: f32,
    #[prost(sint64, tag = "3")]
    time: i64,
    #[prost(float, tag = "8")]
    change_percent: f32,
    #[prost(sint64, tag = "9")]
    day_volume: i64,
    #[prost(float, tag = "10")]
    day_high: f32,
    #[prost(float, tag = "11")]
    day_low: f32,
    #[prost(float, tag = "12")]
    change: f32,
    #[prost(float, tag = "15")]
    open_price: f32,
    #[prost(float, tag = "16")]
    previous_close: f32,
    #[prost(float, tag = "23")]
    bid: f32,
    #[prost(float, tag = "25")]
    ask: f32,
}

struct YahooStreamState {
    last_quote: Quote,
    last_session_volume: f64,
}

struct AlpacaStreamState {
    last_quote: Quote,
    cumulative_price_volume: f64,
}

async fn run_yahoo_market_stream(state: AppState, hub: StreamHub, symbol: String) -> AppResult<()> {
    let seed = fetch_quote(&state.http, DataProvider::Yahoo, &symbol, None).await?;
    let mut stream_state = YahooStreamState {
        last_session_volume: seed.quote.volume.unwrap_or_default(),
        last_quote: seed.quote,
    };

    hub.emit(RealtimeEvent::Status {
        channel: "market".to_string(),
        provider: Some(DataProvider::Yahoo),
        symbol: Some(symbol.clone()),
        state: "connecting".to_string(),
        message: "Connecting to Yahoo realtime stream".to_string(),
    });

    loop {
        let (mut socket, _) = connect_async("wss://streamer.finance.yahoo.com/").await?;
        socket
            .send(Message::Text(
                json!({ "subscribe": [symbol.clone()] }).to_string().into(),
            ))
            .await?;

        hub.emit(RealtimeEvent::Status {
            channel: "market".to_string(),
            provider: Some(DataProvider::Yahoo),
            symbol: Some(symbol.clone()),
            state: "live".to_string(),
            message: "Yahoo realtime stream connected".to_string(),
        });

        while let Some(message) = socket.next().await {
            match message? {
                Message::Text(payload) => {
                    if let Some((quote, candle, raw_json)) =
                        yahoo_message_to_update(payload.as_ref(), &mut stream_state)?
                    {
                        publish_market_update(
                            &state,
                            &hub,
                            DataProvider::Yahoo,
                            symbol.clone(),
                            quote,
                            candle,
                            raw_json,
                        )
                        .await?;
                    }
                }
                Message::Binary(payload) => {
                    let text = String::from_utf8(payload.to_vec())
                        .map_err(|err| AppError::External(err.to_string()))?;
                    if let Some((quote, candle, raw_json)) =
                        yahoo_message_to_update(&text, &mut stream_state)?
                    {
                        publish_market_update(
                            &state,
                            &hub,
                            DataProvider::Yahoo,
                            symbol.clone(),
                            quote,
                            candle,
                            raw_json,
                        )
                        .await?;
                    }
                }
                Message::Close(_) => break,
                Message::Ping(payload) => {
                    socket.send(Message::Pong(payload)).await?;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
            }
        }

        hub.emit(RealtimeEvent::Status {
            channel: "market".to_string(),
            provider: Some(DataProvider::Yahoo),
            symbol: Some(symbol.clone()),
            state: "reconnecting".to_string(),
            message: "Yahoo realtime stream disconnected; retrying".to_string(),
        });
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_alpaca_market_stream(
    state: AppState,
    hub: StreamHub,
    symbol: String,
    credential: StoredCredential,
) -> AppResult<()> {
    let seed = fetch_quote(
        &state.http,
        DataProvider::Alpaca,
        &symbol,
        Some(&credential),
    )
    .await?;
    let mut stream_state = AlpacaStreamState {
        cumulative_price_volume: seed.quote.vwap.unwrap_or(seed.quote.price)
            * seed.quote.volume.unwrap_or_default(),
        last_quote: seed.quote,
    };

    hub.emit(RealtimeEvent::Status {
        channel: "market".to_string(),
        provider: Some(DataProvider::Alpaca),
        symbol: Some(symbol.clone()),
        state: "connecting".to_string(),
        message: "Connecting to Alpaca market stream".to_string(),
    });

    loop {
        let mut request = "wss://stream.data.alpaca.markets/v2/iex".into_client_request()?;
        request
            .headers_mut()
            .insert("Content-Type", HeaderValue::from_static("application/json"));
        let (mut socket, _) = connect_async(request).await?;

        socket
            .send(Message::Text(
                json!({
                    "action": "auth",
                    "key": credential.key_id,
                    "secret": credential.secret_key
                })
                .to_string()
                .into(),
            ))
            .await?;
        socket
            .send(Message::Text(
                json!({
                    "action": "subscribe",
                    "trades": [symbol.clone()],
                    "quotes": [symbol.clone()],
                    "bars": [symbol.clone()]
                })
                .to_string()
                .into(),
            ))
            .await?;

        hub.emit(RealtimeEvent::Status {
            channel: "market".to_string(),
            provider: Some(DataProvider::Alpaca),
            symbol: Some(symbol.clone()),
            state: "live".to_string(),
            message: "Alpaca market stream connected".to_string(),
        });

        while let Some(message) = socket.next().await {
            match message? {
                Message::Text(payload) => {
                    for item in parse_alpaca_message(payload.as_ref())? {
                        if let Some((quote, candle, raw_json)) =
                            alpaca_market_message_to_update(item, &mut stream_state)?
                        {
                            publish_market_update(
                                &state,
                                &hub,
                                DataProvider::Alpaca,
                                symbol.clone(),
                                quote,
                                candle,
                                raw_json,
                            )
                            .await?;
                        }
                    }
                }
                Message::Binary(payload) => {
                    let text = String::from_utf8(payload.to_vec())
                        .map_err(|err| AppError::External(err.to_string()))?;
                    for item in parse_alpaca_message(&text)? {
                        if let Some((quote, candle, raw_json)) =
                            alpaca_market_message_to_update(item, &mut stream_state)?
                        {
                            publish_market_update(
                                &state,
                                &hub,
                                DataProvider::Alpaca,
                                symbol.clone(),
                                quote,
                                candle,
                                raw_json,
                            )
                            .await?;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(payload) => {
                    socket.send(Message::Pong(payload)).await?;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
            }
        }

        hub.emit(RealtimeEvent::Status {
            channel: "market".to_string(),
            provider: Some(DataProvider::Alpaca),
            symbol: Some(symbol.clone()),
            state: "reconnecting".to_string(),
            message: "Alpaca market stream disconnected; retrying".to_string(),
        });
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_alpaca_broker_stream(
    state: AppState,
    hub: StreamHub,
    credential: StoredCredential,
) -> AppResult<()> {
    let url = match credential.environment {
        crate::models::CredentialEnvironment::Paper => "wss://paper-api.alpaca.markets/stream",
        crate::models::CredentialEnvironment::Live => "wss://api.alpaca.markets/stream",
    };
    let credential_id = credential.id.clone();

    hub.emit(RealtimeEvent::Status {
        channel: "broker".to_string(),
        provider: Some(DataProvider::Alpaca),
        symbol: None,
        state: "connecting".to_string(),
        message: format!("Connecting to Alpaca broker stream for {credential_id}"),
    });

    loop {
        let mut request = url.into_client_request()?;
        request
            .headers_mut()
            .insert("Content-Type", HeaderValue::from_static("application/json"));
        let (mut socket, _) = connect_async(request).await?;

        socket
            .send(Message::Text(
                json!({
                    "action": "auth",
                    "key": credential.key_id,
                    "secret": credential.secret_key
                })
                .to_string()
                .into(),
            ))
            .await?;
        socket
            .send(Message::Text(
                json!({
                    "action": "listen",
                    "data": { "streams": ["trade_updates"] }
                })
                .to_string()
                .into(),
            ))
            .await?;

        let _ =
            sync_broker_snapshot(&state, &hub, &credential, Some("initial_sync".to_string())).await;

        hub.emit(RealtimeEvent::Status {
            channel: "broker".to_string(),
            provider: Some(DataProvider::Alpaca),
            symbol: None,
            state: "live".to_string(),
            message: format!("Alpaca broker stream connected for {credential_id}"),
        });

        while let Some(message) = socket.next().await {
            match message? {
                Message::Text(payload) => {
                    for item in parse_alpaca_message(payload.as_ref())? {
                        if item["stream"].as_str() == Some("trade_updates") {
                            let event_name = item["data"]["event"].as_str().map(str::to_string);
                            sync_broker_snapshot(&state, &hub, &credential, event_name).await?;
                        }
                    }
                }
                Message::Binary(payload) => {
                    let text = String::from_utf8(payload.to_vec())
                        .map_err(|err| AppError::External(err.to_string()))?;
                    for item in parse_alpaca_message(&text)? {
                        if item["stream"].as_str() == Some("trade_updates") {
                            let event_name = item["data"]["event"].as_str().map(str::to_string);
                            sync_broker_snapshot(&state, &hub, &credential, event_name).await?;
                        }
                    }
                }
                Message::Close(_) => break,
                Message::Ping(payload) => {
                    socket.send(Message::Pong(payload)).await?;
                }
                Message::Pong(_) => {}
                Message::Frame(_) => {}
            }
        }

        hub.emit(RealtimeEvent::Status {
            channel: "broker".to_string(),
            provider: Some(DataProvider::Alpaca),
            symbol: None,
            state: "reconnecting".to_string(),
            message: format!("Alpaca broker stream disconnected for {credential_id}; retrying"),
        });
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn yahoo_message_to_update(
    payload: &str,
    stream_state: &mut YahooStreamState,
) -> AppResult<Option<(Quote, Option<Candle>, Value)>> {
    let bytes = BASE64
        .decode(payload)
        .map_err(|err| AppError::External(err.to_string()))?;
    let ticker =
        YahooTicker::decode(bytes.as_slice()).map_err(|err| AppError::External(err.to_string()))?;
    if ticker.id.is_empty() || ticker.price <= 0.0 {
        return Ok(None);
    }

    let timestamp = timestamp_from_millis(ticker.time)?;
    let previous_close =
        some_positive_f64(ticker.previous_close as f64).or(stream_state.last_quote.previous_close);
    let price = ticker.price as f64;
    let change = previous_close
        .map(|prev| price - prev)
        .or_else(|| some_non_negative_signed(ticker.change as f64));
    let change_percent = previous_close
        .and_then(|prev| (prev != 0.0).then_some(((price - prev) / prev) * 100.0))
        .or_else(|| some_non_negative_signed(ticker.change_percent as f64));
    let session_volume = if ticker.day_volume > 0 {
        ticker.day_volume as f64
    } else {
        stream_state.last_session_volume
    };

    let quote = Quote {
        symbol: ticker.id.clone(),
        provider: DataProvider::Yahoo,
        price,
        previous_close,
        change,
        change_percent,
        bid: some_positive_f64(ticker.bid as f64).or(stream_state.last_quote.bid),
        ask: some_positive_f64(ticker.ask as f64).or(stream_state.last_quote.ask),
        volume: (session_volume > 0.0).then_some(session_volume),
        vwap: stream_state.last_quote.vwap,
        session_high: some_positive_f64(ticker.day_high as f64)
            .or(stream_state.last_quote.session_high),
        session_low: some_positive_f64(ticker.day_low as f64)
            .or(stream_state.last_quote.session_low),
        timestamp: timestamp.clone(),
    };

    let candle = synthesize_tick_candle(
        &stream_state.last_quote,
        &quote,
        stream_state.last_session_volume,
        session_volume,
    );

    stream_state.last_session_volume = session_volume;
    stream_state.last_quote = quote.clone();

    Ok(Some((
        quote,
        candle,
        json!({
            "id": ticker.id,
            "price": price,
            "time": ticker.time,
            "change_percent": ticker.change_percent,
            "day_volume": ticker.day_volume,
            "day_high": ticker.day_high,
            "day_low": ticker.day_low,
            "change": ticker.change,
            "open_price": ticker.open_price,
            "previous_close": ticker.previous_close,
            "bid": ticker.bid,
            "ask": ticker.ask
        }),
    )))
}

fn alpaca_market_message_to_update(
    item: Value,
    stream_state: &mut AlpacaStreamState,
) -> AppResult<Option<(Quote, Option<Candle>, Value)>> {
    let message_type = item["T"].as_str().unwrap_or_default();
    match message_type {
        "t" => {
            let price = item["p"].as_f64().ok_or_else(|| {
                AppError::External("Alpaca trade stream missing price".to_string())
            })?;
            let timestamp = item["t"]
                .as_str()
                .unwrap_or(stream_state.last_quote.timestamp.as_str())
                .to_string();
            let mut quote = stream_state.last_quote.clone();
            quote.price = price;
            quote.timestamp = timestamp;
            stream_state.last_quote = quote.clone();
            Ok(Some((quote, None, item)))
        }
        "q" => {
            let mut quote = stream_state.last_quote.clone();
            quote.bid = item["bp"].as_f64().or(quote.bid);
            quote.ask = item["ap"].as_f64().or(quote.ask);
            quote.timestamp = item["t"]
                .as_str()
                .unwrap_or(quote.timestamp.as_str())
                .to_string();
            stream_state.last_quote = quote.clone();
            Ok(Some((quote, None, item)))
        }
        "b" => {
            let candle = Candle {
                timestamp: item["t"]
                    .as_str()
                    .ok_or_else(|| {
                        AppError::External("Alpaca bar stream missing timestamp".to_string())
                    })?
                    .to_string(),
                open: item["o"].as_f64().unwrap_or(stream_state.last_quote.price),
                high: item["h"].as_f64().unwrap_or(stream_state.last_quote.price),
                low: item["l"].as_f64().unwrap_or(stream_state.last_quote.price),
                close: item["c"].as_f64().unwrap_or(stream_state.last_quote.price),
                volume: item["v"].as_f64().unwrap_or_default(),
                vwap: item["vw"].as_f64(),
            };

            stream_state.cumulative_price_volume +=
                candle.vwap.unwrap_or(candle.close) * candle.volume;

            let total_volume = stream_state.last_quote.volume.unwrap_or_default() + candle.volume;
            let total_vwap =
                (total_volume > 0.0).then_some(stream_state.cumulative_price_volume / total_volume);

            let mut quote = stream_state.last_quote.clone();
            quote.price = candle.close;
            quote.volume = (total_volume > 0.0).then_some(total_volume);
            quote.vwap = total_vwap;
            quote.session_high = Some(quote.session_high.unwrap_or(candle.high).max(candle.high));
            quote.session_low = Some(quote.session_low.unwrap_or(candle.low).min(candle.low));
            quote.timestamp = candle.timestamp.clone();
            stream_state.last_quote = quote.clone();
            Ok(Some((quote, Some(candle), item)))
        }
        _ => Ok(None),
    }
}

async fn publish_market_update(
    state: &AppState,
    hub: &StreamHub,
    provider: DataProvider,
    symbol: String,
    quote: Quote,
    candle: Option<Candle>,
    raw_json: Value,
) -> AppResult<()> {
    let strategies = persist_market_quote(state, &quote, &raw_json).await?;
    hub.emit(RealtimeEvent::Market {
        provider,
        symbol,
        quote,
        candle,
        strategies,
    });
    Ok(())
}

async fn persist_market_quote(
    state: &AppState,
    quote: &Quote,
    raw_json: &Value,
) -> AppResult<Vec<StrategySummary>> {
    let db = state.db.lock().await;
    db.store_market_snapshot(quote, raw_json)?;
    db.mark_symbol_price(&quote.symbol, quote.price)?;
    db.list_strategies()
}

async fn sync_broker_snapshot(
    state: &AppState,
    hub: &StreamHub,
    credential: &StoredCredential,
    event_name: Option<String>,
) -> AppResult<()> {
    let fetched = fetch_alpaca_broker_sync(&state.http, credential).await?;
    let (broker_sync, strategies, strategy_ids) = {
        let db = state.db.lock().await;
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
        let broker_sync = db
            .broker_sync_state(&credential.id)?
            .ok_or_else(|| AppError::Internal("broker sync did not persist".to_string()))?;
        let strategies = db.list_strategies()?;
        let strategy_ids = strategies
            .iter()
            .filter(|strategy| strategy.credential_id.as_deref() == Some(credential.id.as_str()))
            .map(|strategy| strategy.id.clone())
            .collect();
        (broker_sync, strategies, strategy_ids)
    };

    hub.emit(RealtimeEvent::BrokerSync {
        credential_id: credential.id.clone(),
        strategy_ids,
        broker_sync,
        strategies,
        event: event_name,
    });
    Ok(())
}

fn parse_alpaca_message(payload: &str) -> AppResult<Vec<Value>> {
    let json = serde_json::from_str::<Value>(payload)?;
    Ok(match json {
        Value::Array(items) => items,
        other => vec![other],
    })
}

fn timestamp_from_millis(value: i64) -> AppResult<String> {
    let date_time = Utc.timestamp_millis_opt(value).single().ok_or_else(|| {
        AppError::External("stream payload carried an invalid timestamp".to_string())
    })?;
    Ok(date_time.to_rfc3339())
}

fn synthesize_tick_candle(
    last_quote: &Quote,
    quote: &Quote,
    previous_session_volume: f64,
    session_volume: f64,
) -> Option<Candle> {
    let parsed = chrono::DateTime::parse_from_rfc3339(&quote.timestamp).ok()?;
    let minute = parsed
        .with_second(0)?
        .with_nanosecond(0)?
        .with_timezone(&Utc)
        .to_rfc3339();
    let volume_delta = (session_volume - previous_session_volume).max(0.0);
    Some(Candle {
        timestamp: minute,
        open: last_quote.price,
        high: last_quote.price.max(quote.price),
        low: last_quote.price.min(quote.price),
        close: quote.price,
        volume: volume_delta,
        vwap: quote.vwap,
    })
}

fn some_positive_f64(value: f64) -> Option<f64> {
    (value > 0.0).then_some(value)
}

fn some_non_negative_signed(value: f64) -> Option<f64> {
    (!value.is_nan()).then_some(value)
}
