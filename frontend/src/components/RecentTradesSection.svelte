<script lang="ts">
  import type { TradeRecord } from "../lib/types";
  import { prettyMoney, prettyPct, quantityDigits } from "../lib/format";

  export let trades: TradeRecord[] = [];

  // Filter for closed positions (exits with realized P/L)
  $: closedTrades = trades.filter(t => t.realized_pnl != null);

  // Sort trades by date descending
  $: sortedTrades = [...closedTrades].sort((a, b) => 
    new Date(b.executed_at).getTime() - new Date(a.executed_at).getTime()
  );

  function getPnlColor(pnl: number | null) {
    if (pnl == null || pnl === 0) return "rgba(255,255,255,0.4)";
    return pnl > 0 ? "#4ade80" : "#f87171";
  }
</script>

<div class="recent-trades-container">
  <div class="section-header">
    <h2>Closed Positions</h2>
    <p>Realized performance across all agents.</p>
  </div>

  <div class="table-wrapper">
    <table class="trades-table">
      <thead>
        <tr>
          <th>Time</th>
          <th>Asset</th>
          <th>Agent / Strategy</th>
          <th>Side</th>
          <th>Size</th>
          <th>Price</th>
          <th>Net P/L</th>
          <th>Insight</th>
        </tr>
      </thead>
      <tbody>
        {#each sortedTrades as trade}
          <tr>
            <td class="time">{new Date(trade.executed_at).toLocaleTimeString()}</td>
            <td class="symbol-cell">
              <span class="symbol">{trade.instrument_symbol}</span>
              <span class="asset-type">{trade.asset_type.toUpperCase()}</span>
            </td>
            <td class="strategy-cell">
              <span class="strategy">{trade.strategy_id.replace('-', ' ').toUpperCase()}</span>
            </td>
            <td class="side" class:buy={trade.side === 'buy'} class:sell={trade.side === 'sell'}>
              <span class="side-badge">{trade.side.toUpperCase()}</span>
            </td>
            <td class="qty">{trade.quantity.toFixed(quantityDigits(trade.asset_type))}</td>
            <td class="price">{prettyMoney(trade.price)}</td>
            <td class="pnl" style="color: {getPnlColor(trade.realized_pnl)}">
              <div class="pnl-stack">
                <span class="pnl-cash">
                  {trade.realized_pnl != null 
                    ? (trade.realized_pnl >= 0 ? '+' : '-') + prettyMoney(Math.abs(trade.realized_pnl))
                    : '—'}
                </span>
              </div>
            </td>
            <td class="reason-cell">{trade.reason}</td>
          </tr>
        {:else}
          <tr>
            <td colspan="9" class="empty">No closed positions recorded.</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>

<style>
  .recent-trades-container {
    margin-top: 2rem;
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.05);
    border-radius: 12px;
    overflow: hidden;
  }

  .section-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .section-header h2 {
    font-size: 1.1rem;
    font-weight: 700;
    margin: 0;
  }

  .section-header p {
    font-size: 0.75rem;
    color: rgba(221, 233, 255, 0.5);
    margin: 0.25rem 0 0 0;
  }

  .table-wrapper {
    max-height: 400px;
    overflow-y: auto;
  }

  .trades-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
    text-align: left;
  }

  .trades-table th {
    position: sticky;
    top: 0;
    background: #0f172a;
    padding: 0.75rem 1.5rem;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.4);
    font-weight: 800;
  }

  .trades-table td {
    padding: 1rem 1.5rem;
    border-top: 1px solid rgba(255, 255, 255, 0.03);
    color: #f1f5f9;
    vertical-align: middle;
  }

  .time { font-family: 'JetBrains Mono', monospace; font-size: 0.72rem; color: rgba(255, 255, 255, 0.4); }
  
  .symbol-cell { display: flex; flex-direction: column; gap: 0.1rem; }
  .symbol { font-weight: 800; color: white; font-size: 0.9rem; letter-spacing: -0.02em; }
  .asset-type { font-size: 0.55rem; color: rgba(255,255,255,0.3); font-weight: 700; letter-spacing: 0.05em; }

  .strategy-cell { min-width: 140px; }
  .strategy { font-size: 0.65rem; font-weight: 800; color: #60a5fa; background: rgba(96, 165, 250, 0.1); padding: 0.2rem 0.4rem; border-radius: 4px; }
  
  .side-badge { padding: 0.2rem 0.4rem; border-radius: 4px; background: rgba(255,255,255,0.05); }
  .side.buy .side-badge { color: #4ade80; background: rgba(74, 222, 128, 0.1); }
  .side.sell .side-badge { color: #f87171; background: rgba(248, 113, 113, 0.1); }

  .qty { font-family: 'JetBrains Mono', monospace; font-weight: 600; color: #94a3b8; }
  .price { font-family: 'JetBrains Mono', monospace; font-weight: 700; }

  .pnl { font-family: 'JetBrains Mono', monospace; font-weight: 800; font-size: 0.9rem; }
  
  .reason-cell { 
    font-size: 0.75rem; 
    color: rgba(221, 233, 255, 0.5); 
    max-width: 300px; 
    line-height: 1.4;
    font-style: italic;
  }

  .empty {
    text-align: center;
    padding: 3rem !important;
    color: rgba(255, 255, 255, 0.2);
    font-style: italic;
  }

  tr:hover td {
    background: rgba(255, 255, 255, 0.02);
  }
</style>
