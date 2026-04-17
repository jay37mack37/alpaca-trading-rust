use axum::Json;
use serde_json::{json, Value};

pub async fn root() -> Json<Value> {
    Json(json!({
        "name": "Alpaca Trading Web API",
        "version": "0.1.0",
        "status": "running"
    }))
}

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "healthy"
    }))
}
