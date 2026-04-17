use std::{collections::BTreeSet, fs, path::Path};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        AssetClassTarget, BrokerAccountSummary, BrokerOrderSummary, BrokerPositionSummary,
        BrokerSyncState, CreateCredentialRequest, CreateStrategyRequest, CredentialEnvironment,
        CredentialSummary, DataProvider, ExecutionMode, OptionContractSnapshot,
        OptionEntryStyle, OptionStructurePreset, PositionLeg, PositionRecord, PositionSummary,
        Quote, SignalAction, StoredCredential, StrategyDetailResponse, StrategyKind,
        StrategyRecord, StrategySignal, StrategySummary, TradeLeg, TradeRecord, TradeSide,
        UpdateStrategyRequest,
    },
};

pub struct Database {
    conn: Connection,
    /// Cached AES-256-GCM cipher derived via Argon2id from the master key and the
    /// per-install salt stored in `app_config`. Cached because Argon2id is
    /// intentionally slow, and the master key cannot change without restarting.
    credential_cipher: Aes256Gcm,
}

#[derive(Debug, Clone)]
pub struct LocalTradeInput {
    pub underlying_symbol: String,
    pub instrument_symbol: String,
    pub asset_type: String,
    pub side: TradeSide,
    pub quantity: f64,
    pub price: f64,
    pub multiplier: f64,
    pub option_structure_preset: Option<OptionStructurePreset>,
    pub option_type: Option<String>,
    pub expiration: Option<String>,
    pub strike: Option<f64>,
    pub legs: Vec<TradeLeg>,
}

impl Database {
    pub fn open(
        path: &Path,
        default_watchlist: &[String],
        master_key: &str,
    ) -> AppResult<Self> {
        if master_key.trim().is_empty() {
            return Err(AppError::Internal(
                "AUTO_STONKS_MASTER_KEY is required; set it to a strong, unique secret before starting the backend".to_string(),
            ));
        }
        if master_key == "change-me-before-production" {
            return Err(AppError::Internal(
                "AUTO_STONKS_MASTER_KEY still uses the placeholder value from .env.example; set it to a strong, unique secret".to_string(),
            ));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| AppError::Internal(err.to_string()))?;
        }

        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        let salt = load_or_create_credential_salt(&conn)?;
        let credential_cipher = derive_credential_cipher(master_key, &salt)?;

