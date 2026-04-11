#!/usr/bin/env python3
"""
Main analysis runner.

Fetches data, runs pattern detection, and outputs signals.

Usage:
    # First time: add symbols to watchlist and fetch data
    python analyze.py --add SPY AAPL TSLA

    # Run analysis on all watchlist symbols
    python analyze.py

    # Run specific patterns
    python analyze.py --patterns vwap_deviation gap_analysis

    # Update data then analyze
    python analyze.py --update

    # Show only high-confidence signals
    python analyze.py --min-confidence 0.7

    # Export signals to JSON
    python analyze.py --export signals.json
"""

import argparse
import json
import os
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

# Ensure project root is on path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from db import init_db, get_bar_model, Watchlist, Signal as SignalModel, db
from fetch_bars import fetch_and_store, load_bars, get_watchlist_symbols, add_to_watchlist
from patterns import run_all_patterns, ALL_PATTERNS


def fetch_all_data(symbols: list[str], timeframes: list[str] = None, full: bool = False,
                   source: str = "auto"):
    """Fetch data for all symbols and timeframes."""
    if timeframes is None:
        timeframes = ["1m", "5m", "1h", "1d"]

    print(f"\n{'='*50}")
    print(f"FETCHING DATA")
    print(f"{'='*50}")

    for symbol in symbols:
        print(f"\n  {symbol}:")
        for tf in timeframes:
            try:
                count = fetch_and_store(symbol, tf, full=full, source=source)
                print(f"    {tf}: {count} new bars")
            except Exception as e:
                print(f"    {tf}: ERROR - {e}")


def analyze_symbol(symbol: str, patterns: list[str] = None, min_confidence: float = 0.0):
    """Run analysis on a single symbol. Returns list of Signal objects."""
    print(f"\n  Analyzing {symbol}...")

    # Load data from local DB
    bars = {}
    for tf in ["1m", "5m", "1h", "1d"]:
        model = get_bar_model(tf)
        count = model.select().where(model.symbol == symbol).count()
        if count > 0:
            bars[f"bars_{tf}"] = load_bars(symbol, tf)
            print(f"    {tf}: {count} bars loaded")
        else:
            bars[f"bars_{tf}"] = None
            print(f"    {tf}: no data — run --update first")

    # Run pattern detection
    signals = run_all_patterns(symbol, patterns=patterns, **bars)

    # Filter by confidence
    signals = [s for s in signals if s.confidence >= min_confidence]

    # Sort by timestamp, then confidence
    signals.sort(key=lambda s: (s.timestamp, -s.confidence))

    return signals


def store_signals(signals: list):
    """Store signals in the database."""
    with db.atomic():
        for sig in signals:
            SignalModel.create(
                symbol=sig.symbol,
                timestamp=sig.timestamp,
                pattern=sig.pattern,
                direction=sig.direction,
                confidence=sig.confidence,
                details=json.dumps(sig.details),
            )


def print_signals(signals: list, verbose: bool = False):
    """Print signals in a readable format."""
    if not signals:
        print("\n  No signals detected.")
        return

    print(f"\n{'='*70}")
    print(f"  SIGNALS ({len(signals)} found)")
    print(f"{'='*70}")

    # Group by symbol
    by_symbol = {}
    for sig in signals:
        by_symbol.setdefault(sig.symbol, []).append(sig)

    for symbol, sigs in sorted(by_symbol.items()):
        print(f"\n  {symbol}:")
        for sig in sigs:
            direction_icon = {"bullish": "▲", "bearish": "▼", "neutral": "—"}.get(sig.direction, "?")
            conf_bar = "█" * int(sig.confidence * 10) + "░" * (10 - int(sig.confidence * 10))

            line = (f"    {direction_icon} {sig.pattern:<30} "
                    f"{sig.direction:<8} "
                    f"conf:{sig.confidence:.1f} [{conf_bar}]")

            if verbose:
                details_str = json.dumps(sig.details, indent=6)
                line += f"\n      {sig.timestamp}\n{details_str}"
            else:
                # Show key details inline
                key_details = {}
                for k in ["vwap", "price", "gap_pct", "volume", "avg_volume",
                           "z_score", "roc_pct", "range_high", "range_low",
                           "breakout_price", "breakdown_price", "hist_fill_rate_up"]:
                    if k in sig.details:
                        key_details[k] = sig.details[k]
                if key_details:
                    line += f"  {json.dumps(key_details)}"

            print(line)


