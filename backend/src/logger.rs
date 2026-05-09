use serde::{Deserialize, Serialize};
use chrono::Local;

pub type SystemSource = String;

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
    pub execution_profile: Option<crate::models::ExecutionProfile>,
}

impl SystemEvent {
    pub fn format_kronos(score: f64) -> String {
        let label = if score > 0.6 {
            "BULLISH"
        } else if score < 0.4 {
            "BEARISH"
        } else {
            "NEUTRAL"
        };
        format!("{:.2} ({})", score, label)
    }

    pub fn now(
        source: String,
        strategy_id: Option<String>,
        symbol: String,
        event_type: SystemEventType,
        math_context: String,
        ai_confidence: f64,
        narrative: String,
        execution_profile: Option<crate::models::ExecutionProfile>,
    ) -> Self {
        let sentiment_narrative = if ai_confidence != 0.0 {
            let label = if ai_confidence > 0.6 {
                "BULLISH"
            } else if ai_confidence < 0.4 {
                "BEARISH"
            } else {
                "NEUTRAL"
            };
            format!("{} | Kronos: {} {}", narrative, ai_confidence, label)
        } else {
            narrative
        };

        Self {
            timestamp: Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string(),
            source,
            strategy_id,
            symbol,
            event_type,
            math_context,
            ai_confidence,
            narrative: sentiment_narrative,
            execution_profile,
        }
    }
}
