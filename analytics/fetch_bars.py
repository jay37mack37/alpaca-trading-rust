"""
Fetch OHLCV bar data from Alpaca or Yahoo Finance and store in local SQLite.

Supports incremental updates — only fetches bars newer than the last
stored timestamp for each symbol/timeframe combination.

Data source priority:
  1. Alpaca (if API key has data access)
  2. Yahoo Finance (yfinance) — always available, no key needed

Usage:
    python fetch_bars.py                # Fetch all watchlist symbols, all timeframes
    python fetch_bars.py --symbols AAPL TSLA  # Specific symbols
    python fetch_bars.py --timeframes 1m 5m     # Specific timeframes
    python fetch_bars.py --full                # Re-fetch everything (no incremental)
    python fetch_bars.py --source yfinance     # Force yfinance instead of Alpaca
"""

import argparse
import json
import os
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

# Load .env from project root
from dotenv import load_dotenv
load_dotenv(Path(__file__).resolve().parent.parent / ".env")

import pandas as pd

from db import init_db, get_bar_model, Watchlist, db, BAR_MODELS

ALPACA_API_KEY = os.getenv("ALPACA_API_KEY")
ALPACA_API_SECRET = os.getenv("ALPACA_API_SECRET")

# Alpaca free tier limits per timeframe
MAX_LOOKBACK = {
    "1m": timedelta(days=7),
    "5m": timedelta(days=60),
    "1h": timedelta(days=730),
    "1d": timedelta(days=3650),
}

# yfinance interval mapping
YF_INTERVAL = {
    "1m": "1m",
    "5m": "5m",
    "1h": "1h",
    "1d": "1d",
}

# yfinance max lookback per interval
YF_MAX_LOOKBACK = {
    "1m": timedelta(days=7),     # yfinance only returns 7 days of 1m data
    "5m": timedelta(days=60),    # ~60 days
    "1h": timedelta(days=730),   # ~2 years
    "1d": timedelta(days=3650),  # max available
}


def get_last_timestamp(symbol: str, timeframe: str) -> datetime | None:
    """Get the most recent bar timestamp we have for a symbol/timeframe."""
    model = get_bar_model(timeframe)
    last = (model
            .select(model.timestamp)
            .where(model.symbol == symbol)
            .order_by(model.timestamp.desc())
            .first())
    return last.timestamp if last else None


# ---------------------------------------------------------------------------
# Alpaca data source
# ---------------------------------------------------------------------------

def fetch_bars_alpaca(symbol: str, timeframe: str, start: datetime | None = None,
                      end: datetime | None = None) -> pd.DataFrame:
    """Fetch bars from Alpaca Market Data API."""
    from alpaca.data.historical.stock import StockHistoricalDataClient
    from alpaca.data.requests import StockBarsRequest
    from alpaca.data.timeframe import TimeFrame, TimeFrameUnit

    ALPACA_TIMEFRAME = {
        "1m": TimeFrame(1, TimeFrameUnit.Minute),
        "5m": TimeFrame(5, TimeFrameUnit.Minute),
        "1h": TimeFrame(1, TimeFrameUnit.Hour),
        "1d": TimeFrame(1, TimeFrameUnit.Day),
    }

    client = StockHistoricalDataClient(ALPACA_API_KEY, ALPACA_API_SECRET)
    tf = ALPACA_TIMEFRAME[timeframe]

    if start is None:
        start = datetime.now(timezone.utc) - MAX_LOOKBACK[timeframe]
    if end is None:
        end = datetime.now(timezone.utc)

    request = StockBarsRequest(
        symbol_or_symbols=symbol,
        timeframe=tf,
        start=start,
        end=end,
        adjustment="raw",
    )

    bars = client.get_stock_bars(request)
    df = bars.df

    if df.empty:
        return df

    df = df.reset_index()
    if "timestamp" not in df.columns and "level_1" in df.columns:
        df = df.rename(columns={"level_1": "timestamp"})

    keep_cols = ["timestamp", "open", "high", "low", "close", "volume"]
    optional_cols = ["trade_count", "vwap"]
    for col in optional_cols:
        if col in df.columns:
            keep_cols.append(col)
    df = df[[c for c in keep_cols if c in df.columns]]

    # Ensure timestamp is timezone-aware UTC
    if "timestamp" in df.columns:
        df["timestamp"] = pd.to_datetime(df["timestamp"], utc=True)

    return df


