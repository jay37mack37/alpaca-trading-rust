use crate::error::AppResult;
use crate::models::{CollectResponse, DataProvider};
use crate::services::providers::fetch_quote;
use crate::AppState;
use chrono::Utc;
use futures_util::future::join_all;
use tracing::{info, warn};

pub async fn collector_loop(state: AppState) {
    let interval = std::time::Duration::from_secs(state.config.polling_seconds);
    loop {
        tokio::time::sleep(interval).await;
        match collect_once(&state).await {
            Ok(summary) => info!(
                "collector run complete: {} market data symbols refreshed",
                summary.symbols_collected
            ),
            Err(err) => warn!("collector run failed: {err}"),
        }
    }
}

pub async fn collect_once(state: &AppState) -> AppResult<CollectResponse> {
    let tracked_symbols = {
        let db = state.db.lock().await;
        let mut symbols = db.tracked_symbols_union(&state.config.default_watchlist)?;
        symbols.extend(db.watchlist_symbols_union()?);

        let mut set = std::collections::BTreeSet::new();
        for s in symbols {
            set.insert(s);
        }
        set.into_iter().collect::<Vec<_>>()
    };

    let fetch_futures = tracked_symbols.iter().map(|symbol| async move {
        let fetched = fetch_quote(&state.http, DataProvider::Yahoo, symbol, None).await;
        (symbol, fetched)
    });

    let results = join_all(fetch_futures).await;

    for (symbol, fetched) in results {
        match fetched {
            Ok(snapshot) => {
                let db = state.db.lock().await;
                db.store_market_snapshot(&snapshot.quote, &snapshot.raw_json)?;
                db.mark_symbol_price(symbol, snapshot.quote.price)?;
            }
            Err(err) => warn!("snapshot failed for {symbol}: {err}"),
        }
    }

    Ok(CollectResponse {
        symbols_collected: tracked_symbols.len(),
        strategies_evaluated: 0,
        trades_executed: 0,
        collected_at: Utc::now().to_rfc3339(),
    })
}
