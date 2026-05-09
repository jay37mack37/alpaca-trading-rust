<script lang="ts">
  import type { PositionSummary } from "../lib/types";
  import { api } from "../lib/api";
  import { prettyMoney } from "../lib/format";
  import { createEventDispatcher, onMount } from "svelte";

  export let position: PositionSummary;
  export let strategyName: string = "";

  const dispatch = createEventDispatcher();

  let flattening = false;
  let now = Date.now();

  $: pnlPercent = (position.unrealized_pnl / (position.average_price * Math.abs(position.quantity) * position.multiplier)) * 100;
  $: isNearStop = position.razor_stop ? Math.abs(position.market_price - position.razor_stop) / position.market_price < 0.001 : false;

  $: stagnationTotal = 300; // 5 minutes
  $: stagnationSeconds = position.stagnation_timestamp ? Math.max(0, Math.floor((new Date(position.stagnation_timestamp).getTime() - now) / 1000)) : null;
  $: stagnationProgress = stagnationSeconds !== null ? Math.min(100, Math.max(0, ((stagnationTotal - stagnationSeconds) / stagnationTotal) * 100)) : 0;
  $: isStagnant = stagnationProgress >= 100;

  $: entryTimeStr = position.entry_time ? new Date(position.entry_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false }) : '--:--:--';

  $: marketValue = position.market_price * Math.abs(position.quantity) * position.multiplier;
  $: pnlCash = position.unrealized_pnl;

  $: plannedExitMessage = (() => {
    if (isStagnant) return "Time-Out Exit Imminent";
    if (isNearStop) return "Stop Loss Near (Razor)";
    if (position.take_profit && position.market_price >= position.take_profit) return "Target Reached - Executing Exit";
    if (position.take_profit) return `Targeting ${prettyMoney(position.take_profit)}`;
    return "Standard Risk Exit";
  })();

  onMount(() => {
    const timer = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(timer);
  });

  async function handleFlatten() {
    if (!confirm(`Flatten position in ${position.instrument_symbol}?`)) return;
    flattening = true;
    try {
      await api.flattenPosition(position.strategy_id, position.instrument_symbol);
      dispatch("flattened", position.instrument_symbol);
    } catch (err) {
      console.error("Flatten failed:", err);
      alert("Flatten failed: " + err);
    } finally {
      flattening = false;
    }
  }

  function getPnlColor(pnl: number) {
    if (pnl <= -1) return "#ff4d4d"; // Deep Red
    if (pnl < 0) return "#fb7185";  // Light Red
    if (pnl >= 2) return "#00ff66";  // Neon Green
    if (pnl > 0) return "#a7f3d0";  // Light Green
    return "white";
  }

  function getSentimentColor(val: number) {
    if (val > 0.7) return "#22c55e";
    if (val > 0.4) return "#eab308";
    return "#ef4444";
  }

  function getStrategyLabel(id: string) {
    if (strategyName && strategyName !== id) return strategyName;
    return id.split('-').map(s => s.charAt(0).toUpperCase() + s.slice(1)).join(' ');
  }

  // Price Tracker logic
  $: lowBound = Math.min(position.razor_stop ?? position.market_price, position.take_profit ?? position.market_price, position.average_price);
  $: highBound = Math.max(position.razor_stop ?? position.market_price, position.take_profit ?? position.market_price, position.average_price);
  $: trackerRange = highBound - lowBound;
  $: markerPos = trackerRange > 0 ? ((position.market_price - lowBound) / trackerRange) * 100 : 50;
  $: stopPos = trackerRange > 0 ? (((position.razor_stop ?? lowBound) - lowBound) / trackerRange) * 100 : 0;
  $: entryPos = trackerRange > 0 ? ((position.average_price - lowBound) / trackerRange) * 100 : 50;
  $: targetPos = trackerRange > 0 ? (((position.take_profit ?? highBound) - lowBound) / trackerRange) * 100 : 100;
</script>

