use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

pub mod api;
pub mod auth;
pub mod models;
pub mod routes;

pub use routes::websocket::AppState;

pub fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/login", post(routes::auth::login))
        .route("/api/verify", get(routes::auth::verify_token))
        .route("/api/logout", post(routes::auth::logout))
        .route("/api/config/status", get(routes::auth::get_api_key_status))
        .route("/api/config/api-keys", post(routes::auth::save_api_keys))
        .route("/api/config/password", post(routes::auth::change_password))
        .route("/api/account", get(routes::trading::get_account))
        .route("/api/positions", get(routes::trading::get_positions))
        .route("/api/orders", get(routes::trading::get_orders))
        .route("/api/orders", post(routes::trading::create_order))
        .route("/api/price/{symbol}", get(routes::trading::get_price))
        .route(
            "/api/option-strikes/{symbol}",
            get(routes::trading::get_option_strikes),
        )
        .route(
            "/api/option-quote/{symbol}",
            get(routes::trading::get_option_price),
        )
        .route(
            "/api/option-chain/{symbol}",
            get(routes::trading::get_option_chain),
        )
        .route("/api/orders/{id}", get(routes::orders::get_order_by_id))
        .route("/api/orders/{id}", delete(routes::orders::cancel_order))
        .route(
            "/api/orders/cancel-all",
            post(routes::orders::cancel_all_orders),
        )
        .route("/api/ws/prices", get(routes::websocket::ws_handler))
        .with_state(state)
        .fallback_service(ServeDir::new("static"))
        .nest_service("/static", ServeDir::new("static"))
        .layer(cors)
}
