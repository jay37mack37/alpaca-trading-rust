export type DataProvider = "yahoo" | "alpaca";
export type CredentialEnvironment = "paper" | "live";
export type ExecutionMode = "local_paper" | "alpaca_paper" | "alpaca_live";
export type AssetClassTarget = "equity" | "options";
export type OptionEntryStyle = "long_call" | "long_put";
export type OptionStructurePreset = "single" | "bull_call_spread" | "bear_put_spread";
export type StrategyKind = "vwap_reflexive" | "rsi_mean_reversion" | "sma_trend";
export type TradeSide = "buy" | "sell";

export interface Quote {
  symbol: string;
  provider: DataProvider;
  price: number;
  previous_close: number | null;
  change: number | null;
  change_percent: number | null;
  bid: number | null;
  ask: number | null;
  volume: number | null;
  vwap: number | null;
  session_high: number | null;
  session_low: number | null;
  timestamp: string;
}

export interface Candle {
  timestamp: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  vwap: number | null;
}

export interface OptionContractSnapshot {
  contract_symbol: string;
  underlying_symbol: string;
  provider: DataProvider;
  option_type: string;
  expiration: string;
  strike: number;
  bid: number | null;
  ask: number | null;
  last: number | null;
  implied_volatility: number | null;
  open_interest: number | null;
  volume: number | null;
  in_the_money: boolean | null;
  delta: number | null;
  gamma: number | null;
  theta: number | null;
  vega: number | null;
  moneyness: number | null;
}

export interface CredentialSummary {
  id: string;
  label: string;
  provider: DataProvider;
  environment: CredentialEnvironment;
  use_for_data: boolean;
  use_for_trading: boolean;
  masked_key: string;
  created_at: string;
}

export interface RiskParameters {
  max_position_size: number;
  max_daily_loss: number;
  blacklisted_symbols: string[];
}

export interface StrategySummary {
  id: string;
  name: string;
  kind: StrategyKind;
  enabled: boolean;
  execution_mode: ExecutionMode;
  asset_class_target: AssetClassTarget;
  option_entry_style: OptionEntryStyle;
  option_structure_preset: OptionStructurePreset;
  option_spread_width: number;
  option_target_delta: number;
  option_dte_min: number;
  option_dte_max: number;
  option_max_spread_pct: number;
  option_limit_buffer_pct: number;
  credential_id: string | null;
  starting_cash: number;
  cash_balance: number;
  equity: number;
  tracked_symbols: string[];
  open_positions: number;
  total_trades: number;
  wins: number;
  losses: number;
  win_rate: number;
  pnl: number;
  last_signal: string | null;
  last_run_at: string | null;
  broker_synced_at: string | null;
  broker_equity: number | null;
  broker_buying_power: number | null;
  broker_open_positions: number | null;
  broker_open_orders: number | null;
  risk_parameters: RiskParameters | null;
  run_interval_ms: number;
}

export interface PositionSummary {
  strategy_id: string;
  underlying_symbol: string;
  instrument_symbol: string;
  asset_type: string;
  quantity: number;
  average_price: number;
  market_price: number;
  multiplier: number;
  option_structure_preset: OptionStructurePreset | null;
  option_type: string | null;
  expiration: string | null;
  strike: number | null;
  stale_quote: boolean;
  legs: PositionLeg[];
  market_value: number;
  unrealized_pnl: number;
}

export interface PositionLeg {
  instrument_symbol: string;
  position_side: string;
  ratio_quantity: number;
  average_price: number;
  market_price: number;
  multiplier: number;
  option_type: string | null;
  expiration: string | null;
  strike: number | null;
  stale_quote: boolean;
}

export interface TradeRecord {
  id: string;
  strategy_id: string;
  underlying_symbol: string;
  instrument_symbol: string;
  asset_type: string;
  side: TradeSide;
  quantity: number;
  price: number;
  multiplier: number;
  option_structure_preset: OptionStructurePreset | null;
  option_type: string | null;
  expiration: string | null;
  strike: number | null;
  legs: TradeLeg[];
  provider: DataProvider;
  reason: string;
  execution_mode: ExecutionMode;
  realized_pnl: number | null;
  executed_at: string;
}

export interface TradeLeg {
  instrument_symbol: string;
  side: TradeSide;
  ratio_quantity: number;
  position_intent: string | null;
  price: number;
  multiplier: number;
  option_type: string | null;
  expiration: string | null;
  strike: number | null;
}

export interface StrategyDetailResponse {
  strategy: StrategySummary;
  positions: PositionSummary[];
  trades: TradeRecord[];
  broker_sync: BrokerSyncState | null;
}

