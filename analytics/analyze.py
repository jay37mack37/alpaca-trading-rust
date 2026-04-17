#!/usr/bin/env python3
"""
Main analysis runner.

Fetches data, runs pattern detection, and outputs signals.

Usage:
    # First time: add symbols to watchlist and fetch data
    python analyze.py --add SPY AAPL TSLA

    # Run analysis on all watchlist symbols
    python analyze.py

    # JSON output (for API use)
    python analyze.py --format json --summary
    python analyze.py --format json --symbols SPY --min-confidence 0.5

    # Update data then analyze
    python analyze.py --update

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


def log(msg, json_mode=False):
    """Print to stderr in JSON mode (so stdout stays clean), stdout otherwise."""
    if json_mode:
        print(msg, file=sys.stderr)
    else:
        print(msg)


def fetch_all_data(symbols: list[str], timeframes: list[str] = None, full: bool = False,
                   source: str = "auto", json_mode: bool = False):
    """Fetch data for all symbols and timeframes. Returns list of result dicts."""
    if timeframes is None:
        timeframes = ["1m", "5m", "1h", "1d"]

    log(f"\n{'='*50}\nFETCHING DATA\n{'='*50}", json_mode)

    results = []
    for symbol in symbols:
        log(f"\n  {symbol}:", json_mode)
        for tf in timeframes:
            try:
                count = fetch_and_store(symbol, tf, full=full, source=source)
                log(f"    {tf}: {count} new bars", json_mode)
                results.append({"symbol": symbol, "timeframe": tf, "bars_fetched": count, "status": "ok"})
            except Exception as e:
                log(f"    {tf}: ERROR - {e}", json_mode)
                results.append({"symbol": symbol, "timeframe": tf, "bars_fetched": 0, "status": "error", "error": str(e)})
    return results


def analyze_symbol(symbol: str, patterns: list[str] = None, min_confidence: float = 0.0,
                   json_mode: bool = False):
    """Run analysis on a single symbol. Returns list of Signal objects."""
    log(f"\n  Analyzing {symbol}...", json_mode)

    # Load data from local DB
    bars = {}
    for tf in ["1m", "5m", "1h", "1d"]:
        model = get_bar_model(tf)
        count = model.select().where(model.symbol == symbol).count()
        if count > 0:
            bars[f"bars_{tf}"] = load_bars(symbol, tf)
            log(f"    {tf}: {count} bars loaded", json_mode)
        else:
            bars[f"bars_{tf}"] = None
            log(f"    {tf}: no data — run --update first", json_mode)

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


def build_summary_json(symbols: list[str]) -> dict:
    """Build a data summary dict for JSON output."""
    result = {"symbols": []}
    for symbol in symbols:
        sym_data = {"symbol": symbol, "timeframes": []}
        for tf in ["1m", "5m", "1h", "1d"]:
            model = get_bar_model(tf)
            count = model.select().where(model.symbol == symbol).count()
            tf_info = {"timeframe": tf, "bar_count": count}
            if count > 0:
                first = model.select(model.timestamp).where(
                    model.symbol == symbol
                ).order_by(model.timestamp.asc()).first()
                last = model.select(model.timestamp).where(
                    model.symbol == symbol
                ).order_by(model.timestamp.desc()).first()
                tf_info["first_date"] = first.timestamp.strftime("%Y-%m-%d")
                tf_info["last_date"] = last.timestamp.strftime("%Y-%m-%d")
            sym_data["timeframes"].append(tf_info)
        result["symbols"].append(sym_data)
    return result


def show_summary(symbols: list[str]):
    """Show a summary of stored data per symbol (text mode)."""
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
    parser.add_argument("--format", choices=["text", "json"], default="text", dest="output_format",
                        help="Output format: text (human-readable) or json (structured)")
    parser.add_argument("--watchlist-only", action="store_true",
                        help="Output watchlist as JSON and exit")
    args = parser.parse_args()

    json_mode = args.output_format == "json"
    init_db()

    # Manage watchlist (MUST happen before --watchlist-only check)
    if args.add:
        add_to_watchlist(args.add)
        log(f"Added {args.add} to watchlist", json_mode)

    if args.remove:
        for sym in args.remove:
            sym = sym.upper().strip()
            deleted = Watchlist.delete().where(Watchlist.symbol == sym).execute()
            log(f"Removed {sym} ({deleted} entries)", json_mode)

    # Handle watchlist-only mode (AFTER add/remove so they take effect)
    if args.watchlist_only:
        symbols = get_watchlist_symbols()
        print(json.dumps({"symbols": symbols}))
        return

    # Get symbols to analyze
    symbols = args.symbols or get_watchlist_symbols()
    if not symbols:
        if json_mode:
            print(json.dumps({"error": "No symbols in watchlist. Add symbols first."}))
        else:
            print("No symbols to analyze. Use --add SYMBOL to add to watchlist.")
            print("Example: python analyze.py --add SPY AAPL TSLA")
        sys.exit(1)

    # Fetch data if requested
    fetch_results = None
    if args.update or args.full:
        fetch_results = fetch_all_data(symbols, full=args.full, source=args.source, json_mode=json_mode)
        if json_mode:
            # In JSON mode with fetch, output fetch results and stop unless also analyzing
            pass

    # Show summary if requested
    if args.summary:
        if json_mode:
            print(json.dumps(build_summary_json(symbols), indent=2, default=str))
        else:
            show_summary(symbols)
        return

    # Run analysis
    if not json_mode:
        log(f"\n{'='*50}\n  PATTERN ANALYSIS\n{'='*50}", json_mode=False)
        log(f"  Symbols: {', '.join(symbols)}", json_mode=False)
        log(f"  Patterns: {', '.join(args.patterns) if args.patterns else 'all'}", json_mode=False)
        log(f"  Min confidence: {args.min_confidence}", json_mode=False)

    all_signals = []
    for symbol in symbols:
        try:
            signals = analyze_symbol(symbol, patterns=args.patterns,
                                     min_confidence=args.min_confidence,
                                     json_mode=json_mode)
            all_signals.extend(signals)
            log(f"    → {len(signals)} signals", json_mode)
        except Exception as e:
            log(f"    → ERROR: {e}", json_mode)

    if json_mode:
        # Output signals as JSON to stdout
        output = {
            "signals": [s.to_dict() for s in all_signals],
            "count": len(all_signals),
        }
        if fetch_results:
            output["fetch_results"] = fetch_results
        print(json.dumps(output, indent=2, default=str))
    else:
        # Print results in human-readable format
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