# ---------------------------------------------------------------------------
# Yahoo Finance data source
# ---------------------------------------------------------------------------

def fetch_bars_yfinance(symbol: str, timeframe: str, start: datetime | None = None,
                        end: datetime | None = None) -> pd.DataFrame:
    """Fetch bars from Yahoo Finance (yfinance)."""
    import yfinance as yf

    if start is None:
        start = datetime.now(timezone.utc) - YF_MAX_LOOKBACK[timeframe]
    if end is None:
        end = datetime.now(timezone.utc)

    interval = YF_INTERVAL[timeframe]
    ticker = yf.Ticker(symbol)

    df = ticker.history(
        start=start.strftime("%Y-%m-%d"),
        end=end.strftime("%Y-%m-%d"),
        interval=interval,
        auto_adjust=True,
    )

    if df.empty:
        return pd.DataFrame()

    # Reset index to get the datetime as a column
    df = df.reset_index()

    # The datetime column can be named "Date" (daily) or "Datetime" (intraday)
    ts_col = None
    for col in ["Datetime", "Date", "timestamp"]:
        if col in df.columns:
            ts_col = col
            break
    if ts_col is None:
        # Fallback: use the first column
        ts_col = df.columns[0]

    df = df.rename(columns={ts_col: "timestamp"})

    # Normalize column names to lowercase
    df.columns = [c.lower() for c in df.columns]

    # Keep only what we need
    keep_cols = ["timestamp", "open", "high", "low", "close", "volume"]
    df = df[[c for c in keep_cols if c in df.columns]]

    # Ensure timestamp is timezone-aware UTC
    df["timestamp"] = pd.to_datetime(df["timestamp"], utc=True)

    # Add trade_count and vwap as None (yfinance doesn't provide these)
    df["trade_count"] = None
    df["vwap"] = None

    # Drop any rows with NaN in essential columns
    df = df.dropna(subset=["open", "high", "low", "close"])

    return df


# ---------------------------------------------------------------------------
# Unified fetcher
# ---------------------------------------------------------------------------

def fetch_bars(symbol: str, timeframe: str, start: datetime | None = None,
               end: datetime | None = None, source: str = "auto") -> pd.DataFrame:
    """
    Fetch bars from the best available source.

    Args:
        source: "alpaca", "yfinance", or "auto" (try alpaca, fallback to yfinance)
    """
    if source == "alpaca":
        return fetch_bars_alpaca(symbol, timeframe, start, end)
    elif source == "yfinance":
        return fetch_bars_yfinance(symbol, timeframe, start, end)
    elif source == "auto":
        # Try Alpaca first, fall back to yfinance
        try:
            df = fetch_bars_alpaca(symbol, timeframe, start, end)
            if not df.empty:
                return df
        except Exception as e:
            print(f"    Alpaca failed ({e}), falling back to yfinance")
        return fetch_bars_yfinance(symbol, timeframe, start, end)
    else:
        raise ValueError(f"Unknown source: {source}")


def store_bars(symbol: str, timeframe: str, df: pd.DataFrame) -> int:
    """Insert bars into SQLite. Returns number of new rows inserted."""
    if df.empty:
        return 0

    model = get_bar_model(timeframe)
    count = 0

    with db.atomic():
        for _, row in df.iterrows():
            ts = row["timestamp"]

            # Convert pandas Timestamp to Python datetime for SQLite compatibility
            if isinstance(ts, pd.Timestamp):
                ts = ts.to_pydatetime()
            elif isinstance(ts, str):
                ts = pd.to_datetime(ts, utc=True).to_pydatetime()
            elif hasattr(ts, "tzinfo") and ts.tzinfo is None:
                ts = ts.replace(tzinfo=timezone.utc)

            try:
                model.insert(
                    symbol=symbol,
                    timestamp=ts,
                    open=float(row["open"]),
                    high=float(row["high"]),
                    low=float(row["low"]),
                    close=float(row["close"]),
                    volume=int(row["volume"]) if pd.notna(row.get("volume")) else 0,
                    trade_count=int(row.get("trade_count", 0)) if pd.notna(row.get("trade_count")) else None,
                    vwap=float(row["vwap"]) if pd.notna(row.get("vwap")) else None,
                ).on_conflict_ignore().execute()
                count += 1
            except Exception:
                pass

    return count


