CREATE TABLE IF NOT EXISTS app_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS credentials (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    label TEXT NOT NULL,
    environment TEXT NOT NULL,
    api_key_encrypted TEXT NOT NULL,
    api_secret_encrypted TEXT NOT NULL,
    use_for_data INTEGER NOT NULL,
    use_for_trading INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS watchlist (
    symbol TEXT PRIMARY KEY,
    added_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS strategies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    enabled INTEGER NOT NULL,
    execution_mode TEXT NOT NULL,
    asset_class_target TEXT NOT NULL DEFAULT 'equity',
    option_entry_style TEXT NOT NULL DEFAULT 'long_call',
    option_structure_preset TEXT NOT NULL DEFAULT 'single',
    option_spread_width REAL NOT NULL DEFAULT 5.0,
    option_target_delta REAL NOT NULL DEFAULT 0.30,
    option_dte_min INTEGER NOT NULL DEFAULT 21,
    option_dte_max INTEGER NOT NULL DEFAULT 45,
    option_max_spread_pct REAL NOT NULL DEFAULT 0.12,
    option_limit_buffer_pct REAL NOT NULL DEFAULT 0.05,
    credential_id TEXT,
    starting_cash REAL NOT NULL,
    cash_balance REAL NOT NULL,
    equity REAL NOT NULL,
    tracked_symbols TEXT NOT NULL,
    total_trades INTEGER NOT NULL,
    wins INTEGER NOT NULL,
    losses INTEGER NOT NULL,
    last_signal TEXT,
    last_run_at TEXT,
    run_interval_ms INTEGER NOT NULL DEFAULT 30000,
    state_json TEXT NOT NULL DEFAULT '{}',
    risk_parameters_json TEXT,
    FOREIGN KEY (credential_id) REFERENCES credentials(id)
);

CREATE TABLE IF NOT EXISTS strategy_positions (
    strategy_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    underlying_symbol TEXT NOT NULL DEFAULT '',
    instrument_symbol TEXT NOT NULL DEFAULT '',
    asset_type TEXT NOT NULL,
    quantity REAL NOT NULL,
    average_price REAL NOT NULL,
    market_price REAL NOT NULL,
    multiplier REAL NOT NULL DEFAULT 1.0,
    option_structure_preset TEXT,
    option_type TEXT,
    expiration TEXT,
    strike REAL,
    stale_quote INTEGER NOT NULL DEFAULT 0,
    legs_json TEXT NOT NULL DEFAULT '[]',
    razor_stop REAL,
    stagnation_timestamp TEXT,
    kronos_sentiment REAL,
    take_profit REAL,
    exit_logic TEXT,
    entry_time TEXT,
    buy_logic TEXT,
    entry_math TEXT,
    entry_ai REAL,
    hold_intent TEXT,
    planned_exit TEXT,
    PRIMARY KEY (strategy_id, symbol),
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS trade_log (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    underlying_symbol TEXT NOT NULL DEFAULT '',
    instrument_symbol TEXT NOT NULL DEFAULT '',
    asset_type TEXT NOT NULL DEFAULT 'equity',
    side TEXT NOT NULL,
    quantity REAL NOT NULL,
    price REAL NOT NULL,
    multiplier REAL NOT NULL DEFAULT 1.0,
    option_structure_preset TEXT,
    option_type TEXT,
    expiration TEXT,
    strike REAL,
    legs_json TEXT NOT NULL DEFAULT '[]',
    provider TEXT NOT NULL,
    execution_mode TEXT NOT NULL,
    reason TEXT NOT NULL,
    realized_pnl REAL,
    executed_at TEXT NOT NULL,
    hidden INTEGER NOT NULL DEFAULT 0,
    hold_intent TEXT,
    exit_logic TEXT,
    planned_exit TEXT,
    buy_logic TEXT,
    entry_math TEXT,
    entry_ai REAL,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS market_snapshots (
    id TEXT PRIMARY KEY,
    symbol TEXT NOT NULL,
    provider TEXT NOT NULL,
    price REAL NOT NULL,
    bid REAL,
    ask REAL,
    volume REAL,
    vwap REAL,
    day_high REAL,
    day_low REAL,
    captured_at TEXT NOT NULL,
    raw_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS option_snapshots (
    id TEXT PRIMARY KEY,
    underlying_symbol TEXT NOT NULL,
    provider TEXT NOT NULL,
    contract_symbol TEXT NOT NULL,
    option_type TEXT NOT NULL,
    expiration TEXT NOT NULL,
    strike REAL NOT NULL,
    bid REAL,
    ask REAL,
    last REAL,
    implied_volatility REAL,
    open_interest REAL,
    volume REAL,
    in_the_money INTEGER,
    delta REAL,
    gamma REAL,
    theta REAL,
    vega REAL,
    moneyness REAL,
    captured_at TEXT NOT NULL,
    raw_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS broker_accounts (
    credential_id TEXT PRIMARY KEY,
    environment TEXT NOT NULL,
    account_id TEXT NOT NULL,
    account_number TEXT,
    status TEXT,
    currency TEXT,
    buying_power REAL,
    cash REAL,
    equity REAL,
    portfolio_value REAL,
    last_equity REAL,
    long_market_value REAL,
    short_market_value REAL,
    pattern_day_trader INTEGER NOT NULL,
    trading_blocked INTEGER NOT NULL,
    transfers_blocked INTEGER NOT NULL,
    account_blocked INTEGER NOT NULL,
    synced_at TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS broker_positions (
    credential_id TEXT NOT NULL,
    symbol TEXT NOT NULL,
    asset_class TEXT,
    side TEXT,
    quantity REAL NOT NULL,
    avg_entry_price REAL,
    market_value REAL,
    current_price REAL,
    unrealized_pl REAL,
    unrealized_plpc REAL,
    synced_at TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    PRIMARY KEY (credential_id, symbol),
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS broker_orders (
    credential_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    client_order_id TEXT,
    symbol TEXT,
    side TEXT,
    order_type TEXT,
    order_class TEXT,
    status TEXT,
    quantity REAL,
    filled_qty REAL,
    filled_avg_price REAL,
    time_in_force TEXT,
    submitted_at TEXT,
    updated_at TEXT,
    synced_at TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    PRIMARY KEY (credential_id, order_id),
    FOREIGN KEY (credential_id) REFERENCES credentials(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS watchlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    symbols TEXT NOT NULL
);
