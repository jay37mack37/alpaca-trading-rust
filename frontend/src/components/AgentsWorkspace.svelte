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
    BrokerPositionSummary,
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
  import Sparkline from "./Sparkline.svelte";
  import { api } from "../lib/api";
  import { prettyMoney, quantityDigits, structureLabel, contractLabel, legLabel, parseSymbols } from "../lib/format";
  import { validateStrategyDraft, type ValidationErrors } from "../lib/validation";
  import { type StrategyDraft, runIntervalToDraft, draftToRunIntervalMs } from "../lib/drafts";
  import type { DashboardResponse } from "../lib/types";

  export let strategies: StrategySummary[] = [];
  export let credentials: CredentialSummary[] = [];
  export let openPositions: PositionSummary[] = []; // Renamed to match prop in child
  export let brokerPositions: BrokerPositionSummary[] = [];
  export let recentTrades: any[] = [];
  export let logs: any[] = [];
  export let executionProfile: "standard" | "sniper_0dte" = "standard";

  const dispatch = createEventDispatcher<{
    create: CreateStrategyRequest;
    save: { strategyId: string; payload: UpdateStrategyRequest };
    run: { strategyId: string };
    inspect: { strategyId: string };
    sync: { strategyId: string };
    start: { strategyId: string };
    stop: { strategyId: string };
    panic: void;
  }>();

  async function panicAll() {
    if (!confirm("WARNING: This will STOP all agents and LIQUIDATE all positions on Alpaca. Are you sure?")) return;
    try {
      const resp = await fetch("/api/strategies/panic", { method: "POST" });
      if (resp.ok) {
        alert("Panic liquidation triggered. All agents stopped and positions closing.");
        dispatch("refresh");
      } else {
        const err = await resp.json().catch(() => ({}));
        alert(`Failed to force liquidate all positions: ${err.error || resp.statusText}. Check system logs.`);
      }
    } catch (e) {
      console.error("Panic failed:", e);
    }
  }

  let drafts: Record<string, StrategyDraft> = {};
  let draftErrors: Record<string, ValidationErrors> = {};
  let flipped: Record<string, boolean> = {};
  let consoleOpen: Record<string, boolean> = {};

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
    use_shared_cash: false,
    execution_profile: "standard",
  };
  let createKind: StrategyKind = "vwap_reflexive";
  let createErrors: ValidationErrors = {};

  $: if ((createKind as string) === "jarrod_vwap" && createDraft.tracked_symbols === "AAPL, SPY") {
    createDraft.tracked_symbols = "SPY";
    createDraft.asset_class_target = "options";
  }

  $: createErrors = validateStrategyDraft(createDraft);
  
  // SYNC CREATION DEFAULTS WITH PROFILE
  $: if (strategyProfile === 'sniper') {
    createDraft.execution_profile = 'sniper_0dte';
    if (createDraft.starting_cash === "25000") createDraft.starting_cash = "1000";
    if (createDraft.option_dte_min === "21") {
      createDraft.option_dte_min = "0";
      createDraft.option_dte_max = "0";
    }
  } else {
    createDraft.execution_profile = 'standard';
    if (createDraft.starting_cash === "1000") createDraft.starting_cash = "25000";
    if (createDraft.option_dte_min === "0") {
      createDraft.option_dte_min = "21";
      createDraft.option_dte_max = "45";
    }
  }

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
        use_shared_cash: strategy.use_shared_cash,
        execution_profile: strategy.execution_profile,
      };
      next[strategy.id] = draft;
      nextErrors[strategy.id] = validateStrategyDraft(draft);
    }
    drafts = next;
    draftErrors = nextErrors;
  }

  // --- STRATEGY ORGANIZATION (UNIFIED) ---
  $: activeStrategies = strategies.filter(s => s.id !== 'archived'); // placeholder if needed
  
  // Sort strategies: enabled first, then by name
  $: sortedStrategies = [...strategies].sort((a, b) => {
    if (a.enabled !== b.enabled) return a.enabled ? -1 : 1;
    return a.name.localeCompare(b.name);
  });

  $: filteredProfileLogs = logs;

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
      use_shared_cash: createDraft.use_shared_cash,
      execution_profile: createDraft.execution_profile,
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
      use_shared_cash: false,
      execution_profile: strategyProfile === 'sniper' ? 'sniper_0dte' : 'standard',
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
        use_shared_cash: draft.use_shared_cash,
        execution_profile: draft.execution_profile,
      },
    });
    flipped[strategyId] = false;
    draft.reset_portfolio = false;
  }

  function getDescriptionForKind(kind: StrategyKind) {
    switch (kind) {
      case "listing_arbitrage": return "Snipes new $SPY options via Black-Scholes valuation gaps and Kronos trend filtering.";
      case "vwap_reflexive": return "Automated entries on standard deviation price extensions from the VWAP.";
      case "rsi_mean_reversion": return "RSI Mean Reversion: Entering positions on overbought/oversold RSI conditions with Kronos confirmation.";
      case "sma_trend": return "SMA Trend: Following price momentum based on simple moving average crossovers.";
      case "zero_dte_neutral": return "0DTE Delta-Neutral: Capturing premium decay on same-day SPY expirations.";
      case "gamma_flip": return "Gamma Flip: Harvesting volatility regime shifts based on Net GEX neutral crossovers.";
      case "put_call_parity": return "Put-Call Parity: Arbitraging discrepancies between synthesized and market option prices.";
      case "parity_sniper": return "Parity Sniper: Specialized $S + P - C = K$ gap detector for exploitable option pricing discrepancies.";
      case "vwap_reversion": return "VWAP Reversion: Standard deviation 'Snap Back' strategy for over-extended price action.";
      case "jarrod_vwap": return "Jarrod VWAP Options: SPY calls/puts on VWAP reclaim/breakdown with 5% profit target.";
      case "yield_rotation": return "Yield Rotation: Harvesting risk-free yield in $SGOV during periods of strategy inactivity.";
      case "distribution_sniper": return "Distribution Sniper: Delta-neutral dividend capture strategy for REITs with ITM put hedging.";
      case "gamma_flip": return "Gamma Flip: High-convexity 0DTE option strategy triggered by zero-gamma crossovers.";
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
      case "rsi_mean_reversion": return "RSI Mean Reversion";
      case "sma_trend": return "SMA Trend";
      case "zero_dte_neutral": return "0DTE Delta Neutral";
      case "put_call_parity": return "Put-Call Parity";
      case "yield_rotation": return "Yield Rotation";
      case "distribution_sniper": return "Distribution Sniper";
      case "gamma_flip": return "Gamma Flip";
      default: return (kind as string).replaceAll("_", " ").replace(/\b\w/g, (letter) => letter.toUpperCase());
    }
  }

  let pendingStatus: Record<string, boolean> = {};
  let historicalLogs: Record<string, any[]> = {};
  let loadingLogs: Record<string, boolean> = {};

  async function fetchHistoricalLogs(strategyId: string) {
    if (historicalLogs[strategyId] || loadingLogs[strategyId]) return;
    loadingLogs[strategyId] = true;
    try {
      const dbLogs = await api.fetchStrategyLogs(strategyId);
      if (dbLogs) {
        historicalLogs[strategyId] = dbLogs.map(l => ({
          ...l,
          time: l.timestamp,
        }));
      }
    } catch (e) {
      console.error("Failed to fetch historical logs:", e);
    } finally {
      loadingLogs[strategyId] = false;
    }
  }

  function toggleConsole(strategyId: string) {
    const next = !consoleOpen[strategyId];
    consoleOpen[strategyId] = next;
    if (next) {
      fetchHistoricalLogs(strategyId);
    }
  }

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

  async function deleteStrategy(strategyId: string) {
    if (!confirm("Are you sure you want to delete this agent? All performance history will be lost.")) return;
    try {
      await api.delete(`/strategies/${strategyId}`);
      strategies = strategies.filter(s => s.id !== strategyId);
    } catch (e) {
      console.error("Failed to delete strategy", e);
    }
  }

  function getLogsForStrategy(strategy: StrategySummary) {
    const live = logs.filter(log => log.strategy_id === strategy.id);
    const historic = historicalLogs[strategy.id] || [];
    
    // Combine and sort by time descending
    const combined = [...live, ...historic];
    return combined.sort((a, b) => new Date(b.time).getTime() - new Date(a.time).getTime());
  }
  let activeView: "engines" | "positions" | "archive" = "engines";
  let compactMode = false;
  
  let strategyProfile: "pro" | "sniper" = "pro";
  $: strategyProfile = executionProfile === 'sniper_0dte' ? 'sniper' : 'pro';

  async function handleProfileChange(newVal: string) {
    const backendProfile = newVal === 'sniper' ? 'sniper_0dte' : 'standard';
    try {
      await fetch("/api/config/profile", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ profile: backendProfile })
      });
      // The heartbeat will pick up the change and update the UI globally
    } catch (e) {
      console.error("Failed to update global profile:", e);
    }
  }

  function getHistoricalPnl(strategyId: string) {
    const strategyTrades = recentTrades
      .filter(t => t.strategy_id === strategyId)
      .sort((a, b) => new Date(a.close_time).getTime() - new Date(b.close_time).getTime());
    
    let running = 0;
    const points = [0];
    for (const t of strategyTrades) {
      running += t.pnl;
      points.push(running);
    }
    // ensure at least 2 points for the sparkline
    if (points.length === 1) points.push(0);
    return points;
  }

  function getKellyRecommendation(strategyId: string, currentStartingCash: string) {
    const historicalTrades = recentTrades.filter(t => t.strategy_id === strategyId);
    if (historicalTrades.length < 5) return null; // need some sample size

    const wins = historicalTrades.filter(t => t.pnl > 0);
    const losses = historicalTrades.filter(t => t.pnl < 0);
    
    if (losses.length === 0) return "Aggressive (100% Kelly)"; // no losses yet

    const winRate = wins.length / historicalTrades.length;
    const avgWin = wins.reduce((sum, t) => sum + t.pnl, 0) / wins.length;
    const avgLoss = Math.abs(losses.reduce((sum, t) => sum + t.pnl, 0) / losses.length);
    
    const wlRatio = avgWin / avgLoss;
    const kellyFraction = winRate - (1 - winRate) / wlRatio;
    
    if (kellyFraction <= 0) return "Defensive (Scale Down)";
    
    const cash = parseFloat(currentStartingCash) || 25000;
    const recommended = cash * kellyFraction * 0.5; // Half-Kelly for safety
    return `Rec: $${recommended.toLocaleString(undefined, { maximumFractionDigits: 0 })} (Half-Kelly)`;
  }