export interface BrokerAccountSummary {
  credential_id: string;
  environment: CredentialEnvironment;
  account_id: string;
  account_number: string | null;
  status: string | null;
  currency: string | null;
  buying_power: number | null;
  cash: number | null;
  equity: number | null;
  portfolio_value: number | null;
  last_equity: number | null;
  long_market_value: number | null;
  short_market_value: number | null;
  pattern_day_trader: boolean;
  trading_blocked: boolean;
  transfers_blocked: boolean;
  account_blocked: boolean;
  synced_at: string;
}

export interface BrokerPositionSummary {
  credential_id: string;
  symbol: string;
  asset_class: string | null;
  side: string | null;
  quantity: number;
  avg_entry_price: number | null;
  market_value: number | null;
  current_price: number | null;
  unrealized_pl: number | null;
  unrealized_plpc: number | null;
  synced_at: string;
}

export interface BrokerOrderSummary {
  credential_id: string;
  order_id: string;
  client_order_id: string | null;
  symbol: string | null;
  side: string | null;
  order_type: string | null;
  order_class: string | null;
  status: string | null;
  quantity: number | null;
  filled_qty: number | null;
  filled_avg_price: number | null;
  time_in_force: string | null;
  submitted_at: string | null;
  updated_at: string | null;
  synced_at: string;
}

export interface BrokerSyncState {
  credential_id: string;
  environment: CredentialEnvironment;
  synced_at: string;
  account: BrokerAccountSummary | null;
  positions: BrokerPositionSummary[];
  orders: BrokerOrderSummary[];
}

export interface DashboardResponse {
  symbol: string;
  provider: DataProvider;
  collector_interval_seconds: number;
  quote: Quote;
  candles: Candle[];
  options: OptionContractSnapshot[];
  strategies: StrategySummary[];
  recent_trades: TradeRecord[];
  credentials: CredentialSummary[];
  tracked_symbols: string[];
  watchlists: Watchlist[];
}

export interface CreateCredentialRequest {
  label: string;
  api_key: string;
  api_secret: string;
  environment: CredentialEnvironment;
  use_for_data: boolean;
  use_for_trading: boolean;
}

export interface UpdateStrategyRequest {
  name?: string;
  enabled?: boolean;
  execution_mode?: ExecutionMode;
  asset_class_target?: AssetClassTarget;
  option_entry_style?: OptionEntryStyle;
  option_structure_preset?: OptionStructurePreset;
  option_spread_width?: number;
  option_target_delta?: number;
  option_dte_min?: number;
  option_dte_max?: number;
  option_max_spread_pct?: number;
  option_limit_buffer_pct?: number;
  starting_cash?: number;
  tracked_symbols?: string[];
  credential_id?: string | null;
  clear_credential?: boolean;
  reset_portfolio?: boolean;
  live_confirmation?: string;
  risk_parameters?: RiskParameters | null;
  run_interval_ms?: number;
}

export interface CreateStrategyRequest {
  name: string;
  kind: StrategyKind;
  execution_mode?: ExecutionMode;
  asset_class_target?: AssetClassTarget;
  option_entry_style?: OptionEntryStyle;
  option_structure_preset?: OptionStructurePreset;
  option_spread_width?: number;
  option_target_delta?: number;
  option_dte_min?: number;
  option_dte_max?: number;
  option_max_spread_pct?: number;
  option_limit_buffer_pct?: number;
  starting_cash?: number;
  tracked_symbols: string[];
  credential_id?: string | null;
  enabled?: boolean;
  risk_parameters?: RiskParameters | null;
  run_interval_ms?: number;
}

export interface CollectResponse {
  symbols_collected: number;
  strategies_evaluated: number;
  trades_executed: number;
  collected_at: string;
}

export interface MarketRealtimeEvent {
  type: "market";
  provider: DataProvider;
  symbol: string;
  quote: Quote;
  candle: Candle | null;
  strategies: StrategySummary[];
}

export interface BrokerSyncRealtimeEvent {
  type: "broker_sync";
  credential_id: string;
  strategy_ids: string[];
  broker_sync: BrokerSyncState;
  strategies: StrategySummary[];
  event: string | null;
}

export interface StatusRealtimeEvent {
  type: "status";
  channel: string;
  provider: DataProvider | null;
  symbol: string | null;
  state: string;
  message: string;
}

export type RealtimeEvent =
  | MarketRealtimeEvent
  | BrokerSyncRealtimeEvent
  | StatusRealtimeEvent;

export interface Watchlist {
  id: string;
  name: string;
  symbols: string[];
}

export interface CreateWatchlistRequest {
  name: string;
  symbols: string[];
}

export interface UpdateWatchlistRequest {
  name?: string;
  symbols?: string[];
}