        let db = Self {
            conn,
            credential_cipher,
        };
        db.seed_default_watchlist(default_watchlist)?;
        db.seed_default_strategies()?;
        Ok(db)
    }

    fn init_schema(conn: &Connection) -> AppResult<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
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
            "#,
        )?;

        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN asset_class_target TEXT NOT NULL DEFAULT 'options'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN run_interval_ms INTEGER NOT NULL DEFAULT 30000",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_entry_style TEXT NOT NULL DEFAULT 'long_call'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_structure_preset TEXT NOT NULL DEFAULT 'single'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_spread_width REAL NOT NULL DEFAULT 5.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_target_delta REAL NOT NULL DEFAULT 0.30",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_dte_min INTEGER NOT NULL DEFAULT 21",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_dte_max INTEGER NOT NULL DEFAULT 45",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_max_spread_pct REAL NOT NULL DEFAULT 0.12",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategies ADD COLUMN option_limit_buffer_pct REAL NOT NULL DEFAULT 0.05",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategy_positions ADD COLUMN underlying_symbol TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategy_positions ADD COLUMN instrument_symbol TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategy_positions ADD COLUMN multiplier REAL NOT NULL DEFAULT 1.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategy_positions ADD COLUMN option_structure_preset TEXT",
            [],
        );
        let _ = conn.execute("ALTER TABLE strategy_positions ADD COLUMN option_type TEXT", []);
        let _ = conn.execute("ALTER TABLE strategy_positions ADD COLUMN expiration TEXT", []);
        let _ = conn.execute("ALTER TABLE strategy_positions ADD COLUMN strike REAL", []);
        let _ = conn.execute(
            "ALTER TABLE strategy_positions ADD COLUMN stale_quote INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE strategy_positions ADD COLUMN legs_json TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trade_log ADD COLUMN underlying_symbol TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trade_log ADD COLUMN instrument_symbol TEXT NOT NULL DEFAULT ''",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trade_log ADD COLUMN asset_type TEXT NOT NULL DEFAULT 'equity'",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE trade_log ADD COLUMN multiplier REAL NOT NULL DEFAULT 1.0",
            [],
        );
        let _ = conn.execute("ALTER TABLE trade_log ADD COLUMN option_structure_preset TEXT", []);
        let _ = conn.execute("ALTER TABLE trade_log ADD COLUMN option_type TEXT", []);
        let _ = conn.execute("ALTER TABLE trade_log ADD COLUMN expiration TEXT", []);
        let _ = conn.execute("ALTER TABLE trade_log ADD COLUMN strike REAL", []);
        let _ = conn.execute(
            "ALTER TABLE trade_log ADD COLUMN legs_json TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        let _ = conn.execute(
            "UPDATE strategy_positions
             SET underlying_symbol = COALESCE(NULLIF(underlying_symbol, ''), symbol),
                 instrument_symbol = COALESCE(NULLIF(instrument_symbol, ''), symbol),
                 multiplier = COALESCE(multiplier, 1.0),
                 stale_quote = COALESCE(stale_quote, 0),
                 legs_json = COALESCE(NULLIF(legs_json, ''), '[]')",
            [],
        );
        let _ = conn.execute(
            "UPDATE trade_log
             SET underlying_symbol = COALESCE(NULLIF(underlying_symbol, ''), symbol),
                 instrument_symbol = COALESCE(NULLIF(instrument_symbol, ''), symbol),
                 asset_type = COALESCE(NULLIF(asset_type, ''), 'equity'),
                 multiplier = COALESCE(multiplier, 1.0),
                 legs_json = COALESCE(NULLIF(legs_json, ''), '[]')",
            [],
        );
        // Run migrations for existing databases that were missing these columns
        let _ = conn.execute("ALTER TABLE option_snapshots ADD COLUMN delta REAL", []);
        let _ = conn.execute("ALTER TABLE option_snapshots ADD COLUMN gamma REAL", []);
        let _ = conn.execute("ALTER TABLE option_snapshots ADD COLUMN theta REAL", []);
        let _ = conn.execute("ALTER TABLE option_snapshots ADD COLUMN vega REAL", []);
        let _ = conn.execute("ALTER TABLE option_snapshots ADD COLUMN moneyness REAL", []);

        Ok(())
    }

    fn seed_default_watchlist(&self, defaults: &[String]) -> AppResult<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM watchlist", [], |row| row.get(0))?;

        if count > 0 || defaults.is_empty() {
            return Ok(());
        }

        let now = now();
        for symbol in defaults {
            self.conn.execute(
                "INSERT OR IGNORE INTO watchlist (symbol, added_at) VALUES (?1, ?2)",
                params![symbol.to_uppercase(), now],
            )?;
        }

        Ok(())
    }

    fn seed_default_strategies(&self) -> AppResult<()> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM strategies", [], |row| row.get(0))?;

        if count > 0 {
            return Ok(());
        }

        let now = now();
        let defaults = [
            (
                "vwap-reflexive",
                "VWAP Reflexive Agent",
                StrategyKind::VwapReflexive,
                vec!["AAPL", "SPY"],
            ),
            (
                "rsi-mean-reversion",
                "RSI Mean Reversion",
                StrategyKind::RsiMeanReversion,
                vec!["QQQ", "NVDA"],
            ),
            (
                "sma-trend",
                "SMA Trend Filter",
                StrategyKind::SmaTrend,
                vec!["MSFT", "META"],
            ),
        ];

        for (id, name, kind, symbols) in defaults {
            self.conn.execute(
                "INSERT INTO strategies (
                    id, name, kind, enabled, execution_mode, asset_class_target, option_entry_style,
                    option_structure_preset, option_spread_width, option_target_delta,
                    option_dte_min, option_dte_max, option_max_spread_pct, option_limit_buffer_pct, credential_id,
                    starting_cash, cash_balance, equity, tracked_symbols,
                    total_trades, wins, losses, last_signal, last_run_at, run_interval_ms
                ) VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, 25000.0, 25000.0, 25000.0, ?14, 0, 0, 0, ?15, ?16, ?17)",
                params![
                    id,
                    name,
                    kind.as_str(),
                    execution_mode_to_str(ExecutionMode::LocalPaper),
                    asset_class_target_to_str(AssetClassTarget::Equity),
                    option_entry_style_to_str(OptionEntryStyle::LongCall),
                    option_structure_preset_to_str(OptionStructurePreset::Single),
                    5.0_f64,
                    0.30_f64,
                    21_u32,
                    45_u32,
                    0.12_f64,
                    0.05_f64,
                    serde_json::to_string(&symbols)?,
                    "Seeded strategy",
                    now,
                    30000_u64,
                ],
            )?;
        }

        Ok(())
    }

    pub fn list_credentials(&self) -> AppResult<Vec<CredentialSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, provider, label, environment, api_key_encrypted, use_for_data, use_for_trading, created_at
             FROM credentials
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let encrypted_key: String = row.get(4)?;
            Ok(CredentialSummary {
                id: row.get(0)?,
                provider: provider_from_str(&row.get::<_, String>(1)?)?,
                label: row.get(2)?,
                environment: credential_environment_from_str(&row.get::<_, String>(3)?)?,
                use_for_data: row.get::<_, i64>(5)? != 0,
                use_for_trading: row.get::<_, i64>(6)? != 0,
                masked_key: mask_encrypted_key(&encrypted_key),
                created_at: row.get(7)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn insert_credential(
        &self,
        request: CreateCredentialRequest,
    ) -> AppResult<CredentialSummary> {
        if request.api_key.trim().is_empty() || request.api_secret.trim().is_empty() {
            return Err(AppError::Validation(
                "Alpaca API key and secret are required".to_string(),
            ));
        }

        let id = Uuid::new_v4().to_string();
        let created_at = now();
        let encrypted_key = encrypt(&self.credential_cipher, request.api_key.trim())?;
        let encrypted_secret = encrypt(&self.credential_cipher, request.api_secret.trim())?;
        let provider = DataProvider::Alpaca;

        self.conn.execute(
            "INSERT INTO credentials (
                id, provider, label, environment, api_key_encrypted, api_secret_encrypted,
                use_for_data, use_for_trading, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                provider.as_str(),
                request.label.trim(),
                credential_environment_to_str(request.environment),
                encrypted_key,
                encrypted_secret,
                request.use_for_data as i64,
                request.use_for_trading as i64,
                created_at,
            ],
        )?;

        Ok(CredentialSummary {
            id,
            label: request.label,
            provider,
            environment: request.environment,
            use_for_data: request.use_for_data,
            use_for_trading: request.use_for_trading,
            masked_key: mask_raw_key(request.api_key.trim()),
            created_at,
        })
    }

    pub fn resolve_alpaca_credential(
        &self,
        preferred_id: Option<&str>,
        require_trading: bool,
    ) -> AppResult<Option<StoredCredential>> {
        let sql = if preferred_id.is_some() {
            "SELECT id, label, environment, api_key_encrypted, api_secret_encrypted, use_for_data, use_for_trading
             FROM credentials WHERE id = ?1 LIMIT 1"
        } else if require_trading {
            "SELECT id, label, environment, api_key_encrypted, api_secret_encrypted, use_for_data, use_for_trading
             FROM credentials WHERE provider = 'alpaca' AND use_for_trading = 1
             ORDER BY created_at DESC LIMIT 1"
        } else {
            "SELECT id, label, environment, api_key_encrypted, api_secret_encrypted, use_for_data, use_for_trading
             FROM credentials WHERE provider = 'alpaca' AND use_for_data = 1
             ORDER BY created_at DESC LIMIT 1"
        };

        let cipher = &self.credential_cipher;
        let mut stmt = self.conn.prepare(sql)?;
        let row = if let Some(id) = preferred_id {
            stmt.query_row(params![id], |row| decode_credential_row(row, cipher))
                .optional()?
        } else {
            stmt.query_row([], |row| decode_credential_row(row, cipher))
                .optional()?
        };

        Ok(row)
    }

    pub fn list_strategy_records(&self) -> AppResult<Vec<StrategyRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, enabled, execution_mode, asset_class_target,
                    option_entry_style, option_structure_preset, option_spread_width,
                    option_target_delta, option_dte_min, option_dte_max,
                    option_max_spread_pct, option_limit_buffer_pct, credential_id, starting_cash,
                    cash_balance, equity, tracked_symbols, total_trades, wins, losses,
                    last_signal, last_run_at, run_interval_ms
             FROM strategies
             ORDER BY CASE WHEN kind = 'vwap_reflexive' THEN 0 ELSE 1 END, name ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StrategyRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: strategy_kind_from_str(&row.get::<_, String>(2)?)?,
                enabled: row.get::<_, i64>(3)? != 0,
                execution_mode: execution_mode_from_str(&row.get::<_, String>(4)?)?,
                asset_class_target: asset_class_target_from_str(&row.get::<_, String>(5)?)?,
                option_entry_style: option_entry_style_from_str(&row.get::<_, String>(6)?)?,
                option_structure_preset: option_structure_preset_from_str(&row.get::<_, String>(7)?)?,
                option_spread_width: row.get(8)?,
                option_target_delta: row.get(9)?,
                option_dte_min: row.get(10)?,
                option_dte_max: row.get(11)?,
                option_max_spread_pct: row.get(12)?,
                option_limit_buffer_pct: row.get(13)?,
                credential_id: row.get(14)?,
                starting_cash: row.get(15)?,
                cash_balance: row.get(16)?,
                equity: row.get(17)?,
                tracked_symbols: parse_symbols(&row.get::<_, String>(18)?),
                total_trades: row.get::<_, i64>(19)? as usize,
                wins: row.get::<_, i64>(20)? as usize,
                losses: row.get::<_, i64>(21)? as usize,
                last_signal: row.get(22)?,
                last_run_at: row.get(23)?,
                run_interval_ms: row.get(24)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn insert_strategy(&self, request: CreateStrategyRequest) -> AppResult<StrategySummary> {
        let name = request.name.trim();
        if name.is_empty() {
            return Err(AppError::Validation(
                "strategy name is required".to_string(),
            ));
        }

        if request.tracked_symbols.is_empty() {
            return Err(AppError::Validation(
                "at least one tracked symbol is required".to_string(),
            ));
        }

        let starting_cash = request.starting_cash.unwrap_or(25_000.0);
        if starting_cash <= 0.0 {
            return Err(AppError::Validation(
                "starting cash must be positive".to_string(),
            ));
        }

        let id = format!(
            "{}-{}",
            slugify(name),
            &Uuid::new_v4().simple().to_string()[..8]
        );
        let tracked_symbols = normalize_symbols(&request.tracked_symbols);
        let enabled = request.enabled.unwrap_or(true);
        let execution_mode = request.execution_mode.unwrap_or(ExecutionMode::LocalPaper);
        let options = normalize_strategy_options(
            request.asset_class_target,
            request.option_entry_style,
            request.option_structure_preset,
            request.option_spread_width,
            request.option_target_delta,
            request.option_dte_min,
            request.option_dte_max,
            request.option_max_spread_pct,
            request.option_limit_buffer_pct,
            None,
        )?;
        let run_interval_ms = request.run_interval_ms.unwrap_or(30000);

        self.conn.execute(
            "INSERT INTO strategies (
                id, name, kind, enabled, execution_mode, asset_class_target, option_entry_style,
                option_structure_preset, option_spread_width, option_target_delta, option_dte_min,
                option_dte_max, option_max_spread_pct, option_limit_buffer_pct, credential_id,
                starting_cash, cash_balance, equity, tracked_symbols,
                total_trades, wins, losses, last_signal, last_run_at, run_interval_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16, ?16, ?17, 0, 0, 0, ?18, ?19, ?20)",
            params![
                id,
                name,
                request.kind.as_str(),
                enabled as i64,
                execution_mode_to_str(execution_mode),
                asset_class_target_to_str(options.asset_class_target),
                option_entry_style_to_str(options.option_entry_style),
                option_structure_preset_to_str(options.option_structure_preset),
                options.option_spread_width,
                options.option_target_delta,
                options.option_dte_min,
                options.option_dte_max,
                options.option_max_spread_pct,
                options.option_limit_buffer_pct,
                request
                    .credential_id
                    .filter(|value| !value.trim().is_empty()),
                starting_cash,
                serde_json::to_string(&tracked_symbols)?,
                if enabled {
                    "Agent active"
                } else {
                    "Agent created"
                },
                now(),
                run_interval_ms,
            ],
        )?;

        self.strategy_detail(&id).map(|detail| detail.strategy)
    }

    pub fn list_strategies(&self) -> AppResult<Vec<StrategySummary>> {
        self.list_strategy_records()?
            .into_iter()
            .map(|record| self.map_strategy_summary(record))
            .collect()
    }

    pub fn strategy_detail(&self, strategy_id: &str) -> AppResult<StrategyDetailResponse> {
        let strategy = self
            .list_strategy_records()?
            .into_iter()
            .find(|strategy| strategy.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?;

        Ok(StrategyDetailResponse {
            strategy: self.map_strategy_summary(strategy.clone())?,
            positions: self.list_positions(&strategy.id)?,
            trades: self.list_trades(Some(&strategy.id), 50)?,
            broker_sync: self.strategy_broker_sync(&strategy.id)?,
        })
    }

    pub fn update_strategy(
        &self,
        strategy_id: &str,
        request: UpdateStrategyRequest,
    ) -> AppResult<StrategySummary> {
        let current = self
            .list_strategy_records()?
            .into_iter()
            .find(|strategy| strategy.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?;

        if let Some(cash) = request.starting_cash {
            if cash <= 0.0 {
                return Err(AppError::Validation(
                    "starting cash must be positive".to_string(),
                ));
            }
        }

        let execution_mode = request.execution_mode.unwrap_or(current.execution_mode);
        let options = normalize_strategy_options(
            request.asset_class_target,
            request.option_entry_style,
            request.option_structure_preset,
            request.option_spread_width,
            request.option_target_delta,
            request.option_dte_min,
            request.option_dte_max,
            request.option_max_spread_pct,
            request.option_limit_buffer_pct,
            Some(&current),
        )?;
        let name = request
            .name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(current.name.as_str());
        let credential_id = if request.clear_credential.unwrap_or(false) {
            None
        } else if let Some(value) = request.credential_id {
            (!value.trim().is_empty()).then_some(value)
        } else {
            current.credential_id.clone()
        };
        let tracked_symbols = request
            .tracked_symbols
            .map(|symbols| normalize_symbols(&symbols))
            .unwrap_or(current.tracked_symbols.clone());
        let starting_cash = request.starting_cash.unwrap_or(current.starting_cash);
        let enabled = request.enabled.unwrap_or(current.enabled);
        let run_interval_ms = request.run_interval_ms.unwrap_or(current.run_interval_ms);
        let now = now();

        self.conn.execute(
            "UPDATE strategies
             SET name = ?2,
                 enabled = ?3,
                 execution_mode = ?4,
                 asset_class_target = ?5,
                 option_entry_style = ?6,
                 option_structure_preset = ?7,
                 option_spread_width = ?8,
                 option_target_delta = ?9,
                 option_dte_min = ?10,
                 option_dte_max = ?11,
                 option_max_spread_pct = ?12,
                 option_limit_buffer_pct = ?13,
                 credential_id = ?14,
                 starting_cash = ?15,
                 tracked_symbols = ?16,
                 last_run_at = ?17,
                 run_interval_ms = ?18
             WHERE id = ?1",
            params![
                strategy_id,
                name,
                enabled as i64,
                execution_mode_to_str(execution_mode),
                asset_class_target_to_str(options.asset_class_target),
                option_entry_style_to_str(options.option_entry_style),
                option_structure_preset_to_str(options.option_structure_preset),
                options.option_spread_width,
                options.option_target_delta,
                options.option_dte_min,
                options.option_dte_max,
                options.option_max_spread_pct,
                options.option_limit_buffer_pct,
                credential_id,
                starting_cash,
                serde_json::to_string(&tracked_symbols)?,
                now,
                run_interval_ms,
            ],
        )?;

        if request.reset_portfolio.unwrap_or(false) {
            self.reset_strategy_portfolio(strategy_id, starting_cash)?;
        }

        self.strategy_detail(strategy_id)
            .map(|detail| detail.strategy)
    }

    fn reset_strategy_portfolio(&self, strategy_id: &str, starting_cash: f64) -> AppResult<()> {
        self.conn.execute(
            "UPDATE strategies
             SET cash_balance = ?2, equity = ?2, total_trades = 0, wins = 0, losses = 0, last_signal = 'Portfolio reset'
             WHERE id = ?1",
            params![strategy_id, starting_cash],
        )?;
        self.conn.execute(
            "DELETE FROM strategy_positions WHERE strategy_id = ?1",
            params![strategy_id],
        )?;
        self.conn.execute(
            "DELETE FROM trade_log WHERE strategy_id = ?1",
            params![strategy_id],
        )?;
        Ok(())
    }

    pub fn list_positions(&self, strategy_id: &str) -> AppResult<Vec<PositionSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT underlying_symbol, instrument_symbol, asset_type, quantity, average_price,
                    market_price, multiplier, option_structure_preset, option_type, expiration, strike,
                    stale_quote, legs_json
             FROM strategy_positions
             WHERE strategy_id = ?1
             ORDER BY instrument_symbol ASC",
        )?;

        let rows = stmt.query_map(params![strategy_id], |row| {
            let quantity: f64 = row.get(3)?;
            let average_price: f64 = row.get(4)?;
            let market_price: f64 = row.get(5)?;
            let multiplier: f64 = row.get(6)?;
            let option_structure_preset = row
                .get::<_, Option<String>>(7)?
                .as_deref()
                .map(option_structure_preset_from_str)
                .transpose()?;
            let legs = deserialize_position_legs(&row.get::<_, String>(12)?)?;
            Ok(PositionSummary {
                strategy_id: strategy_id.to_string(),
                underlying_symbol: row.get(0)?,
                instrument_symbol: row.get(1)?,
                asset_type: row.get(2)?,
                quantity,
                average_price,
                market_price,
                multiplier,
                option_structure_preset,
                option_type: row.get(8)?,
                expiration: row.get(9)?,
                strike: row.get(10)?,
                stale_quote: row.get::<_, i64>(11)? != 0,
                legs,
                market_value: quantity * market_price * multiplier,
                unrealized_pnl: (market_price - average_price) * quantity * multiplier,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn get_position_record(
        &self,
        strategy_id: &str,
        instrument_symbol: &str,
    ) -> AppResult<Option<PositionRecord>> {
        self.conn
            .query_row(
                "SELECT underlying_symbol, instrument_symbol, asset_type, quantity, average_price,
                        market_price, multiplier, option_structure_preset, option_type, expiration,
                        strike, stale_quote, legs_json
                 FROM strategy_positions WHERE strategy_id = ?1 AND instrument_symbol = ?2",
                params![strategy_id, instrument_symbol],
                |row| {
                    Ok(PositionRecord {
                        underlying_symbol: row.get(0)?,
                        instrument_symbol: row.get(1)?,
                        asset_type: row.get(2)?,
                        quantity: row.get(3)?,
                        average_price: row.get(4)?,
                        market_price: row.get(5)?,
                        multiplier: row.get(6)?,
                        option_structure_preset: row
                            .get::<_, Option<String>>(7)?
                            .as_deref()
                            .map(option_structure_preset_from_str)
                            .transpose()?,
                        option_type: row.get(8)?,
                        expiration: row.get(9)?,
                        strike: row.get(10)?,
                        stale_quote: row.get::<_, i64>(11)? != 0,
                        legs: deserialize_position_legs(&row.get::<_, String>(12)?)?,
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    pub fn get_position_for_underlying(
        &self,
        strategy_id: &str,
        underlying_symbol: &str,
        asset_class_target: AssetClassTarget,
    ) -> AppResult<Option<PositionRecord>> {
        let sql = match asset_class_target {
            AssetClassTarget::Equity => {
                "SELECT underlying_symbol, instrument_symbol, asset_type, quantity, average_price,
                        market_price, multiplier, option_structure_preset, option_type, expiration,
                        strike, stale_quote, legs_json
                 FROM strategy_positions
                 WHERE strategy_id = ?1 AND instrument_symbol = ?2
                 LIMIT 1"
            }
            AssetClassTarget::Options => {
                "SELECT underlying_symbol, instrument_symbol, asset_type, quantity, average_price,
                        market_price, multiplier, option_structure_preset, option_type, expiration,
                        strike, stale_quote, legs_json
                 FROM strategy_positions
                 WHERE strategy_id = ?1 AND underlying_symbol = ?2 AND asset_type IN ('option', 'option_spread')
                 ORDER BY expiration ASC, instrument_symbol ASC
                 LIMIT 1"
            }
        };

        self.conn
            .query_row(sql, params![strategy_id, underlying_symbol], |row| {
                Ok(PositionRecord {
                    underlying_symbol: row.get(0)?,
                    instrument_symbol: row.get(1)?,
                    asset_type: row.get(2)?,
                    quantity: row.get(3)?,
                    average_price: row.get(4)?,
                    market_price: row.get(5)?,
                    multiplier: row.get(6)?,
                    option_structure_preset: row
                        .get::<_, Option<String>>(7)?
                        .as_deref()
                        .map(option_structure_preset_from_str)
                        .transpose()?,
                    option_type: row.get(8)?,
                    expiration: row.get(9)?,
                    strike: row.get(10)?,
                    stale_quote: row.get::<_, i64>(11)? != 0,
                    legs: deserialize_position_legs(&row.get::<_, String>(12)?)?,
                })
            })
            .optional()
            .map_err(AppError::from)
    }

    pub fn list_trades(
        &self,
        strategy_id: Option<&str>,
        limit: usize,
    ) -> AppResult<Vec<TradeRecord>> {
        let sql = if strategy_id.is_some() {
            "SELECT id, strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, side,
                    quantity, price, multiplier, option_structure_preset, option_type, expiration, strike,
                    legs_json, provider, execution_mode, reason, realized_pnl, executed_at
             FROM trade_log WHERE strategy_id = ?1 ORDER BY executed_at DESC LIMIT ?2"
        } else {
            "SELECT id, strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, side,
                    quantity, price, multiplier, option_structure_preset, option_type, expiration, strike,
                    legs_json, provider, execution_mode, reason, realized_pnl, executed_at
             FROM trade_log ORDER BY executed_at DESC LIMIT ?1"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let rows = if let Some(id) = strategy_id {
            stmt.query_map(params![id, limit as i64], map_trade_record)?
        } else {
            stmt.query_map(params![limit as i64], map_trade_record)?
        };

        rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
    }

    pub fn store_market_snapshot(&self, quote: &Quote, raw_json: &Value) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO market_snapshots (
                id, symbol, provider, price, bid, ask, volume, vwap, day_high, day_low, captured_at, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                Uuid::new_v4().to_string(),
                quote.symbol,
                quote.provider.as_str(),
                quote.price,
                quote.bid,
                quote.ask,
                quote.volume,
                quote.vwap,
                quote.session_high,
                quote.session_low,
                quote.timestamp,
                serde_json::to_string(raw_json)?,
            ],
        )?;
        Ok(())
    }

    pub fn store_option_snapshots(
        &self,
        contracts: &[OptionContractSnapshot],
        raw_json: &Value,
    ) -> AppResult<()> {
        let captured_at = now();
        let raw_json_str = serde_json::to_string(raw_json)?;

        self.conn.execute("BEGIN TRANSACTION", ())?;

        for contract in contracts {
            let res = self.conn.execute(
                "INSERT INTO option_snapshots (
                    id, underlying_symbol, provider, contract_symbol, option_type, expiration, strike,
                    bid, ask, last, implied_volatility, open_interest, volume, in_the_money,
                    delta, gamma, theta, vega, moneyness, captured_at, raw_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
                params![
                    Uuid::new_v4().to_string(),
                    contract.underlying_symbol,
                    contract.provider.as_str(),
                    contract.contract_symbol,
                    contract.option_type,
                    contract.expiration,
                    contract.strike,
                    contract.bid,
                    contract.ask,
                    contract.last,
                    contract.implied_volatility,
                    contract.open_interest,
                    contract.volume,
                    contract.in_the_money.map(|value| value as i64),
                    contract.delta,
                    contract.gamma,
                    contract.theta,
                    contract.vega,
                    contract.moneyness,
                    captured_at,
                    raw_json_str,
                ],
            );
            if let Err(e) = res {
                let _ = self.conn.execute("ROLLBACK", ());
                return Err(e.into());
            }
        }
        self.conn.execute("COMMIT", ())?;
        Ok(())
    }

    pub fn store_broker_sync(
        &self,
        credential_id: &str,
        environment: CredentialEnvironment,
        account: &BrokerAccountSummary,
        positions: &[BrokerPositionSummary],
        orders: &[BrokerOrderSummary],
        raw_account: &Value,
        raw_positions: &Value,
        raw_orders: &Value,
    ) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO broker_accounts (
                credential_id, environment, account_id, account_number, status, currency,
                buying_power, cash, equity, portfolio_value, last_equity, long_market_value,
                short_market_value, pattern_day_trader, trading_blocked, transfers_blocked,
                account_blocked, synced_at, raw_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
             ON CONFLICT(credential_id) DO UPDATE SET
                environment = excluded.environment,
                account_id = excluded.account_id,
                account_number = excluded.account_number,
                status = excluded.status,
                currency = excluded.currency,
                buying_power = excluded.buying_power,
                cash = excluded.cash,
                equity = excluded.equity,
                portfolio_value = excluded.portfolio_value,
                last_equity = excluded.last_equity,
                long_market_value = excluded.long_market_value,
                short_market_value = excluded.short_market_value,
                pattern_day_trader = excluded.pattern_day_trader,
                trading_blocked = excluded.trading_blocked,
                transfers_blocked = excluded.transfers_blocked,
                account_blocked = excluded.account_blocked,
                synced_at = excluded.synced_at,
                raw_json = excluded.raw_json",
            params![
                credential_id,
                credential_environment_to_str(environment),
                account.account_id,
                account.account_number,
                account.status,
                account.currency,
                account.buying_power,
                account.cash,
                account.equity,
                account.portfolio_value,
                account.last_equity,
                account.long_market_value,
                account.short_market_value,
                account.pattern_day_trader as i64,
                account.trading_blocked as i64,
                account.transfers_blocked as i64,
                account.account_blocked as i64,
                account.synced_at,
                serde_json::to_string(raw_account)?,
            ],
        )?;

        self.conn.execute(
            "DELETE FROM broker_positions WHERE credential_id = ?1",
            params![credential_id],
        )?;
        self.conn.execute(
            "DELETE FROM broker_orders WHERE credential_id = ?1",
            params![credential_id],
        )?;

        let positions_json = serde_json::to_string(raw_positions)?;
        for position in positions {
            self.conn.execute(
                "INSERT INTO broker_positions (
                    credential_id, symbol, asset_class, side, quantity, avg_entry_price, market_value,
                    current_price, unrealized_pl, unrealized_plpc, synced_at, raw_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    credential_id,
                    position.symbol,
                    position.asset_class,
                    position.side,
                    position.quantity,
                    position.avg_entry_price,
                    position.market_value,
                    position.current_price,
                    position.unrealized_pl,
                    position.unrealized_plpc,
                    position.synced_at,
                    positions_json,
                ],
            )?;
        }

        let orders_json = serde_json::to_string(raw_orders)?;
        for order in orders {
            self.conn.execute(
                "INSERT INTO broker_orders (
                    credential_id, order_id, client_order_id, symbol, side, order_type, order_class,
                    status, quantity, filled_qty, filled_avg_price, time_in_force, submitted_at,
                    updated_at, synced_at, raw_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    credential_id,
                    order.order_id,
                    order.client_order_id,
                    order.symbol,
                    order.side,
                    order.order_type,
                    order.order_class,
                    order.status,
                    order.quantity,
                    order.filled_qty,
                    order.filled_avg_price,
                    order.time_in_force,
                    order.submitted_at,
                    order.updated_at,
                    order.synced_at,
                    orders_json,
                ],
            )?;
        }

        Ok(())
    }

    pub fn broker_sync_state(&self, credential_id: &str) -> AppResult<Option<BrokerSyncState>> {
        let account = self
            .conn
            .query_row(
                "SELECT environment, account_id, account_number, status, currency, buying_power, cash,
                        equity, portfolio_value, last_equity, long_market_value, short_market_value,
                        pattern_day_trader, trading_blocked, transfers_blocked, account_blocked,
                        synced_at
                 FROM broker_accounts
                 WHERE credential_id = ?1",
                params![credential_id],
                |row| {
                    Ok(BrokerAccountSummary {
                        credential_id: credential_id.to_string(),
                        environment: credential_environment_from_str(&row.get::<_, String>(0)?)?,
                        account_id: row.get(1)?,
                        account_number: row.get(2)?,
                        status: row.get(3)?,
                        currency: row.get(4)?,
                        buying_power: row.get(5)?,
                        cash: row.get(6)?,
                        equity: row.get(7)?,
                        portfolio_value: row.get(8)?,
                        last_equity: row.get(9)?,
                        long_market_value: row.get(10)?,
                        short_market_value: row.get(11)?,
                        pattern_day_trader: row.get::<_, i64>(12)? != 0,
                        trading_blocked: row.get::<_, i64>(13)? != 0,
                        transfers_blocked: row.get::<_, i64>(14)? != 0,
                        account_blocked: row.get::<_, i64>(15)? != 0,
                        synced_at: row.get(16)?,
                    })
                },
            )
            .optional()?;

        let Some(account) = account else {
            return Ok(None);
        };

        let mut positions_stmt = self.conn.prepare(
            "SELECT symbol, asset_class, side, quantity, avg_entry_price, market_value,
                    current_price, unrealized_pl, unrealized_plpc, synced_at
             FROM broker_positions
             WHERE credential_id = ?1
             ORDER BY symbol ASC",
        )?;
        let positions = positions_stmt
            .query_map(params![credential_id], |row| {
                Ok(BrokerPositionSummary {
                    credential_id: credential_id.to_string(),
                    symbol: row.get(0)?,
                    asset_class: row.get(1)?,
                    side: row.get(2)?,
                    quantity: row.get(3)?,
                    avg_entry_price: row.get(4)?,
                    market_value: row.get(5)?,
                    current_price: row.get(6)?,
                    unrealized_pl: row.get(7)?,
                    unrealized_plpc: row.get(8)?,
                    synced_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut orders_stmt = self.conn.prepare(
            "SELECT order_id, client_order_id, symbol, side, order_type, order_class, status, quantity,
                    filled_qty, filled_avg_price, time_in_force, submitted_at, updated_at, synced_at
             FROM broker_orders
             WHERE credential_id = ?1
             ORDER BY COALESCE(updated_at, submitted_at, synced_at) DESC",
        )?;
        let orders = orders_stmt
            .query_map(params![credential_id], |row| {
                Ok(BrokerOrderSummary {
                    credential_id: credential_id.to_string(),
                    order_id: row.get(0)?,
                    client_order_id: row.get(1)?,
                    symbol: row.get(2)?,
                    side: row.get(3)?,
                    order_type: row.get(4)?,
                    order_class: row.get(5)?,
                    status: row.get(6)?,
                    quantity: row.get(7)?,
                    filled_qty: row.get(8)?,
                    filled_avg_price: row.get(9)?,
                    time_in_force: row.get(10)?,
                    submitted_at: row.get(11)?,
                    updated_at: row.get(12)?,
                    synced_at: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(BrokerSyncState {
            credential_id: credential_id.to_string(),
            environment: account.environment,
            synced_at: account.synced_at.clone(),
            account: Some(account),
            positions,
            orders,
        }))
    }

    pub fn strategy_broker_sync(&self, strategy_id: &str) -> AppResult<Option<BrokerSyncState>> {
        let credential_id: Option<String> = self.conn.query_row(
            "SELECT credential_id FROM strategies WHERE id = ?1",
            params![strategy_id],
            |row| row.get(0),
        )?;

        let Some(credential_id) = credential_id else {
            return Ok(None);
        };

        self.broker_sync_state(&credential_id)
    }


    pub fn insert_watchlist(&self, id: &str, name: &str, symbols: &[String]) -> AppResult<()> {
        let normalized = normalize_symbols(symbols);
        let symbols_json = serde_json::to_string(&normalized)?;
        self.conn.execute(
            "INSERT INTO watchlists (id, name, symbols) VALUES (?1, ?2, ?3)",
            params![id, name, symbols_json],
        )?;
        Ok(())
    }

    pub fn list_watchlists(&self) -> AppResult<Vec<crate::models::Watchlist>> {
        let mut stmt = self.conn.prepare("SELECT id, name, symbols FROM watchlists ORDER BY name ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok(crate::models::Watchlist {
                id: row.get(0)?,
                name: row.get(1)?,
                symbols: parse_symbols(&row.get::<_, String>(2)?),
            })
        })?;

        let mut results = Vec::new();
        for record in rows {
            results.push(record?);
        }
        Ok(results)
    }

    pub fn update_watchlist(&self, id: &str, req: &crate::models::UpdateWatchlistRequest) -> AppResult<()> {
        let current: crate::models::Watchlist = self.conn.query_row(
            "SELECT id, name, symbols FROM watchlists WHERE id = ?1",
            params![id],
            |row| {
                Ok(crate::models::Watchlist {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    symbols: parse_symbols(&row.get::<_, String>(2)?),
                })
            },
        ).map_err(|_| AppError::NotFound(format!("watchlist {}", id)))?;

        let new_name = req.name.as_deref().unwrap_or(&current.name);
        let new_symbols = req.symbols.as_ref().unwrap_or(&current.symbols);
        let normalized = normalize_symbols(new_symbols);
        let symbols_json = serde_json::to_string(&normalized)?;

        self.conn.execute(
            "UPDATE watchlists SET name = ?2, symbols = ?3 WHERE id = ?1",
            params![id, new_name, symbols_json],
        )?;
        Ok(())
    }

    pub fn delete_watchlist(&self, id: &str) -> AppResult<()> {
        let deleted = self.conn.execute("DELETE FROM watchlists WHERE id = ?1", params![id])?;
        if deleted == 0 {
            return Err(AppError::NotFound(format!("watchlist {}", id)));
        }
        Ok(())
    }

    pub fn watchlist_symbols_union(&self) -> AppResult<Vec<String>> {
        let watchlists = self.list_watchlists()?;
        let mut all_symbols = std::collections::BTreeSet::new();
        for w in watchlists {
            for symbol in w.symbols {
                all_symbols.insert(symbol);
            }
        }
        Ok(all_symbols.into_iter().collect())
    }

    pub fn tracked_symbols_union(&self, _defaults: &[String]) -> AppResult<Vec<String>> {
        let mut all_symbols = BTreeSet::new();

        let mut stmt = self.conn.prepare("SELECT symbol FROM watchlist")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for symbol in rows {
            if let Ok(symbol) = symbol {
                all_symbols.insert(symbol.to_uppercase());
            }
        }

        for strategy in self.list_strategy_records()? {
            for symbol in strategy.tracked_symbols {
                all_symbols.insert(symbol.to_uppercase());
            }
        }

        Ok(all_symbols.into_iter().collect())
    }

    pub fn insert_watchlist_symbol(&self, symbol: &str) -> AppResult<()> {
        let normalized = symbol.trim().to_uppercase();
        if normalized.is_empty() {
            return Err(AppError::Validation("symbol is required".to_string()));
        }

        self.conn.execute(
            "INSERT OR IGNORE INTO watchlist (symbol, added_at) VALUES (?1, ?2)",
            params![normalized, now()],
        )?;
        Ok(())
    }

    pub fn delete_watchlist_symbol(&self, symbol: &str) -> AppResult<()> {
        let normalized = symbol.trim().to_uppercase();
        self.conn.execute(
            "DELETE FROM watchlist WHERE symbol = ?1",
            params![normalized],
        )?;
        Ok(())
    }

    pub fn mark_symbol_price(&self, symbol: &str, price: f64) -> AppResult<()> {
        self.conn.execute(
            "UPDATE strategy_positions
             SET market_price = ?2, stale_quote = 0
             WHERE instrument_symbol = ?1 AND asset_type = 'equity'",
            params![symbol, price],
        )?;

        self.conn.execute(
            "UPDATE strategies
             SET equity = cash_balance + (
                 SELECT COALESCE(SUM(quantity * market_price * multiplier), 0.0)
                 FROM strategy_positions
                 WHERE strategy_id = strategies.id
             )
             WHERE id IN (
                 SELECT DISTINCT strategy_id
                 FROM strategy_positions
                 WHERE instrument_symbol = ?1 AND asset_type = 'equity'
             )",
            params![symbol],
        )?;

        Ok(())
    }

    pub fn refresh_option_position_quotes(
        &self,
        strategy_id: &str,
        underlying_symbol: &str,
        contracts: &[OptionContractSnapshot],
    ) -> AppResult<()> {
        let mut stmt = self.conn.prepare(
            "SELECT instrument_symbol FROM strategy_positions
             WHERE strategy_id = ?1 AND underlying_symbol = ?2 AND asset_type IN ('option', 'option_spread')",
        )?;
        let position_symbols = stmt
            .query_map(params![strategy_id, underlying_symbol], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        if position_symbols.is_empty() {
            return Ok(());
        }

        for instrument_symbol in position_symbols {
            let Some(position) = self.get_position_record(strategy_id, &instrument_symbol)? else {
                continue;
            };
            let (market_price, stale_quote, legs) = if position.asset_type == "option_spread" {
                let mut total_mark = 0.0;
                let mut any_stale = false;
                let mut updated_legs = Vec::with_capacity(position.legs.len());
                for leg in position.legs {
                    let snapshot = contracts
                        .iter()
                        .find(|contract| contract.contract_symbol == leg.instrument_symbol);
                    let leg_mark = snapshot
                        .and_then(option_contract_mark_price)
                        .unwrap_or(leg.market_price);
                    let leg_stale = snapshot.and_then(option_contract_mark_price).is_none();
                    let sign = if leg.position_side == "short" { -1.0 } else { 1.0 };
                    total_mark += sign * leg_mark;
                    any_stale |= leg_stale;
                    updated_legs.push(PositionLeg {
                        market_price: leg_mark,
                        stale_quote: leg_stale,
                        ..leg
                    });
                }
                (total_mark.max(0.01), any_stale, updated_legs)
            } else {
                let snapshot = contracts
                    .iter()
                    .find(|contract| contract.contract_symbol == instrument_symbol);
                let mark_price = snapshot.and_then(option_contract_mark_price);
                let market_price = mark_price.unwrap_or(position.market_price);
                let stale_quote = mark_price.is_none();
                let legs = if position.legs.is_empty() {
                    vec![PositionLeg {
                        instrument_symbol: position.instrument_symbol.clone(),
                        position_side: "long".to_string(),
                        ratio_quantity: 1,
                        average_price: position.average_price,
                        market_price,
                        multiplier: position.multiplier,
                        option_type: position.option_type.clone(),
                        expiration: position.expiration.clone(),
                        strike: position.strike,
                        stale_quote,
                    }]
                } else {
                    position
                        .legs
                        .into_iter()
                        .map(|leg| PositionLeg { market_price, stale_quote, ..leg })
                        .collect()
                };
                (market_price, stale_quote, legs)
            };
            self.conn.execute(
                "UPDATE strategy_positions
                 SET market_price = ?3, stale_quote = ?4, legs_json = ?5
                 WHERE strategy_id = ?1 AND instrument_symbol = ?2",
                params![
                    strategy_id,
                    instrument_symbol,
                    market_price,
                    stale_quote as i64,
                    serde_json::to_string(&legs)?,
                ],
            )?;
        }

        self.recompute_strategy_equity(strategy_id)?;
        Ok(())
    }

    pub fn mark_strategy_run(&self, strategy_id: &str, signal: &str) -> AppResult<()> {
        self.conn.execute(
            "UPDATE strategies SET last_signal = ?2, last_run_at = ?3 WHERE id = ?1",
            params![strategy_id, signal, now()],
        )?;
        Ok(())
    }

    pub fn execute_local_trade(
        &self,
        strategy_id: &str,
        provider: DataProvider,
        execution_mode: ExecutionMode,
        signal: &StrategySignal,
        trade: &LocalTradeInput,
    ) -> AppResult<Option<TradeRecord>> {
        if matches!(signal.action, SignalAction::Hold) {
            self.mark_strategy_run(strategy_id, &signal.reason)?;
            return Ok(None);
        }

        let strategy = self
            .list_strategy_records()?
            .into_iter()
            .find(|strategy| strategy.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?;

        if trade.asset_type == "equity" {
            self.mark_symbol_price(&trade.instrument_symbol, trade.price)?;
        }

        let existing = self.get_position_record(strategy_id, &trade.instrument_symbol)?;
        let trade_id = Uuid::new_v4().to_string();
        let executed_at = now();
        let mut cash_balance = strategy.cash_balance;
        let mut realized_pnl = None;
        let mut wins = strategy.wins as i64;
        let mut losses = strategy.losses as i64;
        let side = trade.side;
        let quantity = trade.quantity;
        let price = trade.price;
        let multiplier = trade.multiplier;

        match signal.action {
            SignalAction::Buy => {
                if quantity <= 0.0 {
                    self.mark_strategy_run(
                        strategy_id,
                        "Buy signal skipped: insufficient buying power",
                    )?;
                    return Ok(None);
                }

                let position_cost = quantity * price * multiplier;
                if position_cost > cash_balance {
                    self.mark_strategy_run(strategy_id, "Buy signal skipped: insufficient cash")?;
                    return Ok(None);
                }

                cash_balance -= position_cost;
                let updated_position = if let Some(current) = existing {
                    let new_quantity = current.quantity + quantity;
                    let new_average = ((current.quantity * current.average_price)
                        + (quantity * price))
                        / new_quantity;
                    PositionRecord {
                        underlying_symbol: trade.underlying_symbol.clone(),
                        instrument_symbol: trade.instrument_symbol.clone(),
                        asset_type: trade.asset_type.clone(),
                        quantity: new_quantity,
                        average_price: new_average,
                        market_price: price,
                        multiplier,
                        option_structure_preset: trade.option_structure_preset,
                        option_type: trade.option_type.clone(),
                        expiration: trade.expiration.clone(),
                        strike: trade.strike,
                        stale_quote: false,
                        legs: position_legs_from_trade(trade, price, false),
                    }
                } else {
                    PositionRecord {
                        underlying_symbol: trade.underlying_symbol.clone(),
                        instrument_symbol: trade.instrument_symbol.clone(),
                        asset_type: trade.asset_type.clone(),
                        quantity,
                        average_price: price,
                        market_price: price,
                        multiplier,
                        option_structure_preset: trade.option_structure_preset,
                        option_type: trade.option_type.clone(),
                        expiration: trade.expiration.clone(),
                        strike: trade.strike,
                        stale_quote: false,
                        legs: position_legs_from_trade(trade, price, false),
                    }
                };

                self.upsert_position(strategy_id, &updated_position)?;
            }
            SignalAction::Sell => {
                let Some(current) = existing else {
                    self.mark_strategy_run(strategy_id, "Sell signal skipped: no open position")?;
                    return Ok(None);
                };

                if quantity <= 0.0 {
                    self.mark_strategy_run(strategy_id, "Sell signal skipped: zero quantity")?;
                    return Ok(None);
                }

                let proceeds = quantity * price * multiplier;
                cash_balance += proceeds;
                let pnl = (price - current.average_price) * quantity * multiplier;
                realized_pnl = Some(pnl);
                if pnl >= 0.0 {
                    wins += 1;
                } else {
                    losses += 1;
                }

                let remaining = round_position_quantity(current.quantity - quantity, &trade.asset_type);
                if remaining <= 0.0 {
                    self.delete_position(strategy_id, &trade.instrument_symbol)?;
                } else {
                    self.upsert_position(
                        strategy_id,
                        &PositionRecord {
                            underlying_symbol: current.underlying_symbol,
                            instrument_symbol: current.instrument_symbol,
                            asset_type: current.asset_type,
                            quantity: remaining,
                            average_price: current.average_price,
                            market_price: price,
                            multiplier: current.multiplier,
                            option_structure_preset: current.option_structure_preset,
                            option_type: current.option_type,
                            expiration: current.expiration,
                            strike: current.strike,
                            stale_quote: false,
                            legs: current.legs,
                        },
                    )?;
                }
            }
            SignalAction::Hold => unreachable!(),
        }

        self.conn.execute(
            "UPDATE strategies
             SET cash_balance = ?2,
                 total_trades = total_trades + 1,
                 wins = ?3,
                 losses = ?4,
                 last_signal = ?5,
                 last_run_at = ?6
             WHERE id = ?1",
            params![
                strategy_id,
                cash_balance,
                wins,
                losses,
                signal.reason,
                executed_at,
            ],
        )?;

        self.conn.execute(
            "INSERT INTO trade_log (
                id, strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, side,
                quantity, price, multiplier, option_structure_preset, option_type, expiration, strike,
                legs_json, provider, execution_mode, reason, realized_pnl, executed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                trade_id,
                strategy_id,
                trade.instrument_symbol,
                trade.underlying_symbol,
                trade.instrument_symbol,
                trade.asset_type,
                trade_side_to_str(side),
                quantity,
                price,
                multiplier,
                trade.option_structure_preset.map(option_structure_preset_to_str),
                trade.option_type,
                trade.expiration,
                trade.strike,
                serde_json::to_string(&trade.legs)?,
                provider.as_str(),
                execution_mode_to_str(execution_mode),
                signal.reason,
                realized_pnl,
                executed_at,
            ],
        )?;

        self.recompute_strategy_equity(strategy_id)?;

        Ok(Some(TradeRecord {
            id: trade_id,
            strategy_id: strategy_id.to_string(),
            underlying_symbol: trade.underlying_symbol.clone(),
            instrument_symbol: trade.instrument_symbol.clone(),
            asset_type: trade.asset_type.clone(),
            side,
            quantity,
            price,
            multiplier,
            option_structure_preset: trade.option_structure_preset,
            option_type: trade.option_type.clone(),
            expiration: trade.expiration.clone(),
            strike: trade.strike,
            legs: trade.legs.clone(),
            provider,
            reason: signal.reason.clone(),
            execution_mode,
            realized_pnl,
            executed_at,
        }))
    }

    /// Records a trade that was executed against an external broker, using the
    /// actual `fill_quantity` and `fill_price` returned by the broker. Unlike
    /// `execute_local_signal`, this does NOT re-derive the quantity from the
    /// signal's allocation fraction — the broker's fill is authoritative, so
    /// the local ledger reflects what actually happened on the exchange.
    #[allow(clippy::too_many_arguments)]
    pub fn record_broker_fill(
        &self,
        strategy_id: &str,
        symbol: &str,
        side: TradeSide,
        fill_quantity: f64,
        fill_price: f64,
        provider: DataProvider,
        execution_mode: ExecutionMode,
        reason: &str,
    ) -> AppResult<Option<TradeRecord>> {
        if fill_quantity <= 0.0 {
            // Broker reported a zero-qty fill — nothing to record.
            self.mark_strategy_run(strategy_id, reason)?;
            return Ok(None);
        }

        self.mark_symbol_price(symbol, fill_price)?;

        let strategy = self
            .list_strategy_records()?
            .into_iter()
            .find(|candidate| candidate.id == strategy_id)
            .ok_or_else(|| AppError::NotFound(format!("strategy {strategy_id}")))?;

        let existing = self.get_position_record(strategy_id, symbol)?;
        let trade_id = Uuid::new_v4().to_string();
        let executed_at = now();
        let mut cash_balance = strategy.cash_balance;
        let mut realized_pnl = None;
        let mut wins = strategy.wins as i64;
        let mut losses = strategy.losses as i64;

        match side {
            TradeSide::Buy => {
                let position_cost = fill_quantity * fill_price;
                cash_balance -= position_cost;
                let updated = if let Some(current) = existing {
                    let new_quantity = current.quantity + fill_quantity;
                    let new_average = ((current.quantity * current.average_price) + position_cost)
                        / new_quantity;
                    PositionRecord {
                        underlying_symbol: symbol.to_string(),
                        instrument_symbol: symbol.to_string(),
                        asset_type: strategy.asset_class_target.as_str().to_string(),
                        quantity: new_quantity,
                        average_price: new_average,
                        market_price: fill_price,
                        multiplier: 1.0,
                        option_structure_preset: None,
                        option_type: None,
                        expiration: None,
                        strike: None,
                        stale_quote: false,
                        legs: Vec::new(),
                    }
                } else {
                    PositionRecord {
                        underlying_symbol: symbol.to_string(),
                        instrument_symbol: symbol.to_string(),
                        asset_type: strategy.asset_class_target.as_str().to_string(),
                        quantity: fill_quantity,
                        average_price: fill_price,
                        market_price: fill_price,
                        multiplier: 1.0,
                        option_structure_preset: None,
                        option_type: None,
                        expiration: None,
                        strike: None,
                        stale_quote: false,
                        legs: Vec::new(),
                    }
                };
                self.upsert_position(strategy_id, &updated)?;
            }
            TradeSide::Sell => {
                let Some(current) = existing else {
                    self.mark_strategy_run(
                        strategy_id,
                        "Sell fill received with no open local position",
                    )?;
                    return Ok(None);
                };
                let proceeds = fill_quantity * fill_price;
                cash_balance += proceeds;
                let pnl = (fill_price - current.average_price) * fill_quantity;
                realized_pnl = Some(pnl);
                if pnl >= 0.0 {
                    wins += 1;
                } else {
                    losses += 1;
                }

                let remaining = round_quantity(current.quantity - fill_quantity);
                if remaining <= 0.0 {
                    self.delete_position(strategy_id, &current.instrument_symbol)?;
                } else {
                    self.upsert_position(
                        strategy_id,
                        &PositionRecord {
                            underlying_symbol: current.underlying_symbol,
                            instrument_symbol: current.instrument_symbol,
                            asset_type: current.asset_type,
                            quantity: remaining,
                            average_price: current.average_price,
                            market_price: fill_price,
                            multiplier: current.multiplier,
                            option_structure_preset: current.option_structure_preset,
                            option_type: current.option_type,
                            expiration: current.expiration,
                            strike: current.strike,
                            stale_quote: false,
                            legs: current.legs,
                        },
                    )?;
                }
            }
        }

        self.conn.execute(
            "UPDATE strategies
             SET cash_balance = ?2,
                 total_trades = total_trades + 1,
                 wins = ?3,
                 losses = ?4,
                 last_signal = ?5,
                 last_run_at = ?6
             WHERE id = ?1",
            params![strategy_id, cash_balance, wins, losses, reason, executed_at],
        )?;

        self.conn.execute(
            "INSERT INTO trade_log (
                id, strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type,
                side, quantity, price, multiplier, option_structure_preset, option_type,
                expiration, strike, legs_json, provider, execution_mode, reason, realized_pnl,
                executed_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                trade_id,
                strategy_id,
                symbol,
                symbol,
                symbol,
                "equity",
                trade_side_to_str(side),
                fill_quantity,
                fill_price,
                1.0,
                Option::<String>::None,
                Option::<String>::None,
                Option::<String>::None,
                Option::<f64>::None,
                "[]",
                provider.as_str(),
                execution_mode_to_str(execution_mode),
                reason,
                realized_pnl,
                executed_at,
            ],
        )?;

        self.recompute_strategy_equity(strategy_id)?;

        Ok(Some(TradeRecord {
            id: trade_id,
            strategy_id: strategy_id.to_string(),
            underlying_symbol: symbol.to_string(),
            instrument_symbol: symbol.to_string(),
            asset_type: "equity".to_string(),
            side,
            quantity: fill_quantity,
            price: fill_price,
            multiplier: 1.0,
            option_structure_preset: None,
            option_type: None,
            expiration: None,
            strike: None,
            legs: Vec::new(),
            provider,
            reason: reason.to_string(),
            execution_mode,
            realized_pnl,
            executed_at,
        }))
    }

    fn upsert_position(&self, strategy_id: &str, position: &PositionRecord) -> AppResult<()> {
        self.conn.execute(
            "INSERT INTO strategy_positions (
                strategy_id, symbol, underlying_symbol, instrument_symbol, asset_type, quantity,
                average_price, market_price, multiplier, option_structure_preset, option_type, expiration,
                strike, stale_quote, legs_json
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(strategy_id, symbol) DO UPDATE SET
                underlying_symbol = excluded.underlying_symbol,
                instrument_symbol = excluded.instrument_symbol,
                asset_type = excluded.asset_type,
                quantity = excluded.quantity,
                average_price = excluded.average_price,
                market_price = excluded.market_price,
                multiplier = excluded.multiplier,
                option_structure_preset = excluded.option_structure_preset,
                option_type = excluded.option_type,
                expiration = excluded.expiration,
                strike = excluded.strike,
                stale_quote = excluded.stale_quote,
                legs_json = excluded.legs_json",
            params![
                strategy_id,
                position.instrument_symbol,
                position.underlying_symbol,
                position.instrument_symbol,
                position.asset_type,
                position.quantity,
                position.average_price,
                position.market_price,
                position.multiplier,
                position.option_structure_preset.map(option_structure_preset_to_str),
                position.option_type,
                position.expiration,
                position.strike,
                position.stale_quote as i64,
                serde_json::to_string(&position.legs)?,
            ],
        )?;
        Ok(())
    }

    fn delete_position(&self, strategy_id: &str, instrument_symbol: &str) -> AppResult<()> {
        self.conn.execute(
            "DELETE FROM strategy_positions WHERE strategy_id = ?1 AND instrument_symbol = ?2",
            params![strategy_id, instrument_symbol],
        )?;
        Ok(())
    }

    fn recompute_strategy_equity(&self, strategy_id: &str) -> AppResult<()> {
        let cash_balance: f64 = self.conn.query_row(
            "SELECT cash_balance FROM strategies WHERE id = ?1",
            params![strategy_id],
            |row| row.get(0),
        )?;

        let market_value: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(quantity * market_price * multiplier), 0.0)
             FROM strategy_positions WHERE strategy_id = ?1",
            params![strategy_id],
            |row| row.get(0),
        )?;

        self.conn.execute(
            "UPDATE strategies SET equity = ?2 WHERE id = ?1",
            params![strategy_id, cash_balance + market_value],
        )?;

        Ok(())
    }

    fn map_strategy_summary(&self, record: StrategyRecord) -> AppResult<StrategySummary> {
        let open_positions: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM strategy_positions WHERE strategy_id = ?1",
            params![record.id],
            |row| row.get(0),
        )?;
        let completed_trades = record.wins + record.losses;
        let win_rate = if completed_trades > 0 {
            record.wins as f64 / completed_trades as f64
        } else {
            0.0
        };
        let broker_sync = match &record.credential_id {
            Some(credential_id) => self.broker_sync_state(credential_id)?,
            None => None,
        };
        let broker_equity = broker_sync
            .as_ref()
            .and_then(|sync| sync.account.as_ref())
            .and_then(|account| account.equity);
        let broker_buying_power = broker_sync
            .as_ref()
            .and_then(|sync| sync.account.as_ref())
            .and_then(|account| account.buying_power);
        let broker_open_positions = broker_sync.as_ref().map(|sync| sync.positions.len());
        let broker_open_orders = broker_sync.as_ref().map(|sync| sync.orders.len());
        let broker_synced_at = broker_sync.as_ref().map(|sync| sync.synced_at.clone());

        Ok(StrategySummary {
            id: record.id,
            name: record.name,
            kind: record.kind,
            enabled: record.enabled,
            execution_mode: record.execution_mode,
            asset_class_target: record.asset_class_target,
            option_entry_style: record.option_entry_style,
            option_structure_preset: record.option_structure_preset,
            option_spread_width: record.option_spread_width,
            option_target_delta: record.option_target_delta,
            option_dte_min: record.option_dte_min,
            option_dte_max: record.option_dte_max,
            option_max_spread_pct: record.option_max_spread_pct,
            option_limit_buffer_pct: record.option_limit_buffer_pct,
            credential_id: record.credential_id,
            starting_cash: record.starting_cash,
            cash_balance: record.cash_balance,
            equity: record.equity,
            tracked_symbols: record.tracked_symbols,
            open_positions: open_positions as usize,
            total_trades: record.total_trades,
            wins: record.wins,
            losses: record.losses,
            win_rate,
            pnl: record.equity - record.starting_cash,
            last_signal: record.last_signal,
            last_run_at: record.last_run_at,
            broker_synced_at,
            broker_equity,
            broker_buying_power,
            broker_open_positions,
            broker_open_orders,
            run_interval_ms: record.run_interval_ms,
        })
    }
}

fn decode_credential_row(
    row: &rusqlite::Row<'_>,
    cipher: &Aes256Gcm,
) -> Result<StoredCredential, rusqlite::Error> {
    let encrypted_key: String = row.get(3)?;
    let encrypted_secret: String = row.get(4)?;
    Ok(StoredCredential {
        id: row.get(0)?,
        environment: credential_environment_from_str(&row.get::<_, String>(2)?)?,
        key_id: decrypt(cipher, &encrypted_key).map_err(to_sql_err)?,
        secret_key: decrypt(cipher, &encrypted_secret).map_err(to_sql_err)?,
    })
}

fn map_trade_record(row: &rusqlite::Row<'_>) -> Result<TradeRecord, rusqlite::Error> {
    Ok(TradeRecord {
        id: row.get(0)?,
        strategy_id: row.get(1)?,
        underlying_symbol: row.get(3)?,
        instrument_symbol: row.get(4)?,
        asset_type: row.get(5)?,
        side: trade_side_from_str(&row.get::<_, String>(6)?)?,
        quantity: row.get(7)?,
        price: row.get(8)?,
        multiplier: row.get(9)?,
        option_structure_preset: row
            .get::<_, Option<String>>(10)?
            .as_deref()
            .map(option_structure_preset_from_str)
            .transpose()?,
        option_type: row.get(11)?,
        expiration: row.get(12)?,
        strike: row.get(13)?,
        legs: deserialize_trade_legs(&row.get::<_, String>(14)?)?,
        provider: provider_from_str(&row.get::<_, String>(15)?)?,
        execution_mode: execution_mode_from_str(&row.get::<_, String>(16)?)?,
        reason: row.get(17)?,
        realized_pnl: row.get(18)?,
        executed_at: row.get(19)?,
    })
}

fn parse_symbols(json_text: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(json_text)
        .unwrap_or_default()
        .into_iter()
        .map(|symbol| symbol.to_uppercase())
        .collect()
}

fn normalize_symbols(symbols: &[String]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for symbol in symbols {
        let normalized = symbol.trim().to_uppercase();
        if !normalized.is_empty() {
            set.insert(normalized);
        }
    }
    set.into_iter().collect()
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in text.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            last_was_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if !last_was_dash {
            last_was_dash = true;
            Some('-')
        } else {
            None
        };

        if let Some(value) = normalized {
            slug.push(value);
        }
    }

    slug.trim_matches('-').to_string()
}

fn mask_raw_key(raw: &str) -> String {
    if raw.len() <= 8 {
        return "********".to_string();
    }
    format!("{}…{}", &raw[..4], &raw[raw.len() - 4..])
}

fn mask_encrypted_key(encrypted: &str) -> String {
    let prefix = encrypted.chars().take(4).collect::<String>();
    let suffix = encrypted
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("{prefix}…{suffix}")
}

fn encrypt(cipher: &Aes256Gcm, plaintext: &str) -> AppResult<String> {
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|err| AppError::Internal(err.to_string()))?;
    let mut combined = nonce.to_vec();
    combined.extend(ciphertext);
    Ok(STANDARD.encode(combined))
}

fn decrypt(cipher: &Aes256Gcm, encoded: &str) -> AppResult<String> {
    let combined = STANDARD
        .decode(encoded)
        .map_err(|err| AppError::Internal(err.to_string()))?;
    if combined.len() < 12 {
        return Err(AppError::Internal(
            "credential payload too short".to_string(),
        ));
    }
    let (nonce_bytes, cipher_bytes) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, cipher_bytes).map_err(|_| {
        AppError::Unauthorized(
            "failed to decrypt credential; the master key may have changed".to_string(),
        )
    })?;
    String::from_utf8(plaintext).map_err(|err| AppError::Internal(err.to_string()))
}

/// Loads the per-install credential encryption salt from `app_config`, generating
/// and persisting a fresh 16-byte salt on first use. Storing a salt alongside the
/// ciphertext means rotating the master key only requires rebuilding the cached
/// cipher — old ciphertexts remain decryptable as long as the same salt is used.
fn load_or_create_credential_salt(conn: &Connection) -> AppResult<Vec<u8>> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT value FROM app_config WHERE key = 'credential_salt'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(encoded) = existing {
        let bytes = STANDARD
            .decode(encoded.trim())
            .map_err(|err| AppError::Internal(format!("decode credential salt: {err}")))?;
        if bytes.len() < 16 {
            return Err(AppError::Internal(
                "stored credential salt is too short".to_string(),
            ));
        }
        return Ok(bytes);
    }

    let mut salt = vec![0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    let encoded = STANDARD.encode(&salt);
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES ('credential_salt', ?1)",
        params![encoded],
    )?;
    Ok(salt)
}

/// Derives a 32-byte AES-256-GCM key from the master key using Argon2id with
/// the per-install salt. Argon2id is memory-hard and slow by design, so this
/// cipher is cached on the `Database` for the lifetime of the process.
fn derive_credential_cipher(master_key: &str, salt: &[u8]) -> AppResult<Aes256Gcm> {
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::default());
    let mut key = [0u8; 32];
    argon
        .hash_password_into(master_key.as_bytes(), salt, &mut key)
        .map_err(|err| AppError::Internal(format!("argon2 kdf: {err}")))?;
    Ok(Aes256Gcm::new_from_slice(&key).expect("argon2 output is exactly 32 bytes"))
}

