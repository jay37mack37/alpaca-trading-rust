<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import type {
    AssetClassTarget,
    CredentialSummary,
    ExecutionMode,
    OptionEntryStyle,
    OptionStructurePreset,
    StrategySummary,
    UpdateStrategyRequest,
  } from "../lib/types";

  export let strategies: StrategySummary[] = [];
  export let credentials: CredentialSummary[] = [];
  export let selectedStrategyId = "";

  const dispatch = createEventDispatcher<{
    save: { strategyId: string; payload: UpdateStrategyRequest };
    run: { strategyId: string };
    inspect: { strategyId: string };
  }>();

  let runningStrategies: Set<string> = new Set();
  let loadingStrategies: Set<string> = new Set();

  type Draft = {
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
    run_interval: string;
    run_interval_unit: "seconds" | "milliseconds";
  };

  let drafts: Record<string, Draft> = {};

  $: {
    const next: Record<string, Draft> = {};
    for (const strategy of strategies) {
      next[strategy.id] = drafts[strategy.id] ?? {
        enabled: strategy.enabled,
        execution_mode: strategy.execution_mode,
        asset_class_target: strategy.asset_class_target,
        option_entry_style: strategy.option_entry_style,
        option_structure_preset: strategy.option_structure_preset,
        option_spread_width: String(strategy.option_spread_width),
        option_target_delta: strategy.option_target_delta.toFixed(2),
        option_dte_min: String(strategy.option_dte_min),
        option_dte_max: String(strategy.option_dte_max),
        option_max_spread_pct: strategy.option_max_spread_pct.toFixed(2),
        option_limit_buffer_pct: strategy.option_limit_buffer_pct.toFixed(2),
        starting_cash: String(Math.round(strategy.starting_cash)),
        tracked_symbols: strategy.tracked_symbols.join(", "),
        credential_id: strategy.credential_id ?? "",
        live_confirmation: "",
        reset_portfolio: false,
        run_interval:
          String(
            strategy.run_interval_ms % 1000 === 0 && strategy.run_interval_ms !== 0
              ? strategy.run_interval_ms / 1000
              : strategy.run_interval_ms,
          ),
        run_interval_unit:
          strategy.run_interval_ms % 1000 === 0 && strategy.run_interval_ms !== 0
            ? "seconds"
            : "milliseconds",
      };
    }
    drafts = next;
  }

  function parseSymbols(value: string) {
    return value
      .split(",")
      .map((item) => item.trim().toUpperCase())
      .filter(Boolean);
  }

  function structureLabel(value: OptionStructurePreset) {
    return value.replaceAll("_", " ");
  }

  async function toggleStrategy(strategyId: string) {
    if (runningStrategies.has(strategyId)) {
      // Stop the strategy
      loadingStrategies.add(strategyId);
      loadingStrategies = loadingStrategies;
      try {
        await api.stopStrategy(strategyId);
        runningStrategies.delete(strategyId);
        runningStrategies = runningStrategies;
      } catch (error) {
        console.error("Failed to stop strategy:", error);
      } finally {
        loadingStrategies.delete(strategyId);
        loadingStrategies = loadingStrategies;
      }
    } else {
      // Start the strategy
      loadingStrategies.add(strategyId);
      loadingStrategies = loadingStrategies;
      try {
        await api.startStrategy(strategyId);
        runningStrategies.add(strategyId);
        runningStrategies = runningStrategies;
      } catch (error) {
        console.error("Failed to start strategy:", error);
      } finally {
        loadingStrategies.delete(strategyId);
        loadingStrategies = loadingStrategies;
      }
    }
  }

  function save(strategyId: string) {
    const draft = drafts[strategyId];
    dispatch("save", {
      strategyId,
      payload: {
        enabled: draft.enabled,
        execution_mode: draft.execution_mode,
        asset_class_target: draft.asset_class_target,
        option_entry_style: draft.option_entry_style,
        option_structure_preset: draft.option_structure_preset,
        option_spread_width: Number(draft.option_spread_width),
        option_target_delta: Number(draft.option_target_delta),
        option_dte_min: Number(draft.option_dte_min),
        option_dte_max: Number(draft.option_dte_max),
        option_max_spread_pct: Number(draft.option_max_spread_pct),
        option_limit_buffer_pct: Number(draft.option_limit_buffer_pct),
        starting_cash: Number(draft.starting_cash),
        tracked_symbols: parseSymbols(draft.tracked_symbols),
        credential_id: draft.credential_id || null,
        clear_credential: !draft.credential_id,
        reset_portfolio: draft.reset_portfolio,
        live_confirmation: draft.live_confirmation,
        run_interval_ms:
          draft.run_interval_unit === "seconds"
            ? Number(draft.run_interval) * 1000
            : Number(draft.run_interval),
      },
    });
    draft.reset_portfolio = false;
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <p>Strategy Matrix</p>
      <h2>Independent paper portfolios</h2>
    </div>
    <span>Compare each strategy on its own ledger</span>
  </div>

  <div class="strategy-grid">
    {#each strategies as strategy}
      <article class:selected={strategy.id === selectedStrategyId} class="strategy-card">
        <header>
          <div>
            <h3>{strategy.name}</h3>
            <p>{strategy.kind.replaceAll("_", " ")}</p>
          </div>
          <div class="header-actions">
            <button 
              type="button" 
              class:running={runningStrategies.has(strategy.id)}
              class:loading={loadingStrategies.has(strategy.id)}
              disabled={loadingStrategies.has(strategy.id)}
              on:click={() => toggleStrategy(strategy.id)}
            >
              {#if loadingStrategies.has(strategy.id)}
                Loading...
              {:else if runningStrategies.has(strategy.id)}
                Stop Strategy
              {:else}
                Execute
              {/if}
            </button>
            <button type="button" class="ghost" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>
              Inspect
            </button>
            <button type="button" class="ghost" on:click={() => dispatch("run", { strategyId: strategy.id })}>
              Run now
            </button>
          </div>
        </header>

        <div class="stats">
          <div>
            <span>Equity</span>
            <strong>${strategy.equity.toLocaleString()}</strong>
          </div>
          <div>
            <span>PnL</span>
            <strong class:positive={strategy.pnl >= 0} class:negative={strategy.pnl < 0}>
              {strategy.pnl >= 0 ? "+" : ""}${strategy.pnl.toFixed(2)}
            </strong>
          </div>
          <div>
            <span>Win rate</span>
            <strong>{(strategy.win_rate * 100).toFixed(1)}%</strong>
          </div>
          <div>
            <span>Positions</span>
            <strong>{strategy.open_positions}</strong>
          </div>
        </div>

        {#if strategy.broker_synced_at}
          <div class="broker-summary">
            <span>Broker sync {new Date(strategy.broker_synced_at).toLocaleString()}</span>
            <span>
              Equity {strategy.broker_equity != null ? `$${strategy.broker_equity.toLocaleString()}` : "—"}
            </span>
            <span>
              Buying power {strategy.broker_buying_power != null ? `$${strategy.broker_buying_power.toLocaleString()}` : "—"}
            </span>
          </div>
        {/if}

        <div class="config-chips">
          <span>{strategy.execution_mode.replaceAll("_", " ")}</span>
          <span>{strategy.asset_class_target}</span>
          {#if strategy.asset_class_target === "options"}
            <span>{structureLabel(strategy.option_structure_preset)}</span>
          {/if}
        </div>

        <label>
          <span>Enabled</span>
          <input
            id={`strategy-enabled-${strategy.id}`}
            name={`strategy_enabled_${strategy.id}`}
            type="checkbox"
            bind:checked={drafts[strategy.id].enabled}
          />
        </label>

        <label>
          <span>Execution mode</span>
          <select
            id={`strategy-execution-mode-${strategy.id}`}
            name={`strategy_execution_mode_${strategy.id}`}
            bind:value={drafts[strategy.id].execution_mode}
          >
            <option value="local_paper">Local paper</option>
            <option value="alpaca_paper">Alpaca paper</option>
            <option value="alpaca_live">Alpaca live</option>
          </select>
        </label>

        <label>
          <span>Asset class</span>
          <select
            id={`strategy-asset-class-${strategy.id}`}
            name={`strategy_asset_class_${strategy.id}`}
            bind:value={drafts[strategy.id].asset_class_target}
          >
            <option value="equity">Equity</option>
            <option value="options">Options</option>
          </select>
        </label>

        <label>
          <span>Starting cash</span>
          <input
            id={`strategy-starting-cash-${strategy.id}`}
            name={`strategy_starting_cash_${strategy.id}`}
            type="number"
            min="1000"
            step="500"
            bind:value={drafts[strategy.id].starting_cash}
          />
        </label>

        <label>
          <span>Tracked symbols</span>
          <input
            id={`strategy-tracked-symbols-${strategy.id}`}
            name={`strategy_tracked_symbols_${strategy.id}`}
            type="text"
            bind:value={drafts[strategy.id].tracked_symbols}
          />
        </label>

        <label>
          <span>Run interval mode</span>
          <select
            id={`strategy-run-interval-unit-${strategy.id}`}
            name={`strategy_run_interval_unit_${strategy.id}`}
            bind:value={drafts[strategy.id].run_interval_unit}
          >
            <option value="seconds">Seconds</option>
            <option value="milliseconds">Milliseconds</option>
          </select>
        </label>

        <label>
          <span>Run interval</span>
          <input
            id={`strategy-run-interval-${strategy.id}`}
            name={`strategy_run_interval_${strategy.id}`}
            type="number"
            min="1"
            step="1"
            bind:value={drafts[strategy.id].run_interval}
          />
        </label>

        <label>
          <span>Credential</span>
          <select
            id={`strategy-credential-${strategy.id}`}
            name={`strategy_credential_${strategy.id}`}
            bind:value={drafts[strategy.id].credential_id}
          >
            <option value="">No broker credential</option>
            {#each credentials as credential}
              <option value={credential.id}>
                {credential.label} · {credential.environment}
              </option>
            {/each}
          </select>
        </label>

        {#if drafts[strategy.id].asset_class_target === "options"}
          <label>
            <span>Options structure</span>
            <select
              id={`strategy-option-structure-${strategy.id}`}
              name={`strategy_option_structure_${strategy.id}`}
              bind:value={drafts[strategy.id].option_structure_preset}
            >
              <option value="single">Single contract</option>
              <option value="bull_call_spread">Bull call spread</option>
              <option value="bear_put_spread">Bear put spread</option>
            </select>
          </label>

          {#if drafts[strategy.id].option_structure_preset === "single"}
            <label>
              <span>Option entry style</span>
              <select
                id={`strategy-option-style-${strategy.id}`}
                name={`strategy_option_style_${strategy.id}`}
                bind:value={drafts[strategy.id].option_entry_style}
              >
                <option value="long_call">Long call</option>
                <option value="long_put">Long put</option>
              </select>
            </label>
          {/if}

          {#if drafts[strategy.id].option_structure_preset !== "single"}
            <label>
              <span>Spread width</span>
              <input
                id={`strategy-option-width-${strategy.id}`}
                name={`strategy_option_width_${strategy.id}`}
                type="number"
                min="0.5"
                step="0.5"
                bind:value={drafts[strategy.id].option_spread_width}
              />
            </label>
          {/if}

          <label>
            <span>Target delta</span>
            <input
              id={`strategy-option-delta-${strategy.id}`}
              name={`strategy_option_delta_${strategy.id}`}
              type="number"
              min="0"
              max="1"
              step="0.01"
              bind:value={drafts[strategy.id].option_target_delta}
            />
          </label>

          <label>
            <span>Min DTE</span>
            <input
              id={`strategy-option-dte-min-${strategy.id}`}
              name={`strategy_option_dte_min_${strategy.id}`}
              type="number"
              min="1"
              step="1"
              bind:value={drafts[strategy.id].option_dte_min}
            />
          </label>

          <label>
            <span>Max DTE</span>
            <input
              id={`strategy-option-dte-max-${strategy.id}`}
              name={`strategy_option_dte_max_${strategy.id}`}
              type="number"
              min="1"
              step="1"
              bind:value={drafts[strategy.id].option_dte_max}
            />
          </label>

          <label>
            <span>Max spread pct</span>
            <input
              id={`strategy-option-spread-${strategy.id}`}
              name={`strategy_option_spread_${strategy.id}`}
              type="number"
              min="0"
              max="1"
              step="0.01"
              bind:value={drafts[strategy.id].option_max_spread_pct}
            />
          </label>

          <label>
            <span>Limit buffer pct</span>
            <input
              id={`strategy-option-buffer-${strategy.id}`}
              name={`strategy_option_buffer_${strategy.id}`}
              type="number"
              min="0"
              max="1"
              step="0.01"
              bind:value={drafts[strategy.id].option_limit_buffer_pct}
            />
          </label>
        {/if}

        {#if drafts[strategy.id].execution_mode === "alpaca_live"}
          <label class="danger">
            <span>Live confirmation phrase</span>
            <input
              id={`strategy-live-confirmation-${strategy.id}`}
              name={`strategy_live_confirmation_${strategy.id}`}
              type="text"
              placeholder="TRADE REAL MONEY"
              bind:value={drafts[strategy.id].live_confirmation}
            />
          </label>
        {/if}

        <label class="inline-checkbox">
          <input
            id={`strategy-reset-portfolio-${strategy.id}`}
            name={`strategy_reset_portfolio_${strategy.id}`}
            type="checkbox"
            bind:checked={drafts[strategy.id].reset_portfolio}
          />
          <span>Reset the strategy ledger when saving</span>
        </label>

        <footer>
          <p>{strategy.last_signal ?? "No signal yet"}</p>
          <button type="button" on:click={() => save(strategy.id)}>Save strategy</button>
        </footer>
      </article>
    {/each}
  </div>
</section>

<style>
  .panel {
    padding: 1.2rem;
    border-radius: 26px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: linear-gradient(180deg, rgba(15, 19, 32, 0.96), rgba(10, 12, 20, 0.92));
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    align-items: baseline;
  }

  .panel-header p,
  .panel-header span {
    margin: 0;
    color: rgba(221, 233, 255, 0.64);
  }

  .panel-header h2 {
    margin: 0.2rem 0 0;
    color: white;
  }

  .strategy-grid {
    display: grid;
    gap: 1rem;
  }

  .strategy-card {
    padding: 1rem;
    border-radius: 22px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .selected {
    border-color: rgba(108, 176, 255, 0.45);
    box-shadow: 0 0 0 1px rgba(108, 176, 255, 0.18);
  }

  .header-actions {
    display: flex;
    gap: 0.5rem;
  }

  header,
  footer {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: center;
  }

  header h3,
  footer p {
    margin: 0;
    color: white;
  }

  header p {
    margin: 0.15rem 0 0;
    color: rgba(221, 233, 255, 0.65);
    text-transform: capitalize;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.8rem;
    margin: 1rem 0;
  }

  .broker-summary {
    display: grid;
    gap: 0.2rem;
    padding: 0.75rem 0.85rem;
    margin-bottom: 0.85rem;
    border-radius: 16px;
    background: rgba(90, 166, 255, 0.08);
    border: 1px solid rgba(90, 166, 255, 0.14);
  }

  .broker-summary span {
    color: rgba(221, 233, 255, 0.78);
    font-size: 0.84rem;
  }

  .config-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
    margin-bottom: 0.85rem;
  }

  .config-chips span {
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.05);
    color: rgba(221, 233, 255, 0.82);
    font-size: 0.8rem;
    text-transform: capitalize;
  }

  .stats span,
  label span,
  footer p {
    color: rgba(221, 233, 255, 0.68);
    font-size: 0.85rem;
  }

  .stats strong {
    display: block;
    margin-top: 0.2rem;
    color: white;
  }

  .positive {
    color: #70f7b1;
  }

  .negative {
    color: #ff8a8a;
  }

  label {
    display: grid;
    gap: 0.35rem;
    margin-bottom: 0.75rem;
  }

  input,
  select,
  button {
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(8, 11, 19, 0.88);
    color: white;
    padding: 0.8rem 0.9rem;
    font: inherit;
  }

  button {
    cursor: pointer;
    background: linear-gradient(135deg, #3a7bfd, #63d7ff);
    color: #07101d;
    font-weight: 700;
  }

  .ghost {
    background: transparent;
    color: white;
  }

  .inline-checkbox {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .inline-checkbox input {
    width: auto;
    padding: 0;
  }

  .danger input {
    border-color: rgba(255, 117, 117, 0.35);
  }

  button.running {
    background: linear-gradient(135deg, #ff6b6b, #ff8787);
    color: white;
  }

  button.loading {
    opacity: 0.6;
    cursor: not-allowed;
  }

  button:disabled {
    cursor: not-allowed;
  }

  @media (max-width: 720px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
