<script lang="ts">
  import type { TradeRecord } from "../lib/types";
  import { prettyMoney, prettyPct, quantityDigits, prettyTime } from "../lib/format";

  export let trades: TradeRecord[] = [];

  // Filter for closed positions (exits with realized P/L)
  // Filter for closed positions (exits with realized P/L) and not hidden
  $: closedTrades = trades.filter(t => t.realized_pnl != null && !t.hidden);
  $: openTrades = trades.filter(t => t.realized_pnl == null && t.side === "buy" && !t.hidden);
  
  // Sort trades by date descending
  $: sortedTrades = [...closedTrades].sort((a, b) => 
    new Date(b.executed_at).getTime() - new Date(a.executed_at).getTime()
  );
  $: sortedOpenTrades = [...openTrades].sort((a, b) => 
    new Date(b.executed_at).getTime() - new Date(a.executed_at).getTime()
  );

  async function handleHide(tradeId: string) {
    try {
      const resp = await fetch(`/api/strategies/trades/${tradeId}/hide`, { method: "DELETE" });
      if (resp.ok) {
        // Optimistically remove or wait for next poll
        trades = trades.filter(t => t.id !== tradeId);
      }
    } catch (e) {
      console.error("Failed to hide trade:", e);
    }
  }

  function getPnlColor(pnl: number | null) {
    if (pnl == null || pnl === 0) return "rgba(255,255,255,0.4)";
    return pnl > 0 ? "#4ade80" : "#f87171";
  }
</script>

