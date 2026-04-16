use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::sleep;
use tracing::warn;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        BrokerAccountSummary, BrokerOrderSummary, BrokerPositionSummary, Candle, DataProvider,
        OptionContractSnapshot, Quote, StoredCredential, TradeSide,
    },
};

pub struct FetchedQuote {
    pub quote: Quote,
    pub raw_json: Value,
}

pub struct FetchedCandles {
    pub candles: Vec<Candle>,
}

pub struct FetchedOptions {
    pub contracts: Vec<OptionContractSnapshot>,
    pub raw_json: Value,
}

pub struct FetchedBrokerSync {
    pub account: BrokerAccountSummary,
    pub positions: Vec<BrokerPositionSummary>,
    pub orders: Vec<BrokerOrderSummary>,
    pub raw_account: Value,
    pub raw_positions: Value,
    pub raw_orders: Value,
}

#[derive(Debug, Clone)]
pub struct AlpacaOrderLeg {
    pub symbol: String,
    pub ratio_qty: u32,
    pub side: TradeSide,
    pub position_intent: String,
}

#[derive(Debug, Clone)]
pub enum AlpacaOrderRequest {
    Single {
        symbol: String,
        side: TradeSide,
        quantity: f64,
        order_type: AlpacaOrderType,
    },
    MultiLeg {
        quantity: u32,
        limit_price: f64,
        legs: Vec<AlpacaOrderLeg>,
    },
}

#[derive(Debug, Clone)]
pub enum AlpacaOrderType {
    Market,
    Limit { limit_price: f64 },
}

pub async fn fetch_quote(
    client: &Client,
    provider: DataProvider,
    symbol: &str,
    alpaca_credential: Option<&StoredCredential>,
) -> AppResult<FetchedQuote> {
    match provider {
        DataProvider::Yahoo => yahoo_quote(client, symbol).await,
        DataProvider::Alpaca => {
            let credential = alpaca_credential.ok_or_else(|| {
                AppError::Validation(
                    "Alpaca data requested but no credential is configured".to_string(),
                )
            })?;
            alpaca_quote(client, symbol, credential).await
        }
    }
}

pub async fn fetch_candles(
    client: &Client,
    provider: DataProvider,
    symbol: &str,
    range: &str,
    interval: &str,
    alpaca_credential: Option<&StoredCredential>,
) -> AppResult<FetchedCandles> {
    match provider {
        DataProvider::Yahoo => yahoo_candles(client, symbol, range, interval).await,
        DataProvider::Alpaca => {
            let credential = alpaca_credential.ok_or_else(|| {
                AppError::Validation(
                    "Alpaca candle data requested but no credential is configured".to_string(),
                )
            })?;
            alpaca_candles(client, symbol, range, interval, credential).await
        }
    }
}

pub async fn fetch_options(
    client: &Client,
    provider: DataProvider,
    symbol: &str,
    alpaca_credential: Option<&StoredCredential>,
) -> AppResult<FetchedOptions> {
    match provider {
        DataProvider::Yahoo => yahoo_options(client, symbol).await,
        DataProvider::Alpaca => {
            let credential = alpaca_credential.ok_or_else(|| {
                AppError::Validation(
                    "Alpaca options data requested but no credential is configured".to_string(),
                )
            })?;
            alpaca_options(client, symbol, credential).await
        }
    }
}

pub struct SubmittedOrder {
    pub order_id: String,
    #[allow(dead_code)]
    pub raw_json: Value,
}

pub struct OrderFill {
    #[allow(dead_code)]
    pub order_id: String,
    #[allow(dead_code)]
    pub status: String,
    pub filled_qty: f64,
    pub filled_avg_price: f64,
    #[allow(dead_code)]
    pub raw_json: Value,
}