<div class="pos-card" class:shake={isNearStop} class:danger={isNearStop} class:stagnant={isStagnant}>
  <div class="card-header">
    <div class="symbol-info">
      <span class="symbol">{position.instrument_symbol}</span>
      <span class="strategy-type">{getStrategyLabel(position.strategy_id)}</span>
    </div>
    <div class="side" class:long={position.quantity > 0} class:short={position.quantity < 0}>
      {position.quantity > 0 ? 'LONG' : 'SHORT'}
    </div>
  </div>

  <!-- Alpaca Core Metrics -->
  <div class="metrics-grid">
    <div class="metric-item">
      <span class="label">MARKET PRICE</span>
      <span class="value main">{prettyMoney(position.market_price)}</span>
    </div>
    <div class="metric-item">
      <span class="label">QUANTITY</span>
      <span class="value main">{position.quantity}</span>
    </div>
    <div class="metric-item">
      <span class="label">TOTAL P/L ($)</span>
      <span class="value pnl main" style="color: {getPnlColor(pnlPercent)}">
        {pnlCash >= 0 ? '+' : '-'}{prettyMoney(Math.abs(pnlCash))}
      </span>
    </div>
    <div class="metric-item">
      <span class="label">P/L (%)</span>
      <span class="value" style="color: {getPnlColor(pnlPercent)}">
        {pnlPercent >= 0 ? '+' : ''}{pnlPercent.toFixed(2)}%
      </span>
    </div>
  </div>

  <!-- Trade Intelligence: Foundation -->
  <div class="intel-section foundation">
    <div class="section-label">TRADE FOUNDATION</div>
    <div class="intel-grid">
      <div class="intel-item">
        <span class="sub-label">BUY LOGIC</span>
        <span class="intel-value">{position.buy_logic || 'Manual / Initialized'}</span>
      </div>
      <div class="intel-item">
        <span class="sub-label">HOLD INTENTION</span>
        <span class="intel-value">{position.hold_intent || 'Standard Risk Adjusted'}</span>
      </div>
      <div class="intel-item">
        <span class="sub-label">MATH EDGE / AI</span>
        <span class="intel-value">
          {position.entry_math || 'N/A'} 
          {position.entry_ai ? `(AI: ${position.entry_ai.toFixed(2)})` : ''}
        </span>
      </div>
    </div>
  </div>

  <!-- Live Telemetry: Kronos -->
  <div class="telemetry-bar">
    <div class="kronos-info">
      <span class="label">LIVE KRONOS SCORE</span>
      <span class="value" style="color: {getSentimentColor(position.kronos_sentiment ?? 0.5)}">
        {(position.kronos_sentiment ?? 0.0).toFixed(4)}
      </span>
    </div>
    <div class="gauge">
      <div class="gauge-fill" style="width: {(position.kronos_sentiment ?? 0.5) * 100}%; background: {getSentimentColor(position.kronos_sentiment ?? 0.5)}"></div>
    </div>
  </div>

  <!-- Exit Strategy & Intelligence -->
  <div class="intel-section exit" class:danger={isStagnant || isNearStop}>
    <div class="section-label">EXIT INTELLIGENCE</div>
    <div class="exit-banner">
      <div class="exit-info">
        <span class="sub-label">EXIT LOGIC: {position.exit_logic ?? 'SMART DISCONNECT'}</span>
        <span class="banner-timer" class:pulse={isStagnant}>
          {isStagnant ? 'DEADLINE REACHED' : (stagnationSeconds !== null ? `T-MINUS ${Math.floor(stagnationSeconds / 60)}:${(stagnationSeconds % 60).toString().padStart(2, '0')}` : 'STABLE')}
        </span>
      </div>
      <div class="planned-action">{position.planned_exit || plannedExitMessage}</div>
    </div>
  </div>

  <!-- Price Tracker Bar -->
  <div class="tracker-shell">
    <div class="tracker-line">
      <div class="marker stop" style="left: {stopPos}%"></div>
      <div class="marker entry" style="left: {entryPos}%"></div>
      <div class="marker target" style="left: {targetPos}%"></div>
      <div class="marker current" style="left: {markerPos}%"></div>
    </div>
    <div class="tracker-labels">
      <span>STOP LOSS</span>
      <span>ENTRY</span>
      <span>TAKE PROFIT</span>
    </div>
  </div>

  <div class="button-row">
    <div class="entry-meta">
      <span class="sub-label">ENTRY: {prettyMoney(position.average_price)}</span>
      <span class="sub-label">OPENED: {entryTimeStr}</span>
    </div>
    <button class="flatten-btn" on:click={handleFlatten} disabled={flattening}>
      {flattening ? '...' : 'FLATTEN'}
    </button>
  </div>
</div>