def fetch_and_store(symbol: str, timeframe: str, full: bool = False,
                    source: str = "auto") -> int:
    """
    Fetch bars for a symbol/timeframe and store them.
    If full=False, only fetch from last stored timestamp (incremental).
    """
    start = None if full else get_last_timestamp(symbol, timeframe)

    if start:
        # Start from 1 bar before last stored to handle potential gaps
        start = start - timedelta(minutes=1)

    df = fetch_bars(symbol, timeframe, start=start, source=source)
    count = store_bars(symbol, timeframe, df)
    return count


def load_bars(symbol: str, timeframe: str, start: datetime | None = None,
              end: datetime | None = None) -> pd.DataFrame:
    """
    Load bars from local SQLite for a symbol/timeframe.
    Returns a pandas DataFrame sorted by timestamp.
    """
    model = get_bar_model(timeframe)
    query = model.select().where(model.symbol == symbol)

    if start:
        query = query.where(model.timestamp >= start)
    if end:
        query = query.where(model.timestamp <= end)

    query = query.order_by(model.timestamp)

    rows = list(query.dicts())
    if not rows:
        return pd.DataFrame()

    df = pd.DataFrame(rows)
    df["timestamp"] = pd.to_datetime(df["timestamp"], utc=True)
    df = df.set_index("timestamp")

    # Ensure numeric types
    for col in ["open", "high", "low", "close", "vwap"]:
        if col in df.columns:
            df[col] = pd.to_numeric(df[col], errors="coerce")
    if "volume" in df.columns:
        df["volume"] = pd.to_numeric(df["volume"], errors="coerce")

    return df


def get_watchlist_symbols() -> list[str]:
    """Get all symbols from the watchlist table."""
    return [w.symbol for w in Watchlist.select()]


def add_to_watchlist(symbols: list[str]):
    """Add symbols to the watchlist if not already there."""
    for sym in symbols:
        sym = sym.upper().strip()
        Watchlist.get_or_create(symbol=sym)


def main():
    parser = argparse.ArgumentParser(description="Fetch bar data from Alpaca or Yahoo Finance")
    parser.add_argument("--symbols", nargs="+", help="Symbols to fetch (default: watchlist)")
    parser.add_argument("--timeframes", nargs="+", choices=list(BAR_MODELS.keys()),
                        default=list(BAR_MODELS.keys()), help="Timeframes to fetch")
    parser.add_argument("--full", action="store_true", help="Re-fetch all data (no incremental)")
    parser.add_argument("--source", choices=["alpaca", "yfinance", "auto"], default="auto",
                        help="Data source (default: auto = try alpaca, fallback yfinance)")
    parser.add_argument("--add", nargs="+", help="Add symbols to watchlist before fetching")
    parser.add_argument("--format", choices=["text", "json"], default="text", dest="output_format",
                        help="Output format: text (human-readable) or json (structured)")
    args = parser.parse_args()

    json_mode = args.output_format == "json"
    init_db()

    if args.add:
        add_to_watchlist(args.add)

    symbols = args.symbols or get_watchlist_symbols()
    if not symbols:
        if json_mode:
            print(json.dumps({"error": "No symbols in watchlist", "results": []}))
        else:
            print("No symbols in watchlist. Use --add SYMBOL to add some, or --symbols AAPL TSLA")
        sys.exit(1)

    if not json_mode:
        print(f"Fetching data for {len(symbols)} symbol(s): {symbols}")
        print(f"Timeframes: {args.timeframes}")
        print(f"Source: {args.source}")
        print(f"Mode: {'full' if args.full else 'incremental'}")
        print()

    results = []
    for symbol in symbols:
        for tf in args.timeframes:
            try:
                count = fetch_and_store(symbol, tf, full=args.full, source=args.source)
                results.append({"symbol": symbol, "timeframe": tf, "bars_fetched": count, "status": "ok"})
                if not json_mode:
                    print(f"  {symbol} {tf}: {count} bars stored")
            except Exception as e:
                results.append({"symbol": symbol, "timeframe": tf, "bars_fetched": 0, "status": "error", "error": str(e)})
                if not json_mode:
                    print(f"  {symbol} {tf}: ERROR - {e}")

    if json_mode:
        print(json.dumps({"results": results}, indent=2))
    else:
        print("\nDone.")


if __name__ == "__main__":
    main()