def export_signals(signals: list, filepath: str):
    """Export signals to a JSON file."""
    data = [s.to_dict() for s in signals]
    with open(filepath, "w") as f:
        json.dump(data, f, indent=2, default=str)
    print(f"\n  Exported {len(signals)} signals to {filepath}")


def show_summary(symbols: list[str]):
    """Show a summary of stored data per symbol."""
    print(f"\n{'='*50}")
    print(f"  DATA SUMMARY")
    print(f"{'='*50}")

    for symbol in symbols:
        print(f"\n  {symbol}:")
        for tf in ["1m", "5m", "1h", "1d"]:
            model = get_bar_model(tf)
            count = model.select().where(model.symbol == symbol).count()
            if count > 0:
                first = model.select(model.timestamp).where(
                    model.symbol == symbol
                ).order_by(model.timestamp.asc()).first()
                last = model.select(model.timestamp).where(
                    model.symbol == symbol
                ).order_by(model.timestamp.desc()).first()
                print(f"    {tf}: {count} bars  ({first.timestamp.strftime('%Y-%m-%d')} → {last.timestamp.strftime('%Y-%m-%d')})")
            else:
                print(f"    {tf}: no data")


def main():
    parser = argparse.ArgumentParser(description="Run pattern analysis on market data")
    parser.add_argument("--add", nargs="+", help="Add symbols to watchlist")
    parser.add_argument("--remove", nargs="+", help="Remove symbols from watchlist")
    parser.add_argument("--symbols", nargs="+", help="Analyze specific symbols (overrides watchlist)")
    parser.add_argument("--update", action="store_true", help="Fetch latest data before analyzing")
    parser.add_argument("--full", action="store_true", help="Full re-fetch (no incremental)")
    parser.add_argument("--patterns", nargs="+", choices=list(ALL_PATTERNS.keys()),
                        help="Run specific patterns only")
    parser.add_argument("--min-confidence", type=float, default=0.0,
                        help="Minimum confidence threshold (0.0-1.0)")
    parser.add_argument("--verbose", "-v", action="store_true", help="Show detailed signal info")
    parser.add_argument("--export", metavar="FILE", help="Export signals to JSON file")
    parser.add_argument("--summary", action="store_true", help="Show data summary and exit")
    parser.add_argument("--store", action="store_true", help="Store signals in the database")
    parser.add_argument("--source", choices=["alpaca", "yfinance", "auto"], default="yfinance",
                        help="Data source for fetching (default: yfinance)")
    args = parser.parse_args()

    init_db()

    # Manage watchlist
    if args.add:
        add_to_watchlist(args.add)
        print(f"Added {args.add} to watchlist")

    if args.remove:
        for sym in args.remove:
            sym = sym.upper().strip()
            deleted = Watchlist.delete().where(Watchlist.symbol == sym).execute()
            print(f"Removed {sym} ({deleted} entries)")

    # Get symbols to analyze
    symbols = args.symbols or get_watchlist_symbols()
    if not symbols:
        print("No symbols to analyze. Use --add SYMBOL to add to watchlist.")
        print("Example: python analyze.py --add SPY AAPL TSLA")
        sys.exit(1)

    # Fetch data if requested
    if args.update or args.full:
        fetch_all_data(symbols, full=args.full, source=args.source)

    # Show summary if requested
    if args.summary:
        show_summary(symbols)
        return

    # Run analysis
    print(f"\n{'='*50}")
    print(f"  PATTERN ANALYSIS")
    print(f"{'='*50}")
    print(f"  Symbols: {', '.join(symbols)}")
    print(f"  Patterns: {', '.join(args.patterns) if args.patterns else 'all'}")
    print(f"  Min confidence: {args.min_confidence}")

    all_signals = []
    for symbol in symbols:
        try:
            signals = analyze_symbol(symbol, patterns=args.patterns,
                                     min_confidence=args.min_confidence)
            all_signals.extend(signals)
            print(f"    → {len(signals)} signals")
        except Exception as e:
            print(f"    → ERROR: {e}")

    # Print results
    print_signals(all_signals, verbose=args.verbose)

    # Store if requested
    if args.store and all_signals:
        store_signals(all_signals)
        print(f"\n  Stored {len(all_signals)} signals in database")

    # Export if requested
    if args.export and all_signals:
        export_signals(all_signals, args.export)

    print(f"\n  Total signals: {len(all_signals)}")

    return all_signals


if __name__ == "__main__":
    main()