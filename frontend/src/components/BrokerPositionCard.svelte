<script lang="ts">
  import type { BrokerPositionSummary } from "../lib/types";
  import { prettyMoney } from "../lib/format";

  export let position: BrokerPositionSummary;

  $: pnlPercent = position.unrealized_plpc ? position.unrealized_plpc * 100 : 0;
  $: pnlCash = position.unrealized_pl ?? 0;

  function getPnlColor(pnl: number) {
    if (pnl <= -1) return "#ff4d4d";
    if (pnl < 0) return "#fb7185";
    if (pnl >= 2) return "#00ff66";
    if (pnl > 0) return "#a7f3d0";
    return "white";
  }
</script>

<div class="broker-card">
  <div class="card-header">
    <div class="symbol-info">
      <span class="symbol">{position.symbol}</span>
      <span class="type-label">BROKER MIRROR</span>
    </div>
    <div class="side" class:long={position.side === "long"} class:short={position.side === "short"}>
      {(position.side ?? 'LONG').toUpperCase()}
    </div>
  </div>

  <div class="metrics">
    <div class="metric">
      <span class="label">MARKET PRICE</span>
      <span class="value">{prettyMoney(position.current_price ?? 0)}</span>
    </div>
    <div class="metric">
      <span class="label">AVG COST</span>
      <span class="value">{prettyMoney(position.avg_entry_price ?? 0)}</span>
    </div>
    <div class="metric">
      <span class="label">QUANTITY</span>
      <span class="value">{position.quantity}</span>
    </div>
    <div class="metric">
      <span class="label">P/L (%)</span>
      <span class="value" style="color: {getPnlColor(pnlPercent)}">
        {pnlPercent >= 0 ? '+' : ''}{pnlPercent.toFixed(2)}%
      </span>
    </div>
  </div>

  <div class="logic-section">
    {#if position.entry_logic}
      <div class="logic-header">
        <span class="logic-label">ENTRY LOGIC</span>
        {#if position.strategy_id}
          <span class="strat-badge">{position.strategy_id}</span>
        {/if}
      </div>
      <p class="logic-text">{position.entry_logic}</p>
    {:else}
      <div class="logic-header">
        <span class="logic-label" style="color: #fb7185">BROKEN MIRROR</span>
      </div>
      <p class="logic-text muted">Metadata lost. This position was manually entered or local database was wiped.</p>
    {/if}
  </div>

  <div class="footer">
    <span class="pnl-cash" style="color: {getPnlColor(pnlPercent)}">
      {pnlCash >= 0 ? '+' : '-'}{prettyMoney(Math.abs(pnlCash))}
    </span>
    <span class="sync-time">Synced {new Date(position.synced_at).toLocaleTimeString()}</span>
  </div>
</div>

<style>
  .broker-card {
    background: rgba(15, 23, 42, 0.6);
    border: 1px dashed rgba(255, 255, 255, 0.15);
    border-radius: 12px;
    padding: 1rem;
    min-width: 280px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    transition: all 0.2s;
  }

  .broker-card:hover {
    border-style: solid;
    border-color: rgba(255, 255, 255, 0.3);
    background: rgba(15, 23, 42, 0.8);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
  }

  .symbol {
    font-size: 1.2rem;
    font-weight: 800;
    color: white;
  }

  .type-label {
    display: block;
    font-size: 0.6rem;
    color: #94a3b8;
    font-weight: 700;
    letter-spacing: 0.05em;
  }

  .side {
    font-size: 0.55rem;
    font-weight: 900;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
  }

  .long { background: rgba(34, 197, 94, 0.1); color: #4ade80; }
  .short { background: rgba(239, 68, 68, 0.1); color: #f87171; }

  .metrics {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .metric {
    display: flex;
    flex-direction: column;
  }

  .label {
    font-size: 0.5rem;
    color: rgba(255, 255, 255, 0.3);
    font-weight: 700;
  }

  .value {
    font-size: 0.85rem;
    font-weight: 700;
    color: #f1f5f9;
  }

  .footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    padding-top: 0.75rem;
  }

  .pnl-cash {
    font-size: 0.9rem;
    font-weight: 800;
    font-family: 'JetBrains Mono', monospace;
  }

  .logic-section {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 8px;
    padding: 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .logic-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .logic-label {
    font-size: 0.5rem;
    font-weight: 800;
    color: #94a3b8;
    letter-spacing: 0.05em;
  }

  .strat-badge {
    font-size: 0.5rem;
    background: rgba(255, 255, 255, 0.1);
    color: #cbd5e1;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    font-weight: 600;
  }

  .logic-text {
    font-size: 0.7rem;
    color: #cbd5e1;
    line-height: 1.4;
    margin: 0;
    font-weight: 500;
  }

  .muted {
    color: rgba(255, 255, 255, 0.3);
    font-style: italic;
  }

  .sync-time {
    font-size: 0.6rem;
    color: rgba(255, 255, 255, 0.2);
  }
</style>
