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

  $: stagnationSeconds = position.stagnation_timestamp ? Math.max(0, Math.floor((new Date(position.stagnation_timestamp).getTime() - now) / 1000)) : null;

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

  function getSentimentColor(val: number) {
    if (val > 0.7) return "#22c55e";
    if (val > 0.4) return "#eab308";
    return "#ef4444";
  }

  function getStrategyLabel(id: string) {
    return id.split('-').map(s => s.charAt(0).toUpperCase() + s.slice(1)).join(' ');
  }
</script>

<div class="pos-card" class:shake={isNearStop} class:danger={isNearStop}>
  <div class="card-header">
    <div class="symbol-info">
      <span class="symbol">{position.instrument_symbol}</span>
      <span class="strategy-type">{getStrategyLabel(position.strategy_id)}</span>
    </div>
    <div class="side" class:long={position.quantity > 0} class:short={position.quantity < 0}>
      {position.quantity > 0 ? 'LONG' : 'SHORT'}
    </div>
  </div>

  <div class="pnl-section">
    <div class="pnl-header">
      <span class="label">Live P&L</span>
      <span class="value" class:positive={pnlPercent >= 0} class:negative={pnlPercent < 0}>
        {pnlPercent >= 0 ? '+' : ''}{pnlPercent.toFixed(2)}%
      </span>
    </div>
    <div class="pnl-bar-container">
      <div class="pnl-bar" style="width: {Math.min(100, Math.abs(pnlPercent) * 2)}%; background: {pnlPercent >= 0 ? '#22c55e' : '#ef4444'}; margin-left: {pnlPercent >= 0 ? '50%' : 'calc(50% - ' + Math.min(50, Math.abs(pnlPercent) * 2) + '%)'}"></div>
      <div class="center-line"></div>
    </div>
  </div>

  <div class="risk-sentiment-grid">
    <div class="risk-box">
      <span class="label">RAZOR STOP</span>
      <span class="value">{position.razor_stop ? prettyMoney(position.razor_stop) : 'None'}</span>
    </div>
    <div class="risk-box">
      <span class="label">STAGNATION</span>
      <span class="value timer">{stagnationSeconds !== null ? `${Math.floor(stagnationSeconds / 60)}:${String(stagnationSeconds % 60).padStart(2, '0')}` : 'N/A'}</span>
    </div>
    <div class="sentiment-box">
       <span class="label">KRONOS AI</span>
       <div class="gauge">
         <div class="gauge-fill" style="width: {(position.kronos_sentiment ?? 0.5) * 100}%; background: {getSentimentColor(position.kronos_sentiment ?? 0.5)}"></div>
       </div>
    </div>
  </div>

  <button class="flatten-btn" on:click={handleFlatten} disabled={flattening}>
    {flattening ? 'FLATTENING...' : 'FLATTEN'}
  </button>
</div>

<style>
  .pos-card {
    background: linear-gradient(135deg, rgba(255, 255, 255, 0.05) 0%, rgba(255, 255, 255, 0.02) 100%);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 1.25rem;
    min-width: 300px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    position: relative;
    overflow: hidden;
    transition: all 0.2s ease;
  }

  .pos-card:hover {
    border-color: rgba(255, 255, 255, 0.15);
  }

  .pos-card.danger {
    border-color: #ef4444;
    box-shadow: 0 0 20px rgba(239, 68, 68, 0.2);
    animation: pulse-red 2s infinite;
  }

  @keyframes pulse-red {
    0% { background: rgba(255, 255, 255, 0.05); }
    50% { background: rgba(239, 68, 68, 0.08); }
    100% { background: rgba(255, 255, 255, 0.05); }
  }

  .shake {
    animation: shake 0.5s cubic-bezier(.36,.07,.19,.97) both;
    transform: translate3d(0, 0, 0);
  }

  @keyframes shake {
    10%, 90% { transform: translate3d(-1px, 0, 0); }
    20%, 80% { transform: translate3d(2px, 0, 0); }
    30%, 50%, 70% { transform: translate3d(-4px, 0, 0); }
    40%, 60% { transform: translate3d(4px, 0, 0); }
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .symbol-info {
    display: flex;
    flex-direction: column;
  }

  .symbol {
    font-size: 1.3rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: white;
  }

  .strategy-type {
    font-size: 0.7rem;
    color: rgba(221, 233, 255, 0.5);
    text-transform: uppercase;
    font-weight: 600;
  }

  .side {
    font-size: 0.65rem;
    font-weight: 900;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .long { background: rgba(34, 197, 94, 0.15); color: #22c55e; border: 1px solid rgba(34, 197, 94, 0.2); }
  .short { background: rgba(239, 68, 68, 0.15); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.2); }

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

  .label { color: rgba(221, 233, 255, 0.4); font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; font-weight: 600; }
  .value { font-weight: 700; font-size: 0.9rem; color: white; }
  .positive { color: #22c55e !important; }
  .negative { color: #ef4444 !important; }

  .pnl-bar-container {
    height: 4px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }

  .pnl-bar {
    height: 100%;
    position: absolute;
    transition: all 0.3s ease;
  }

  .center-line {
    position: absolute;
    left: 50%;
    top: 0;
    bottom: 0;
    width: 1px;
    background: rgba(255, 255, 255, 0.3);
    z-index: 1;
  }

  .risk-sentiment-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    padding: 0.75rem 0;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .risk-box, .sentiment-box {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .sentiment-box {
    grid-column: 1 / -1;
  }

  .timer { font-family: 'JetBrains Mono', monospace; font-size: 0.85rem; color: #60a5fa; }

  .gauge {
    height: 3px;
    background: rgba(255, 255, 255, 0.05);
    border-radius: 1.5px;
    margin-top: 0.25rem;
  }

  .gauge-fill {
    height: 100%;
    width: 0%;
    transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
    border-radius: 1.5px;
  }

  .flatten-btn {
    border: 1px solid rgba(239, 68, 68, 0.3);
    background: rgba(239, 68, 68, 0.05);
    color: #ef4444;
    padding: 0.75rem;
    border-radius: 8px;
    font-weight: 800;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
    letter-spacing: 0.1em;
    margin-top: 0.5rem;
  }

  .flatten-btn:hover:not(:disabled) {
    background: #ef4444;
    color: white;
    box-shadow: 0 4px 12px rgba(239, 68, 68, 0.2);
  }

  .flatten-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
