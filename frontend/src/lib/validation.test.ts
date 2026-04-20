import { describe, it, expect } from "vitest";
import { validateStrategyDraft } from "./validation";
import type { StrategyDraft } from "./drafts";

const baseDraft: StrategyDraft = {
  name: "Test Strategy",
  enabled: true,
  execution_mode: "local_paper",
  asset_class_target: "equity",
  option_entry_style: "long_call",
  option_structure_preset: "single",
  option_spread_width: "1.0",
  option_target_delta: "0.5",
  option_dte_min: "1",
  option_dte_max: "30",
  option_max_spread_pct: "0.05",
  option_limit_buffer_pct: "0.01",
  starting_cash: "10000",
  tracked_symbols: "AAPL, MSFT",
  credential_id: "",
  live_confirmation: "",
  reset_portfolio: false,
  max_position_size: "5000",
  max_daily_loss: "500",
  blacklisted_symbols: "",
  run_interval: "60",
  run_interval_unit: "seconds",
};

describe("validateStrategyDraft", () => {
  it("should return no errors for a valid equity draft", () => {
    const errors = validateStrategyDraft(baseDraft);
    expect(Object.keys(errors).length).toBe(0);
  });

  it("should validate starting cash", () => {
    const draft = { ...baseDraft, starting_cash: "500" };
    const errors = validateStrategyDraft(draft);
    expect(errors.starting_cash).toBe("Starting cash must be at least 1000");
  });

  it("should validate run interval", () => {
    const draft = { ...baseDraft, run_interval: "0" };
    const errors = validateStrategyDraft(draft);
    expect(errors.run_interval).toBe("Run interval must be at least 1");
  });

  it("should validate tracked symbols", () => {
    const draft = { ...baseDraft, tracked_symbols: "" };
    const errors = validateStrategyDraft(draft);
    expect(errors.tracked_symbols).toBe("At least one tracked symbol is required");
  });

  it("should validate live confirmation", () => {
    const draft = { ...baseDraft, execution_mode: "alpaca_live" as const, live_confirmation: "WRONG" };
    const errors = validateStrategyDraft(draft);
    expect(errors.live_confirmation).toBe("Type 'TRADE REAL MONEY' to confirm live trading");
  });

  it("should validate option DTE range", () => {
    const draft: StrategyDraft = {
      ...baseDraft,
      asset_class_target: "options",
      option_dte_min: "30",
      option_dte_max: "10",
    };
    const errors = validateStrategyDraft(draft);
    expect(errors.option_dte_min).toBe("Min DTE cannot be greater than Max DTE");
  });

  it("should validate option numeric percentages", () => {
    const draft: StrategyDraft = {
      ...baseDraft,
      asset_class_target: "options",
      option_target_delta: "1.5",
      option_max_spread_pct: "-0.1",
      option_limit_buffer_pct: "2.0",
    };
    const errors = validateStrategyDraft(draft);
    expect(errors.option_target_delta).toBe("Target delta must be between 0 and 1");
    expect(errors.option_max_spread_pct).toBe("Max spread pct must be between 0 and 1");
    expect(errors.option_limit_buffer_pct).toBe("Limit buffer pct must be between 0 and 1");
  });

  it("should validate spread width for non-single structures", () => {
    const draft: StrategyDraft = {
      ...baseDraft,
      asset_class_target: "options",
      option_structure_preset: "bull_call_spread",
      option_spread_width: "0.1",
    };
    const errors = validateStrategyDraft(draft);
    expect(errors.option_spread_width).toBe("Spread width must be at least 0.5");
  });
});