fn to_sql_err(error: AppError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn round_quantity(value: f64) -> f64 {
    (value * 1000.0).floor() / 1000.0
}

fn round_position_quantity(value: f64, asset_type: &str) -> f64 {
    if asset_type == "option" || asset_type == "option_spread" {
        value.floor().max(0.0)
    } else {
        round_quantity(value)
    }
}

fn option_contract_mark_price(contract: &OptionContractSnapshot) -> Option<f64> {
    contract
        .last
        .or_else(|| match (contract.bid, contract.ask) {
            (Some(bid), Some(ask)) if bid > 0.0 && ask > 0.0 => Some((bid + ask) / 2.0),
            (Some(bid), None) if bid > 0.0 => Some(bid),
            (None, Some(ask)) if ask > 0.0 => Some(ask),
            _ => None,
        })
}

#[derive(Debug, Clone, Copy)]
struct NormalizedStrategyOptions {
    asset_class_target: AssetClassTarget,
    option_entry_style: OptionEntryStyle,
    option_structure_preset: OptionStructurePreset,
    option_spread_width: f64,
    option_target_delta: f64,
    option_dte_min: u32,
    option_dte_max: u32,
    option_max_spread_pct: f64,
    option_limit_buffer_pct: f64,
}

fn normalize_strategy_options(
    asset_class_target: Option<AssetClassTarget>,
    option_entry_style: Option<OptionEntryStyle>,
    option_structure_preset: Option<OptionStructurePreset>,
    option_spread_width: Option<f64>,
    option_target_delta: Option<f64>,
    option_dte_min: Option<u32>,
    option_dte_max: Option<u32>,
    option_max_spread_pct: Option<f64>,
    option_limit_buffer_pct: Option<f64>,
    current: Option<&StrategyRecord>,
) -> AppResult<NormalizedStrategyOptions> {
    let defaults = current.map(|strategy| NormalizedStrategyOptions {
        asset_class_target: strategy.asset_class_target,
        option_entry_style: strategy.option_entry_style,
        option_structure_preset: strategy.option_structure_preset,
        option_spread_width: strategy.option_spread_width,
        option_target_delta: strategy.option_target_delta,
        option_dte_min: strategy.option_dte_min,
        option_dte_max: strategy.option_dte_max,
        option_max_spread_pct: strategy.option_max_spread_pct,
        option_limit_buffer_pct: strategy.option_limit_buffer_pct,
    });

    let options = NormalizedStrategyOptions {
        asset_class_target: asset_class_target
            .or_else(|| defaults.map(|value| value.asset_class_target))
            .unwrap_or_default(),
        option_entry_style: option_entry_style
            .or_else(|| defaults.map(|value| value.option_entry_style))
            .unwrap_or_default(),
        option_structure_preset: option_structure_preset
            .or_else(|| defaults.map(|value| value.option_structure_preset))
            .unwrap_or_default(),
        option_spread_width: option_spread_width
            .or_else(|| defaults.map(|value| value.option_spread_width))
            .unwrap_or(5.0),
        option_target_delta: option_target_delta
            .or_else(|| defaults.map(|value| value.option_target_delta))
            .unwrap_or(0.30),
        option_dte_min: option_dte_min
            .or_else(|| defaults.map(|value| value.option_dte_min))
            .unwrap_or(21),
        option_dte_max: option_dte_max
            .or_else(|| defaults.map(|value| value.option_dte_max))
            .unwrap_or(45),
        option_max_spread_pct: option_max_spread_pct
            .or_else(|| defaults.map(|value| value.option_max_spread_pct))
            .unwrap_or(0.12),
        option_limit_buffer_pct: option_limit_buffer_pct
            .or_else(|| defaults.map(|value| value.option_limit_buffer_pct))
            .unwrap_or(0.05),
    };

    if !(0.0..=1.0).contains(&options.option_target_delta) {
        return Err(AppError::Validation(
            "option target delta must be between 0 and 1".to_string(),
        ));
    }
    if options.option_dte_min == 0 {
        return Err(AppError::Validation(
            "option minimum DTE must be at least 1".to_string(),
        ));
    }
    if options.option_dte_max < options.option_dte_min {
        return Err(AppError::Validation(
            "option maximum DTE must be greater than or equal to minimum DTE".to_string(),
        ));
    }
    if !(0.0..=1.0).contains(&options.option_max_spread_pct) {
        return Err(AppError::Validation(
            "option max spread percent must be between 0 and 1".to_string(),
        ));
    }
    if !(0.0..=1.0).contains(&options.option_limit_buffer_pct) {
        return Err(AppError::Validation(
            "option limit buffer percent must be between 0 and 1".to_string(),
        ));
    }
    if options.option_spread_width <= 0.0 {
        return Err(AppError::Validation(
            "option spread width must be positive".to_string(),
        ));
    }

    Ok(options)
}

fn deserialize_position_legs(json_text: &str) -> Result<Vec<PositionLeg>, rusqlite::Error> {
    serde_json::from_str(json_text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!("invalid position legs json: {err}"))),
        )
    })
}