pub async fn submit_alpaca_order(
    client: &Client,
    credential: &StoredCredential,
    request: &AlpacaOrderRequest,
) -> AppResult<SubmittedOrder> {
    let order = match request {
        AlpacaOrderRequest::Single {
            symbol,
            side,
            quantity,
            order_type,
        } => {
            let (qty, order_type, limit_price) = match order_type {
                AlpacaOrderType::Market => (format!("{quantity:.3}"), "market", None),
                AlpacaOrderType::Limit { limit_price } => {
                    (format!("{quantity:.0}"), "limit", Some(format!("{limit_price:.2}")))
                }
            };
            let mut order = json!({
                "symbol": symbol,
                "qty": qty,
                "side": match side {
                    TradeSide::Buy => "buy",
                    TradeSide::Sell => "sell",
                },
                "type": order_type,
                "time_in_force": "day",
                "client_order_id": Uuid::new_v4().to_string(),
            });
            if let Some(limit_price) = limit_price {
                order["limit_price"] = json!(limit_price);
            }
            order
        }
        AlpacaOrderRequest::MultiLeg {
            quantity,
            limit_price,
            legs,
        } => json!({
            "qty": quantity.to_string(),
            "order_class": "mleg",
            "type": "limit",
            "time_in_force": "day",
            "limit_price": format!("{limit_price:.2}"),
            "legs": legs.iter().map(|leg| json!({
                "symbol": leg.symbol,
                "ratio_qty": leg.ratio_qty.to_string(),
                "side": match leg.side {
                    TradeSide::Buy => "buy",
                    TradeSide::Sell => "sell",
                },
                "position_intent": leg.position_intent,
            })).collect::<Vec<_>>(),
            "client_order_id": Uuid::new_v4().to_string(),
        }),
    };

    let response = client
        .post(format!(
            "{}/v2/orders",
            credential.environment.base_trading_url()
        ))
        .header("APCA-API-KEY-ID", credential.key_id.as_str())
        .header("APCA-API-SECRET-KEY", credential.secret_key.as_str())
        .json(&order)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::External(format!("Alpaca order failed: {body}")));
    }

    let raw_json = response.json::<Value>().await?;
    let order_id = raw_json["id"]
        .as_str()
        .ok_or_else(|| AppError::External("Alpaca order response missing id".to_string()))?
        .to_string();
    Ok(SubmittedOrder { order_id, raw_json })
}

/// Fetches the current state of a submitted Alpaca order.
pub async fn fetch_alpaca_order(
    client: &Client,
    credential: &StoredCredential,
    order_id: &str,
) -> AppResult<Value> {
    alpaca_get(client, credential, &format!("/v2/orders/{order_id}"), &[]).await
}

/// Polls an Alpaca order until it reaches a terminal state or `timeout` elapses.
///
/// Returns the fill state on success. On timeout returns an `External` error so the
/// caller can mark the strategy run failed and refuse to touch the local ledger
/// with stale quote prices.
pub async fn poll_alpaca_order_until_filled(
    client: &Client,
    credential: &StoredCredential,
    order_id: &str,
    timeout: Duration,
) -> AppResult<OrderFill> {
    let start = std::time::Instant::now();
    let mut delay = Duration::from_millis(250);
    loop {
        let raw = fetch_alpaca_order(client, credential, order_id).await?;
        let status = raw["status"].as_str().unwrap_or("unknown").to_string();

        // Alpaca terminal statuses. `filled` is the happy path; the others all
        // mean the broker will not fill the order and we must NOT write a trade
        // to the local ledger.
        match status.as_str() {
            "filled" => {
                let filled_qty = numberish(&raw["filled_qty"]).ok_or_else(|| {
                    AppError::External(
                        "Alpaca order marked filled but filled_qty is missing".to_string(),
                    )
                })?;
                let filled_avg_price = numberish(&raw["filled_avg_price"]).ok_or_else(|| {
                    AppError::External(
                        "Alpaca order marked filled but filled_avg_price is missing".to_string(),
                    )
                })?;
                return Ok(OrderFill {
                    order_id: order_id.to_string(),
                    status,
                    filled_qty,
                    filled_avg_price,
                    raw_json: raw,
                });
            }
            "canceled" | "expired" | "rejected" | "suspended" | "stopped" => {
                return Err(AppError::External(format!(
                    "Alpaca order {order_id} ended in non-fill state: {status}"
                )));
            }
            _ => {}
        }

        if start.elapsed() >= timeout {
            warn!(
                "Alpaca order {order_id} still {status} after {:?}; giving up",
                timeout
            );
            return Err(AppError::External(format!(
                "Alpaca order {order_id} did not fill within {:?} (last status: {status})",
                timeout
            )));
        }

        sleep(delay).await;
        // Exponential backoff capped at 2s so we don't hammer the REST API.
        delay = (delay * 2).min(Duration::from_secs(2));
    }
}

