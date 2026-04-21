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
  import { prettyMoney, quantityDigits, structureLabel, contractLabel, legLabel, parseSymbols } from "../lib/format";
  import { validateStrategyDraft, type ValidationErrors } from "../lib/validation";
  import { type StrategyDraft, runIntervalToDraft, draftToRunIntervalMs } from "../lib/drafts";
  import type { DashboardResponse } from "../lib/types";

  export let strategies: StrategySummary[] = [];
  export let credentials: CredentialSummary[] = [];
  export let selectedStrategyId = "";
  export let selectedStrategyDetail: StrategyDetailResponse | null = null;
  export let detailLoading = false;
  export let collectorIntervalSeconds = 0;
  export let viewMode: "active" | "remodeling" = "active";

  const dispatch = createEventDispatcher<{
    create: CreateStrategyRequest;
    save: { strategyId: string; payload: UpdateStrategyRequest };
    run: { strategyId: string };
    inspect: { strategyId: string };
    sync: { strategyId: string };
    start: { strategyId: string };
    stop: { strategyId: string };
  }>();

  let drafts: Record<string, StrategyDraft> = {};
  let draftErrors: Record<string, ValidationErrors> = {};
  let flipped: Record<string, boolean> = {};

  let createDraft: StrategyDraft = {
    name: "",
    enabled: true,
    execution_mode: "local_paper",
    asset_class_target: "equity",
    option_entry_style: "long_call",
    option_structure_preset: "single",
    option_spread_width: "5",
    option_target_delta: "0.30",
    option_dte_min: "21",
    option_dte_max: "45",
    option_max_spread_pct: "0.12",
    option_limit_buffer_pct: "0.05",
    starting_cash: "25000",
    tracked_symbols: "AAPL, SPY",
    credential_id: "",
    live_confirmation: "",
    reset_portfolio: false,
    max_position_size: "5000",
    max_daily_loss: "500",
    blacklisted_symbols: "",
    run_interval: "30",
    run_interval_unit: "seconds",
  };
  let createKind: StrategyKind = "vwap_reflexive";
  let createErrors: ValidationErrors = {};

  $: createErrors = validateStrategyDraft(createDraft);

  $: {
    const next: Record<string, StrategyDraft> = {};
    const nextErrors: Record<string, ValidationErrors> = {};
    for (const strategy of strategies) {
      const interval = runIntervalToDraft(strategy.run_interval_ms);
      const draft = drafts[strategy.id] ?? {
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
        run_interval: interval.value,
        run_interval_unit: interval.unit,
      };
      next[strategy.id] = draft;
      nextErrors[strategy.id] = validateStrategyDraft(draft);
    }
    drafts = next;
    draftErrors = nextErrors;
  }

  function createAgent() {
    dispatch("create", {
      name: createDraft.name.trim() || `${labelForKind(createKind)} Agent`,
      kind: createKind,
      execution_mode: createDraft.execution_mode,
      asset_class_target: createDraft.asset_class_target,
      option_entry_style: createDraft.option_entry_style,
      option_structure_preset: createDraft.option_structure_preset,
      option_spread_width: Number(createDraft.option_spread_width),
      option_target_delta: Number(createDraft.option_target_delta),
      option_dte_min: Number(createDraft.option_dte_min),
      option_dte_max: Number(createDraft.option_dte_max),
      option_max_spread_pct: Number(createDraft.option_max_spread_pct),
      option_limit_buffer_pct: Number(createDraft.option_limit_buffer_pct),
      starting_cash: Number(createDraft.starting_cash),
      tracked_symbols: parseSymbols(createDraft.tracked_symbols),
      credential_id: createDraft.credential_id || null,
      enabled: true,
      run_interval_ms: draftToRunIntervalMs(createDraft.run_interval, createDraft.run_interval_unit),
      live_confirmation: createDraft.live_confirmation,
    });
    createDraft = {
      name: "",
      enabled: true,
      execution_mode: "local_paper",
      asset_class_target: "equity",
      option_entry_style: "long_call",
      option_structure_preset: "single",
      option_spread_width: "5",
      option_target_delta: "0.30",
      option_dte_min: "21",
      option_dte_max: "45",
      option_max_spread_pct: "0.12",
      option_limit_buffer_pct: "0.05",
      starting_cash: "25000",
      tracked_symbols: "AAPL, SPY",
      credential_id: "",
      live_confirmation: "",
      reset_portfolio: false,
      max_position_size: "5000",
      max_daily_loss: "500",
      blacklisted_symbols: "",
      run_interval: "30",
      run_interval_unit: "seconds",
    };
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
        run_interval_ms: draftToRunIntervalMs(draft.run_interval, draft.run_interval_unit),
      },
    });
    flipped[strategyId] = false;
    draft.reset_portfolio = false;
  }

  function getDescriptionForKind(kind: StrategyKind) {
    switch (kind) {
      case "listing_arbitrage": return "Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering.";
      case "vwap_reflexive": return "Automated entries on standard deviation price extensions from the VWAP.";
      case "rsi_mean_reversion": return "Gamma Scalping: Harvesting theta while maintaining a delta-neutral profile.";
      case "sma_trend": return "0DTE Delta-Neutral: Capturing premium decay on same-day SPY expirations.";
      case "put_call_parity": return "Put-Call Parity: Arbitraging discrepancies between synthesized and market option prices.";
      default: return "Automated algorithmic execution strategy.";
    }
  }

  function labelForKind(kind: StrategyKind) {
    switch (kind) {
      case "listing_arbitrage": return "Listing Arbitrage";
      case "parity_sniper": return "Parity Sniper";
      case "vwap_reversion": return "VWAP Reversion";
      case "vwap_reflexive": return "VWAP Mean Reversion";
      case "rsi_mean_reversion": return "Gamma Scalping";
      case "sma_trend": return "0DTE Delta-Neutral";
      case "put_call_parity": return "Put-Call Parity";
      default: return (kind as string).replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
    }
  }

  let pendingStatus: Record<string, boolean> = {};

  async function toggleAgent(strategy: StrategySummary) {
    const nextEnabled = !strategy.enabled;
    const strategyId = strategy.id;

    // Optimistic Update
    pendingStatus[strategyId] = nextEnabled;
    pendingStatus = pendingStatus;

    try {
      if (nextEnabled) {
        await api.startStrategy(strategyId);
        dispatch("start", { strategyId });
      } else {
        await api.stopStrategy(strategyId);
        dispatch("stop", { strategyId });
      }
    } catch (err) {
      console.error("Failed to toggle agent:", err);
      delete pendingStatus[strategyId];
      pendingStatus = pendingStatus;
    } finally {
      setTimeout(() => {
        delete pendingStatus[strategyId];
        pendingStatus = pendingStatus;
      }, 1000);
    }
  }

  function getEffectiveEnabled(strategy: StrategySummary) {
    return pendingStatus[strategy.id] ?? strategy.enabled;
  }
