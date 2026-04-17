<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type {
    AssetClassTarget,
    CreateStrategyRequest,
    CredentialSummary,
    ExecutionMode,
    OptionEntryStyle,
    OptionStructurePreset,
    StrategyDetailResponse,
    StrategyKind,
    StrategySummary,
    UpdateStrategyRequest,
  } from "../lib/types";
  import BrokerSyncPanel from "./BrokerSyncPanel.svelte";
  import InteractiveTicker from "./InteractiveTicker.svelte";
  import AgentCardTicker from "./AgentCardTicker.svelte";
  import { api } from "../lib/api";
  import type { DashboardResponse } from "../lib/types";

  export let strategies: StrategySummary[] = [];
  export let credentials: CredentialSummary[] = [];
  export let selectedStrategyId = "";
  export let selectedStrategyDetail: StrategyDetailResponse | null = null;
  export let detailLoading = false;
  export let collectorIntervalSeconds = 0;

  const dispatch = createEventDispatcher<{
    create: CreateStrategyRequest;
    save: { strategyId: string; payload: UpdateStrategyRequest };
    run: { strategyId: string };
    inspect: { strategyId: string };
    sync: { strategyId: string };
  }>();

  type Draft = {
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
    run_interval_unit: "seconds" | "milliseconds";
  };

  let drafts: Record<string, Draft> = {};
  let flipped: Record<string, boolean> = {};

  let createName = "";
  let createKind: StrategyKind = "vwap_reflexive";
  let createSymbols = "AAPL, SPY";
  let createStartingCash = "25000";
  let createExecutionMode: ExecutionMode = "local_paper";
  let createAssetClassTarget: AssetClassTarget = "equity";
  let createOptionEntryStyle: OptionEntryStyle = "long_call";
  let createOptionStructurePreset: OptionStructurePreset = "single";
  let createOptionSpreadWidth = "5";
  let createOptionTargetDelta = "0.30";
  let createOptionDteMin = "21";
  let createOptionDteMax = "45";
  let createOptionMaxSpreadPct = "0.12";
  let createOptionLimitBufferPct = "0.05";
  let createCredentialId = "";
  let createRunInterval = "30";
  let createRunIntervalUnit: "seconds" | "milliseconds" = "seconds";



  $: {
    const next: Record<string, Draft> = {};
    for (const strategy of strategies) {
      next[strategy.id] = drafts[strategy.id] ?? {
        name: strategy.name,
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
        max_position_size: strategy.risk_parameters?.max_position_size != null ? String(strategy.risk_parameters.max_position_size) : "5000",
        max_daily_loss: strategy.risk_parameters?.max_daily_loss != null ? String(strategy.risk_parameters.max_daily_loss) : "500",
        blacklisted_symbols: strategy.risk_parameters?.blacklisted_symbols?.join(", ") || "",
        run_interval: String(strategy.run_interval_ms % 1000 === 0 && strategy.run_interval_ms !== 0 ? strategy.run_interval_ms / 1000 : strategy.run_interval_ms),
        run_interval_unit: strategy.run_interval_ms % 1000 === 0 && strategy.run_interval_ms !== 0 ? "seconds" : "milliseconds",
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

  function createAgent() {
    dispatch("create", {
      name: createName.trim() || `${labelForKind(createKind)} Agent`,
      kind: createKind,
      execution_mode: createExecutionMode,
      asset_class_target: createAssetClassTarget,
      option_entry_style: createOptionEntryStyle,
      option_structure_preset: createOptionStructurePreset,
      option_spread_width: Number(createOptionSpreadWidth),
      option_target_delta: Number(createOptionTargetDelta),
      option_dte_min: Number(createOptionDteMin),
      option_dte_max: Number(createOptionDteMax),
      option_max_spread_pct: Number(createOptionMaxSpreadPct),
      option_limit_buffer_pct: Number(createOptionLimitBufferPct),
      starting_cash: Number(createStartingCash),
      tracked_symbols: parseSymbols(createSymbols),
      credential_id: createCredentialId || null,
      enabled: true,
      run_interval_ms: createRunIntervalUnit === "seconds" ? Number(createRunInterval) * 1000 : Number(createRunInterval),
    });
    createName = "";
    createSymbols = "AAPL, SPY";
    createStartingCash = "25000";
    createExecutionMode = "local_paper";
    createAssetClassTarget = "equity";
    createOptionEntryStyle = "long_call";
    createOptionStructurePreset = "single";
    createOptionSpreadWidth = "5";
    createOptionTargetDelta = "0.30";
    createOptionDteMin = "21";
    createOptionDteMax = "45";
    createOptionMaxSpreadPct = "0.12";
    createOptionLimitBufferPct = "0.05";
    createCredentialId = "";
    createRunInterval = "30";
    createRunIntervalUnit = "seconds";
  }

  function save(strategyId: string) {
    const draft = drafts[strategyId];
    const risk_parameters = {
      max_position_size: Number(draft.max_position_size),
      max_daily_loss: Number(draft.max_daily_loss),
      blacklisted_symbols: parseSymbols(draft.blacklisted_symbols),
    };

    dispatch("save", {
      strategyId,
      payload: {
        name: draft.name,
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
        risk_parameters,
        run_interval_ms: draft.run_interval_unit === "seconds" ? Number(draft.run_interval) * 1000 : Number(draft.run_interval),
      },
    });
    flipped[strategyId] = false;
    draft.reset_portfolio = false;
  }

  function labelForKind(kind: StrategyKind) {
    return kind
      .replaceAll("_", " ")
      .replace(/\b\w/g, (letter) => letter.toUpperCase());
  }

  function money(value: number | null | undefined) {
    return value == null ? "—" : `$${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
  }

  function quantityDigits(assetType: string) {
    return assetType === "equity" ? 3 : 0;
  }

  function legLabel(leg: {
    instrument_symbol: string;
    option_type?: string | null;
    strike?: number | null;
    expiration?: string | null;
  }) {
    const bits = [
      leg.option_type?.replace("_", " "),
      leg.strike != null ? `$${leg.strike}` : null,
      leg.expiration ? new Date(leg.expiration).toLocaleDateString() : null,
    ].filter(Boolean);
    return bits.length > 0 ? bits.join(" · ") : leg.instrument_symbol;
  }

  function contractLabel(tradeOrPosition: {
    asset_type: string;
    instrument_symbol: string;
    option_structure_preset?: OptionStructurePreset | null;
    option_type?: string | null;
    expiration?: string | null;
    strike?: number | null;
    underlying_symbol?: string | null;
  }) {
    if (tradeOrPosition.asset_type === "option_spread") {
      return [
        tradeOrPosition.underlying_symbol ?? tradeOrPosition.instrument_symbol,
        structureLabel(tradeOrPosition.option_structure_preset),
      ].join(" · ");
    }
    if (tradeOrPosition.asset_type !== "option") return tradeOrPosition.instrument_symbol;
    const bits = [
      tradeOrPosition.instrument_symbol,
      tradeOrPosition.option_type?.replace("_", " "),
      tradeOrPosition.strike != null ? `$${tradeOrPosition.strike}` : null,
      tradeOrPosition.expiration ? new Date(tradeOrPosition.expiration).toLocaleDateString() : null,
    ].filter(Boolean);
    return bits.join(" · ");
  }

  function structureLabel(value: OptionStructurePreset | null | undefined) {
    return (value ?? "single").replaceAll("_", " ");
  }
</script>

<section class="workspace">
  <div class="workspace-header">
    <div>
      <p>Agents</p>
      <h2>Running strategy instances</h2>
    </div>
    <div class="runtime-chip">
      Backend loop {collectorIntervalSeconds > 0 ? `every ${collectorIntervalSeconds}s` : "manual only"}
    </div>
  </div>

  <section class="create-agent">
    <div class="create-copy">
      <p>New Agent</p>
      <h3>Spin up a strategy instance</h3>
      <span>Create a named agent, save it once, and its threaded agent will evaluate it independently while the backend is online.</span>
    </div>
    <div class="create-grid">
      <label>
        <span>Name</span>
        <input id="agent-create-name" name="agent_create_name" bind:value={createName} placeholder="Opening Range NVDA" />
      </label>
      <label>
        <span>Template</span>
        <select id="agent-create-kind" name="agent_create_kind" bind:value={createKind}>
          <option value="vwap_reflexive">VWAP Reflexive</option>
          <option value="rsi_mean_reversion">RSI Mean Reversion</option>
          <option value="sma_trend">SMA Trend</option>
        </select>
      </label>
      <label>
        <span>Tracked symbols</span>
        <input id="agent-create-symbols" name="agent_create_symbols" bind:value={createSymbols} placeholder="AAPL, SPY" />
      </label>
      <label>
        <span>Starting cash</span>
        <input id="agent-create-cash" name="agent_create_cash" type="number" min="1000" step="500" bind:value={createStartingCash} />
      </label>
      <label>
        <span>Execution mode</span>
        <select id="agent-create-mode" name="agent_create_mode" bind:value={createExecutionMode}>
          <option value="local_paper">Local paper</option>
          <option value="alpaca_paper">Alpaca paper</option>
          <option value="alpaca_live">Alpaca live</option>
        </select>
      </label>
      <label>
        <span>Asset class</span>
        <select id="agent-create-asset-class" name="agent_create_asset_class" bind:value={createAssetClassTarget}>
          <option value="equity">Equity</option>
          <option value="options">Options</option>
        </select>
      </label>
      <label>
        <span>Credential</span>
        <select id="agent-create-credential" name="agent_create_credential" bind:value={createCredentialId}>
          <option value="">No broker credential</option>
          {#each credentials as credential}
            <option value={credential.id}>{credential.label} · {credential.environment}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>Run Interval Mode</span>
        <select id="agent-create-interval-unit" name="agent_create_interval_unit" bind:value={createRunIntervalUnit}>
          <option value="seconds">Seconds</option>
          <option value="milliseconds">Milliseconds</option>
        </select>
      </label>
      <label>
        <span>Run Interval</span>
        <input id="agent-create-interval" name="agent_create_interval" type="number" min="1" step="1" bind:value={createRunInterval} />
      </label>
      {#if createAssetClassTarget === "options"}
        <label>
          <span>Options structure</span>
          <select id="agent-create-option-structure" name="agent_create_option_structure" bind:value={createOptionStructurePreset}>
            <option value="single">Single contract</option>
            <option value="bull_call_spread">Bull call spread</option>
            <option value="bear_put_spread">Bear put spread</option>
          </select>
        </label>
        {#if createOptionStructurePreset === "single"}
          <label>
            <span>Option entry style</span>
            <select id="agent-create-option-style" name="agent_create_option_style" bind:value={createOptionEntryStyle}>
              <option value="long_call">Long call</option>
              <option value="long_put">Long put</option>
            </select>
          </label>
        {/if}
        {#if createOptionStructurePreset !== "single"}
          <label>
            <span>Spread width</span>
            <input id="agent-create-option-width" name="agent_create_option_width" type="number" min="0.5" step="0.5" bind:value={createOptionSpreadWidth} />
          </label>
        {/if}
        <label>
          <span>Target delta</span>
          <input id="agent-create-option-delta" name="agent_create_option_delta" type="number" min="0" max="1" step="0.01" bind:value={createOptionTargetDelta} />
        </label>
        <label>
          <span>Min DTE</span>
          <input id="agent-create-option-dte-min" name="agent_create_option_dte_min" type="number" min="1" step="1" bind:value={createOptionDteMin} />
        </label>
        <label>
          <span>Max DTE</span>
          <input id="agent-create-option-dte-max" name="agent_create_option_dte_max" type="number" min="1" step="1" bind:value={createOptionDteMax} />
        </label>
        <label>
          <span>Max spread pct</span>
          <input id="agent-create-option-spread" name="agent_create_option_spread" type="number" min="0" max="1" step="0.01" bind:value={createOptionMaxSpreadPct} />
        </label>
        <label>
          <span>Limit buffer pct</span>
          <input id="agent-create-option-buffer" name="agent_create_option_buffer" type="number" min="0" max="1" step="0.01" bind:value={createOptionLimitBufferPct} />
        </label>
      {/if}
    </div>
    <button type="button" class="create-button" on:click={createAgent}>Create and run</button>
  </section>

  <section class="agents-layout">
    <div class="agent-list">
      {#each strategies as strategy}
        <article class:selected={strategy.id === selectedStrategyId} class:flipped={flipped[strategy.id]} class="agent-card">
          <div class="agent-card-inner">
            <div class="agent-face agent-face--front">
              <header>
                <div>
                  <p>{labelForKind(strategy.kind)}</p>
                  <h3>{strategy.name}</h3>
                </div>
                <span class:running={strategy.enabled} class="status-chip">
                  {strategy.enabled ? "Running" : "Paused"}
                </span>
              </header>

              <div class="mini-stats">
                <div><span>Equity</span><strong>{money(strategy.equity)}</strong></div>
                <div><span>PnL</span><strong>{strategy.pnl >= 0 ? "+" : ""}{money(strategy.pnl)}</strong></div>
                <div><span>Trades</span><strong>{strategy.total_trades}</strong></div>
                <div><span>Watching</span><strong>{strategy.tracked_symbols.join(", ")}</strong></div>
              </div>

              <div class="agent-meta">
                <span>{strategy.execution_mode.replaceAll("_", " ")}</span>
                <span>{strategy.asset_class_target}</span>
                {#if strategy.asset_class_target === "options"}
                  <span>{structureLabel(strategy.option_structure_preset)}</span>
                {/if}
              </div>

              <div class="agent-signal">{strategy.last_signal ?? "No signal yet"}</div>

              {#if strategy.tracked_symbols.length > 0}
                <AgentCardTicker
                  symbol={strategy.tracked_symbols[0]}
                  showVwap={strategy.kind === "vwap_reflexive"}
                  height={120}
                />
              {/if}

              <footer>
                <button type="button" class="ghost" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>
                  Activity
                </button>
                <button type="button" class="ghost" on:click={() => dispatch("run", { strategyId: strategy.id })}>
                  Run now
                </button>
                <button type="button" on:click={() => { flipped[strategy.id] = true; dispatch("inspect", { strategyId: strategy.id }); }}>
                  Settings
                </button>
              </footer>
            </div>

            <div class="agent-face agent-face--back">
              <header>
                <div>
                  <p>Settings</p>
                  <h3>{strategy.name}</h3>
                </div>
                <button type="button" class="ghost" on:click={() => (flipped[strategy.id] = false)}>
                  Close
                </button>
              </header>

              <div class="settings-grid">
                <label>
                  <span>Name</span>
                  <input id={`agent-name-${strategy.id}`} name={`agent_name_${strategy.id}`} bind:value={drafts[strategy.id].name} />
                </label>
                <label>
                  <span>Enabled</span>
                  <input id={`agent-enabled-${strategy.id}`} name={`agent_enabled_${strategy.id}`} type="checkbox" bind:checked={drafts[strategy.id].enabled} />
                </label>
                <label>
                  <span>Execution mode</span>
                  <select
                    id={`agent-mode-${strategy.id}`}
                    name={`agent_mode_${strategy.id}`}
                    value={drafts[strategy.id].execution_mode}
                    on:change={(e) => {
                      const value = e.currentTarget.value as ExecutionMode;
                      if (value === "alpaca_live") {
                        const confirmed = window.prompt("WARNING: You are about to enable live trading with REAL MONEY. This could lead to severe financial loss. Type 'TRADE REAL MONEY' to proceed.");
                        if (confirmed !== "TRADE REAL MONEY") {
                          if (confirmed !== null) {
                            window.alert("Confirmation phrase incorrect. Live trading not enabled.");
                          }
                          e.currentTarget.value = drafts[strategy.id].execution_mode;
                          return;
                        }
                        drafts[strategy.id].live_confirmation = confirmed;
                      }
                      drafts[strategy.id].execution_mode = value;
                    }}
                  >
                    <option value="local_paper">Local paper</option>
                    <option value="alpaca_paper">Alpaca paper</option>
                    <option value="alpaca_live">Alpaca live</option>
                  </select>
                </label>
                <label>
                  <span>Asset class</span>
                  <select id={`agent-asset-class-${strategy.id}`} name={`agent_asset_class_${strategy.id}`} bind:value={drafts[strategy.id].asset_class_target}>
                    <option value="equity">Equity</option>
                    <option value="options">Options</option>
                  </select>
                </label>
                <label>
                  <span>Credential</span>
                  <select id={`agent-credential-${strategy.id}`} name={`agent_credential_${strategy.id}`} bind:value={drafts[strategy.id].credential_id}>
                    <option value="">No broker credential</option>
                    {#each credentials as credential}
                      <option value={credential.id}>{credential.label} · {credential.environment}</option>
                    {/each}
                  </select>
                </label>
                <label>
                  <span>Starting cash</span>
                  <input id={`agent-cash-${strategy.id}`} name={`agent_cash_${strategy.id}`} type="number" min="1000" step="500" bind:value={drafts[strategy.id].starting_cash} />
                </label>
                <label>
                  <span>Tracked symbols</span>
                  <input id={`agent-symbols-${strategy.id}`} name={`agent_symbols_${strategy.id}`} bind:value={drafts[strategy.id].tracked_symbols} />
                </label>
                <label>
                  <span>Run Interval Mode</span>
                  <select id={`agent-interval-unit-${strategy.id}`} name={`agent_interval_unit_${strategy.id}`} bind:value={drafts[strategy.id].run_interval_unit}>
                    <option value="seconds">Seconds</option>
                    <option value="milliseconds">Milliseconds</option>
                  </select>
                </label>
                <label>
                  <span>Run Interval Duration</span>
                  <input id={`agent-interval-duration-${strategy.id}`} name={`agent_interval_duration_${strategy.id}`} type="number" min="1" step="1" bind:value={drafts[strategy.id].run_interval} />
                </label>
                {#if drafts[strategy.id].asset_class_target === "options"}
                  <label>
                    <span>Options structure</span>
                    <select id={`agent-option-structure-${strategy.id}`} name={`agent_option_structure_${strategy.id}`} bind:value={drafts[strategy.id].option_structure_preset}>
                      <option value="single">Single contract</option>
                      <option value="bull_call_spread">Bull call spread</option>
                      <option value="bear_put_spread">Bear put spread</option>
                    </select>
                  </label>
                  {#if drafts[strategy.id].option_structure_preset === "single"}
                    <label>
                      <span>Option entry style</span>
                      <select id={`agent-option-style-${strategy.id}`} name={`agent_option_style_${strategy.id}`} bind:value={drafts[strategy.id].option_entry_style}>
                        <option value="long_call">Long call</option>
                        <option value="long_put">Long put</option>
                      </select>
                    </label>
                  {/if}
                  {#if drafts[strategy.id].option_structure_preset !== "single"}
                    <label>
                      <span>Spread width</span>
                      <input id={`agent-option-width-${strategy.id}`} name={`agent_option_width_${strategy.id}`} type="number" min="0.5" step="0.5" bind:value={drafts[strategy.id].option_spread_width} />
                    </label>
                  {/if}
                  <label>
                    <span>Target delta</span>
                    <input id={`agent-option-delta-${strategy.id}`} name={`agent_option_delta_${strategy.id}`} type="number" min="0" max="1" step="0.01" bind:value={drafts[strategy.id].option_target_delta} />
                  </label>
                  <label>
                    <span>Min DTE</span>
                    <input id={`agent-option-dte-min-${strategy.id}`} name={`agent_option_dte_min_${strategy.id}`} type="number" min="1" step="1" bind:value={drafts[strategy.id].option_dte_min} />
                  </label>
                  <label>
                    <span>Max DTE</span>
                    <input id={`agent-option-dte-max-${strategy.id}`} name={`agent_option_dte_max_${strategy.id}`} type="number" min="1" step="1" bind:value={drafts[strategy.id].option_dte_max} />
                  </label>
                  <label>
                    <span>Max spread pct</span>
                    <input id={`agent-option-spread-${strategy.id}`} name={`agent_option_spread_${strategy.id}`} type="number" min="0" max="1" step="0.01" bind:value={drafts[strategy.id].option_max_spread_pct} />
                  </label>
                  <label>
                    <span>Limit buffer pct</span>
                    <input id={`agent-option-buffer-${strategy.id}`} name={`agent_option_buffer_${strategy.id}`} type="number" min="0" max="1" step="0.01" bind:value={drafts[strategy.id].option_limit_buffer_pct} />
                  </label>
                {/if}
                {#if drafts[strategy.id].execution_mode === "alpaca_live"}
                  <label class="danger">
                    <span>Live confirmation phrase</span>
                    <input
                      id={`agent-live-confirmation-${strategy.id}`}
                      name={`agent_live_confirmation_${strategy.id}`}
                      placeholder="TRADE REAL MONEY"
                      bind:value={drafts[strategy.id].live_confirmation}
                    />
                  </label>
                {/if}

                <div class="risk-controls">
                  <header>
                    <h4>Pre-Trade Risk Limits</h4>
                  </header>
                  <div class="risk-controls-grid">
                    <label>
                      <span>Max Position ($)</span>
                      <input
                        type="number"
                        min="0"
                        step="100"
                        bind:value={drafts[strategy.id].max_position_size}
                      />
                    </label>
                    <label>
                      <span>Max Loss ($)</span>
                      <input
                        type="number"
                        min="0"
                        step="50"
                        bind:value={drafts[strategy.id].max_daily_loss}
                      />
                    </label>
                    <label class="full-width">
                      <span>Blacklisted symbols</span>
                      <input
                        type="text"
                        placeholder="GME, AMC"
                        bind:value={drafts[strategy.id].blacklisted_symbols}
                      />
                    </label>
                  </div>
                </div>

                <label class="inline-checkbox">
                  <input id={`agent-reset-${strategy.id}`} name={`agent_reset_${strategy.id}`} type="checkbox" bind:checked={drafts[strategy.id].reset_portfolio} />
                  <span>Reset ledger on save</span>
                </label>
              </div>

              <footer>
                <button type="button" class="ghost" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>
                  View activity
                </button>
                <button type="button" on:click={() => save(strategy.id)}>Save agent</button>
              </footer>
            </div>
          </div>
        </article>
      {/each}
    </div>

    <aside class="agent-detail">
      <div class="detail-header">
        <div>
          <p>Agent Activity</p>
          <h3>{selectedStrategyDetail?.strategy.name ?? "Select an agent"}</h3>
        </div>
      </div>

      {#if detailLoading}
        <div class="empty">Loading agent detail…</div>
      {:else if !selectedStrategyDetail}
        <div class="empty">Pick an agent to review trades, positions, and broker state.</div>
      {:else}
        <div class="detail-grid">
          <article>
            <span>Last run</span>
            <strong>{selectedStrategyDetail.strategy.last_run_at ? new Date(selectedStrategyDetail.strategy.last_run_at).toLocaleString() : "—"}</strong>
          </article>
          <article>
            <span>Win rate</span>
            <strong>{(selectedStrategyDetail.strategy.win_rate * 100).toFixed(1)}%</strong>
          </article>
          <article>
            <span>Open positions</span>
            <strong>{selectedStrategyDetail.positions.length}</strong>
          </article>
          <article>
            <span>Execution</span>
            <strong>{selectedStrategyDetail.strategy.execution_mode.replaceAll("_", " ")} · {selectedStrategyDetail.strategy.asset_class_target} · {structureLabel(selectedStrategyDetail.strategy.option_structure_preset)}</strong>
          </article>
        </div>

        <div class="detail-block">
          <h4>Open positions</h4>
          {#if selectedStrategyDetail.positions.length === 0}
            <div class="empty empty--small">No open positions.</div>
          {:else}
            <div class="trade-feed">
              {#each selectedStrategyDetail.positions as position}
                <article>
                  <header>
                    <strong>{contractLabel(position)}</strong>
                    <span>{position.asset_type}</span>
                  </header>
                  <p>{position.quantity.toFixed(quantityDigits(position.asset_type))} @ ${position.average_price.toFixed(2)} · value {money(position.market_value)}</p>
                  <small>UPL {money(position.unrealized_pnl)}{position.stale_quote ? " · stale quote" : ""}</small>
                  {#if position.legs.length > 0}
                    <small>{position.legs.map((leg) => `${leg.position_side} ${legLabel(leg)}`).join(" | ")}</small>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        </div>

        <div class="detail-block">
          <h4>Recent trades</h4>
          {#if selectedStrategyDetail.trades.length === 0}
            <div class="empty empty--small">No trades yet.</div>
          {:else}
            <div class="trade-feed">
              {#each selectedStrategyDetail.trades.slice(0, 8) as trade}
                <article>
                  <header>
                    <strong>{contractLabel(trade)}</strong>
                    <span>{trade.side}</span>
                  </header>
                  <p>{trade.reason}</p>
                  <small>{trade.quantity.toFixed(quantityDigits(trade.asset_type))} @ ${trade.price.toFixed(2)} · {new Date(trade.executed_at).toLocaleString()}</small>
                  {#if trade.legs.length > 0}
                    <small>{trade.legs.map((leg) => `${leg.position_intent ?? leg.side} ${legLabel(leg)}`).join(" | ")}</small>
                  {/if}
                </article>
              {/each}
            </div>
          {/if}
        </div>

        <BrokerSyncPanel detail={selectedStrategyDetail} loading={detailLoading} on:sync={(event) => dispatch("sync", event.detail)} />
      {/if}
    </aside>
  </section>
</section>

<style>
  .workspace {
    display: grid;
    gap: 1rem;
  }

  .workspace-header,
  .detail-header,
  .create-agent {
    border-radius: 28px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .workspace-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 1.2rem 1.3rem;
    background:
      radial-gradient(circle at top right, rgba(107, 231, 255, 0.16), transparent 35%),
      linear-gradient(180deg, rgba(13, 22, 38, 0.96), rgba(8, 12, 22, 0.92));
  }

  .workspace-header p,
  .detail-header p,
  .create-copy p {
    margin: 0;
    color: rgba(221, 233, 255, 0.62);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 0.78rem;
  }

  .workspace-header h2,
  .detail-header h3,
  .create-copy h3 {
    margin: 0.25rem 0 0;
    color: white;
  }

  .runtime-chip,
  .status-chip {
    padding: 0.65rem 0.85rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    color: rgba(236, 243, 255, 0.82);
  }

  .running {
    background: rgba(91, 231, 172, 0.12);
    border-color: rgba(91, 231, 172, 0.22);
    color: #b8ffe1;
  }

  .create-agent {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    background:
      radial-gradient(circle at top left, rgba(249, 212, 119, 0.14), transparent 35%),
      linear-gradient(180deg, rgba(29, 24, 18, 0.96), rgba(14, 11, 9, 0.94));
  }

  .create-copy span {
    display: block;
    margin-top: 0.55rem;
    color: rgba(236, 243, 255, 0.78);
    line-height: 1.5;
  }

  .create-grid,
  .settings-grid,
  .detail-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem;
  }

  label {
    display: grid;
    gap: 0.35rem;
  }

  label span,
  .mini-stats span,
  .detail-grid span {
    color: rgba(221, 233, 255, 0.66);
    font-size: 0.82rem;
  }

  input,
  select,
  button {
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(8, 11, 19, 0.88);
    color: white;
    padding: 0.5rem 0.6rem;
    font-size: 0.85rem;
  }

  button {
    cursor: pointer;
  }

  .create-button,
  .agent-face--front footer button:last-child,
  .agent-face--back footer button:last-child {
    background: linear-gradient(135deg, #f0b450, #f7dc72);
    color: #180e00;
    font-weight: 700;
  }

  .ghost {
    background: rgba(255, 255, 255, 0.05);
    color: rgba(236, 243, 255, 0.86);
  }

  .agents-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.45fr) minmax(340px, 0.95fr);
    gap: 1rem;
  }

  .agent-list {
    display: grid;
    gap: 1rem;
  }

  .agent-card {
    perspective: 1500px;
  }

  .agent-card-inner {
    position: relative;
    min-height: 760px;
    transform-style: preserve-3d;
    transition: transform 260ms ease;
  }

  .agent-card.flipped .agent-card-inner {
    transform: rotateY(180deg);
  }

  .agent-face {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 1rem;
    border-radius: 28px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    backface-visibility: hidden;
  }

  .agent-face--front {
    background:
      radial-gradient(circle at top right, rgba(90, 166, 255, 0.14), transparent 36%),
      linear-gradient(180deg, rgba(13, 20, 34, 0.96), rgba(8, 11, 19, 0.92));
  }

  .agent-face--back {
    background:
      radial-gradient(circle at top right, rgba(249, 212, 119, 0.12), transparent 38%),
      linear-gradient(180deg, rgba(24, 19, 15, 0.96), rgba(12, 10, 9, 0.92));
    transform: rotateY(180deg);
    justify-content: space-between;
  }

  .selected .agent-face {
    border-color: rgba(108, 176, 255, 0.4);
    box-shadow: 0 0 0 1px rgba(108, 176, 255, 0.14);
  }

  .agent-face header,
  .agent-face footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .agent-face h3,
  .mini-stats strong,
  .detail-grid strong,
  h4 {
    margin: 0;
    color: white;
  }

  .agent-signal,
  .detail-block,
  .empty,
  .detail-grid article {
    padding: 0.95rem;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.07);
  }

  .mini-stats {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.7rem;
  }

  .agent-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0.45rem;
  }

  .agent-meta span {
    padding: 0.35rem 0.6rem;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.05);
    color: rgba(236, 243, 255, 0.78);
    font-size: 0.78rem;
    text-transform: capitalize;
  }

  .agent-detail {
    display: grid;
    gap: 1rem;
    align-content: start;
  }

  .detail-header,
  .detail-block {
    padding: 1.1rem;
    background: linear-gradient(180deg, rgba(16, 24, 21, 0.96), rgba(9, 13, 11, 0.92));
  }

  .trade-feed {
    display: grid;
    gap: 0.75rem;
  }

  .trade-feed article {
    padding: 0.8rem 0.9rem;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.07);
  }

  .trade-feed header {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    color: white;
  }

  .trade-feed p,
  .trade-feed small,
  .agent-signal,
  .empty {
    margin: 0;
    color: rgba(231, 239, 255, 0.78);
    line-height: 1.45;
  }

  .danger {
    grid-column: 1 / -1;
  }

  .risk-controls {
    grid-column: 1 / -1;
    margin: 0;
    padding: 0.5rem 0.7rem;
    border-radius: 12px;
    background: rgba(255, 117, 117, 0.05);
    border: 1px dashed rgba(255, 117, 117, 0.2);
  }

  .risk-controls header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.4rem;
  }

  .risk-controls h4 {
    margin: 0;
    color: #ff8a8a;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .risk-controls-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.4rem;
  }

  .risk-controls-grid .full-width {
    grid-column: 1 / -1;
  }

  .risk-controls-grid input {
    padding: 0.4rem 0.5rem;
    font-size: 0.8rem;
  }

  .risk-controls-grid span {
    font-size: 0.75rem;
  }

  .inline-checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    grid-column: 1 / -1;
    font-size: 0.8rem;
  }

  .empty--small {
    padding: 0.8rem 0.9rem;
  }

  @media (max-width: 1200px) {
    .agents-layout {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 720px) {
    .create-grid,
    .settings-grid,
    .detail-grid,
    .mini-stats {
      grid-template-columns: 1fr;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .agent-card-inner {
      min-height: auto;
      transition: none;
      transform: none !important;
    }

    .agent-face {
      position: static;
      backface-visibility: visible;
    }

    .agent-face--back {
      display: none;
      transform: none;
    }

    .agent-card.flipped .agent-face--front {
      display: none;
    }

    .agent-card.flipped .agent-face--back {
      display: grid;
    }
  }
</style>
