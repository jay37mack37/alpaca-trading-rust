"""
Day trading pattern detection library.

Each detector takes a symbol and a DataFrame of bars, and returns a list of
Signal dicts with keys: timestamp, pattern, direction, confidence, details.

Patterns implemented:
    - VWAP deviation (z-score of price vs VWAP)
    - Opening range breakout (15-min and 30-min)
    - Intraday mean reversion (price vs VWAP z-score)
    - Gap analysis (overnight gap fill probability)
    - Unusual volume detection (volume vs rolling average)
    - Momentum / rate of change
"""

import json
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from typing import Optional

import numpy as np
import pandas as pd


@dataclass
class Signal:
    timestamp: datetime
    symbol: str
    pattern: str
    direction: str  # "bullish", "bearish", "neutral"
    confidence: float  # 0.0 to 1.0
    details: dict

    def to_dict(self):
        d = asdict(self)
        d["timestamp"] = self.timestamp.isoformat()
        return d


# ---------------------------------------------------------------------------
# VWAP
# ---------------------------------------------------------------------------

def compute_vwap(bars_1m: pd.DataFrame) -> pd.Series:
    """
    Compute cumulative intraday VWAP from 1-minute bars.

    Expects bars_1m with columns: close, high, low, volume.
    Resets each trading day.
    """
    if bars_1m.empty or "volume" not in bars_1m.columns:
        return pd.Series(dtype=float)

    typical_price = (bars_1m["high"] + bars_1m["low"] + bars_1m["close"]) / 3
    tp_vol = typical_price * bars_1m["volume"]

    # Group by date to reset VWAP each day
    dates = bars_1m.index.date
    cum_tp_vol = tp_vol.groupby(dates).cumsum()
    cum_vol = bars_1m["volume"].groupby(dates).cumsum()

    vwap = cum_tp_vol / cum_vol.replace(0, np.nan)
    return vwap


def vwap_deviation(symbol: str, bars_1m: pd.DataFrame) -> list[Signal]:
    """
    Detect VWAP deviation signals.

    When price deviates significantly from VWAP, it tends to mean-revert.

    Signals:
        - Bullish: price is significantly BELOW VWAP (expect reversion up)
        - Bearish: price is significantly ABOVE VWAP (expect reversion down)
    """
    if len(bars_1m) < 20:
        return []

    vwap = compute_vwap(bars_1m)
    if vwap.empty:
        return []

    price = bars_1m["close"]
    deviation = (price - vwap) / vwap

    # Compute rolling z-score of deviation
    rolling_mean = deviation.rolling(60, min_periods=20).mean()
    rolling_std = deviation.rolling(60, min_periods=20).std()
    z_score = (deviation - rolling_mean) / rolling_std.replace(0, np.nan)

    signals = []
    for ts, z in z_score.dropna().items():
        if abs(z) < 1.5:
            continue

        direction = "bearish" if z > 0 else "bullish"
        confidence = min(abs(z) / 4.0, 1.0)  # scale 0-1

        signals.append(Signal(
            timestamp=ts,
            symbol=symbol,
            pattern="vwap_deviation",
            direction=direction,
            confidence=round(confidence, 3),
            details={
                "vwap": round(float(vwap.loc[ts]), 4),
                "price": round(float(price.loc[ts]), 4),
                "z_score": round(float(z), 3),
                "deviation_pct": round(float(deviation.loc[ts]) * 100, 3),
            }
        ))

    return signals


# ---------------------------------------------------------------------------
# Opening Range Breakout
# ---------------------------------------------------------------------------

