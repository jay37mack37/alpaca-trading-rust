<script lang="ts">
  import type { PositionSummary } from "../lib/types";
  import { api } from "../lib/api";
  import { prettyMoney } from "../lib/format";
  import { createEventDispatcher, onMount } from "svelte";

  export let position: PositionSummary;

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

  <div class="alpaca-summary">
    <div class="summary-item">
      <span class="label">Price</span>
      <span class="value main">{prettyMoney(position.market_price)}</span>
    </div>
    <div class="summary-item">
      <span class="label">Qty</span>
      <span class="value main">{position.quantity}</span>
    </div>
    <div class="summary-item">
      <span class="label">Market Value</span>
      <span class="value main">{prettyMoney(marketValue)}</span>
    </div>
    <div class="summary-item">
      <span class="label">Total P/L ($)</span>
      <span class="value pnl main" style="color: {getPnlColor(pnlPercent)}">
        {pnlCash >= 0 ? '+' : '-'}{prettyMoney(Math.abs(pnlCash))}
      </span>
    </div>
  </div>

  <div class="exit-strategy-banner" class:danger={isStagnant || isNearStop}>
    <div class="banner-top">
      <span class="banner-label">EXIT STRATEGY: {position.exit_logic ?? 'SMART DISCONNECT'}</span>
      <span class="banner-timer" class:pulse={isStagnant}>
        {isStagnant ? 'DEADLINE REACHED' : (stagnationSeconds !== null ? `T-MINUS ${Math.floor(stagnationSeconds / 60)}:${(stagnationSeconds % 60).toString().padStart(2, '0')}` : 'STABLE')}
      </span>
    </div>
    <div class="planned-action">{plannedExitMessage}</div>
  </div>

  <div class="pnl-section">
    <div class="pnl-header">
      <span class="label">Live P&L (%)</span>
      <span class="value" style="color: {getPnlColor(pnlPercent)}">
        {pnlPercent >= 0 ? '+' : ''}{pnlPercent.toFixed(2)}%
      </span>
    </div>
    <div class="pnl-bar-container">
      <div class="pnl-bar" style="width: {Math.min(100, Math.abs(pnlPercent) * 2)}%; background: {getPnlColor(pnlPercent)}; margin-left: {pnlPercent >= 0 ? '50%' : 'calc(50% - ' + Math.min(50, Math.abs(pnlPercent) * 2) + '%)'}"></div>
      <div class="center-line"></div>
    </div>
  </div>

  <div class="target-section">
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
  </div>

  <div class="risk-sentiment-grid">
    <div class="risk-box">
      <span class="label">STAGNATION PROGRESS</span>
      <div class="stagnation-bar">
        <div class="fill" style="width: {stagnationProgress}%"></div>
      </div>
    </div>
    <div class="sentiment-box">
       <span class="label">AI CONFIDENCE / SENTIMENT</span>
       <div class="gauge">
         <div class="gauge-fill" style="width: {(position.kronos_sentiment ?? 0.5) * 100}%; background: {getSentimentColor(position.kronos_sentiment ?? 0.5)}"></div>
       </div>
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
    background: linear-gradient(135deg, rgba(30, 41, 59, 0.95) 0%, rgba(15, 23, 42, 0.98) 100%);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 16px;
    padding: 1.5rem;
    min-width: 320px;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    position: relative;
    overflow: hidden;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
  }

  .pos-card:hover {
    border-color: rgba(96, 165, 250, 0.4);
    transform: translateY(-4px);
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
  }

  .pos-card.danger {
    border-color: #ef4444;
    box-shadow: 0 0 30px rgba(239, 68, 68, 0.15);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .symbol-info {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .symbol {
    font-size: 1.5rem;
    font-weight: 800;
    letter-spacing: -0.03em;
    background: linear-gradient(to right, #fff, #94a3b8);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  .strategy-type {
    font-size: 0.65rem;
    color: #60a5fa;
    text-transform: uppercase;
    font-weight: 700;
    letter-spacing: 0.1em;
  }

  .side {
    font-size: 0.6rem;
    font-weight: 900;
    padding: 0.25rem 0.6rem;
    border-radius: 6px;
    letter-spacing: 0.1em;
    box-shadow: inset 0 0 10px rgba(255, 255, 255, 0.05);
  }

  .long { background: rgba(34, 197, 94, 0.1); color: #4ade80; border: 1px solid rgba(34, 197, 94, 0.2); }
  .short { background: rgba(239, 68, 68, 0.1); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.2); }

  .alpaca-summary {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(255, 255, 255, 0.03);
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .summary-item {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .summary-item .label {
    font-size: 0.5rem;
    color: rgba(255, 255, 255, 0.3);
    font-weight: 700;
    text-transform: uppercase;
  }

  .summary-item .value.main {
    font-size: 0.85rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .summary-item .value.pnl {
    font-family: 'JetBrains Mono', monospace;
  }

  .exit-strategy-banner {
    background: rgba(30, 41, 59, 0.5);
    border: 1px solid rgba(96, 165, 250, 0.2);
    border-radius: 10px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    position: relative;
    overflow: hidden;
  }

  .exit-strategy-banner::before {
    content: '';
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 3px;
    background: #3b82f6;
  }

  .exit-strategy-banner.danger {
    border-color: rgba(249, 115, 22, 0.4);
    background: rgba(249, 115, 22, 0.05);
  }

  .exit-strategy-banner.danger::before {
    background: #f97316;
  }

  .banner-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .banner-label {
    font-size: 0.55rem;
    font-weight: 800;
    color: rgba(255, 255, 255, 0.5);
    letter-spacing: 0.05em;
  }

  .banner-timer {
    font-size: 0.6rem;
    font-weight: 900;
    color: #60a5fa;
    font-family: 'JetBrains Mono', monospace;
  }

  .banner-timer.pulse {
    color: #f97316;
    animation: blink 1s infinite;
  }

  @keyframes blink {
    50% { opacity: 0.5; }
  }

  .planned-action {
    font-size: 0.8rem;
    color: #f1f5f9;
    font-weight: 700;
  }

  .pnl-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .pnl-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .label { color: rgba(221, 233, 255, 0.4); font-size: 0.6rem; text-transform: uppercase; letter-spacing: 0.08em; font-weight: 700; }
  .value { font-weight: 800; font-size: 0.9rem; color: white; }

  .pnl-bar-container {
    height: 6px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 3px;
    position: relative;
    overflow: hidden;
  }

  .center-line {
    position: absolute;
    left: 50%;
    top: 0;
    bottom: 0;
    width: 2px;
    background: rgba(255, 255, 255, 0.2);
    z-index: 1;
  }

  .pnl-bar {
    height: 100%;
    position: absolute;
    transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .tracker-shell {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
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
    font-size: 0.5rem;
    font-weight: 800;
    color: rgba(255, 255, 255, 0.2);
  }

  .risk-sentiment-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1.25rem;
    padding-top: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
  }

  .risk-box, .sentiment-box {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .stagnation-bar, .gauge {
    height: 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 2px;
    overflow: hidden;
  }

  .fill, .gauge-fill {
    height: 100%;
    transition: all 0.5s ease;
  }

  .button-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 0.5rem;
  }

  .entry-meta {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .sub-label {
    font-size: 0.55rem;
    color: rgba(255, 255, 255, 0.3);
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  .flatten-btn {
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    padding: 0.5rem 1.25rem;
    border-radius: 8px;
    font-size: 0.65rem;
    font-weight: 900;
    letter-spacing: 0.1em;
    cursor: pointer;
    transition: all 0.2s;
  }

  .flatten-btn:hover {
    background: #ef4444;
    color: white;
    border-color: #ef4444;
    box-shadow: 0 0 15px rgba(239, 68, 68, 0.3);
  }

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
