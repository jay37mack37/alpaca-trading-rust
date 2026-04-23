<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type {
    AssetClassTarget,
    CreateStrategyRequest,
    CredentialSummary,
    ExecutionMode,
    OptionEntryStyle,
    OptionStructurePreset,
    PositionSummary,
    StrategyDetailResponse,
    StrategyKind,
    StrategySummary,
    UpdateStrategyRequest,
  } from "../lib/types";
  import InteractiveTicker from "./InteractiveTicker.svelte";
  import AgentCardTicker from "./AgentCardTicker.svelte";
  import StrategyLogTable from "./StrategyLogTable.svelte";
  import PositionsSection from "./PositionsSection.svelte";
  import RecentTradesSection from "./RecentTradesSection.svelte";
  import { api } from "../lib/api";
  import { prettyMoney, quantityDigits, structureLabel, contractLabel, legLabel, parseSymbols } from "../lib/format";
  import { validateStrategyDraft, type ValidationErrors } from "../lib/validation";
  import { type StrategyDraft, runIntervalToDraft, draftToRunIntervalMs } from "../lib/drafts";
  import type { DashboardResponse } from "../lib/types";

  export let strategies: StrategySummary[] = [];
  export let credentials: CredentialSummary[] = [];
  export let positions: PositionSummary[] = [];
  export let recentTrades: any[] = [];
  export let logs: any[] = [];

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

  $: if (createKind === "jarrod_vwap" && createDraft.tracked_symbols === "AAPL, SPY") {
    createDraft.tracked_symbols = "SPY";
    createDraft.asset_class_target = "options";
  }

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
        blacklisted_symbols: strategy.risk_parameters?.blacklisted_symbols?.join(", ") ?? "",
        run_interval: interval.value,
        run_interval_unit: interval.unit,
      };
      next[strategy.id] = draft;
      nextErrors[strategy.id] = validateStrategyDraft(draft);
    }
    drafts = next;
    draftErrors = nextErrors;
  }

  $: workingStrats = strategies.filter(s => s.id === 'parity-sniper' || s.id === 'vwap-reversion' || s.id === 'jarrod-vwap');
  $: researchStrats = strategies.filter(s => s.id !== 'parity-sniper' && s.id !== 'vwap-reversion' && s.id !== 'jarrod-vwap');

  function createAgent() {
    dispatch("create", {
      name: createDraft.name.trim() || `${labelForKind(createKind)} Agent`,
      kind: createKind,
      enabled: createDraft.enabled,
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
      run_interval_ms: draftToRunIntervalMs(createDraft.run_interval, createDraft.run_interval_unit),
      live_confirmation: createDraft.live_confirmation,
      risk_parameters: {
        max_position_size: Number(createDraft.max_position_size),
        max_daily_loss: Number(createDraft.max_daily_loss),
        blacklisted_symbols: parseSymbols(createDraft.blacklisted_symbols),
      },
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
      case "parity_sniper": return "Parity Sniper: Specialized $S + P - C = K$ gap detector for exploitable option pricing discrepancies.";
      case "vwap_reversion": return "VWAP Reversion: Standard deviation 'Snap Back' strategy for over-extended price action.";
      case "jarrod_vwap": return "Jarrod VWAP Options: SPY calls/puts on VWAP reclaim/breakdown with 5% profit target.";
      default: return "Automated algorithmic execution strategy.";
    }
  }

  function labelForKind(kind: StrategyKind) {
    switch (kind) {
      case "listing_arbitrage": return "Listing Arbitrage";
      case "parity_sniper": return "Parity Sniper";
      case "vwap_reversion": return "VWAP Reversion";
      case "jarrod_vwap": return "Jarrod VWAP Reclaim";
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
  <!-- Open Combat Positions at the very top -->
  <PositionsSection {positions} />

  <section class="agents-layout">
    <div class="section-header">
      <h2>Live Execution Engines</h2>
      <p>Hardened strategies certified for production trading.</p>
    </div>

    <RecentTradesSection trades={recentTrades} />
    <div class="workstation-grid">
      {#each workingStrats as strategy}
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

              <div class="config-actions">
                <button class="save-button" on:click={() => save(strategy.id)}>Save changes</button>
                <button class="cancel-button" on:click={() => flipped[strategy.id] = false}>Cancel</button>
              </div>
            </div>
          {:else}
            <div class="strat-header">
              <div class="strat-title">
                <h3>{strategy.name}</h3>
                <p class="strat-desc">{strategy.description || getDescriptionForKind(strategy.kind)}</p>
              </div>
              <div class="status-indicator" class:active={getEffectiveEnabled(strategy)}>
                <span class="dot"></span>
                {getEffectiveEnabled(strategy) ? 'Active' : 'Idle'}
              </div>
            </div>

            <div class="strat-actions">
              <button
                type="button"
                class="run-button"
                class:dim={getEffectiveEnabled(strategy)}
                on:click={() => !getEffectiveEnabled(strategy) && toggleAgent(strategy)}
              >
                EXECUTE
              </button>
              <button
                type="button"
                class="stop-button"
                class:dim={!getEffectiveEnabled(strategy)}
                on:click={() => getEffectiveEnabled(strategy) && toggleAgent(strategy)}
              >
                STOP
              </button>
            </div>

            <div class="strat-meta">
              <button class="ghost-link" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>Activity</button>
              <button class="ghost-link" on:click={() => { flipped[strategy.id] = !flipped[strategy.id] }}>Config</button>
            </div>
          {/if}
        </article>
      {/each}
    </div>

    <div class="section-header mt-4">
      <h2>R&D / Experimental Lab</h2>
      <p>Strategies currently in testing or simulation mode.</p>
    </div>
    <div class="workstation-grid mb-4">
      {#each researchStrats as strategy}
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

              <div class="config-actions">
                <button class="save-button" on:click={() => save(strategy.id)}>Save changes</button>
                <button class="cancel-button" on:click={() => flipped[strategy.id] = false}>Cancel</button>
              </div>
            </div>
          {:else}
            <div class="strat-header">
              <div class="strat-title">
                <h3>{strategy.name}</h3>
                <p class="strat-desc">{strategy.description || getDescriptionForKind(strategy.kind)}</p>
              </div>
              <div class="status-indicator" class:active={getEffectiveEnabled(strategy)}>
                <span class="dot"></span>
                {getEffectiveEnabled(strategy) ? 'Active' : 'Idle'}
              </div>
            </div>

            <div class="strat-actions">
               <button
                type="button"
                class="run-button"
                class:dim={getEffectiveEnabled(strategy)}
                on:click={() => !getEffectiveEnabled(strategy) && toggleAgent(strategy)}
              >
                EXECUTE
              </button>
              <button
                type="button"
                class="stop-button"
                class:dim={!getEffectiveEnabled(strategy)}
                on:click={() => getEffectiveEnabled(strategy) && toggleAgent(strategy)}
              >
                STOP
              </button>
            </div>

            <div class="strat-meta">
              <button class="ghost-link" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>Activity</button>
              <button class="ghost-link" on:click={() => { flipped[strategy.id] = !flipped[strategy.id] }}>Config</button>
            </div>
          {/if}
        </article>
      {/each}
    </div>

    <div class="log-panel-wrapper">
      <StrategyLogTable {logs} />
    </div>
  </section>
</section>

<style>
  .workspace {
    padding: 1rem 1.5rem 1.5rem;
    max-width: 1400px;
    margin: 0 auto;
    color: white;
  }

  .agents-layout {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
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

  input.invalid {
    border-color: #ff8a8a !important;
  }

  .error-msg {
    color: #ff8a8a;
    font-size: 0.75rem;
    margin-top: 0.25rem;
  }

  .section-header {
    margin-bottom: 1.5rem;
    border-left: 3px solid #22c55e;
    padding-left: 1rem;
  }

  .section-header h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 0.25rem;
  }

  .section-header p {
    font-size: 0.85rem;
    color: rgba(221, 233, 255, 0.6);
  }

  .mt-4 { margin-top: 3rem; }
  .mb-4 { margin-bottom: 2rem; }

  .workstation-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 1.5rem;
  }

  .strat-card {
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    transition: all 0.2s ease;
    cursor: pointer;
  }

  .strat-card:hover { border-color: rgba(255, 255, 255, 0.15); }
  .strat-card.active { border-color: rgba(34, 197, 94, 0.3); box-shadow: 0 0 20px rgba(34, 197, 94, 0.05); }

  .strat-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .strat-title h3 { margin: 0 0 0.25rem 0; font-size: 1.1rem; }
  .strat-desc { font-size: 0.8rem; color: rgba(221, 233, 255, 0.6); line-height: 1.4; }

  .status-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: rgba(221, 233, 255, 0.5);
    background: rgba(0, 0, 0, 0.2);
    padding: 0.25rem 0.6rem;
    border-radius: 20px;
  }

  .status-indicator.active { color: #22c55e; background: rgba(34, 197, 94, 0.1); }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }

  .strat-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .run-button, .stop-button {
    padding: 0.8rem;
    font-size: 0.85rem;
    font-weight: 700;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .run-button { background: #22c55e; color: #052e16; }
  .run-button.dim { background: rgba(34, 197, 94, 0.2); color: rgba(255, 255, 255, 0.3); cursor: default; }
  .stop-button { background: rgba(239, 68, 68, 0.1); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.2); }
  .stop-button.dim { background: transparent; color: rgba(221, 233, 255, 0.2); border-color: rgba(255, 255, 255, 0.05); cursor: default; }

  .strat-meta {
    display: flex;
    gap: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 1rem;
  }

  .ghost-link {
    background: none;
    border: none;
    color: rgba(221, 233, 255, 0.4);
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0;
  }

  .ghost-link:hover { color: white; }

  .log-panel-wrapper { grid-column: 1 / -1; flex: 1; display: flex; flex-direction: column; min-height: 0; }

  .config-form { display: grid; gap: 1rem; }
  label { display: grid; gap: 0.35rem; }
  label span { color: rgba(221, 233, 255, 0.6); font-size: 0.75rem; }
  input, select { background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 6px; color: white; padding: 0.5rem; font-size: 0.8rem; }
  .config-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .save-button { background: #22c55e; color: #052e16; border: none; padding: 0.4rem 1rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
  .cancel-button { background: transparent; color: white; border: none; cursor: pointer; font-size: 0.8rem; }

</style>
