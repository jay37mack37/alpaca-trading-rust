use crate::models::{normalize_symbol, AppConfig};

impl AppConfig {
    pub fn from_env() -> Self {
        use std::env;
        use std::path::PathBuf;

        let default_watchlist = env::var("AUTO_STONKS_WATCHLIST")
            .unwrap_or_else(|_| "AAPL,SPY,QQQ,NVDA,MSFT".to_string())
            .split(',')
            .map(normalize_symbol)
            .filter(|symbol| !symbol.is_empty())
            .collect();

        let allowed_origins = env::var("AUTO_STONKS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| {
                "http://127.0.0.1:5173,http://127.0.0.1:5174,http://localhost:5173,http://localhost:5174"
                    .to_string()
            })
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();

        Self {
            host: env::var("AUTO_STONKS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("AUTO_STONKS_PORT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(8080),
            database_path: env::var("AUTO_STONKS_DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("data/autostonks.db")),
            default_watchlist,
            polling_seconds: env::var("AUTO_STONKS_POLL_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(120),
            allowed_origins,
        }
    }
}
