use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::time::{self, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::api::ws_manager::WsManager;
use crate::models::websocket::{QuoteUpdate, TradeUpdate, WsUpdate};

pub struct PriceStreamer {
    ws_manager: Arc<WsManager>,
    api_key: Option<String>,
    api_secret: Option<String>,
}

impl PriceStreamer {
    pub fn new(
        ws_manager: Arc<WsManager>,
        api_key: Option<String>,
        api_secret: Option<String>,
    ) -> Self {
        Self {
            ws_manager,
            api_key,
            api_secret,
        }
    }

    pub async fn start(&self) {
        if let (Some(key), Some(secret)) = (&self.api_key, &self.api_secret) {
            let ws_manager = self.ws_manager.clone();
            let key = key.clone();
            let secret = secret.clone();

            tokio::spawn(async move {
                run_alpaca_streams(ws_manager, key, secret).await;
            });
        } else {
            let ws_manager = self.ws_manager.clone();
            tokio::spawn(async move {
                run_fallback_polling(ws_manager).await;
            });
        }
    }
}

async fn run_alpaca_streams(ws_manager: Arc<WsManager>, key: String, secret: String) {
    // We need two streams: Stocks and Options
    let stock_url = "wss://stream.data.alpaca.markets/v2/iex";
    let option_url = "wss://stream.data.alpaca.markets/v1beta1/options";

    let stock_handle = tokio::spawn(handle_alpaca_ws(
        ws_manager.clone(),
        stock_url.to_string(),
        key.clone(),
        secret.clone(),
        Some(false),
    ));

    let option_handle = tokio::spawn(handle_alpaca_ws(
        ws_manager,
        option_url.to_string(),
        key,
        secret,
        Some(true),
    ));

    let _ = tokio::join!(stock_handle, option_handle);
}

async fn handle_alpaca_ws(
    ws_manager: Arc<WsManager>,
    url: String,
    key: String,
    secret: String,
    is_options: Option<bool>, // is_options is used for logging/special handling if needed
) {
    loop {
        tracing::info!("Connecting to Alpaca WS: {}", url);
        match connect_async(&url).await {
            Ok((mut ws_stream, _)) => {
                // 1. Wait for welcome message
                if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
                    tracing::info!("Received welcome: {}", text);
                }

                // 2. Authenticate
                let auth_msg = json!({
                    "action": "auth",
                    "key": key,
                    "secret": secret
                });
                let _ = ws_stream
                    .send(Message::Text(auth_msg.to_string().into()))
                    .await;

                // 3. Wait for auth success
                if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
                    tracing::info!("Auth response: {}", text);
                    if !text.contains("authenticated") {
                        time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }

                // 4. Main loop
                let mut last_subscribed_symbols: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                let mut interval = time::interval(Duration::from_secs(1));

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            let current_vec = ws_manager.get_active_symbols();
                            let current_symbols: std::collections::HashSet<String> = current_vec.iter().cloned().collect();

                            if current_symbols != last_subscribed_symbols {
                                // Filter symbols based on whether this is stock or option WS
                                // (Alpaca expects certain formats)
                                let filtered: Vec<String> = current_symbols.iter()
                                    .filter(|s| {
                                        let is_opt = s.len() > 10 && s.chars().any(|c| c.is_ascii_digit());
                                        // Simplified: stocks if not looks like OCC, options if looks like OCC
                                        // This matches the user's assumption of OCC format
                                        if is_options.unwrap_or(false) { is_opt } else { !is_opt }
                                    })
                                    .cloned()
                                    .collect();

                                if !filtered.is_empty() {
                                    let sub_msg = json!({
                                        "action": "subscribe",
                                        "trades": filtered,
                                        "quotes": filtered
                                    });
                                    let _ = ws_stream.send(Message::Text(sub_msg.to_string().into())).await;
                                }
                                last_subscribed_symbols = current_symbols;
                            }
                        }
                        msg = ws_stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(updates) = serde_json::from_str::<Value>(&text) {
                                        if let Some(arr) = updates.as_array() {
                                            for item in arr {
                                                process_alpaca_message(item, &ws_manager);
                                            }
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None => break,
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("WS Connection error: {}. Retrying in 5s...", e);
                time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

fn process_alpaca_message(item: &Value, ws_manager: &Arc<WsManager>) {
    let msg_type = item.get("T").and_then(|t| t.as_str()).unwrap_or("");
    match msg_type {
        "t" | "trd" => {
            let symbol = item.get("S").and_then(|s| s.as_str()).unwrap_or("");
            let price = item.get("p").and_then(|p| p.as_f64()).unwrap_or(0.0);
            let size = item.get("s").and_then(|s| s.as_u64()).unwrap_or(0) as u32;
            let timestamp = item
                .get("t")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let _ = ws_manager.tx.send(WsUpdate::Trade(TradeUpdate {
                symbol: symbol.to_string(),
                price,
                size,
                timestamp,
            }));
        }
        "q" | "qte" => {
            let symbol = item.get("S").and_then(|s| s.as_str()).unwrap_or("");
            let bid = item.get("bp").and_then(|p| p.as_f64()).unwrap_or(0.0);
            let ask = item.get("ap").and_then(|p| p.as_f64()).unwrap_or(0.0);
            let size = item.get("as").and_then(|s| s.as_u64()).unwrap_or(0) as u32;
            let timestamp = item
                .get("t")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();

            let _ = ws_manager.tx.send(WsUpdate::Quote(QuoteUpdate {
                symbol: symbol.to_string(),
                bid,
                ask,
                size,
                timestamp,
            }));
        }
        _ => {}
    }
}

async fn run_fallback_polling(ws_manager: Arc<WsManager>) {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .build()
        .unwrap_or_default();
    let mut interval = time::interval(Duration::from_secs(2));

    loop {
        interval.tick().await;
        let symbols = ws_manager.get_active_symbols();
        if symbols.is_empty() {
            continue;
        }

        // Yahoo Finance Quote API
        let symbols_str = symbols.join(",");
        let url = format!(
            "https://query1.finance.yahoo.com/v7/finance/quote?symbols={}",
            symbols_str
        );

        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<Value>().await {
                    if let Some(result) = data
                        .get("quoteResponse")
                        .and_then(|r| r.get("result"))
                        .and_then(|res| res.as_array())
                    {
                        for quote in result {
                            let symbol = quote.get("symbol").and_then(|s| s.as_str()).unwrap_or("");
                            let price = quote
                                .get("regularMarketPrice")
                                .and_then(|p| p.as_f64())
                                .unwrap_or(0.0);
                            let bid = quote.get("bid").and_then(|p| p.as_f64()).unwrap_or(price);
                            let ask = quote.get("ask").and_then(|p| p.as_f64()).unwrap_or(price);
                            let size = quote
                                .get("regularMarketVolume")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;

                            let _ = ws_manager.tx.send(WsUpdate::Quote(QuoteUpdate {
                                symbol: symbol.to_string(),
                                bid,
                                ask,
                                size,
                                timestamp: Utc::now().to_rfc3339(),
                            }));

                            let _ = ws_manager.tx.send(WsUpdate::Trade(TradeUpdate {
                                symbol: symbol.to_string(),
                                price,
                                size,
                                timestamp: Utc::now().to_rfc3339(),
                            }));
                        }
                    }
                }
            }
            Err(e) => tracing::error!("Fallback polling error: {}", e),
        }
    }
}