def opening_range_breakout(symbol: str, bars_1m: pd.DataFrame,
                           range_minutes: int = 15) -> list[Signal]:
    """
    Detect opening range breakout signals.

    The opening range is the high/low of the first N minutes.
    A breakout above the opening range high is bullish;
    a breakdown below the opening range low is bearish.

    Args:
        range_minutes: Opening range duration (15 or 30).
    """
    if bars_1m.empty:
        return []

    signals = []
    dates = bars_1m.index.date

    for date in sorted(set(dates)):
        day_bars = bars_1m[bars_1m.index.date == date]
        if len(day_bars) < range_minutes + 5:
            continue

        # Opening range bars
        opening = day_bars.iloc[:range_minutes]
        range_high = opening["high"].max()
        range_low = opening["low"].min()
        range_size = range_high - range_low

        # Skip if opening range is tiny (no volatility = no signal)
        if range_size < 0.01:
            continue

        # Check rest of day for breakouts
        rest = day_bars.iloc[range_minutes:]
        for ts, row in rest.iterrows():
            if row["close"] > range_high:
                # Breakout above range
                strength = (row["close"] - range_high) / range_size
                signals.append(Signal(
                    timestamp=ts,
                    symbol=symbol,
                    pattern=f"opening_range_breakout_{range_minutes}m",
                    direction="bullish",
                    confidence=min(strength * 2, 1.0),
                    details={
                        "range_high": round(float(range_high), 4),
                        "range_low": round(float(range_low), 4),
                        "range_size": round(float(range_size), 4),
                        "breakout_price": round(float(row["close"]), 4),
                    }
                ))
                break  # Only first breakout per day

            elif row["close"] < range_low:
                # Breakdown below range
                strength = (range_low - row["close"]) / range_size
                signals.append(Signal(
                    timestamp=ts,
                    symbol=symbol,
                    pattern=f"opening_range_breakout_{range_minutes}m",
                    direction="bearish",
                    confidence=min(strength * 2, 1.0),
                    details={
                        "range_high": round(float(range_high), 4),
                        "range_low": round(float(range_low), 4),
                        "range_size": round(float(range_size), 4),
                        "breakdown_price": round(float(row["close"]), 4),
                    }
                ))
                break

    return signals


# ---------------------------------------------------------------------------
# Intraday Mean Reversion
# ---------------------------------------------------------------------------

def intraday_mean_reversion(symbol: str, bars_1m: pd.DataFrame) -> list[Signal]:
    """
    Detect intraday mean reversion signals.

    Uses Bollinger Bands on 1-minute closes relative to VWAP.
    When price touches the lower band, expect reversion to VWAP (bullish).
    When price touches the upper band, expect reversion to VWAP (bearish).
    """
    if len(bars_1m) < 60:
        return []

    vwap = compute_vwap(bars_1m)
    if vwap.empty:
        return []

    price = bars_1m["close"]
    deviation = (price - vwap) / vwap

    # Bollinger Bands on the deviation
    window = 120  # 2 hours
    rolling_mean = deviation.rolling(window, min_periods=30).mean()
    rolling_std = deviation.rolling(window, min_periods=30).std()
    upper_band = rolling_mean + 2 * rolling_std
    lower_band = rolling_mean - 2 * rolling_std

    signals = []
    for ts in deviation.index:
        if pd.isna(rolling_std.loc[ts]) or rolling_std.loc[ts] == 0:
            continue

        dev = deviation.loc[ts]
        ub = upper_band.loc[ts]
        lb = lower_band.loc[ts]

        if dev > ub:
            # Above upper band → bearish mean reversion
            confidence = min((dev - ub) / (rolling_std.loc[ts] + 1e-10), 1.0)
            signals.append(Signal(
                timestamp=ts,
                symbol=symbol,
                pattern="intraday_mean_reversion",
                direction="bearish",
                confidence=round(confidence, 3),
                details={
                    "vwap": round(float(vwap.loc[ts]), 4),
                    "price": round(float(price.loc[ts]), 4),
                    "deviation_pct": round(float(dev) * 100, 3),
                    "band_type": "upper",
                }
            ))
        elif dev < lb:
            # Below lower band → bullish mean reversion
            confidence = min((lb - dev) / (rolling_std.loc[ts] + 1e-10), 1.0)
            signals.append(Signal(
                timestamp=ts,
                symbol=symbol,
                pattern="intraday_mean_reversion",
                direction="bullish",
                confidence=round(confidence, 3),
                details={
                    "vwap": round(float(vwap.loc[ts]), 4),
                    "price": round(float(price.loc[ts]), 4),
                    "deviation_pct": round(float(dev) * 100, 3),
                    "band_type": "lower",
                }
            ))

    return signals


# ---------------------------------------------------------------------------
# Gap Analysis
# ---------------------------------------------------------------------------

