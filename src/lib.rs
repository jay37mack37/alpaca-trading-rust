pub mod api;
pub mod auth;
pub mod models;
pub mod routes;

use std::sync::Arc;
use crate::api::alpaca::AlpacaApi;
use crate::api::ws_manager::WsManager;

#[derive(Clone)]
pub struct AppState {
    pub alpaca: Option<Arc<dyn AlpacaApi>>,
    pub ws_manager: Arc<WsManager>,
}