</script>

<section class="workspace">
  <div class="workspace-header">
    <p>AutoStonks Command Center</p>
    <h2>{viewMode === "active" ? "Multi-Strategy Workstation" : "Remodeling & Setup"}</h2>
  </div>

  {#if viewMode === "remodeling"}
  <section class="create-agent">
    <div class="create-copy">
      <p>New Agent</p>
      <h3>Spin up a strategy instance</h3>
      <span>Create a named agent, save it once, and its threaded agent will evaluate it independently while the backend is online.</span>
    </div>
    <div class="create-grid">
      <label>
        <span>Name</span>
        <input id="agent-create-name" name="agent_create_name" class:invalid={createErrors.name} bind:value={createDraft.name} placeholder="Opening Range NVDA" />
        {#if createErrors.name}<span class="error-msg">{createErrors.name}</span>{/if}
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
        <input id="agent-create-symbols" name="agent_create_symbols" class:invalid={createErrors.tracked_symbols} bind:value={createDraft.tracked_symbols} placeholder="AAPL, SPY" />
        {#if createErrors.tracked_symbols}<span class="error-msg">{createErrors.tracked_symbols}</span>{/if}
      </label>
      <label>
        <span>Starting cash</span>
        <input id="agent-create-cash" name="agent_create_cash" type="number" min="1000" step="500" class:invalid={createErrors.starting_cash} bind:value={createDraft.starting_cash} />
        {#if createErrors.starting_cash}<span class="error-msg">{createErrors.starting_cash}</span>{/if}
      </label>
      <label>
        <span>Execution mode</span>
        <select id="agent-create-mode" name="agent_create_mode" bind:value={createDraft.execution_mode}>
          <option value="local_paper">Local paper</option>
          <option value="alpaca_paper">Alpaca paper</option>
          <option value="alpaca_live">Alpaca live</option>
        </select>
      </label>
      <label>
        <span>Asset class</span>
        <select id="agent-create-asset-class" name="agent_create_asset_class" bind:value={createDraft.asset_class_target}>
          <option value="equity">Equity</option>
          <option value="options">Options</option>
        </select>
      </label>
      <label>
        <span>Credential</span>
        <select id="agent-create-credential" name="agent_create_credential" bind:value={createDraft.credential_id}>
          <option value="">No broker credential</option>
          {#each credentials as credential}
            <option value={credential.id}>{credential.label} · {credential.environment}</option>
          {/each}
        </select>
      </label>
      <label>
        <span>Run Interval Mode</span>
        <select id="agent-create-interval-unit" name="agent_create_interval_unit" bind:value={createDraft.run_interval_unit}>
          <option value="seconds">Seconds</option>
          <option value="milliseconds">Milliseconds</option>
        </select>
      </label>
      <label>
        <span>Run Interval</span>
        <input id="agent-create-interval" name="agent_create_interval" type="number" min="1" step="1" class:invalid={createErrors.run_interval} bind:value={createDraft.run_interval} />
        {#if createErrors.run_interval}<span class="error-msg">{createErrors.run_interval}</span>{/if}
      </label>
      {#if createDraft.asset_class_target === "options"}
        <label>
          <span>Options structure</span>
          <select id="agent-create-option-structure" name="agent_create_option_structure" bind:value={createDraft.option_structure_preset}>
            <option value="single">Single contract</option>
            <option value="bull_call_spread">Bull call spread</option>
            <option value="bear_put_spread">Bear put spread</option>
          </select>
        </label>
        {#if createDraft.option_structure_preset === "single"}
          <label>
            <span>Option entry style</span>
            <select id="agent-create-option-style" name="agent_create_option_style" bind:value={createDraft.option_entry_style}>
              <option value="long_call">Long call</option>
              <option value="long_put">Long put</option>
            </select>
          </label>
        {/if}
        {#if createDraft.option_structure_preset !== "single"}
          <label>
            <span>Spread width</span>
            <input id="agent-create-option-width" name="agent_create_option_width" type="number" min="0.5" step="0.5" class:invalid={createErrors.option_spread_width} bind:value={createDraft.option_spread_width} />
            {#if createErrors.option_spread_width}<span class="error-msg">{createErrors.option_spread_width}</span>{/if}
          </label>
        {/if}
        <label>
          <span>Target delta</span>
          <input id="agent-create-option-delta" name="agent_create_option_delta" type="number" min="0" max="1" step="0.01" class:invalid={createErrors.option_target_delta} bind:value={createDraft.option_target_delta} />
          {#if createErrors.option_target_delta}<span class="error-msg">{createErrors.option_target_delta}</span>{/if}
        </label>
        <label>
          <span>Min DTE</span>
          <input id="agent-create-option-dte-min" name="agent_create_option_dte_min" type="number" min="1" step="1" class:invalid={createErrors.option_dte_min} bind:value={createDraft.option_dte_min} />
          {#if createErrors.option_dte_min}<span class="error-msg">{createErrors.option_dte_min}</span>{/if}
        </label>
        <label>
          <span>Max DTE</span>
          <input id="agent-create-option-dte-max" name="agent_create_option_dte_max" type="number" min="1" step="1" class:invalid={createErrors.option_dte_max} bind:value={createDraft.option_dte_max} />
          {#if createErrors.option_dte_max}<span class="error-msg">{createErrors.option_dte_max}</span>{/if}
        </label>
        <label>
          <span>Max spread pct</span>
          <input id="agent-create-option-spread" name="agent_create_option_spread" type="number" min="0" max="1" step="0.01" class:invalid={createErrors.option_max_spread_pct} bind:value={createDraft.option_max_spread_pct} />
          {#if createErrors.option_max_spread_pct}<span class="error-msg">{createErrors.option_max_spread_pct}</span>{/if}
        </label>
        <label>
          <span>Limit buffer pct</span>
          <input id="agent-create-option-buffer" name="agent_create_option_buffer" type="number" min="0" max="1" step="0.01" class:invalid={createErrors.option_limit_buffer_pct} bind:value={createDraft.option_limit_buffer_pct} />
          {#if createErrors.option_limit_buffer_pct}<span class="error-msg">{createErrors.option_limit_buffer_pct}</span>{/if}
        </label>
      {/if}
      {#if createDraft.execution_mode === "alpaca_live"}
        <label class="danger">
          <span>Live confirmation phrase</span>
          <input id="agent-create-confirmation" name="agent_create_confirmation" class:invalid={createErrors.live_confirmation} bind:value={createDraft.live_confirmation} placeholder="TRADE REAL MONEY" />
          {#if createErrors.live_confirmation}<span class="error-msg">{createErrors.live_confirmation}</span>{/if}
        </label>
      {/if}
    </div>
    <button type="button" class="create-button" disabled={Object.keys(createErrors).length > 0} on:click={createAgent}>Create and run</button>
  </section>
  {/if}

  <section class="agents-layout">
    <div class="workstation-grid">
      {#each strategies as strategy}
        <article class="strat-card" class:active={getEffectiveEnabled(strategy)}>
          {#if flipped[strategy.id]}
            <div class="config-form">
              <label>
                <span>Name</span>
                <input type="text" class:invalid={draftErrors[strategy.id]?.name} bind:value={drafts[strategy.id].name} />
                {#if draftErrors[strategy.id]?.name}<span class="error-msg">{draftErrors[strategy.id].name}</span>{/if}
              </label>

              <label>
                <span>Execution mode</span>
                <select bind:value={drafts[strategy.id].execution_mode}>
                  <option value="local_paper">Local paper</option>
                  <option value="alpaca_paper">Alpaca paper</option>
                  <option value="alpaca_live">Alpaca live</option>
                </select>
              </label>

              <label>
                <span>Asset class</span>
                <select bind:value={drafts[strategy.id].asset_class_target}>
                  <option value="equity">Equity</option>
                  <option value="options">Options</option>
                </select>
              </label>

              <label>
                <span>Starting cash</span>
                <input type="number" class:invalid={draftErrors[strategy.id]?.starting_cash} bind:value={drafts[strategy.id].starting_cash} />
                {#if draftErrors[strategy.id]?.starting_cash}<span class="error-msg">{draftErrors[strategy.id].starting_cash}</span>{/if}
              </label>

              <label>
                <span>Tracked symbols</span>
                <input type="text" class:invalid={draftErrors[strategy.id]?.tracked_symbols} bind:value={drafts[strategy.id].tracked_symbols} />
                {#if draftErrors[strategy.id]?.tracked_symbols}<span class="error-msg">{draftErrors[strategy.id].tracked_symbols}</span>{/if}
              </label>

              <label>
                <span>Run interval</span>
                <div style="display: flex; gap: 4px;">
                  <input type="number" style="flex: 1" class:invalid={draftErrors[strategy.id]?.run_interval} bind:value={drafts[strategy.id].run_interval} />
                  <select bind:value={drafts[strategy.id].run_interval_unit}>
                    <option value="seconds">s</option>
                    <option value="milliseconds">ms</option>
                  </select>
                </div>
                {#if draftErrors[strategy.id]?.run_interval}<span class="error-msg">{draftErrors[strategy.id].run_interval}</span>{/if}
              </label>

              <label>
                <span>Credential</span>
                <select bind:value={drafts[strategy.id].credential_id}>
                  <option value="">No broker credential</option>
                  {#each credentials as credential}
                    <option value={credential.id}>{credential.label} · {credential.environment}</option>
                  {/each}
                </select>
              </label>

              {#if drafts[strategy.id].asset_class_target === "options"}
                <label>
                  <span>Options structure</span>
                  <select bind:value={drafts[strategy.id].option_structure_preset}>
                    <option value="single">Single contract</option>
                    <option value="bull_call_spread">Bull call spread</option>
                    <option value="bear_put_spread">Bear put spread</option>
                  </select>
                </label>
                {#if drafts[strategy.id].option_structure_preset === "single"}
                  <label>
                    <span>Option style</span>
                    <select bind:value={drafts[strategy.id].option_entry_style}>
                      <option value="long_call">Long call</option>
                      <option value="long_put">Long put</option>
                    </select>
                  </label>
                {/if}
                {#if drafts[strategy.id].option_structure_preset !== "single"}
                  <label>
                    <span>Spread width</span>
                    <input type="number" class:invalid={draftErrors[strategy.id]?.option_spread_width} bind:value={drafts[strategy.id].option_spread_width} />
                    {#if draftErrors[strategy.id]?.option_spread_width}<span class="error-msg">{draftErrors[strategy.id].option_spread_width}</span>{/if}
                  </label>
                {/if}
                <label>
                  <span>Target delta</span>
                  <input type="number" step="0.01" class:invalid={draftErrors[strategy.id]?.option_target_delta} bind:value={drafts[strategy.id].option_target_delta} />
                  {#if draftErrors[strategy.id]?.option_target_delta}<span class="error-msg">{draftErrors[strategy.id].option_target_delta}</span>{/if}
                </label>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 8px;">
                  <label>
                    <span>Min DTE</span>
                    <input type="number" class:invalid={draftErrors[strategy.id]?.option_dte_min} bind:value={drafts[strategy.id].option_dte_min} />
                    {#if draftErrors[strategy.id]?.option_dte_min}<span class="error-msg">{draftErrors[strategy.id].option_dte_min}</span>{/if}
                  </label>
                  <label>
                    <span>Max DTE</span>
                    <input type="number" class:invalid={draftErrors[strategy.id]?.option_dte_max} bind:value={drafts[strategy.id].option_dte_max} />
                    {#if draftErrors[strategy.id]?.option_dte_max}<span class="error-msg">{draftErrors[strategy.id].option_dte_max}</span>{/if}
                  </label>
                </div>
              {/if}

              {#if drafts[strategy.id].execution_mode === "alpaca_live"}
                <label class="danger">
                  <span>Live confirmation</span>
                  <input type="text" class:invalid={draftErrors[strategy.id]?.live_confirmation} bind:value={drafts[strategy.id].live_confirmation} placeholder="TRADE REAL MONEY" />
                  {#if draftErrors[strategy.id]?.live_confirmation}<span class="error-msg">{draftErrors[strategy.id].live_confirmation}</span>{/if}
                </label>
              {/if}

              <label class="inline-checkbox">
                <input type="checkbox" bind:checked={drafts[strategy.id].reset_portfolio} />
                <span>Reset the strategy ledger when saving</span>
              </label>

              <div class="strat-footer" style="justify-content: flex-end;">
                <button class="ghost-link" on:click={() => flipped[strategy.id] = false}>Cancel</button>
                <button class="btn-save" disabled={Object.keys(draftErrors[strategy.id] || {}).length > 0} on:click={() => save(strategy.id)}>Save</button>
              </div>
            </div>
          {:else}
            <div class="strat-header">
              <div class="strat-title">
                <h3>{labelForKind(strategy.kind)}</h3>
                <p class="strat-desc">{getDescriptionForKind(strategy.kind)}</p>
              </div>
              <div class="strat-status">
                <span class="status-dot" class:live={getEffectiveEnabled(strategy)}></span>
                <span class="status-label">{getEffectiveEnabled(strategy) ? "Live" : "Idle"}</span>
              </div>
            </div>

            <div class="strat-actions">
              <button
                type="button"
                class="btn-execute"
                class:dim={getEffectiveEnabled(strategy)}
                on:click={() => !getEffectiveEnabled(strategy) && toggleAgent(strategy)}
              >
                Execute
              </button>
              <button
                type="button"
                class="btn-stop"
                class:dim={!getEffectiveEnabled(strategy)}
                on:click={() => getEffectiveEnabled(strategy) && toggleAgent(strategy)}
              >
                Stop
              </button>
            </div>

            <div class="strat-footer">
              <button class="ghost-link" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>
                Activity
              </button>
              <button class="ghost-link" on:click={() => { flipped[strategy.id] = true; dispatch("inspect", { strategyId: strategy.id }); }}>
                Config
              </button>
            </div>
          {/if}
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
                  <p>{position.quantity.toFixed(quantityDigits(position.asset_type))} @ ${position.average_price.toFixed(2)} · value {prettyMoney(position.market_value)}</p>
                  <small>UPL {prettyMoney(position.unrealized_pnl)}{position.stale_quote ? " · stale quote" : ""}</small>
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

  .workspace-header {
    margin-bottom: 2rem;
  }

  .workspace-header h2 {
    font-size: 1.8rem;
    font-weight: 700;
    margin: 0;
  }

  .workspace-header p {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 2px;
    color: rgba(221, 233, 255, 0.66);
    margin-bottom: 4px;
  }

  .create-agent {
    display: grid;
    gap: 1rem;
    padding: 1.25rem;
    border-radius: 28px;
    border: 1px solid rgba(255, 255, 255, 0.08);
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

  .create-button {
    background: linear-gradient(135deg, #f0b450, #f7dc72);
    color: #180e00;
    font-weight: 700;
  }

  .create-button:disabled,
  .btn-save:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-save {
    background: linear-gradient(135deg, #3a7bfd, #63d7ff);
    color: #07101d;
    font-weight: 700;
    border: none;
    padding: 8px 16px;
    border-radius: 8px;
  }

  .config-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .config-form label {
    display: grid;
    gap: 4px;
  }

  .config-form input,
  .config-form select {
    padding: 8px;
    font-size: 0.85rem;
  }

  .inline-checkbox {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: rgba(221, 233, 255, 0.8);
  }

  .inline-checkbox input {
    width: auto;
    margin: 0;
  }

  input.invalid,
  select.invalid {
    border-color: #ff8a8a !important;
  }

  .error-msg {
    color: #ff8a8a;
    font-size: 0.75rem;
    margin-top: 0.25rem;
  }

  .workstation-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 1.5rem;
    margin-bottom: 3rem;
  }

  .strat-card {
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    transition: all 0.2s ease;
  }

  .strat-card.active {
    border-color: rgba(34, 197, 94, 0.3);
    box-shadow: 0 0 20px rgba(34, 197, 94, 0.05);
  }

  .strat-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .strat-title h3 {
    margin: 0 0 6px 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .strat-desc {
    font-size: 0.85rem;
    color: rgba(221, 233, 255, 0.66);
    line-height: 1.4;
    margin: 0;
  }

  .strat-status {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(0, 0, 0, 0.2);
    padding: 4px 10px;
    border-radius: 100px;
    font-size: 0.75rem;
    color: rgba(221, 233, 255, 0.66);
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #4b5563;
  }

  .status-dot.live {
    background: #4ade80;
    box-shadow: 0 0 8px #4ade80;
  }

  .strat-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }

  .btn-execute, .btn-stop {
    border: none;
    padding: 12px;
    border-radius: 8px;
    font-weight: 700;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s ease;
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .btn-execute {
    background: #10b981;
    color: #062016;
  }

  .btn-stop {
    background: #ef4444;
    color: #fff;
  }

  .btn-execute.dim, .btn-stop.dim {
    opacity: 0.2;
    cursor: default;
    filter: grayscale(0.5);
  }

  .btn-execute:not(.dim):hover { background: #34d399; transform: translateY(-1px); }
  .btn-stop:not(.dim):hover { background: #f87171; transform: translateY(-1px); }

  .strat-footer {
    display: flex;
    gap: 16px;
    margin-top: auto;
    padding-top: 10px;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .ghost-link {
    background: none;
    border: none;
    color: rgba(221, 233, 255, 0.66);
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0;
  }

  .ghost-link:hover { color: white; }

  .agent-detail {
    display: grid;
    gap: 1rem;
    align-content: start;
  }

  .detail-header,
  .detail-block {
    padding: 1.1rem;
    border-radius: 28px;
    border: 1px solid rgba(255, 255, 255, 0.08);
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
  .empty {
    margin: 0;
    color: rgba(231, 239, 255, 0.78);
    line-height: 1.45;
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
    .detail-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