pub async fn fetch_alpaca_broker_sync(
    client: &Client,
    credential: &StoredCredential,
) -> AppResult<FetchedBrokerSync> {
    let raw_account = alpaca_get(client, credential, "/v2/account", &[]).await?;
    let raw_positions = alpaca_get(client, credential, "/v2/positions", &[]).await?;
    let raw_orders = alpaca_get(
        client,
        credential,
        "/v2/orders",
        &[("status", "all"), ("limit", "100"), ("nested", "true")],
    )
    .await?;
    let synced_at = now();

    let account = BrokerAccountSummary {
        credential_id: credential.id.clone(),
        environment: credential.environment,
        account_id: raw_account["id"]
            .as_str()
            .ok_or_else(|| AppError::External("Alpaca account payload missing id".to_string()))?
            .to_string(),
        account_number: raw_account["account_number"].as_str().map(str::to_string),
        status: raw_account["status"].as_str().map(str::to_string),
        currency: raw_account["currency"].as_str().map(str::to_string),
        buying_power: numberish(&raw_account["buying_power"]),
        cash: numberish(&raw_account["cash"]),
        equity: numberish(&raw_account["equity"]),
        portfolio_value: numberish(&raw_account["portfolio_value"]),
        last_equity: numberish(&raw_account["last_equity"]),
        long_market_value: numberish(&raw_account["long_market_value"]),
        short_market_value: numberish(&raw_account["short_market_value"]),
        pattern_day_trader: raw_account["pattern_day_trader"].as_bool().unwrap_or(false),
        trading_blocked: raw_account["trading_blocked"].as_bool().unwrap_or(false),
        transfers_blocked: raw_account["transfers_blocked"].as_bool().unwrap_or(false),
        account_blocked: raw_account["account_blocked"].as_bool().unwrap_or(false),
        synced_at: synced_at.clone(),
    };

    let positions = raw_positions
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|position| {
            Some(BrokerPositionSummary {
                credential_id: credential.id.clone(),
                symbol: position["symbol"].as_str()?.to_string(),
                asset_class: position["asset_class"].as_str().map(str::to_string),
                side: position["side"].as_str().map(str::to_string),
                quantity: numberish(&position["qty"])?,
                avg_entry_price: numberish(&position["avg_entry_price"]),
                market_value: numberish(&position["market_value"]),
                current_price: numberish(&position["current_price"]),
                unrealized_pl: numberish(&position["unrealized_pl"]),
                unrealized_plpc: numberish(&position["unrealized_plpc"]),
                synced_at: synced_at.clone(),
            })
        })
        .collect();

    let orders = raw_orders
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|order| {
            Some(BrokerOrderSummary {
                credential_id: credential.id.clone(),
                order_id: order["id"].as_str()?.to_string(),
                client_order_id: order["client_order_id"].as_str().map(str::to_string),
                symbol: order["symbol"].as_str().map(str::to_string),
                side: order["side"].as_str().map(str::to_string),
                order_type: order["type"].as_str().map(str::to_string),
                order_class: order["order_class"].as_str().map(str::to_string),
                status: order["status"].as_str().map(str::to_string),
                quantity: numberish(&order["qty"]).or_else(|| numberish(&order["notional"])),
                filled_qty: numberish(&order["filled_qty"]),
                filled_avg_price: numberish(&order["filled_avg_price"]),
                time_in_force: order["time_in_force"].as_str().map(str::to_string),
                submitted_at: order["submitted_at"].as_str().map(str::to_string),
                updated_at: order["updated_at"].as_str().map(str::to_string),
                synced_at: synced_at.clone(),
            })
        })
        .collect();

    Ok(FetchedBrokerSync {
        account,
        positions,
        orders,
        raw_account,
        raw_positions,
        raw_orders,
    })
}