fn deserialize_trade_legs(json_text: &str) -> Result<Vec<TradeLeg>, rusqlite::Error> {
    serde_json::from_str(json_text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!("invalid trade legs json: {err}"))),
        )
    })
}

fn position_legs_from_trade(
    trade: &LocalTradeInput,
    market_price: f64,
    stale_quote: bool,
) -> Vec<PositionLeg> {
    if !trade.legs.is_empty() {
        return trade
            .legs
            .iter()
            .map(|leg| PositionLeg {
                instrument_symbol: leg.instrument_symbol.clone(),
                position_side: match leg.position_intent.as_deref() {
                    Some("sell_to_open") | Some("buy_to_close") => "short".to_string(),
                    _ => "long".to_string(),
                },
                ratio_quantity: leg.ratio_quantity,
                average_price: leg.price,
                market_price: leg.price,
                multiplier: leg.multiplier,
                option_type: leg.option_type.clone(),
                expiration: leg.expiration.clone(),
                strike: leg.strike,
                stale_quote,
            })
            .collect();
    }

    if trade.asset_type == "option" {
        vec![PositionLeg {
            instrument_symbol: trade.instrument_symbol.clone(),
            position_side: "long".to_string(),
            ratio_quantity: 1,
            average_price: trade.price,
            market_price,
            multiplier: trade.multiplier,
            option_type: trade.option_type.clone(),
            expiration: trade.expiration.clone(),
            strike: trade.strike,
            stale_quote,
        }]
    } else {
        Vec::new()
    }
}

