use serde::{Deserialize, Serialize};
pub mod telemetry;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_path: PathBuf,
    pub default_watchlist: Vec<String>,
    pub polling_seconds: u64,
    pub allowed_origins: Vec<String>,
    pub mock_alpaca: bool,
}

pub fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_uppercase()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum DataProvider {
    #[default]
    Yahoo,
    Alpaca,
}

impl DataProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yahoo => "yahoo",
            Self::Alpaca => "alpaca",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CredentialEnvironment {
    Paper,
    Live,
}

impl CredentialEnvironment {
    pub fn base_trading_url(self) -> &'static str {
        match self {
            Self::Paper => "https://paper-api.alpaca.markets",
            Self::Live => "https://api.alpaca.markets",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    LocalPaper,
    AlpacaPaper,
    AlpacaLive,
}

impl ExecutionMode {
    pub fn requires_external_broker(self) -> bool {
        !matches!(self, Self::LocalPaper)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AssetClassTarget {
    #[default]
    Equity,
    Options,
}

impl AssetClassTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equity => "equity",
            Self::Options => "options",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OptionEntryStyle {
    #[default]
    LongCall,
    LongPut,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OptionStructurePreset {
    #[default]
    Single,
    BullCallSpread,
    BearPutSpread,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StrategyKind {
    VwapReflexive,
    RsiMeanReversion,
    SmaTrend,
    ListingArbitrage,
    PutCallParity,
    ParitySniper,
    VwapReversion,
}

impl StrategyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VwapReflexive => "vwap_reflexive",
            Self::RsiMeanReversion => "rsi_mean_reversion",
            Self::SmaTrend => "sma_trend",
            Self::ListingArbitrage => "listing_arbitrage",
            Self::PutCallParity => "put_call_parity",
            Self::ParitySniper => "parity_sniper",
            Self::VwapReversion => "vwap_reversion",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: String,
    pub provider: DataProvider,
    pub price: f64,
    pub previous_close: Option<f64>,
    pub change: Option<f64>,
    pub change_percent: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub volume: Option<f64>,
    pub vwap: Option<f64>,
    pub session_high: Option<f64>,
    pub session_low: Option<f64>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub vwap: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionContractSnapshot {
    pub contract_symbol: String,
    pub underlying_symbol: String,
    pub provider: DataProvider,
    pub option_type: String,
    pub expiration: String,
    pub strike: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub last: Option<f64>,
    pub implied_volatility: Option<f64>,
    pub open_interest: Option<f64>,
    pub volume: Option<f64>,
    pub in_the_money: Option<bool>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub theta: Option<f64>,
    pub vega: Option<f64>,
    pub moneyness: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSummary {
    pub id: String,
    pub label: String,
    pub provider: DataProvider,
    pub environment: CredentialEnvironment,
    pub use_for_data: bool,
    pub use_for_trading: bool,
    pub masked_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct StoredCredential {
    pub id: String,
    pub environment: CredentialEnvironment,
    pub key_id: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerAccountSummary {
    pub credential_id: String,
    pub environment: CredentialEnvironment,
    pub account_id: String,
    pub account_number: Option<String>,
    pub status: Option<String>,
    pub currency: Option<String>,
    pub buying_power: Option<f64>,
    pub cash: Option<f64>,
    pub equity: Option<f64>,
    pub portfolio_value: Option<f64>,
    pub last_equity: Option<f64>,
    pub long_market_value: Option<f64>,
    pub short_market_value: Option<f64>,
    pub pattern_day_trader: bool,
    pub trading_blocked: bool,
    pub transfers_blocked: bool,
    pub account_blocked: bool,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerPositionSummary {
    pub credential_id: String,
    pub symbol: String,
    pub asset_class: Option<String>,
    pub side: Option<String>,
    pub quantity: f64,
    pub avg_entry_price: Option<f64>,
    pub market_value: Option<f64>,
    pub current_price: Option<f64>,
    pub unrealized_pl: Option<f64>,
    pub unrealized_plpc: Option<f64>,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerOrderSummary {
    pub credential_id: String,
    pub order_id: String,
    pub client_order_id: Option<String>,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub order_type: Option<String>,
    pub order_class: Option<String>,
    pub status: Option<String>,
    pub quantity: Option<f64>,
    pub filled_qty: Option<f64>,
    pub filled_avg_price: Option<f64>,
    pub time_in_force: Option<String>,
    pub submitted_at: Option<String>,
    pub updated_at: Option<String>,
    pub synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerSyncState {
    pub credential_id: String,
    pub environment: CredentialEnvironment,
    pub synced_at: String,
    pub account: Option<BrokerAccountSummary>,
    pub positions: Vec<BrokerPositionSummary>,
    pub orders: Vec<BrokerOrderSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySummary {
    pub id: String,
    pub name: String,
    pub kind: StrategyKind,
    pub enabled: bool,
    pub execution_mode: ExecutionMode,
    pub asset_class_target: AssetClassTarget,
    pub option_entry_style: OptionEntryStyle,
    pub option_structure_preset: OptionStructurePreset,
    pub option_spread_width: f64,
    pub option_target_delta: f64,
    pub option_dte_min: u32,
    pub option_dte_max: u32,
    pub option_max_spread_pct: f64,
    pub option_limit_buffer_pct: f64,
    pub credential_id: Option<String>,
    pub starting_cash: f64,
    pub cash_balance: f64,
    pub equity: f64,
    pub tracked_symbols: Vec<String>,
    pub open_positions: usize,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub pnl: f64,
    pub last_signal: Option<String>,
    pub last_run_at: Option<String>,
    pub broker_synced_at: Option<String>,
    pub broker_equity: Option<f64>,
    pub broker_buying_power: Option<f64>,
    pub broker_open_positions: Option<usize>,
    pub broker_open_orders: Option<usize>,
    pub run_interval_ms: u64,
    pub state_json: serde_json::Value,
    pub risk_parameters: Option<RiskParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionLeg {
    pub instrument_symbol: String,
    pub position_side: String,
    pub ratio_quantity: u32,
    pub average_price: f64,
    pub market_price: f64,
    pub multiplier: f64,
    pub option_type: Option<String>,
    pub expiration: Option<String>,
    pub strike: Option<f64>,
    pub stale_quote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSummary {
    pub strategy_id: String,
    pub underlying_symbol: String,
    pub instrument_symbol: String,
    pub asset_type: String,
    pub quantity: f64,
    pub average_price: f64,
    pub market_price: f64,
    pub multiplier: f64,
    pub option_structure_preset: Option<OptionStructurePreset>,
    pub option_type: Option<String>,
    pub expiration: Option<String>,
    pub strike: Option<f64>,
    pub stale_quote: bool,
    pub legs: Vec<PositionLeg>,
    pub market_value: f64,
    pub unrealized_pnl: f64,
    pub razor_stop: Option<f64>,
    pub stagnation_timestamp: Option<String>,
    pub kronos_sentiment: Option<f64>,
    pub take_profit: Option<f64>,
    pub exit_logic: Option<String>,
    pub entry_time: Option<String>,
    pub buy_logic: Option<String>,
    pub entry_math: Option<String>,
    pub entry_ai: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLeg {
    pub instrument_symbol: String,
    pub side: TradeSide,
    pub ratio_quantity: u32,
    pub position_intent: Option<String>,
    pub price: f64,
    pub multiplier: f64,
    pub option_type: Option<String>,
    pub expiration: Option<String>,
    pub strike: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: String,
    pub strategy_id: String,
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
    pub provider: DataProvider,
    pub reason: String,
    pub execution_mode: ExecutionMode,
    pub realized_pnl: Option<f64>,
    pub executed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDetailResponse {
    pub strategy: StrategySummary,
    pub positions: Vec<PositionSummary>,
    pub trades: Vec<TradeRecord>,
    pub broker_sync: Option<BrokerSyncState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub symbol: String,
    pub provider: DataProvider,
    pub collector_interval_seconds: u64,
    pub quote: Quote,
    pub candles: Vec<Candle>,
    pub options: Vec<OptionContractSnapshot>,
    pub strategies: Vec<StrategySummary>,
    pub recent_trades: Vec<TradeRecord>,
    pub credentials: Vec<CredentialSummary>,
    pub tracked_symbols: Vec<String>,
    pub watchlists: Vec<Watchlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCredentialRequest {
    pub label: String,
    pub api_key: String,
    pub api_secret: String,
    pub environment: CredentialEnvironment,
    pub use_for_data: bool,
    pub use_for_trading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistAddRequest {
    pub symbol: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParameters {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    #[serde(default)]
    pub blacklisted_symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStrategyRequest {
    pub name: String,
    pub kind: StrategyKind,
    pub execution_mode: Option<ExecutionMode>,
    pub asset_class_target: Option<AssetClassTarget>,
    pub option_entry_style: Option<OptionEntryStyle>,
    pub option_structure_preset: Option<OptionStructurePreset>,
    pub option_spread_width: Option<f64>,
    pub option_target_delta: Option<f64>,
    pub option_dte_min: Option<u32>,
    pub option_dte_max: Option<u32>,
    pub option_max_spread_pct: Option<f64>,
    pub option_limit_buffer_pct: Option<f64>,
    pub starting_cash: Option<f64>,
    pub tracked_symbols: Vec<String>,
    pub credential_id: Option<String>,
    pub enabled: Option<bool>,
    pub live_confirmation: Option<String>,
    pub run_interval_ms: Option<u64>,
    pub risk_parameters: Option<RiskParameters>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateStrategyRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub execution_mode: Option<ExecutionMode>,
    pub asset_class_target: Option<AssetClassTarget>,
    pub option_entry_style: Option<OptionEntryStyle>,
    pub option_structure_preset: Option<OptionStructurePreset>,
    pub option_spread_width: Option<f64>,
    pub option_target_delta: Option<f64>,
    pub option_dte_min: Option<u32>,
    pub option_dte_max: Option<u32>,
    pub option_max_spread_pct: Option<f64>,
    pub option_limit_buffer_pct: Option<f64>,
    pub starting_cash: Option<f64>,
    pub tracked_symbols: Option<Vec<String>>,
    pub credential_id: Option<String>,
    pub clear_credential: Option<bool>,
    pub reset_portfolio: Option<bool>,
    pub live_confirmation: Option<String>,
    pub run_interval_ms: Option<u64>,
    pub risk_parameters: Option<RiskParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectResponse {
    pub symbols_collected: usize,
    pub strategies_evaluated: usize,
    pub trades_executed: usize,
    pub collected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeEvent {
    Market {
        provider: DataProvider,
        symbol: String,
        quote: Quote,
        candle: Option<Candle>,
        strategies: Vec<StrategySummary>,
    },
    BrokerSync {
        credential_id: String,
        strategy_ids: Vec<String>,
        broker_sync: BrokerSyncState,
        strategies: Vec<StrategySummary>,
        event: Option<String>,
    },
    Positions {
        positions: Vec<PositionSummary>,
    },
    Status {
        channel: String,
        provider: Option<DataProvider>,
        symbol: Option<String>,
        state: String,
        message: String,
    },
    Log {
        strategy_id: String,
        symbol: String,
        source: String, // PARITY_SNIPER, VWAP_REVERSION, SYSTEM
        math_edge: String,
        ai_score: String,
        decision: String,
        narrative: String,
        time: String,
    },
    Notification {
        strategy_id: Option<String>,
        level: String, // info, warning, error
        title: String,
        message: String,
    },
    Heartbeat {
        timestamp: u64,
        buying_power: f64,
        kronos_active: bool,
        alpaca_active: bool,
        options_active: bool,
    },
    System {
        event: telemetry::SystemEvent,
    },
    SystemLog {
        event: crate::logger::SystemEvent,
    },
}

impl RealtimeEvent {
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::Market { .. } => "market",
            Self::BrokerSync { .. } => "broker_sync",
            Self::Positions { .. } => "positions",
            Self::Status { .. } => "status",
            Self::Log { .. } => "log",
            Self::Notification { .. } => "notification",
            Self::Heartbeat { .. } => "heartbeat",
            Self::System { .. } => "system",
            Self::SystemLog { .. } => "system_log",
        }
    }

    pub fn broadcast_notification(
        streams: &crate::services::streaming::StreamHub,
        strategy_id: Option<&str>,
        level: &str,
        title: &str,
        message: &str,
    ) {
        let _ = streams.send_event(Self::Notification {
            strategy_id: strategy_id.map(|s| s.to_string()),
            level: level.to_string(),
            title: title.to_string(),
            message: message.to_string(),
        });
    }
}

#[derive(Debug, Clone)]
pub struct StrategyRecord {
    pub id: String,
    pub name: String,
    pub kind: StrategyKind,
    pub enabled: bool,
    pub execution_mode: ExecutionMode,
    pub asset_class_target: AssetClassTarget,
    pub option_entry_style: OptionEntryStyle,
    pub option_structure_preset: OptionStructurePreset,
    pub option_spread_width: f64,
    pub option_target_delta: f64,
    pub option_dte_min: u32,
    pub option_dte_max: u32,
    pub option_max_spread_pct: f64,
    pub option_limit_buffer_pct: f64,
    pub credential_id: Option<String>,
    pub starting_cash: f64,
    pub cash_balance: f64,
    pub equity: f64,
    pub tracked_symbols: Vec<String>,
    pub total_trades: usize,
    pub wins: usize,
    pub losses: usize,
    pub last_signal: Option<String>,
    pub last_run_at: Option<String>,
    pub run_interval_ms: u64,
    pub state_json: serde_json::Value,
    pub risk_parameters: Option<RiskParameters>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PositionRecord {
    pub underlying_symbol: String,
    pub instrument_symbol: String,
    pub asset_type: String,
    pub quantity: f64,
    pub average_price: f64,
    pub market_price: f64,
    pub multiplier: f64,
    pub option_structure_preset: Option<OptionStructurePreset>,
    pub option_type: Option<String>,
    pub expiration: Option<String>,
    pub strike: Option<f64>,
    pub stale_quote: bool,
    pub legs: Vec<PositionLeg>,
    pub razor_stop: Option<f64>,
    pub stagnation_timestamp: Option<String>,
    pub kronos_sentiment: Option<f64>,
    pub take_profit: Option<f64>,
    pub exit_logic: Option<String>,
    pub entry_time: Option<String>,
    pub buy_logic: Option<String>,
    pub entry_math: Option<String>,
    pub entry_ai: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SignalAction {
    #[default]
    Hold,
    Buy,
    Sell,
}

impl SignalAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::Hold => "hold",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StrategySignal {
    pub action: SignalAction,
    pub allocation_fraction: f64,
    pub reason: String,
    pub limit_price: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub trailing_stop: Option<f64>,
    pub walk_to_mid: Option<bool>,
    pub split_exit: Option<bool>, // for 50/50 scalp/runner
    pub log_type: Option<String>, // NEW/DRIFT/HEARTBEAT
    pub new_state: Option<serde_json::Value>,
    pub source: Option<String>,
    pub math_edge: Option<String>,
    pub ai_score: Option<String>,
    pub stagnation_timeout_ms: Option<u64>,
    pub exit_logic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watchlist {
    pub id: String,
    pub name: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWatchlistRequest {
    pub name: String,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWatchlistRequest {
    pub name: Option<String>,
    pub symbols: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub now: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DashboardQuery {
    pub symbol: Option<String>,
    pub provider: Option<DataProvider>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ProviderQuery {
    pub provider: Option<DataProvider>,
}

#[derive(Debug, serde::Deserialize)]
pub struct CandleQuery {
    pub provider: Option<DataProvider>,
    pub range: Option<String>,
    pub interval: Option<String>,
    pub new_state: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RealtimeStreamQuery {
    pub symbol: Option<String>,
    pub provider: Option<DataProvider>,
    pub strategy_id: Option<String>,
}
