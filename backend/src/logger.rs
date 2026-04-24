use serde::{Deserialize, Serialize};
use chrono::Local;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemSource {
    Parity,
    Vwap,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SystemEventType {
    Scan,
    Signal,
    Haggle,
    Protection,
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub timestamp: String,
    pub source: SystemSource,
    pub strategy_id: Option<String>,
    pub symbol: String,
    pub event_type: SystemEventType,
    pub math_context: String,
    pub ai_confidence: f64,
    pub narrative: String,
}

impl SystemEvent {
    pub fn now(
        source: SystemSource,
        strategy_id: Option<String>,
        symbol: String,
        event_type: SystemEventType,
        math_context: String,
        ai_confidence: f64,
        narrative: String,
    ) -> Self {
        Self {
            timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
            source,
            strategy_id,
            symbol,
            event_type,
            math_context,
            ai_confidence,
            narrative,
        }
    }
}