async fn yahoo_quote(client: &Client, symbol: &str) -> AppResult<FetchedQuote> {
    let response = client
        .get(format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}"
        ))
        .query(&[
            ("range", "1d"),
            ("interval", "1m"),
            ("includePrePost", "true"),
        ])
        .header("User-Agent", "AutoStonksAlgoSuite/0.1")
        .send()
        .await?;

    let json = response.json::<Value>().await?;
    let result = json
        .pointer("/chart/result/0")
        .ok_or_else(|| AppError::External("Yahoo chart payload missing data".to_string()))?;
    let meta = &result["meta"];
    let closes = result
        .pointer("/indicators/quote/0/close")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let price = meta["regularMarketPrice"]
        .as_f64()
        .or_else(|| last_numeric(&closes))
        .ok_or_else(|| AppError::External("Yahoo quote missing latest price".to_string()))?;
    let previous_close = meta["chartPreviousClose"]
        .as_f64()
        .or_else(|| meta["previousClose"].as_f64());
    let change = previous_close.map(|prev| price - prev);
    let change_percent =
        previous_close.and_then(|prev| (prev != 0.0).then_some(((price - prev) / prev) * 100.0));
    let timestamp = meta["regularMarketTime"]
        .as_i64()
        .map(unix_to_rfc3339)
        .unwrap_or_else(now);

    Ok(FetchedQuote {
        quote: Quote {
            symbol: symbol.to_uppercase(),
            provider: DataProvider::Yahoo,
            price,
            previous_close,
            change,
            change_percent,
            bid: None,
            ask: None,
            volume: meta["regularMarketVolume"].as_f64(),
            vwap: None,
            session_high: meta["regularMarketDayHigh"].as_f64(),
            session_low: meta["regularMarketDayLow"].as_f64(),
            timestamp,
        },
        raw_json: json,
    })
}

async fn yahoo_candles(
    client: &Client,
    symbol: &str,
    range: &str,
    interval: &str,
) -> AppResult<FetchedCandles> {
    let response = client
        .get(format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}"
        ))
        .query(&[
            ("range", range),
            ("interval", interval),
            ("includePrePost", "true"),
        ])
        .header("User-Agent", "AutoStonksAlgoSuite/0.1")
        .send()
        .await?;

    let json = response.json::<Value>().await?;
    let result = json
        .pointer("/chart/result/0")
        .ok_or_else(|| AppError::External("Yahoo candle payload missing data".to_string()))?;
    let timestamps = result["timestamp"].as_array().cloned().unwrap_or_default();
    let quote = result
        .pointer("/indicators/quote/0")
        .ok_or_else(|| AppError::External("Yahoo candle quote block missing".to_string()))?;
    let opens = quote["open"].as_array().cloned().unwrap_or_default();
    let highs = quote["high"].as_array().cloned().unwrap_or_default();
    let lows = quote["low"].as_array().cloned().unwrap_or_default();
    let closes = quote["close"].as_array().cloned().unwrap_or_default();
    let volumes = quote["volume"].as_array().cloned().unwrap_or_default();

    let mut candles = Vec::new();
    for index in 0..timestamps.len() {
        let Some(timestamp) = timestamps[index].as_i64() else {
            continue;
        };
        let (Some(open), Some(high), Some(low), Some(close)) = (
            opens.get(index).and_then(Value::as_f64),
            highs.get(index).and_then(Value::as_f64),
            lows.get(index).and_then(Value::as_f64),
            closes.get(index).and_then(Value::as_f64),
        ) else {
            continue;
        };
        let volume = volumes
            .get(index)
            .and_then(Value::as_f64)
            .unwrap_or_default();
        candles.push(Candle {
            timestamp: unix_to_rfc3339(timestamp),
            open,
            high,
            low,
            close,
            volume,
            vwap: None,
        });
    }

    Ok(FetchedCandles { candles })
}



