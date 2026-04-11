use axum::{http::HeaderMap, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::routes::auth::get_username_from_headers;

/// Mutex to serialize Python subprocess calls (prevent concurrent SQLite writes)
static ANALYTICS_LOCK: once_cell::sync::Lazy<Mutex<()>> =
    once_cell::sync::Lazy::new(|| Mutex::new(()));

#[derive(Deserialize)]
pub struct WatchlistUpdate {
    pub add: Option<Vec<String>>,
    pub remove: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct FetchRequest {
    pub symbols: Option<Vec<String>>,
    pub timeframes: Option<Vec<String>>,
    pub source: Option<String>,
    pub full: Option<bool>,
}

#[derive(Deserialize)]
pub struct AnalyzeRequest {
    pub symbols: Option<Vec<String>>,
    pub patterns: Option<Vec<String>>,
    pub min_confidence: Option<f64>,
    pub store: Option<bool>,
    #[serde(rename = "update")]
    pub update_data: Option<bool>,
    pub source: Option<String>,
}

#[derive(Serialize)]
struct PatternInfo {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    timeframes: &'static [&'static str],
}

const PATTERNS: &[PatternInfo] = &[
    PatternInfo {
        id: "vwap_deviation",
        name: "VWAP Deviation",
        description: "Price deviates significantly from VWAP, expecting mean reversion",
        timeframes: &["1m"],
    },
    PatternInfo {
        id: "opening_range_15m",
        name: "Opening Range Breakout (15m)",
        description: "Breakout above/below the first 15 minutes' trading range",
        timeframes: &["1m"],
    },
    PatternInfo {
        id: "opening_range_30m",
        name: "Opening Range Breakout (30m)",
        description: "Breakout above/below the first 30 minutes' trading range",
        timeframes: &["1m"],
    },
    PatternInfo {
        id: "intraday_mean_reversion",
        name: "Intraday Mean Reversion",
        description: "Bollinger Band touches relative to VWAP indicating mean reversion",
        timeframes: &["1m"],
    },
    PatternInfo {
        id: "gap_analysis",
        name: "Gap Analysis",
        description: "Overnight gaps and fill probability estimation",
        timeframes: &["1d"],
    },
    PatternInfo {
        id: "unusual_volume_1m",
        name: "Unusual Volume (1m)",
        description: "Volume spikes on 1-minute bars (>2 standard deviations)",
        timeframes: &["1m"],
    },
    PatternInfo {
        id: "unusual_volume_1d",
        name: "Unusual Volume (1d)",
        description: "Volume spikes on daily bars (>2 standard deviations)",
        timeframes: &["1d"],
    },
    PatternInfo {
        id: "momentum_1d",
        name: "Momentum (1d)",
        description: "Rate of change momentum signals on daily bars (5, 10, 20 day periods)",
        timeframes: &["1d"],
    },
];

/// Get the project root directory
fn project_dir() -> AppResult<PathBuf> {
    std::env::current_dir().map_err(|e| AppError::Internal(format!("Cannot determine working directory: {e}")))
}

/// Run a Python analytics script and return its JSON output
async fn run_analytics_script(script: &str, args: &[String], timeout_secs: u64) -> AppResult<Value> {
    let _lock = ANALYTICS_LOCK.lock().await;

    let project_dir = project_dir()?;
    let script_path = project_dir.join("analytics").join(script);

    let mut cmd = Command::new("python3");
    cmd.arg(&script_path)
        .args(args)
        .arg("--format")
        .arg("json")
        .current_dir(project_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| AppError::Internal("Analytics script timed out".to_string()))?
        .map_err(|e| AppError::Internal(format!("Failed to execute Python: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(stderr.trim().to_string()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(json!({}));
    }

    serde_json::from_str(&stdout).map_err(|e| AppError::Internal(format!("JSON parse error: {e}")))
}

// --- Handler functions ---

pub async fn get_watchlist(headers: HeaderMap) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let args: Vec<String> = vec!["--watchlist-only".to_string()];
    let result = run_analytics_script("analyze.py", &args, 10).await?;
    Ok(Json(result))
}

pub async fn update_watchlist(headers: HeaderMap, Json(body): Json<WatchlistUpdate>) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let mut args: Vec<String> = Vec::new();

    if let Some(add) = &body.add {
        args.push("--add".to_string());
        args.extend(add.iter().cloned());
    }
    if let Some(remove) = &body.remove {
        args.push("--remove".to_string());
        args.extend(remove.iter().cloned());
    }

    args.push("--watchlist-only".to_string());

    let result = run_analytics_script("analyze.py", &args, 10).await?;
    Ok(Json(result))
}

pub async fn fetch_data(headers: HeaderMap, Json(body): Json<FetchRequest>) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let mut args: Vec<String> = Vec::new();

    if let Some(symbols) = &body.symbols {
        args.push("--symbols".to_string());
        args.extend(symbols.iter().cloned());
    }

    if let Some(timeframes) = &body.timeframes {
        args.push("--timeframes".to_string());
        args.extend(timeframes.iter().cloned());
    }

    if let Some(source) = &body.source {
        args.push("--source".to_string());
        args.push(source.clone());
    }

    if body.full.unwrap_or(false) {
        args.push("--full".to_string());
    }

    let result = run_analytics_script("fetch_bars.py", &args, 120).await?;
    Ok(Json(result))
}

pub async fn get_summary(headers: HeaderMap) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let args: Vec<String> = vec!["--summary".to_string()];
    let result = run_analytics_script("analyze.py", &args, 15).await?;
    Ok(Json(result))
}

pub async fn run_analysis(headers: HeaderMap, Json(body): Json<AnalyzeRequest>) -> AppResult<Json<Value>> {
    let _username = get_username_from_headers(&headers)?;

    let mut args: Vec<String> = Vec::new();

    if let Some(symbols) = &body.symbols {
        args.push("--symbols".to_string());
        args.extend(symbols.iter().cloned());
    }

    if let Some(patterns) = &body.patterns {
        args.push("--patterns".to_string());
        args.extend(patterns.iter().cloned());
    }

    if let Some(conf) = body.min_confidence {
        args.push("--min-confidence".to_string());
        args.push(conf.to_string());
    }

    if body.store.unwrap_or(false) {
        args.push("--store".to_string());
    }

    if body.update_data.unwrap_or(false) {
        args.push("--update".to_string());
    }

    if let Some(source) = &body.source {
        args.push("--source".to_string());
        args.push(source.clone());
    }

    let result = run_analytics_script("analyze.py", &args, 60).await?;
    Ok(Json(result))
}

pub async fn get_patterns() -> Json<Value> {
    Json(json!({
        "patterns": PATTERNS.iter().map(|p| json!({
            "id": p.id,
            "name": p.name,
            "description": p.description,
            "timeframes": p.timeframes,
        })).collect::<Vec<_>>()
    }))
}