<div class="recent-trades-container">
  <div class="section-header">
    <div class="header-left">
      <h2>Trade Archive</h2>
      <p>Performance audit for closed positions.</p>
    </div>
  </div>

  <div class="table-wrapper" style="margin-bottom: 2rem;">
    <h3 style="padding: 0 1.5rem; color: #22c55e; font-size: 0.9rem; margin-bottom: 0.5rem; margin-top: 1rem; font-weight: 800;">EXECUTED BUYS (LIVE)</h3>
    <table class="trades-table">
      <thead>
        <tr>
          <th>Executed At</th>
          <th>Symbol</th>
          <th>Agent</th>
          <th>Entry Price</th>
          <th>Intel</th>
          <th class="action-head">Action</th>
        </tr>
      </thead>
      <tbody>
        {#each sortedOpenTrades as trade}
          <tr>
            <td class="time">
              {new Date(trade.executed_at).toLocaleDateString()}
              <div class="at-time">{prettyTime(trade.executed_at)}</div>
            </td>
            <td class="symbol-cell">
              <span class="symbol">{trade.instrument_symbol}</span>
              <span class="asset-type">{trade.asset_type.toUpperCase()}</span>
            </td>
            <td class="strategy-cell">
              <span class="strategy">{trade.strategy_id.replace('-', ' ').toUpperCase()}</span>
            </td>
            <td class="pnl" style="color: #60a5fa">
              <div class="pnl-stack">
                <span class="pnl-cash">{prettyMoney(trade.price)}</span>
                <span class="qty-size">{trade.quantity} units</span>
              </div>
            </td>
            <td class="intel-cell">
              <div class="intel-block">
                <span class="intel-title">{trade.buy_logic || 'MANUAL'}</span>
                <span class="intel-sub">{trade.reason}</span>
              </div>
            </td>
            <td class="action-cell">
              <button class="hide-btn" on:click={() => handleHide(trade.id)}>HIDE</button>
            </td>
          </tr>
        {:else}
          <tr>
            <td colspan="6" class="empty-row">No recent entries found.</td>
          </tr>
        {/each}
      </tbody>
    </table>

    <h3 style="padding: 0 1.5rem; color: #94a3b8; font-size: 0.9rem; margin-bottom: 0.5rem; margin-top: 1rem;">Closed Positions (Archive)</h3>
    <table class="trades-table">
      <thead>
        <tr>
          <th>Archived At</th>
          <th>Symbol</th>
          <th>Agent</th>
          <th>P/L ($)</th>
          <th>Foundation (Entry)</th>
          <th>Exit Intelligence</th>
          <th>Audit Quote</th>
          <th class="action-head">Action</th>
        </tr>
      </thead>
      <tbody>
        {#each sortedTrades as trade}
          <tr>
            <td class="time">
              {new Date(trade.executed_at).toLocaleDateString()}
              <div class="at-time">{prettyTime(trade.executed_at)}</div>
            </td>
            <td class="symbol-cell">
              <span class="symbol">{trade.instrument_symbol}</span>
              <span class="asset-type">{trade.asset_type.toUpperCase()}</span>
            </td>
            <td class="strategy-cell">
              <span class="strategy">{trade.strategy_id.replace('-', ' ').toUpperCase()}</span>
            </td>
            <td class="pnl" style="color: {getPnlColor(trade.realized_pnl)}">
              <div class="pnl-stack">
                <span class="pnl-cash">
                  {trade.realized_pnl != null 
                    ? (trade.realized_pnl >= 0 ? '+' : '-') + prettyMoney(Math.abs(trade.realized_pnl))
                    : '—'}
                </span>
                <span class="qty-size">{trade.quantity} @ {prettyMoney(trade.price)}</span>
              </div>
            </td>
            <td class="intel-cell">
              <div class="intel-block">
                <span class="intel-title">BUY: {trade.buy_logic || 'MANUAL'}</span>
                <span class="intel-sub">HOLD: {trade.hold_intent || 'STANDARD'}</span>
                {#if trade.entry_math || trade.entry_ai}
                  <span class="intel-meta">Edge: {trade.entry_math || '—'} (AI: {trade.entry_ai?.toFixed(2) || '—'})</span>
                {/if}
              </div>
            </td>
            <td class="intel-cell">
              <div class="intel-block">
                <span class="intel-title">EXIT: {trade.exit_logic || 'MANUAL'}</span>
                <span class="intel-sub">{trade.planned_exit || 'COMPLETED'}</span>
              </div>
            </td>
            <td class="reason-cell">
              <div class="reason-scroll">{trade.reason}</div>
            </td>
            <td class="action-cell">
              <button class="hide-btn" on:click={() => handleHide(trade.id)} title="Archive and hide from history">
                HIDE
              </button>
            </td>
          </tr>
        {:else}
          <tr>
            <td colspan="8" class="empty">No closed positions recorded.</td>
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
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .section-header h2 {
    font-size: 1.1rem;
    font-weight: 700;
    margin: 0;
    color: #94a3b8;
  }

  .section-header p {
    font-size: 0.75rem;
    color: rgba(221, 233, 255, 0.4);
    margin: 0.25rem 0 0 0;
  }

  .table-wrapper {
    max-height: 500px;
    overflow-y: auto;
  }

  .trades-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
    text-align: left;
  }

  .trades-table th {
    position: sticky;
    top: 0;
    background: #0f172a;
    padding: 0.75rem 1rem;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: rgba(255, 255, 255, 0.3);
    font-weight: 800;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .trades-table td {
    padding: 1rem;
    border-top: 1px solid rgba(255, 255, 255, 0.03);
    color: #f1f5f9;
    vertical-align: top;
  }

  .time { font-family: 'JetBrains Mono', monospace; font-size: 0.7rem; color: rgba(255, 255, 255, 0.3); }
  .at-time { font-size: 0.6rem; margin-top: 2px; }
  
  .symbol-cell { display: flex; flex-direction: column; gap: 0.1rem; }
  .symbol { font-weight: 800; color: white; font-size: 0.85rem; }
  .asset-type { font-size: 0.5rem; color: rgba(255,255,255,0.2); font-weight: 700; }

  .strategy { font-size: 0.6rem; font-weight: 800; color: #60a5fa; background: rgba(96, 165, 250, 0.1); padding: 0.2rem 0.4rem; border-radius: 4px; display: inline-block; }
  
  .pnl { font-family: 'JetBrains Mono', monospace; font-weight: 800; font-size: 0.85rem; }
  .pnl-stack { display: flex; flex-direction: column; gap: 2px; }
  .qty-size { font-size: 0.65rem; color: rgba(255,255,255,0.3); font-weight: 500; }

  .intel-cell { min-width: 150px; }
  .intel-block { display: flex; flex-direction: column; gap: 2px; }
  .intel-title { font-size: 0.65rem; font-weight: 800; color: white; opacity: 0.8; }
  .intel-sub { font-size: 0.6rem; color: #94a3b8; font-weight: 600; text-transform: uppercase; }
  .intel-meta { font-size: 0.55rem; color: rgba(255,255,255,0.3); }

  .reason-cell { max-width: 250px; }
  .reason-scroll { 
    font-size: 0.7rem; 
    color: rgba(221, 233, 255, 0.4); 
    line-height: 1.3;
    font-style: italic;
    max-height: 3.9em;
    overflow-y: auto;
  }

  .action-cell { width: 60px; text-align: center; }
  .hide-btn {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.1);
    color: rgba(255,255,255,0.4);
    font-size: 0.55rem;
    font-weight: 800;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .hide-btn:hover {
    background: rgba(248, 113, 113, 0.1);
    border-color: rgba(248, 113, 113, 0.2);
    color: #f87171;
  }

  .empty {
    text-align: center;
    padding: 3rem !important;
    color: rgba(255, 255, 255, 0.2);
    font-style: italic;
  }

  tr:hover td {
    background: rgba(255, 255, 255, 0.01);
  }
</style>