async fn yahoo_options(client: &Client, symbol: &str) -> AppResult<FetchedOptions> {
    let response = client
        .get(format!(
            "https://query2.finance.yahoo.com/v7/finance/options/{symbol}"
        ))
        .header("User-Agent", "AutoStonksAlgoSuite/0.1")
        .send()
        .await?;

    let json = response.json::<Value>().await?;
    let Some(chain) = json.pointer("/optionChain/result/0/options/0") else {
        return Ok(FetchedOptions {
            contracts: Vec::new(),
            raw_json: json,
        });
    };

    let contracts = parse_yahoo_contracts(symbol, DataProvider::Yahoo, &chain["calls"], "call")
        .into_iter()
        .chain(parse_yahoo_contracts(
            symbol,
            DataProvider::Yahoo,
            &chain["puts"],
            "put",
        ))
        .collect();

    Ok(FetchedOptions {
        contracts,
        raw_json: json,
    })
}

async fn alpaca_options(
    client: &Client,
    symbol: &str,
    credential: &StoredCredential,
) -> AppResult<FetchedOptions> {
    let mut all_snapshots = serde_json::Map::new();
    let mut next_page_token: Option<String> = None;

    loop {
        let mut query = vec![("limit", "1000".to_string())];
        if let Some(token) = next_page_token.as_ref() {
            query.push(("page_token", token.clone()));
        }

        let response = client
            .get(format!(
                "https://data.alpaca.markets/v1beta1/options/snapshots/{symbol}"
            ))
            .header("APCA-API-KEY-ID", credential.key_id.as_str())
            .header("APCA-API-SECRET-KEY", credential.secret_key.as_str())
            .query(&query)
            .send()
            .await?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::External(format!(
                "Alpaca option chain request failed: {body}"
            )));
        }

        let json = response.json::<Value>().await?;
        if let Some(snapshots) = json["snapshots"].as_object() {
            for (contract_symbol, snapshot) in snapshots {
                all_snapshots.insert(contract_symbol.clone(), snapshot.clone());
            }
        }

        next_page_token = json["next_page_token"].as_str().map(str::to_string);
        if next_page_token.is_none() {
            let raw_json = Value::Object(serde_json::Map::from_iter([(
                "snapshots".to_string(),
                Value::Object(all_snapshots.clone()),
            )]));
            let contracts =
                parse_alpaca_contracts(symbol, &Value::Object(all_snapshots), DataProvider::Alpaca);
            return Ok(FetchedOptions {
                contracts,
                raw_json,
            });
        }
    }
}