def gap_analysis(symbol: str, bars_1d: pd.DataFrame) -> list[Signal]:
    """
    Detect overnight gaps and estimate fill probability.

    A gap up occurs when today's open > yesterday's high.
    A gap down occurs when today's open < yesterday's low.

    Gap fill probability is based on historical fill rate for similar-sized gaps.
    """
    if len(bars_1d) < 10:
        return []

    df = bars_1d.copy()
    df["prev_high"] = df["high"].shift(1)
    df["prev_low"] = df["low"].shift(1)
    df["prev_close"] = df["close"].shift(1)

    signals = []
    gaps = []

    for ts, row in df.dropna(subset=["prev_high"]).iterrows():
        gap_up = row["open"] - row["prev_high"]
        gap_down = row["prev_low"] - row["open"]

        if gap_up > 0:
            gap_pct = gap_up / row["prev_close"] * 100
            # Check if gap filled intraday
            filled = row["low"] <= row["prev_high"]
            gaps.append({"pct": gap_pct, "filled": filled, "direction": "up"})

            direction = "bullish"
            # Gaps that fill tend to be smaller — large gaps often don't fill same day
            confidence = min(gap_pct / 3.0, 1.0) if not filled else 0.3

            signals.append(Signal(
                timestamp=ts,
                symbol=symbol,
                pattern="gap_up",
                direction=direction,
                confidence=round(confidence, 3),
                details={
                    "gap_pct": round(float(gap_pct), 3),
                    "gap_size": round(float(gap_up), 4),
                    "prev_high": round(float(row["prev_high"]), 4),
                    "open": round(float(row["open"]), 4),
                    "filled": filled,
                }
            ))

        elif gap_down > 0:
            gap_pct = gap_down / row["prev_close"] * 100
            filled = row["high"] >= row["prev_low"]
            gaps.append({"pct": gap_pct, "filled": filled, "direction": "down"})

            direction = "bearish"
            confidence = min(gap_pct / 3.0, 1.0) if not filled else 0.3

            signals.append(Signal(
                timestamp=ts,
                symbol=symbol,
                pattern="gap_down",
                direction=direction,
                confidence=round(confidence, 3),
                details={
                    "gap_pct": round(float(gap_pct), 3),
                    "gap_size": round(float(gap_down), 4),
                    "prev_low": round(float(row["prev_low"]), 4),
                    "open": round(float(row["open"]), 4),
                    "filled": filled,
                }
            ))

    # Compute historical fill rates
    if gaps:
        fill_rate_up = sum(1 for g in gaps if g["direction"] == "up" and g["filled"]) / max(sum(1 for g in gaps if g["direction"] == "up"), 1)
        fill_rate_down = sum(1 for g in gaps if g["direction"] == "down" and g["filled"]) / max(sum(1 for g in gaps if g["direction"] == "down"), 1)
        avg_gap_pct = np.mean([g["pct"] for g in gaps])

        # Attach fill-rate metadata to all signals from this run
        for sig in signals:
            sig.details["hist_fill_rate_up"] = round(fill_rate_up, 3)
            sig.details["hist_fill_rate_down"] = round(fill_rate_down, 3)
            sig.details["avg_gap_pct"] = round(float(avg_gap_pct), 3)

    return signals


# ---------------------------------------------------------------------------
# Unusual Volume
# ---------------------------------------------------------------------------

def unusual_volume(symbol: str, bars: pd.DataFrame, timeframe: str = "1d",
                   threshold: float = 2.0) -> list[Signal]:
    """
    Detect unusual volume spikes.

    Compares current volume to a rolling average. If volume exceeds
    threshold * rolling_avg, it's flagged as unusual.

    Args:
        threshold: How many standard deviations above mean to flag.
    """
    if len(bars) < 20:
        return []

    window = 20
    vol = bars["volume"]
    rolling_mean = vol.rolling(window, min_periods=10).mean()
    rolling_std = vol.rolling(window, min_periods=10).std()

    # Volume z-score
    vol_z = (vol - rolling_mean) / rolling_std.replace(0, np.nan)

    signals = []
    for ts, z in vol_z.dropna().items():
        if z < threshold:
            continue

        row = bars.loc[ts]
        direction = "bullish" if row["close"] > row["open"] else "bearish"
        # Higher z-score = more unusual = higher confidence
        confidence = min(z / 5.0, 1.0)

        signals.append(Signal(
            timestamp=ts,
            symbol=symbol,
            pattern=f"unusual_volume_{timeframe}",
            direction=direction,
            confidence=round(confidence, 3),
            details={
                "volume": int(row["volume"]),
                "avg_volume": int(rolling_mean.loc[ts]),
                "volume_z_score": round(float(z), 2),
                "price_change_pct": round(float((row["close"] - row["open"]) / row["open"] * 100), 3),
            }
        ))

    return signals