<style>
  .pos-card {
    background: linear-gradient(135deg, rgba(30, 41, 59, 1) 0%, rgba(15, 23, 42, 1) 100%);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 1.25rem;
    min-width: 340px;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    position: relative;
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
  }

  .pos-card:hover {
    border-color: rgba(96, 165, 250, 0.4);
    transform: translateY(-2px);
  }

  .pos-card.danger {
    border-color: #ef4444;
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .symbol {
    font-size: 1.4rem;
    font-weight: 800;
    color: #f8fafc;
    letter-spacing: -0.02em;
  }

  .strategy-type {
    display: block;
    font-size: 0.6rem;
    color: #60a5fa;
    text-transform: uppercase;
    font-weight: 700;
    letter-spacing: 0.05em;
  }

  .side {
    font-size: 0.6rem;
    font-weight: 900;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .long { background: rgba(34, 197, 94, 0.1); color: #4ade80; border: 1px solid rgba(34, 197, 94, 0.2); }
  .short { background: rgba(239, 68, 68, 0.1); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.2); }

  .metrics-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 8px;
  }

  .metric-item {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .label { font-size: 0.55rem; color: rgba(255, 255, 255, 0.3); font-weight: 700; text-transform: uppercase; }
  .value { font-weight: 800; font-size: 0.9rem; color: white; }
  .value.main { font-size: 1rem; color: #f1f5f9; }
  .value.pnl { font-family: 'JetBrains Mono', monospace; }

  .intel-section {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0.75rem;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 8px;
    border-left: 3px solid #3b82f6;
  }

  .intel-section.exit {
    border-left-color: #f59e0b;
  }

  .intel-section.exit.danger {
    border-left-color: #ef4444;
    background: rgba(239, 68, 68, 0.05);
  }

  .section-label {
    font-size: 0.5rem;
    font-weight: 800;
    color: rgba(255, 255, 255, 0.4);
    letter-spacing: 0.1em;
  }

  .intel-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 0.5rem;
  }

  .intel-item {
    display: flex;
    flex-direction: column;
  }

  .sub-label {
    font-size: 0.5rem;
    color: rgba(255, 255, 255, 0.3);
    font-weight: 600;
    text-transform: uppercase;
  }

  .intel-value {
    font-size: 0.7rem;
    color: #e2e8f0;
    font-weight: 600;
    line-height: 1.2;
  }

  .telemetry-bar {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .kronos-info {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .gauge {
    height: 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 2px;
    overflow: hidden;
  }

  .gauge-fill {
    height: 100%;
    transition: all 1s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .exit-banner {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .exit-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .banner-timer {
    font-size: 0.6rem;
    font-weight: 900;
    color: #3b82f6;
    font-family: 'JetBrains Mono', monospace;
  }

  .banner-timer.pulse {
    color: #ef4444;
    animation: blink 1s infinite;
  }

  .planned-action {
    font-size: 0.75rem;
    color: #f8fafc;
    font-weight: 700;
  }

  .tracker-shell {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.5rem 0;
  }

  .tracker-line {
    height: 2px;
    background: rgba(255, 255, 255, 0.1);
    position: relative;
  }

  .marker {
    position: absolute;
    top: 50%;
    transform: translate(-50%, -50%);
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .marker.stop { background: #ef4444; }
  .marker.entry { background: #94a3b8; width: 4px; height: 4px; }
  .marker.target { background: #22c55e; }
  .marker.current { 
    background: #facc15; 
    width: 10px; 
    height: 10px; 
    box-shadow: 0 0 10px rgba(250, 204, 21, 0.6);
    z-index: 10;
  }

  .tracker-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.45rem;
    font-weight: 800;
    color: rgba(255, 255, 255, 0.2);
  }

  .button-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 1rem;
  }

  .flatten-btn {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    padding: 0.4rem 1rem;
    border-radius: 6px;
    font-size: 0.6rem;
    font-weight: 900;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: all 0.2s;
  }

  .flatten-btn:hover {
    background: #ef4444;
    color: white;
  }

  @keyframes blink { 50% { opacity: 0.5; } }

  .shake {
    animation: shake 0.5s cubic-bezier(.36,.07,.19,.97) both;
  }

  @keyframes shake {
    10%, 90% { transform: translate3d(-1px, 0, 0); }
    20%, 80% { transform: translate3d(2px, 0, 0); }
    30%, 50%, 70% { transform: translate3d(-4px, 0, 0); }
    40%, 60% { transform: translate3d(4px, 0, 0); }
  }
</style>