fn provider_from_str(value: &str) -> Result<DataProvider, rusqlite::Error> {
    match value {
        "yahoo" => Ok(DataProvider::Yahoo),
        "alpaca" => Ok(DataProvider::Alpaca),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!("unknown provider {other}"))),
        )),
    }
}

fn credential_environment_from_str(value: &str) -> Result<CredentialEnvironment, rusqlite::Error> {
    match value {
        "paper" => Ok(CredentialEnvironment::Paper),
        "live" => Ok(CredentialEnvironment::Live),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!("unknown environment {other}"))),
        )),
    }
}

fn execution_mode_from_str(value: &str) -> Result<ExecutionMode, rusqlite::Error> {
    match value {
        "local_paper" => Ok(ExecutionMode::LocalPaper),
        "alpaca_paper" => Ok(ExecutionMode::AlpacaPaper),
        "alpaca_live" => Ok(ExecutionMode::AlpacaLive),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!(
                "unknown execution mode {other}"
            ))),
        )),
    }
}

fn asset_class_target_from_str(value: &str) -> Result<AssetClassTarget, rusqlite::Error> {
    match value {
        "equity" => Ok(AssetClassTarget::Equity),
        "options" => Ok(AssetClassTarget::Options),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!(
                "unknown asset class target {other}"
            ))),
        )),
    }
}