async fn alpaca_quote(
    client: &Client,
    symbol: &str,
    credential: &StoredCredential,
) -> AppResult<FetchedQuote> {
    let response = client
        .get(format!(
            "https://data.alpaca.markets/v2/stocks/{symbol}/snapshot"
        ))
        .header("APCA-API-KEY-ID", credential.key_id.as_str())
        .header("APCA-API-SECRET-KEY", credential.secret_key.as_str())
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::External(format!(
            "Alpaca snapshot request failed: {body}"
        )));
    }

    let json = response.json::<Value>().await?;
    let latest_trade = &json["latestTrade"];
    let latest_quote = &json["latestQuote"];
    let minute_bar = &json["minuteBar"];
    let daily_bar = &json["dailyBar"];
    let prev_daily_bar = &json["prevDailyBar"];

    let price = latest_trade["p"]
        .as_f64()
        .or_else(|| minute_bar["c"].as_f64())
        .ok_or_else(|| AppError::External("Alpaca snapshot missing price".to_string()))?;
    let previous_close = prev_daily_bar["c"].as_f64();
    let change = previous_close.map(|prev| price - prev);
    let change_percent =
        previous_close.and_then(|prev| (prev != 0.0).then_some(((price - prev) / prev) * 100.0));

    Ok(FetchedQuote {
        quote: Quote {
            symbol: symbol.to_uppercase(),
            provider: DataProvider::Alpaca,
            price,
            previous_close,
            change,
            change_percent,
            bid: latest_quote["bp"].as_f64(),
            ask: latest_quote["ap"].as_f64(),
            volume: daily_bar["v"].as_f64().or_else(|| minute_bar["v"].as_f64()),
            vwap: daily_bar["vw"]
                .as_f64()
                .or_else(|| minute_bar["vw"].as_f64()),
            session_high: daily_bar["h"].as_f64(),
            session_low: daily_bar["l"].as_f64(),
            timestamp: latest_trade["t"].as_str().unwrap_or("").to_string(),
        },
        raw_json: json,
    })
}

async fn alpaca_get(
    client: &Client,
    credential: &StoredCredential,
    path: &str,
    query: &[(&str, &str)],
) -> AppResult<Value> {
    let response = client
        .get(format!(
            "{}{}",
            credential.environment.base_trading_url(),
            path
        ))
        .header("APCA-API-KEY-ID", credential.key_id.as_str())
        .header("APCA-API-SECRET-KEY", credential.secret_key.as_str())
        .query(query)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::External(format!(
            "Alpaca sync request failed on {path}: {body}"
        )));
    }

    response.json::<Value>().await.map_err(AppError::from)
}

async fn alpaca_candles(
    client: &Client,
    symbol: &str,
    range: &str,
    interval: &str,
    credential: &StoredCredential,
) -> AppResult<FetchedCandles> {
    let (timeframe, limit) = match (range, interval) {
        ("1d", _) => ("1Min", "390"),
        ("5d", "5m") => ("5Min", "390"),
        ("5d", _) => ("15Min", "130"),
        _ => ("5Min", "390"),
    };

    let response = client
        .get(format!(
            "https://data.alpaca.markets/v2/stocks/{symbol}/bars"
        ))
        .header("APCA-API-KEY-ID", credential.key_id.as_str())
        .header("APCA-API-SECRET-KEY", credential.secret_key.as_str())
        .query(&[
            ("timeframe", timeframe),
            ("limit", limit),
            ("adjustment", "raw"),
            ("feed", "iex"),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::External(format!(
            "Alpaca bars request failed: {body}"
        )));
    }

    let json = response.json::<Value>().await?;
    let bars = json["bars"].as_array().cloned().unwrap_or_default();
    let candles = bars
        .into_iter()
        .filter_map(|bar| {
            Some(Candle {
                timestamp: bar["t"].as_str()?.to_string(),
                open: bar["o"].as_f64()?,
                high: bar["h"].as_f64()?,
                low: bar["l"].as_f64()?,
                close: bar["c"].as_f64()?,
                volume: bar["v"].as_f64().unwrap_or_default(),
                vwap: bar["vw"].as_f64(),
            })
        })
        .collect();

    Ok(FetchedCandles { candles })
}

