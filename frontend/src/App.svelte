<script lang="ts">
  import { onMount } from "svelte";
  import DiagnosticHeader from "./components/DiagnosticHeader.svelte";
  import AgentsWorkspace from "./components/AgentsWorkspace.svelte";
  import InteractiveTicker from "./components/InteractiveTicker.svelte";
  import MetricTile from "./components/MetricTile.svelte";
  import OptionsPanel from "./components/OptionsPanel.svelte";
  import StrategyLogTable from "./components/StrategyLogTable.svelte";
  import AnalyticsWorkspace from "./components/AnalyticsWorkspace.svelte";
  import { api, apiTokenConfigured, fetchSetupStatus } from "./lib/api";
  import WelcomePage from "./components/WelcomePage.svelte";
  import { prettyMoney, prettyPct, quantityDigits, structureLabel, contractLabel, legLabel } from "./lib/format";
  import type {
    CreateCredentialRequest,
    CreateStrategyRequest,
    DashboardResponse,
    DataProvider,
    RealtimeEvent,
    StrategyDetailResponse,
    TradeRecord,
    UpdateStrategyRequest,
  } from "./lib/types";

  let page: "market" | "workstation" | "remodeling" | "analytics" = "market";
  let setupMode: "checking" | "welcome" | "dashboard" = "checking";
  let symbol = "SPY";
  let symbolDraft = "SPY";
  let provider: DataProvider = "yahoo";
  let dashboard: DashboardResponse | null = null;
  let selectedStrategyId = "";
  let selectedStrategyDetail: StrategyDetailResponse | null = null;
  let detailLoading = false;
  let loading = false;
  let error = "";
  let status = "";
  let vwapGap: number | null = null;
  let mounted = false;
  let stream: EventSource | null = null;
  let streamKey = "";
  let streamState: "idle" | "connecting" | "live" | "reconnecting" = "idle";
  let openPositions: any[] = [];
  let strategyLogs: Array<{
    time: string;
    symbol: string;
    source: string;
    math_edge: string;
    ai_score: string;
    decision: string;
    narrative: string;
  }> = [];
  let heartbeatBuyingPower: number = 0;
  let heartbeatsProcessed: number = 0;
  let lastHeartbeatAt: number = 0;
  let kronosLatency: number = 0;
  let kronosActive: boolean = false;
  let alpacaActive: boolean = false;
  let optionsActive: boolean = false;

  function computeIntradayVwapGap() {
    if (!dashboard) return null;
    if (dashboard.quote.vwap) {
      return ((dashboard.quote.price - dashboard.quote.vwap) / dashboard.quote.vwap) * 100;
    }
    if (!dashboard.candles.length) return null;

    let cumulativePriceVolume = 0;
    let cumulativeVolume = 0;
    for (const candle of dashboard.candles) {
      if (!candle.volume) continue;
      cumulativePriceVolume += ((candle.high + candle.low + candle.close) / 3) * candle.volume;
      cumulativeVolume += candle.volume;
    }
    if (!cumulativeVolume) return null;
    const vwap = cumulativePriceVolume / cumulativeVolume;
    return ((dashboard.quote.price - vwap) / vwap) * 100;
  }

  $: vwapGap = dashboard ? computeIntradayVwapGap() : null;

  function upsertCandle(
    existing: DashboardResponse["candles"],
    incoming: DashboardResponse["candles"][number] | null,
  ) {
    if (!incoming) return existing;
    const index = existing.findIndex((candle) => candle.timestamp === incoming.timestamp);
    if (index >= 0) {
      const copy = existing.slice();
      copy[index] = {
        ...copy[index],
        high: Math.max(copy[index].high, incoming.high),
        low: Math.min(copy[index].low, incoming.low),
        close: incoming.close,
        volume: copy[index].volume + incoming.volume,
        vwap: incoming.vwap ?? copy[index].vwap,
      };
      return copy;
    }
    return [...existing, incoming].sort((left, right) => Date.parse(left.timestamp) - Date.parse(right.timestamp));
  }

  function patchSelectedStrategy(strategies: DashboardResponse["strategies"]) {
    if (!selectedStrategyDetail) return;
    const next = strategies.find((strategy) => strategy.id === selectedStrategyDetail?.strategy.id);
    if (next) {
      selectedStrategyDetail = {
        ...selectedStrategyDetail,
        strategy: next,
      };
    }
  }

  function handleRealtimeEvent(event: RealtimeEvent) {
    switch (event.type) {
      case "positions":
        openPositions = event.positions;
        break;
      case "market":
        if (!dashboard || dashboard.symbol !== event.symbol || dashboard.provider !== event.provider) return;
        dashboard = {
          ...dashboard,
          quote: event.quote,
          candles: upsertCandle(dashboard.candles, event.candle),
          strategies: event.strategies,
        };
        patchSelectedStrategy(event.strategies);
        break;

      case "broker_sync":
        if (dashboard) {
          dashboard = {
            ...dashboard,
            strategies: event.strategies,
          };
        }
        patchSelectedStrategy(event.strategies);
        if (
          selectedStrategyDetail &&
          (event.strategy_ids.includes(selectedStrategyDetail.strategy.id) ||
            selectedStrategyDetail.strategy.credential_id === event.credential_id)
        ) {
          selectedStrategyDetail = {
            ...selectedStrategyDetail,
            strategy:
              event.strategies.find((strategy) => strategy.id === selectedStrategyDetail?.strategy.id) ??
              selectedStrategyDetail.strategy,
            broker_sync: event.broker_sync,
          };
        }
        if (event.event) {
          status = `Broker update: ${event.event.replaceAll("_", " ")}`;
        }
        break;

      case "log":
        strategyLogs = [event, ...strategyLogs].slice(0, 200);
        break;

      case "heartbeat":
        heartbeatBuyingPower = event.buying_power;
        lastHeartbeatAt = Date.now();
        kronosLatency = Math.max(0, lastHeartbeatAt - event.timestamp);
        kronosActive = event.kronos_active;
        alpacaActive = event.alpaca_active;
        optionsActive = event.options_active;
        heartbeatsProcessed++;
        break;

      case "status":
        streamState = event.state === "live" ? "live" : event.state === "connecting" ? "connecting" : "reconnecting";
        if (event.state === "failed") {
          error = event.message;
        }
        break;

      case "system": {
        const { timestamp, strategy, symbol, edge, ai_confirmation, message } = event.event;
        strategyLogs = [{
          time: timestamp,
          symbol,
          source: strategy,
          math_edge: `${(edge * 100).toFixed(1)}%`,
          ai_score: ai_confirmation.toFixed(2),
          decision: edge > 0.01 ? "Opportunity" : "Scanning",
          narrative: message
        }, ...strategyLogs].slice(0, 200);
        break;
      }

      case "system_log": {
        const { timestamp, source, symbol, event_type, math_context, ai_confidence, narrative } = event.event;
        strategyLogs = [{
          time: timestamp,
          symbol,
          source: source,
          math_edge: math_context,
          ai_score: ai_confidence.toFixed(2),
          decision: event_type,
          narrative: narrative
        }, ...strategyLogs].slice(0, 200);
        break;
      }
    }
  }

  function closeStream() {
    stream?.close();
    stream = null;
    streamKey = "";
    if (streamState !== "idle") {
      streamState = "idle";
    }
  }

  function connectStream() {
    if (!dashboard) return;
    const nextKey = [dashboard.symbol, dashboard.provider, selectedStrategyId].join(":");
    if (stream && streamKey === nextKey) return;

    closeStream();
    streamKey = nextKey;
    streamState = "connecting";
    stream = new EventSource(api.streamUrl(dashboard.symbol, dashboard.provider, selectedStrategyId || undefined));
    stream.addEventListener("market", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("broker_sync", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("status", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("log", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("heartbeat", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("system", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("system_log", (message) => {
      handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.addEventListener("positions", (message) => {
       handleRealtimeEvent(JSON.parse(message.data) as RealtimeEvent);
    });
    stream.onopen = () => {
      streamState = "live";
      error = "";
    };
    stream.onerror = () => {
      if (streamState !== "idle") {
        streamState = "reconnecting";
      }
    };
  }

  async function loadDashboard(skipDetailLoad = false) {
    loading = true;
    error = "";
    try {
      dashboard = await api.dashboard(symbol, provider);
      symbolDraft = dashboard.symbol;
      if (!skipDetailLoad && dashboard.strategies.length > 0) {
        const nextSelected =
          dashboard.strategies.find((strategy) => strategy.id === selectedStrategyId)?.id ??
          dashboard.strategies.find((strategy) => strategy.credential_id)?.id ??
          dashboard.strategies[0]?.id;
        if (nextSelected) {
          await loadStrategyDetail(nextSelected);
        }
      }
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load dashboard";
    } finally {
      loading = false;
    }
  }

  async function loadStrategyDetail(strategyId: string) {
    selectedStrategyId = strategyId;
    detailLoading = true;
    try {
      selectedStrategyDetail = await api.strategyDetail(strategyId);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load strategy detail";
    } finally {
      detailLoading = false;
    }
  }

  async function createStrategy(event: CustomEvent<CreateStrategyRequest>) {
    try {
      const created = await api.createStrategy(event.detail);
      status = `${created.name} created and enabled.`;
      page = "workstation";
      await Promise.all([loadDashboard(true), loadStrategyDetail(created.id)]);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to create agent";
    }
  }

  async function saveStrategy(event: CustomEvent<{ strategyId: string; payload: UpdateStrategyRequest }>) {
    try {
      await api.updateStrategy(event.detail.strategyId, event.detail.payload);
      status = "Agent settings saved.";
      await Promise.all([loadDashboard(true), loadStrategyDetail(event.detail.strategyId)]);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to update agent";
    }
  }

  async function runStrategy(event: CustomEvent<{ strategyId: string }>) {
    try {
      const trade = await api.runStrategy(event.detail.strategyId, symbol);
      status = trade
        ? `Executed ${trade.side} ${contractLabel(trade)} at ${trade.price.toFixed(2)}`
        : "No trade was generated on this run.";
      await Promise.all([loadDashboard(true), loadStrategyDetail(event.detail.strategyId)]);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to run agent";
    }
  }

  async function inspectStrategy(event: CustomEvent<{ strategyId: string }>) {
    await loadStrategyDetail(event.detail.strategyId);
  }

  async function syncStrategy(event: CustomEvent<{ strategyId: string }>) {
    try {
      await api.syncStrategy(event.detail.strategyId);
      status = "Alpaca state synced.";
      await Promise.all([loadDashboard(true), loadStrategyDetail(event.detail.strategyId)]);
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to sync Alpaca state";
    }
  }

  async function storeCredential(event: CustomEvent<CreateCredentialRequest>) {
    try {
      await api.createCredential(event.detail);
      status = "Credential stored.";
      await loadDashboard();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to store credential";
    }
  }

  async function applySymbol() {
    const normalized = symbolDraft.trim().toUpperCase() || "SPY";
    if (normalized === symbol) {
      return;
    }
    symbol = normalized;
    await loadDashboard();
  }

  async function selectWatchSymbol(nextSymbol: string) {
    symbol = nextSymbol;
    symbolDraft = nextSymbol;
    await loadDashboard();
  }

  async function changeProvider(nextProvider: DataProvider) {
    provider = nextProvider;
    await loadDashboard();
  }

  async function globalPanic() {
    try {
      await api.panic();
      status = "Global panic executed. All strategies stopped and orders cancelled.";
      error = "";
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to execute global panic";
    }
  }

  onMount(() => {
    mounted = true;

    let interval: ReturnType<typeof setInterval> | undefined;

    void (async () => {
      if (!apiTokenConfigured()) {
        setupMode = "welcome";
        return;
      }

      try {
        const status = await fetchSetupStatus();
        if (!status.has_credentials) {
          setupMode = "welcome";
          return;
        }
      } catch {
        // Backend unreachable but token configured — show dashboard
      }

      setupMode = "dashboard";
      void loadDashboard();
      interval = setInterval(() => {
        void loadDashboard();
      }, 120000);
    })();

    return () => {
      mounted = false;
      closeStream();
      if (interval) clearInterval(interval);
    };
  });

  $: if (mounted && dashboard) {
    connectStream();
  }

  let totalBuyingPower = 0;
  $: {
    if (dashboard) {
      totalBuyingPower = heartbeatBuyingPower || dashboard.strategies.reduce((acc, s) => acc + (s.broker_buying_power ?? 0), 0);
    }
  }
</script>

<svelte:head>
  <title>AutoStonks Algo Suite</title>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
</svelte:head>

<main class="shell">
  {#if setupMode === "checking"}
    <div class="banner status" role="status" aria-live="polite">Checking setup...</div>
  {:else if setupMode === "welcome"}
    <WelcomePage on:complete={() => { setupMode = "dashboard"; void loadDashboard(); }} />
  {:else}
  <DiagnosticHeader
    alpacaActive={alpacaActive}
    kronosActive={kronosActive}
    optionsActive={optionsActive}
    buyingPower={heartbeatBuyingPower || totalBuyingPower}
    kronosLatency={kronosLatency}
  />
  <header class="topbar">
    <div>
      <p class="eyebrow">AutoStonks Control Room</p>
      <h1>Live ticker center stage. Agents on their own deck.</h1>
    </div>
    <div class="header-controls">
      <nav class="tab-strip" aria-label="Primary">
        <button class:active={page === "market"} type="button" on:click={() => (page = "market")}>Market</button>
        <button class:active={page === "workstation"} type="button" on:click={() => (page = "workstation")}>Workstation</button>
        <button class:active={page === "analytics"} type="button" on:click={() => (page = "analytics")}>Analytics</button>
      </nav>
      <button type="button" class="panic-button" on:click={globalPanic} title="Stop all running strategies">
        ⚠️ Global Panic
      </button>
    </div>
  </header>

  {#if !apiTokenConfigured()}
    <div class="banner error">
      VITE_API_TOKEN is not set. Copy the token printed by the backend on first
      start into <code>frontend/.env</code> as <code>VITE_API_TOKEN=&lt;token&gt;</code>
      and restart the Vite dev server.
    </div>
  {/if}
  {#if error}
    <div class="banner error" role="status" aria-live="polite" aria-atomic="true">{error}</div>
  {/if}
  {#if status}
    <div class="banner status" role="status" aria-live="polite" aria-atomic="true">{status}</div>
  {/if}

  {#if dashboard}
    {#if page === "market"}
      <section class="market-layout">
        <aside class="market-rail">
          <section class="rail-card">
            <p class="rail-label">Ticker Control</p>
            <label>
              <span>Symbol</span>
              <input id="market-symbol" name="market_symbol" bind:value={symbolDraft} on:keydown={(event) => event.key === "Enter" && void applySymbol()} />
            </label>
            <label>
              <span>Provider</span>
              <select id="market-provider" name="market_provider" bind:value={provider} on:change={() => void changeProvider(provider)}>
                <option value="yahoo">Yahoo</option>
                <option value="alpaca">Alpaca</option>
              </select>
            </label>
            <button type="button" on:click={applySymbol}>Load ticker</button>
          </section>

          <section class="rail-card">
            <p class="rail-label">Watchlist</p>
            <div class="watchlist">
              {#each dashboard.tracked_symbols as tracked}
                <button class:active={tracked === symbol} type="button" on:click={() => void selectWatchSymbol(tracked)}>
                  {tracked}
                </button>
              {/each}
            </div>
          </section>

          <section class="rail-card rail-card--status">
            <p class="rail-label">Status</p>
            <div class={`stream-indicator stream-indicator--${streamState}`}>Realtime {streamState}</div>
            <div class="status-grid">
              <article><span>Source</span><strong>{dashboard.provider}</strong></article>
              <article><span>Collector</span><strong>{dashboard.collector_interval_seconds > 0 ? `${dashboard.collector_interval_seconds}s` : "manual"}</strong></article>
              <article><span>Agents</span><strong>{dashboard.strategies.filter((strategy) => strategy.enabled).length} running</strong></article>
              <article><span>Options</span><strong>{dashboard.options.length} cached</strong></article>
            </div>
          </section>
        </aside>

        <section class="market-stage">
          <div class="ticker-stage">
            <p class="stage-kicker">Live ticker board</p>
            <h2>{dashboard.symbol}</h2>
            <div class="stage-price">{prettyMoney(dashboard.quote.price)}</div>
            <div class:positive={dashboard.quote.change_percent != null && dashboard.quote.change_percent >= 0} class:negative={dashboard.quote.change_percent != null && dashboard.quote.change_percent < 0} class="stage-change">
              {prettyPct(dashboard.quote.change_percent)}
            </div>
            <p class="stage-meta">
              High {prettyMoney(dashboard.quote.session_high)} · Low {prettyMoney(dashboard.quote.session_low)} · Volume {dashboard.quote.volume ? dashboard.quote.volume.toLocaleString() : "—"}
            </p>
          </div>

          <InteractiveTicker symbol={dashboard.symbol} candles={dashboard.candles} />

          <section class="metrics">
            <MetricTile
              label="VWAP Gap"
              value={prettyPct(vwapGap)}
              detail={dashboard.quote.vwap ? `VWAP ${prettyMoney(dashboard.quote.vwap)}` : "Derived from intraday candles"}
              tone={vwapGap != null && vwapGap >= 0 ? "positive" : "negative"}
            />
            <MetricTile
              label="Session Volume"
              value={dashboard.quote.volume ? dashboard.quote.volume.toLocaleString() : "—"}
              detail={`Provider ${dashboard.provider}`}
            />
            <MetricTile
              label="Active Agents"
              value={`${dashboard.strategies.filter((strategy) => strategy.enabled).length}`}
              detail={`${dashboard.strategies.length} total configured`}
            />
          </section>

          <section class="market-foot">
            <aside class="activity-panel">
              <div class="panel-header">
                <div>
                  <p>Recent activity</p>
                  <h2>Trade log</h2>
                </div>
              </div>

              <div class="trade-list">
                {#each dashboard.recent_trades as trade}
                  <article>
                    <header>
                      <strong>{contractLabel(trade)}</strong>
                      <span class:buy={trade.side === "buy"} class:sell={trade.side === "sell"}>
                        {trade.side}
                      </span>
                    </header>
                    <p>{trade.reason}</p>
                    <small>
                      {trade.quantity.toFixed(quantityDigits(trade.asset_type))} @ ${trade.price.toFixed(2)} · {trade.execution_mode.replaceAll("_", " ")}
                    </small>
                    {#if trade.legs.length > 0}
                      <small>
                        {trade.legs.map((leg) => `${leg.position_intent ?? leg.side} ${legLabel(leg) || leg.instrument_symbol}`).join(" | ")}
                      </small>
                    {/if}
                  </article>
                {/each}
              </div>
            </aside>

            <OptionsPanel options={dashboard.options} />
          </section>
        </section>
      </section>
    {:else if page === "workstation"}
      <section class="workstation-page">
        <AgentsWorkspace
          strategies={dashboard.strategies}
          credentials={dashboard.credentials}
          positions={openPositions}
          recentTrades={dashboard.recent_trades}
          logs={strategyLogs}
          on:create={createStrategy}
          on:save={saveStrategy}
          on:run={runStrategy}
          on:inspect={inspectStrategy}
          on:sync={syncStrategy}
          on:start={async () => { await loadDashboard(true); }}
          on:stop={async () => { await loadDashboard(true); }}
        />
      </section>
    {:else if page === "analytics"}
      <section class="analytics-page">
        <AnalyticsWorkspace />
      </section>
    {/if}
  {:else if loading}
    <div class="banner status" role="status" aria-live="polite" aria-atomic="true">Loading market board…</div>
  {/if}
  {/if}
</main>