fn option_entry_style_from_str(value: &str) -> Result<OptionEntryStyle, rusqlite::Error> {
    match value {
        "long_call" => Ok(OptionEntryStyle::LongCall),
        "long_put" => Ok(OptionEntryStyle::LongPut),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!(
                "unknown option entry style {other}"
            ))),
        )),
    }
}

fn option_structure_preset_from_str(
    value: &str,
) -> Result<OptionStructurePreset, rusqlite::Error> {
    match value {
        "single" => Ok(OptionStructurePreset::Single),
        "bull_call_spread" => Ok(OptionStructurePreset::BullCallSpread),
        "bear_put_spread" => Ok(OptionStructurePreset::BearPutSpread),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!(
                "unknown option structure preset {other}"
            ))),
        )),
    }
}

fn strategy_kind_from_str(value: &str) -> Result<StrategyKind, rusqlite::Error> {
    match value {
        "vwap_reflexive" => Ok(StrategyKind::VwapReflexive),
        "rsi_mean_reversion" => Ok(StrategyKind::RsiMeanReversion),
        "sma_trend" => Ok(StrategyKind::SmaTrend),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!("unknown strategy kind {other}"))),
        )),
    }
}

fn trade_side_from_str(value: &str) -> Result<TradeSide, rusqlite::Error> {
    match value {
        "buy" => Ok(TradeSide::Buy),
        "sell" => Ok(TradeSide::Sell),
        other => Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::Internal(format!("unknown trade side {other}"))),
        )),
    }
}