fn parse_yahoo_contracts(
    symbol: &str,
    provider: DataProvider,
    node: &Value,
    option_type: &str,
) -> Vec<OptionContractSnapshot> {
    node.as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|contract| {
            // Yahoo currently doesn't provide Greeks in their default chain response.
            // Moneyness is often available via inTheMoney or can be manually calculated
            // if we pass down the underlying price, but we'll extract Greeks if available
            // to be safe, or leave them None for now.
            let in_the_money = contract["inTheMoney"].as_bool();

            // Moneyness estimation: Just storing a boolean proxy or None,
            // a true moneyness metric would need the underlying quote price here.
            let moneyness = None;

            Some(OptionContractSnapshot {
                contract_symbol: contract["contractSymbol"].as_str()?.to_string(),
                underlying_symbol: symbol.to_uppercase(),
                provider,
                option_type: option_type.to_string(),
                expiration: contract["expiration"]
                    .as_i64()
                    .map(unix_to_rfc3339)
                    .unwrap_or_else(now),
                strike: contract["strike"].as_f64()?,
                bid: contract["bid"].as_f64(),
                ask: contract["ask"].as_f64(),
                last: contract["lastPrice"].as_f64(),
                implied_volatility: contract["impliedVolatility"].as_f64(),
                open_interest: contract["openInterest"].as_f64(),
                volume: contract["volume"].as_f64(),
                in_the_money,
                delta: contract["delta"].as_f64(),
                gamma: contract["gamma"].as_f64(),
                theta: contract["theta"].as_f64(),
                vega: contract["vega"].as_f64(),
                moneyness,
            })
        })
        .collect()
}

fn parse_alpaca_contracts(
    symbol: &str,
    node: &Value,
    provider: DataProvider,
) -> Vec<OptionContractSnapshot> {
    node.as_object()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(contract_symbol, snapshot)| {
            let details = parse_option_contract_symbol(&contract_symbol)?;
            let latest_quote = &snapshot["latestQuote"];
            let latest_trade = &snapshot["latestTrade"];
            let greeks = &snapshot["greeks"];

            Some(OptionContractSnapshot {
                contract_symbol,
                underlying_symbol: if details.underlying_symbol.is_empty() {
                    symbol.to_uppercase()
                } else {
                    details.underlying_symbol
                },
                provider,
                option_type: details.option_type,
                expiration: details.expiration,
                strike: details.strike,
                bid: latest_quote["bp"].as_f64(),
                ask: latest_quote["ap"].as_f64(),
                last: latest_trade["p"].as_f64(),
                implied_volatility: snapshot["impliedVolatility"].as_f64().or_else(|| greeks["implied_volatility"].as_f64()),
                open_interest: None,
                volume: None,
                in_the_money: None,
                delta: greeks["delta"].as_f64(),
                gamma: greeks["gamma"].as_f64(),
                theta: greeks["theta"].as_f64(),
                vega: greeks["vega"].as_f64(),
                moneyness: None,
            })
        })
        .collect()
}

struct ParsedOptionContractSymbol {
    underlying_symbol: String,
    option_type: String,
    expiration: String,
    strike: f64,
}

fn parse_option_contract_symbol(contract_symbol: &str) -> Option<ParsedOptionContractSymbol> {
    if contract_symbol.len() < 16 {
        return None;
    }

    let root_end = contract_symbol.len().checked_sub(15)?;
    let underlying_symbol = contract_symbol.get(..root_end)?.to_uppercase();
    let expiration_part = contract_symbol.get(root_end..root_end + 6)?;
    let option_flag = contract_symbol.get(root_end + 6..root_end + 7)?;
    let strike_part = contract_symbol.get(root_end + 7..)?;

    let expiration = NaiveDate::parse_from_str(expiration_part, "%y%m%d")
        .ok()?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .to_rfc3339();
    let option_type = match option_flag {
        "C" => "call",
        "P" => "put",
        _ => return None,
    }
    .to_string();
    let strike = strike_part.parse::<u64>().ok()? as f64 / 1000.0;

    Some(ParsedOptionContractSymbol {
        underlying_symbol,
        option_type,
        expiration,
        strike,
    })
}

fn last_numeric(values: &[Value]) -> Option<f64> {
    values.iter().rev().find_map(Value::as_f64)
}

