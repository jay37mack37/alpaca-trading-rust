import type { StrategyDraft } from "./drafts";
import { parseSymbols } from "./format";

export type ValidationErrors = Partial<Record<keyof StrategyDraft, string>>;

export function validateStrategyDraft(draft: StrategyDraft): ValidationErrors {
  const errors: ValidationErrors = {};

  if (!draft.name.trim()) {
    errors.name = "Name is required";
  }

  const startingCash = Number(draft.starting_cash);
  if (isNaN(startingCash) || startingCash < 1000) {
    errors.starting_cash = "Starting cash must be at least 1,000";
  }

  const symbols = parseSymbols(draft.tracked_symbols);
  if (symbols.length === 0) {
    errors.tracked_symbols = "At least one tracked symbol is required";
  }

  const runInterval = Number(draft.run_interval);
  if (isNaN(runInterval) || runInterval < 1) {
    errors.run_interval = "Run interval must be at least 1";
  }

  if (draft.asset_class_target === "options") {
    const targetDelta = Number(draft.option_target_delta);
    if (isNaN(targetDelta) || targetDelta < 0 || targetDelta > 1) {
      errors.option_target_delta = "Target delta must be between 0 and 1";
    }

    const dteMin = Number(draft.option_dte_min);
    if (isNaN(dteMin) || dteMin < 0) {
      errors.option_dte_min = "Min DTE must be at least 0";
    }

    const dteMax = Number(draft.option_dte_max);
    if (isNaN(dteMax) || dteMax < (isNaN(dteMin) ? 0 : dteMin)) {
      errors.option_dte_max = "Max DTE must be at least Min DTE";
    }

    const maxSpread = Number(draft.option_max_spread_pct);
    if (isNaN(maxSpread) || maxSpread < 0 || maxSpread > 1) {
      errors.option_max_spread_pct = "Max spread pct must be between 0 and 1";
    }

    const limitBuffer = Number(draft.option_limit_buffer_pct);
    if (isNaN(limitBuffer) || limitBuffer < 0 || limitBuffer > 1) {
      errors.option_limit_buffer_pct = "Limit buffer pct must be between 0 and 1";
    }

    if (draft.option_structure_preset !== "single") {
      const spreadWidth = Number(draft.option_spread_width);
      if (isNaN(spreadWidth) || spreadWidth <= 0) {
        errors.option_spread_width = "Spread width must be positive";
      }
    }
  }

  if (draft.execution_mode === "alpaca_live") {
    if (draft.live_confirmation !== "TRADE REAL MONEY") {
      errors.live_confirmation = "Must match 'TRADE REAL MONEY'";
    }
  }

  const maxPositionSize = Number(draft.max_position_size);
  if (isNaN(maxPositionSize) || maxPositionSize < 0) {
    errors.max_position_size = "Max position size must be at least 0";
  }

  const maxDailyLoss = Number(draft.max_daily_loss);
  if (isNaN(maxDailyLoss) || maxDailyLoss < 0) {
    errors.max_daily_loss = "Max daily loss must be at least 0";
  }

  return errors;
}
