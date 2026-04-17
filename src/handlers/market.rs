use axum::{
    extract::{Path, Query, State},
};
use crate::models::{DashboardResponse, DashboardQuery, DataProvider, normalize_symbol, ProviderQuery, CandleQuery};
use crate::error::{AppResult, ApiResponse};
use crate::{AppState, resolve_alpaca_credential, top_option_contracts};
use crate::services::providers::{fetch_quote, fetch_candles, fetch_options};
use crate::services::db::Database;
use serde_json::json;
use tracing::warn;

pub async fn dashboard(
    State(state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> AppResult<ApiResponse<DashboardResponse>> {
    let symbol = normalize_symbol(query.symbol.as_deref().unwrap_or("AAPL"));
    let provider = query.provider.unwrap_or(DataProvider::Yahoo);
    let credential = match provider {
        DataProvider::Yahoo => None,
        DataProvider::Alpaca => resolve_alpaca_credential(&state, None, false).await?,
    };

    let quote = fetch_quote(&state.http, provider, &symbol, credential.as_ref()).await?;
    let candles = fetch_candles(
        &state.http,
        provider,
        &symbol,
        "1d",
        "1m",
        credential.as_ref(),
    )
    .await?;
    let options = match fetch_options(&state.http, provider, &symbol, credential.as_ref()).await {
        Ok(options) => options,
        Err(err) => {
            warn!("options fetch failed for {symbol}: {err}");
            crate::services::providers::FetchedOptions {
                contracts: Vec::new(),
                raw_json: json!({}),
            }
        }
    };

    let (strategies, recent_trades, credentials, tracked_symbols, watchlists) = {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.store_market_snapshot(&quote.quote, &quote.raw_json)?;
        db.store_option_snapshots(&options.contracts, &options.raw_json)?;

        let mut symbols = db.tracked_symbols_union(&state.config.default_watchlist)?;
        symbols.extend(db.watchlist_symbols_union()?);

        let mut set = std::collections::BTreeSet::new();
        for s in symbols {
            set.insert(s);
        }
        let tracked_symbols = set.into_iter().collect();

        (
            db.list_strategies()?,
            db.list_trades(None, 24)?,
            db.list_credentials()?,
            tracked_symbols,
            db.list_watchlists()?,
        )
    };

    Ok(ApiResponse {
        success: true,
        data: Some(DashboardResponse {
            symbol,
            provider,
            collector_interval_seconds: state.config.polling_seconds,
            quote: quote.quote,
            candles: candles.candles,
            options: top_option_contracts(options.contracts),
            strategies,
            recent_trades,
            credentials,
            tracked_symbols,
            watchlists,
        }),
        error: None,
    })
}

pub async fn market_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(query): Query<ProviderQuery>,
) -> AppResult<ApiResponse<crate::models::Quote>> {
    let provider = query.provider.unwrap_or(DataProvider::Yahoo);
    let symbol = normalize_symbol(&symbol);
    let credential = match provider {
        DataProvider::Yahoo => None,
        DataProvider::Alpaca => resolve_alpaca_credential(&state, None, false).await?,
    };
    let fetched = fetch_quote(&state.http, provider, &symbol, credential.as_ref()).await?;
    {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.store_market_snapshot(&fetched.quote, &fetched.raw_json)?;
    }
    Ok(ApiResponse {
        success: true,
        data: Some(fetched.quote),
        error: None,
    })
}

pub async fn market_candles(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(query): Query<CandleQuery>,
) -> AppResult<ApiResponse<Vec<crate::models::Candle>>> {
    let provider = query.provider.unwrap_or(DataProvider::Yahoo);
    let symbol = normalize_symbol(&symbol);
    let credential = match provider {
        DataProvider::Yahoo => None,
        DataProvider::Alpaca => resolve_alpaca_credential(&state, None, false).await?,
    };
    let fetched = fetch_candles(
        &state.http,
        provider,
        &symbol,
        query.range.as_deref().unwrap_or("1d"),
        query.interval.as_deref().unwrap_or("1m"),
        credential.as_ref(),
    )
    .await?;
    Ok(ApiResponse {
        success: true,
        data: Some(fetched.candles),
        error: None,
    })
}

pub async fn options_chain(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    Query(query): Query<ProviderQuery>,
) -> AppResult<ApiResponse<Vec<crate::models::OptionContractSnapshot>>> {
    let symbol = normalize_symbol(&symbol);
    let provider = query.provider.unwrap_or(DataProvider::Yahoo);
    let credential = match provider {
        DataProvider::Yahoo => None,
        DataProvider::Alpaca => resolve_alpaca_credential(&state, None, false).await?,
    };
    let fetched = fetch_options(&state.http, provider, &symbol, credential.as_ref()).await?;
    {
        let db = state.db.lock().await;
        let db: &Database = &*db;
        db.store_option_snapshots(&fetched.contracts, &fetched.raw_json)?;
    }
    Ok(ApiResponse {
        success: true,
        data: Some(top_option_contracts(fetched.contracts)),
        error: None,
    })
}
