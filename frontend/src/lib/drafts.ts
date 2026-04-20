import type {
  AssetClassTarget,
  ExecutionMode,
  OptionEntryStyle,
  OptionStructurePreset,
} from "./types";

export type RunIntervalUnit = "seconds" | "milliseconds";

export interface StrategyDraft {
  name: string;
  enabled: boolean;
  execution_mode: ExecutionMode;
  asset_class_target: AssetClassTarget;
  option_entry_style: OptionEntryStyle;
  option_structure_preset: OptionStructurePreset;
  option_spread_width: string;
  option_target_delta: string;
  option_dte_min: string;
  option_dte_max: string;
  option_max_spread_pct: string;
  option_limit_buffer_pct: string;
  starting_cash: string;
  tracked_symbols: string;
  credential_id: string;
  live_confirmation: string;
  reset_portfolio: boolean;
  max_position_size: string;
  max_daily_loss: string;
  blacklisted_symbols: string;
  run_interval: string;
  run_interval_unit: RunIntervalUnit;
}

export function runIntervalToDraft(ms: number): { value: string; unit: RunIntervalUnit } {
  const useSeconds = ms % 1000 === 0 && ms !== 0;
  return {
    value: String(useSeconds ? ms / 1000 : ms),
    unit: useSeconds ? "seconds" : "milliseconds",
  };
}

export function draftToRunIntervalMs(value: string, unit: RunIntervalUnit): number {
  return unit === "seconds" ? Number(value) * 1000 : Number(value);
}