fn credential_environment_to_str(value: CredentialEnvironment) -> &'static str {
    match value {
        CredentialEnvironment::Paper => "paper",
        CredentialEnvironment::Live => "live",
    }
}

fn execution_mode_to_str(value: ExecutionMode) -> &'static str {
    match value {
        ExecutionMode::LocalPaper => "local_paper",
        ExecutionMode::AlpacaPaper => "alpaca_paper",
        ExecutionMode::AlpacaLive => "alpaca_live",
    }
}

fn asset_class_target_to_str(value: AssetClassTarget) -> &'static str {
    match value {
        AssetClassTarget::Equity => "equity",
        AssetClassTarget::Options => "options",
    }
}

fn option_entry_style_to_str(value: OptionEntryStyle) -> &'static str {
    match value {
        OptionEntryStyle::LongCall => "long_call",
        OptionEntryStyle::LongPut => "long_put",
    }
}

fn option_structure_preset_to_str(value: OptionStructurePreset) -> &'static str {
    match value {
        OptionStructurePreset::Single => "single",
        OptionStructurePreset::BullCallSpread => "bull_call_spread",
        OptionStructurePreset::BearPutSpread => "bear_put_spread",
    }
}

fn trade_side_to_str(value: TradeSide) -> &'static str {
    match value {
        TradeSide::Buy => "buy",
        TradeSide::Sell => "sell",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_raw_key() {
        // Length <= 8
        assert_eq!(mask_raw_key(""), "********");
        assert_eq!(mask_raw_key("123"), "********");
        assert_eq!(mask_raw_key("12345678"), "********");

        // Length > 8
        assert_eq!(mask_raw_key("123456789"), "1234…6789");
        assert_eq!(mask_raw_key("PK1234567890ABCDEFGH"), "PK12…EFGH");
        assert_eq!(mask_raw_key("abcdefghijklmnopqrstuvwxyz"), "abcd…wxyz");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("hello-world"), "hello-world");
        assert_eq!(slugify("  hello   world  "), "hello-world");
        assert_eq!(slugify("hello!@#$%^&*()world"), "hello-world");
        assert_eq!(slugify("123 456"), "123-456");
        assert_eq!(slugify("---hello---"), "hello");
        assert_eq!(slugify(""), "");
        assert_eq!(slugify("!@#$"), "");
        assert_eq!(
            slugify("Complex Title: With Symbols & More!"),
            "complex-title-with-symbols-more"
        );
        assert_eq!(slugify("unicode-test-🚀-sparkles"), "unicode-test-sparkles");
    }

    #[test]
    fn test_normalize_symbols() {
        // Happy path: capitalize and sort
        assert_eq!(
            normalize_symbols(&["aapl".to_string(), "tsla".to_string(), "msft".to_string()]),
            vec!["AAPL".to_string(), "MSFT".to_string(), "TSLA".to_string()]
        );

        // Deduplication
        assert_eq!(
            normalize_symbols(&["aapl".to_string(), "AAPL".to_string(), "AaPl".to_string()]),
            vec!["AAPL".to_string()]
        );

        // Trimming whitespace
        assert_eq!(
            normalize_symbols(&["  aapl  ".to_string(), "\ttsla\n".to_string()]),
            vec!["AAPL".to_string(), "TSLA".to_string()]
        );

        // Skipping empty strings
        assert_eq!(
            normalize_symbols(&["".to_string(), "   ".to_string(), "aapl".to_string()]),
            vec!["AAPL".to_string()]
        );

        // Empty input
        let empty: Vec<String> = vec![];
        assert_eq!(
            normalize_symbols(&empty),
            empty
        );
    }
}
