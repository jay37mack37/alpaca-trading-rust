"""
SQLite database layer for market data storage using peewee ORM.

Tables:
    - watchlist: symbols to track
    - bars_1m, bars_5m, bars_1h, bars_1d: OHLCV bar data per timeframe
    - signals: pattern detection output
"""

import os
from datetime import datetime
from peewee import (
    SqliteDatabase,
    Model,
    CharField,
    DateTimeField,
    FloatField,
    IntegerField,
    TextField,
    CompositeKey,
    SQL,
)

DB_PATH = os.path.join(os.path.dirname(__file__), "market_data.db")
db = SqliteDatabase(DB_PATH, pragmas={
    "journal_mode": "wal",
    "cache_size": -1024 * 64,
    "foreign_keys": 1,
})


class BaseModel(Model):
    class Meta:
        database = db


class Watchlist(BaseModel):
    symbol = CharField(primary_key=True)
    added_at = DateTimeField(default=datetime.utcnow)

    class Meta:
        table_name = "watchlist"


class BarModel(BaseModel):
    """Base class for OHLCV bar tables. Not used directly."""
    symbol = CharField(index=True)
    timestamp = DateTimeField(index=True)
    open = FloatField()
    high = FloatField()
    low = FloatField()
    close = FloatField()
    volume = IntegerField()
    trade_count = IntegerField(null=True)
    vwap = FloatField(null=True)

    class Meta:
        indexes = (
            (("symbol", "timestamp"), True),  # unique composite
        )


class Bar1m(BarModel):
    class Meta:
        table_name = "bars_1m"


class Bar5m(BarModel):
    class Meta:
        table_name = "bars_5m"


class Bar1h(BarModel):
    class Meta:
        table_name = "bars_1h"


class Bar1d(BarModel):
    class Meta:
        table_name = "bars_1d"


class Signal(BaseModel):
    id = IntegerField(primary_key=True)
    symbol = CharField(index=True)
    timestamp = DateTimeField(index=True)
    pattern = CharField()  # e.g. "vwap_deviation", "opening_range_breakout"
    direction = CharField()  # "bullish", "bearish", "neutral"
    confidence = FloatField()  # 0.0 to 1.0
    details = TextField(null=True)  # JSON string with pattern-specific data
    created_at = DateTimeField(default=datetime.utcnow)

    class Meta:
        table_name = "signals"
        indexes = (
            (("symbol", "pattern", "timestamp"), False),
        )


# Map timeframe strings to their model classes
BAR_MODELS = {
    "1m": Bar1m,
    "5m": Bar5m,
    "1h": Bar1h,
    "1d": Bar1d,
}


def init_db():
    """Create all tables if they don't exist."""
    db.connect(reuse_if_open=True)
    db.create_tables([Watchlist, Bar1m, Bar5m, Bar1h, Bar1d, Signal])


def get_bar_model(timeframe: str):
    """Get the peewee model class for a given timeframe string."""
    return BAR_MODELS[timeframe]


if __name__ == "__main__":
    init_db()
    print(f"Database initialized at {DB_PATH}")