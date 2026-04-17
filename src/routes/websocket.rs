use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::api::alpaca::AlpacaClient;
use crate::api::ws_manager::WsManager;
use crate::auth;
use crate::models::websocket::{WsAction, WsUpdate};
use crate::strategies::StrategyManager;

#[derive(Deserialize)]
pub struct WsParams {
    token: String,
}

use crate::error::AppError;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match auth::verify_token(&params.token) {
        Some(username) => ws.on_upgrade(move |socket| handle_socket(socket, state, username)),
        None => AppError::Unauthorized("Invalid or expired token".to_string()).into_response(),
    }
}

#[derive(Clone)]
pub struct AppState {
    pub alpaca: Option<AlpacaClient>,
    pub ws_manager: Arc<WsManager>,
    pub strategy_manager: Arc<StrategyManager>,
}

async fn handle_socket(socket: WebSocket, state: AppState, _username: String) {
    let (mut sender, mut receiver) = socket.split::<Message>();

    let subscribed_symbols = Arc::new(RwLock::new(HashSet::new()));
    let mut last_request_time = Instant::now();
    let mut request_count = 0;

    // Channel for direct messages to this client
    let (direct_tx, mut direct_rx) = mpsc::channel::<WsUpdate>(10);

    // Subscribe to the global broadcast channel
    let mut rx = state.ws_manager.tx.subscribe();

    // Task for sending updates to this client
    let subscribed_symbols_send = subscribed_symbols.clone();
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle broadcasted price updates
                Ok(update) = rx.recv() => {
                    let should_send = {
                        let subs = subscribed_symbols_send.read().unwrap();
                        match &update {
                            WsUpdate::Trade(t) => subs.contains(&t.symbol),
                            WsUpdate::Quote(q) => subs.contains(&q.symbol),
                            _ => false, // Don't broadcast errors/status updates globally
                        }
                    };

                    if should_send {
                        if let Ok(msg) = serde_json::to_string(&update) {
                            if sender.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                // Handle direct messages (errors, subscription status)
                Some(update) = direct_rx.recv() => {
                    if let Ok(msg) = serde_json::to_string(&update) {
                        if sender.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Task for receiving messages from this client
    let ws_manager = state.ws_manager.clone();
    let subscribed_symbols_recv = subscribed_symbols.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Rate limiting
                    let now = Instant::now();
                    if now.duration_since(last_request_time) < Duration::from_secs(1) {
                        request_count += 1;
                    } else {
                        last_request_time = now;
                        request_count = 1;
                    }

                    if request_count > 5 {
                        let _ = direct_tx
                            .send(WsUpdate::Error {
                                message: "Rate limit exceeded (5 requests per second)".to_string(),
                            })
                            .await;
                        continue;
                    }

                    if let Ok(action) = serde_json::from_str::<WsAction>(&text) {
                        match action {
                            WsAction::Subscribe { symbols } => {
                                let mut newly_added = Vec::new();
                                {
                                    let mut subs = subscribed_symbols_recv.write().unwrap();
                                    for sym in symbols {
                                        if subs.len() >= 100 {
                                            break;
                                        }
                                        if subs.insert(sym.clone()) {
                                            newly_added.push(sym);
                                        }
                                    }
                                }
                                ws_manager.add_symbols(&newly_added);
                                let current_subs = subscribed_symbols_recv
                                    .read()
                                    .unwrap()
                                    .iter()
                                    .cloned()
                                    .collect();
                                let _ = direct_tx
                                    .send(WsUpdate::SubscriptionStatus {
                                        subscribed: current_subs,
                                    })
                                    .await;
                            }
                            WsAction::Unsubscribe { symbols } => {
                                {
                                    let mut subs = subscribed_symbols_recv.write().unwrap();
                                    for sym in symbols {
                                        subs.remove(&sym);
                                    }
                                }
                                let current_subs = subscribed_symbols_recv
                                    .read()
                                    .unwrap()
                                    .iter()
                                    .cloned()
                                    .collect();
                                let _ = direct_tx
                                    .send(WsUpdate::SubscriptionStatus {
                                        subscribed: current_subs,
                                    })
                                    .await;
                            }
                        }
                    }
                }
                Message::Ping(_) => {
                    // Axum handles PONG automatically for Message::Ping
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