</script>

<section class="workspace">
  <nav class="workstation-nav">
    <button class:active={activeView === 'engines'} on:click={() => activeView = 'engines'}>
      <span class="icon">🚀</span> Live Engines
    </button>
    <button class:active={activeView === 'positions'} on:click={() => activeView = 'positions'}>
      <span class="icon">🎯</span> Open Positions
      {#if openPositions.length > 0}
        <span class="badge">{openPositions.length}</span>
      {/if}
    </button>
    <button class:active={activeView === 'archive'} on:click={() => activeView = 'archive'}>
      <span class="icon">📜</span> Trade Archive
    </button>
  </nav>

  {#if activeView === "positions"}
    <div class="view-stage fade-in">
      <div class="section-header">
        <h2>Open Combat Positions</h2>
        <p>Active risk currently deployed in the market.</p>
      </div>
      <PositionsSection positions={openPositions} {brokerPositions} {strategies} />
    </div>
  {:else if activeView === "engines"}
    <section class="agents-layout fade-in">
      <div class="section-header engines-header">
      <div>
        <h2>Production Core</h2>
        <p>High-conviction, risk-managed execution engines filtered by GEX & Kronos AI.</p>
      </div>
      <div class="header-actions">
        <div class="profile-selector">
          <span class="profile-label">Execution Profile:</span>
          <div class="dropdown-wrapper">
            <select class="profile-dropdown" value={strategyProfile} on:change={(e) => handleProfileChange(e.target.value)}>
              <option value="pro">💼 Pro Mode (Standard)</option>
              <option value="sniper">🎯 0DTE Sniper (1k Base)</option>
            </select>
          </div>
          <button class="icon-button" on:click={() => compactMode = !compactMode} title="Toggle View Mode">
            {compactMode ? '📱' : '🖥️'}
          </button>
          <button class="panic-button" on:click={panicAll} title="LIQUIDATE ALL POSITIONS">
            ☢️ PANIC
          </button>
        </div>
      </div>
    </div>

    <div class="workstation-grid mb-4" class:compact={compactMode}>
      {#if sortedStrategies.length === 0}
        <div class="empty-state">No agents found. Create one to get started.</div>
      {/if}
      {#each sortedStrategies as strategy}
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
                <span>Execution Profile</span>
                <select bind:value={drafts[strategy.id].execution_profile}>
                  <option value="standard">💼 Standard (Pro)</option>
                  <option value="sniper_0dte">🎯 Sniper (0DTE/1k)</option>
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
                <label>
                  <span>Target delta</span>
                  <input type="number" step="0.01" class:invalid={draftErrors[strategy.id]?.option_target_delta} bind:value={drafts[strategy.id].option_target_delta} />
                </label>
              {/if}

              <div class="config-actions">
                <button class="save-button" on:click={() => save(strategy.id)}>Save changes</button>
                <button class="cancel-button" on:click={() => flipped[strategy.id] = false}>Cancel</button>
              </div>
            </div>
          {:else}
            <div class="strat-header">
              <div class="strat-title">
                <div class="category-tag-row">
                  <div class="category-tag" class:passive={strategy.kind === 'yield_rotation'} class:daily={['parity_sniper', 'vwap_reversion'].includes(strategy.kind)} class:aggressive={['zero_dte_neutral', 'gamma_flip'].includes(strategy.kind)}>
                    {labelForKind(strategy.kind).toUpperCase()}
                  </div>
                  {#if strategy.execution_profile === 'sniper_0dte'}
                    <div class="profile-badge sniper">0DTE</div>
                  {:else}
                    <div class="profile-badge standard">PRO</div>
                  {/if}
                </div>
                <h3>{strategy.name}</h3>
                <p class="strat-desc">{strategy.description || getDescriptionForKind(strategy.kind)}</p>
              </div>
              <div class="status-indicator" class:active={getEffectiveEnabled(strategy)}>
                <span class="dot"></span>
                {getEffectiveEnabled(strategy) ? 'Active' : 'Idle'}
              </div>
            </div>

            <div class="pot-status">
              <div class="pot-label">Execution Pot</div>
              <div class="pot-value">{prettyMoney(strategy.cash_balance)} / {prettyMoney(strategy.starting_cash)}</div>
              <div class="pot-bar">
                <div class="pot-fill" style="width: {Math.min(100, (strategy.cash_balance / strategy.starting_cash) * 100)}%"></div>
              </div>
            </div>

            <div class="strat-actions">
              <button class="run-button" class:dim={getEffectiveEnabled(strategy)} on:click={() => toggleAgent(strategy)}>EXECUTE</button>
              <button class="stop-button" class:dim={!getEffectiveEnabled(strategy)} on:click={() => toggleAgent(strategy)}>STOP</button>
            </div>
            
            <div class="strat-meta">
              <button class="ghost-link" class:active={consoleOpen[strategy.id]} on:click={() => toggleConsole(strategy.id)}>Console</button>
              <button class="ghost-link" on:click={() => dispatch("inspect", { strategyId: strategy.id })}>Analytics</button>
              <button class="ghost-link" on:click={() => { flipped[strategy.id] = true }}>Config</button>
            </div>
          {/if}
        </article>
      {/each}
    </div>
    <div class="section-header mt-4">
      <h2>Intelligence Feed</h2>
      <p>Unified telemetry across all active execution engines.</p>
    </div>
    <div class="log-panel-wrapper mb-4">
      <StrategyLogTable logs={filteredProfileLogs} />
    </div>
  </section>
  {:else if activeView === "archive"}
    <section class="archive-layout fade-in">
      <div class="section-header">
        <h2>Trade Archive</h2>
        <p>Historical execution logs and system audit trail.</p>
      </div>
      <RecentTradesSection trades={recentTrades} />
    </section>
  {/if}
</section>

<style>
  .workspace {
    padding: 0.5rem 1.5rem 1.5rem;
    max-width: 1400px;
    margin: 0 auto;
    color: white;
  }

  .workstation-nav {
    display: flex;
    gap: 1rem;
    margin-bottom: 2rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    padding-bottom: 0.5rem;
  }

  .workstation-nav button {
    background: none;
    border: none;
    color: rgba(221, 233, 255, 0.4);
    font-size: 0.9rem;
    font-weight: 500;
    padding: 0.75rem 1rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-radius: 8px;
    transition: all 0.2s;
  }

  .workstation-nav button:hover {
    color: white;
    background: rgba(255, 255, 255, 0.03);
  }

  .workstation-nav button.active {
    color: #22c55e;
    background: rgba(34, 197, 94, 0.05);
  }

  .icon { font-size: 1.1rem; }
  
  .badge {
    background: #22c55e;
    color: #052e16;
    font-size: 0.7rem;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 10px;
    margin-left: 4px;
  }

  .view-stage, .agents-layout, .archive-layout {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .fade-in {
    animation: fadeIn 0.3s ease-out;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateY(5px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .panic-button {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    padding: 0.5rem 1rem;
    border-radius: 8px;
    font-size: 0.75rem;
    font-weight: 800;
    cursor: pointer;
    transition: all 0.2s;
    margin-left: 0.5rem;
  }

  .panic-button:hover {
    background: #ef4444;
    color: white;
    box-shadow: 0 0 15px rgba(239, 68, 68, 0.4);
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
  .category-tag-row {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-bottom: 8px;
  }
  
  .profile-badge {
    font-size: 0.65rem;
    font-weight: 800;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
  }
  .profile-badge.sniper {
    background: rgba(255, 0, 0, 0.2);
    color: #ff4d4d;
    border: 1px solid rgba(255, 0, 0, 0.3);
  }
  .profile-badge.standard {
    background: rgba(0, 255, 0, 0.1);
    color: #4ade80;
    border: 1px solid rgba(0, 255, 0, 0.2);
  }
  .stop-button { background: rgba(239, 68, 68, 0.1); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.2); }
  .stop-button.dim { background: transparent; color: rgba(221, 233, 255, 0.2); border-color: rgba(255, 255, 255, 0.05); cursor: default; }

  .strat-meta {
    display: flex;
    gap: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 1rem;
  }

  .category-tag {
    font-size: 0.6rem;
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.5rem;
    padding: 2px 6px;
    border-radius: 4px;
    width: fit-content;
  }
  .category-tag.passive { background: rgba(34, 197, 94, 0.1); color: #22c55e; }
  .category-tag.daily { background: rgba(34, 211, 238, 0.1); color: var(--accent-cyan); }
  .category-tag.aggressive { background: rgba(251, 191, 36, 0.1); color: var(--accent-gold); }

  .pot-status {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 8px;
    padding: 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.03);
  }

  .pot-label {
    font-size: 0.65rem;
    color: rgba(255, 255, 255, 0.4);
    text-transform: uppercase;
    margin-bottom: 4px;
  }

  .pot-value {
    font-size: 1rem;
    font-weight: 700;
    margin-bottom: 8px;
  }
  .pot-value.low { color: #ef4444; }

  .pot-bar {
    height: 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 2px;
    overflow: hidden;
  }

  .pot-fill {
    height: 100%;
    background: #22c55e;
    transition: width 0.5s ease-out;
  }
  .pot-fill.aggressive { background: var(--accent-gold); }

  .run-button.aggressive {
    background: var(--accent-gold);
    color: #000;
  }

  .log-panel-wrapper { grid-column: 1 / -1; flex: 1; display: flex; flex-direction: column; min-height: 0; }

  .config-form { display: grid; gap: 1rem; }
  label { display: grid; gap: 0.35rem; }
  label span { color: rgba(221, 233, 255, 0.6); font-size: 0.75rem; }
  input, select { background: rgba(0, 0, 0, 0.3); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 6px; color: white; padding: 0.5rem; font-size: 0.8rem; }
  .config-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 0.5rem; }
  .save-button { background: #22c55e; color: #052e16; border: none; padding: 0.4rem 1rem; border-radius: 4px; cursor: pointer; font-size: 0.8rem; }
  .cancel-button { background: transparent; color: white; border: none; cursor: pointer; font-size: 0.8rem; }

  .engines-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 1rem;
    margin-bottom: 0.5rem;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .profile-selector {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    background: rgba(255, 255, 255, 0.03);
    padding: 6px 12px;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .profile-label {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.4);
  }

  .profile-dropdown {
    background: #1a1b1e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: white;
    font-size: 0.85rem;
    font-weight: 500;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    outline: none;
    transition: all 0.2s;
  }

  .profile-dropdown:hover {
    border-color: #22c55e;
    background: #25262b;
  }

  .icon-button {
    background: none;
    border: none;
    font-size: 1.1rem;
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    transition: background 0.2s;
  }

  .icon-button:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .toggle-button {
    background: rgba(221, 233, 255, 0.05);
    border: 1px solid rgba(221, 233, 255, 0.1);
    color: rgba(221, 233, 255, 0.8);
    padding: 0.5rem 1rem;
    border-radius: 8px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .toggle-button:hover {
    background: rgba(221, 233, 255, 0.1);
    color: white;
  }

  .workstation-grid.compact {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .workstation-grid.compact .strat-card {
    display: grid;
    grid-template-columns: 1.5fr 1fr 1fr auto;
    align-items: center;
    padding: 0.75rem 1.5rem;
    gap: 2rem;
  }

  .workstation-grid.compact .strat-desc {
    display: none;
  }

  .workstation-grid.compact .strat-actions {
    margin: 0;
    display: flex;
    flex-direction: row;
    gap: 8px;
  }

  .workstation-grid.compact h3 {
    font-size: 1rem;
    margin: 0;
  }

  .workstation-grid.compact .status-indicator {
    padding: 4px 8px;
    font-size: 0.75rem;
  }

  .workstation-grid.compact .strat-meta {
    border: none;
    padding: 0;
    margin: 0;
  }

</style>