# ---------------------------------------------------------------------------
# Momentum / Rate of Change
# ---------------------------------------------------------------------------

def momentum(symbol: str, bars: pd.DataFrame, timeframe: str = "1d",
             periods: list[int] = [5, 10, 20]) -> list[Signal]:
    """
    Detect momentum signals using rate of change over multiple periods.

    When short-term ROC is above long-term ROC and accelerating → bullish.
    When short-term ROC is below long-term ROC and decelerating → bearish.
    """
    if len(bars) < max(periods) + 5:
        return []

    price = bars["close"]
    signals = []

    for period in periods:
        if len(bars) < period * 2:
            continue

        roc = price.pct_change(periods=period) * 100
        roc_mean = roc.rolling(period, min_periods=5).mean()
        roc_std = roc.rolling(period, min_periods=5).std()

        # Current ROC z-score relative to its own distribution
        roc_z = (roc - roc_mean) / roc_std.replace(0, np.nan)

        for ts, z in roc_z.dropna().items():
            if abs(z) < 1.5:
                continue

            direction = "bullish" if z > 0 else "bearish"
            confidence = min(abs(z) / 3.0, 1.0)

            signals.append(Signal(
                timestamp=ts,
                symbol=symbol,
                pattern=f"momentum_{period}{timeframe}",
                direction=direction,
                confidence=round(confidence, 3),
                details={
                    "roc_pct": round(float(roc.loc[ts]), 3),
                    "roc_z_score": round(float(z), 2),
                    "period": period,
                }
            ))

    return signals


# ---------------------------------------------------------------------------
# Aggregate runner
# ---------------------------------------------------------------------------

def _require(df, func, sym, **kwargs):
    """Run func only if df is not None and not empty."""
    if df is None or (isinstance(df, pd.DataFrame) and df.empty):
        return []
    return func(sym, df, **kwargs)


ALL_PATTERNS = {
    "vwap_deviation": lambda sym, bars_1m=None, **kw: _require(bars_1m, vwap_deviation, sym),
    "opening_range_15m": lambda sym, bars_1m=None, **kw: _require(bars_1m, opening_range_breakout, sym, range_minutes=15),
    "opening_range_30m": lambda sym, bars_1m=None, **kw: _require(bars_1m, opening_range_breakout, sym, range_minutes=30),
    "intraday_mean_reversion": lambda sym, bars_1m=None, **kw: _require(bars_1m, intraday_mean_reversion, sym),
    "gap_analysis": lambda sym, bars_1d=None, **kw: _require(bars_1d, gap_analysis, sym),
    "unusual_volume_1m": lambda sym, bars_1m=None, **kw: _require(bars_1m, unusual_volume, sym, timeframe="1m"),
    "unusual_volume_1d": lambda sym, bars_1d=None, **kw: _require(bars_1d, unusual_volume, sym, timeframe="1d"),
    "momentum_1d": lambda sym, bars_1d=None, **kw: _require(bars_1d, momentum, sym, timeframe="1d"),
}


def run_all_patterns(symbol: str, bars_1m: pd.DataFrame = None,
                     bars_5m: pd.DataFrame = None,
                     bars_1h: pd.DataFrame = None,
                     bars_1d: pd.DataFrame = None,
                     patterns: list[str] = None) -> list[Signal]:
    """
    Run all pattern detectors on the given data.
    Pass in whatever bar dataframes you have; patterns will skip if data is missing.
    """
    if patterns is None:
        patterns = list(ALL_PATTERNS.keys())

    all_signals = []
    kwargs = {"bars_1m": bars_1m, "bars_5m": bars_5m, "bars_1h": bars_1h, "bars_1d": bars_1d}

    for pattern_name in patterns:
        if pattern_name not in ALL_PATTERNS:
            print(f"  Unknown pattern: {pattern_name}")
            continue
        try:
            sigs = ALL_PATTERNS[pattern_name](symbol, **kwargs)
            all_signals.extend(sigs)
        except Exception as e:
            print(f"  Error in {pattern_name}: {e}")

    return all_signals