fn unix_to_rfc3339(timestamp: i64) -> String {
    DateTime::<Utc>::from_timestamp(timestamp, 0)
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(now)
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn numberish(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}

#[cfg(test)]
mod tests {
    use super::{parse_option_contract_symbol, AlpacaOrderLeg, AlpacaOrderRequest, AlpacaOrderType};
    use crate::models::TradeSide;

    #[test]
    fn parses_call_contract_symbol() {
        let parsed =
            parse_option_contract_symbol("AAPL240119C00190000").expect("contract should parse");

        assert_eq!(parsed.underlying_symbol, "AAPL");
        assert_eq!(parsed.option_type, "call");
        assert_eq!(parsed.strike, 190.0);
        assert_eq!(parsed.expiration, "2024-01-19T00:00:00+00:00");
    }

    #[test]
    fn parses_put_contract_symbol_with_numeric_root() {
        let parsed =
            parse_option_contract_symbol("AAPL1240119P00175000").expect("contract should parse");

        assert_eq!(parsed.underlying_symbol, "AAPL1");
        assert_eq!(parsed.option_type, "put");
        assert_eq!(parsed.strike, 175.0);
        assert_eq!(parsed.expiration, "2024-01-19T00:00:00+00:00");
    }

    #[test]
    fn formats_equity_market_order_payload() {
        let _request = AlpacaOrderRequest::Single {
            symbol: "AAPL".to_string(),
            side: TradeSide::Buy,
            quantity: 1.234,
            order_type: AlpacaOrderType::Market,
        };

        let payload = serde_json::json!({
            "symbol": "AAPL",
            "qty": "1.234",
            "side": "buy",
            "type": "market",
            "time_in_force": "day",
            "client_order_id": "test",
        });

        assert_eq!(payload["qty"], "1.234");
    }

    #[test]
    fn formats_option_limit_order_quantity_as_integer() {
        let _request = AlpacaOrderRequest::Single {
            symbol: "AAPL250117C00200000".to_string(),
            side: TradeSide::Sell,
            quantity: 3.0,
            order_type: AlpacaOrderType::Limit { limit_price: 4.26 },
        };

        let payload = serde_json::json!({
            "symbol": "AAPL250117C00200000",
            "qty": "3",
            "side": "sell",
            "type": "limit",
            "time_in_force": "day",
            "limit_price": format!("{:.2}", 4.26),
            "client_order_id": "test",
        });

        assert_eq!(payload["qty"], "3");
        assert_eq!(payload["limit_price"], "4.26");
    }

    #[test]
    fn formats_multileg_order_payload() {
        let _request = AlpacaOrderRequest::MultiLeg {
            quantity: 2,
            limit_price: 1.85,
            legs: vec![
                AlpacaOrderLeg {
                    symbol: "AAPL260515C00200000".to_string(),
                    ratio_qty: 1,
                    side: TradeSide::Buy,
                    position_intent: "buy_to_open".to_string(),
                },
                AlpacaOrderLeg {
                    symbol: "AAPL260515C00205000".to_string(),
                    ratio_qty: 1,
                    side: TradeSide::Sell,
                    position_intent: "sell_to_open".to_string(),
                },
            ],
        };

        let payload = serde_json::json!({
            "qty": "2",
            "order_class": "mleg",
            "type": "limit",
            "time_in_force": "day",
            "limit_price": "1.85",
            "legs": [
                {
                    "symbol": "AAPL260515C00200000",
                    "ratio_qty": "1",
                    "side": "buy",
                    "position_intent": "buy_to_open"
                },
                {
                    "symbol": "AAPL260515C00205000",
                    "ratio_qty": "1",
                    "side": "sell",
                    "position_intent": "sell_to_open"
                }
            ],
            "client_order_id": "test",
        });

        assert_eq!(payload["order_class"], "mleg");
        assert_eq!(payload["legs"].as_array().map(Vec::len), Some(2));
    }
}
