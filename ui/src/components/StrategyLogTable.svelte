<script lang="ts">
  export let logs: Array<{
    time: string;
    symbol: string;
    source: string;
    math_edge: string;
    ai_score: string;
    decision: string;
    narrative: string;
  }> = [];

  function getDecisionTone(decision: string) {
    const d = decision.toLowerCase();
    if (d === "buy") return "positive";
    if (d === "sell") return "negative";
    if (d === "hold" || d === "skip") return "neutral";
    return "neutral";
  }
</script>

<div class="log-panel">
  <div class="panel-header">
    <div>
      <p>System Events</p>
      <h2>Intelligence Feed</h2>
    </div>
    <button type="button" class="btn-ghost" on:click={() => (logs = [])}>🗑 Clear Feed</button>
  </div>

  <div class="log-container">
    <table class="log-table">
      <thead>
        <tr>
          <th>Time</th>
          <th>Source</th>
          <th>Symbol</th>
          <th>Math Edge</th>
          <th>AI Score</th>
          <th>Narrative</th>
        </tr>
      </thead>
      <tbody>
        {#if logs.length === 0}
          <tr>
            <td colspan="6" class="empty-state">Waiting for engine cycles...</td>
          </tr>
        {:else}
          {#each logs as log}
            <tr class="log-row" class:row-buy={log.decision.toLowerCase() === 'buy'} class:row-exit={log.decision.toLowerCase() === 'exit' || log.decision.toLowerCase() === 'skip'} class:row-heartbeat={log.decision.toLowerCase() === 'heartbeat' || log.decision.toLowerCase() === 'hold'}>
              <td class="timestamp">{log.time}</td>
              <td class="type-cell" class:type-parity={log.source.includes('PARITY')} class:type-vwap={log.source.includes('VWAP')} class:type-system={log.source.includes('SYSTEM')}>{log.source}</td>
              <td class="symbol-cell"><strong>{log.symbol}</strong></td>
              <td class="edge-cell">{log.math_edge}</td>
              <td class="kronos-cell">{log.ai_score}</td>
              <td class="decision-cell">
                <div class="decision-wrap">
                  <span class="decision-text">{log.decision}:</span>
                  <span class="reasoning-text">{log.narrative}</span>
                </div>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<style>
  .log-row.row-buy { background: rgba(34, 197, 94, 0.15); border-left: 4px solid #4ade80; }
  .log-row.row-exit { background: rgba(239, 68, 68, 0.1); border-left: 4px solid #f87171; }
  .log-row.row-heartbeat { background: rgba(59, 130, 246, 0.1); border-left: 4px solid #60a5fa; }

  .decision-wrap { display: flex; gap: 8px; align-items: baseline; }
  .decision-text { font-weight: 700; text-transform: uppercase; font-size: 0.75rem; min-width: 60px; }
  .reasoning-text { opacity: 0.8; font-style: italic; }

  .type-cell { font-weight: 600; color: rgba(255,255,255,0.7); font-size: 0.7rem; }
  .edge-cell, .kronos-cell { font-family: var(--font-mono); }

  .log-panel {
    background: rgba(13, 17, 23, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
    padding: 16px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .panel-header h2 {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
  }

  .panel-header p {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-dim);
    margin: 0 0 4px 0;
  }

  .log-container {
    overflow: auto;
    max-height: 500px;
  }

  .log-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .log-table th {
    text-align: left;
    padding: 12px 20px;
    background: rgba(0, 0, 0, 0.2);
    color: var(--color-text-dim);
    font-weight: 500;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .log-table td {
    padding: 8px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    font-size: 0.8rem;
  }

  .log-table tr:nth-child(even) td {
    background: rgba(255, 255, 255, 0.01);
  }

  .timestamp {
    color: var(--color-text-dim);
    font-family: var(--font-mono);
    white-space: nowrap;
    width: 100px;
  }

  .type-cell {
    font-weight: 700;
    font-size: 0.7rem;
    letter-spacing: 0.05em;
    width: 120px;
    text-transform: uppercase;
  }

  .type-parity { color: #c084fc; }
  .type-vwap { color: #22d3ee; }
  .type-system { color: rgba(255, 255, 255, 0.5); }

  .symbol-cell {
    width: 100px;
  }

  .edge-cell, .kronos-cell {
    font-family: var(--font-mono);
    width: 80px;
  }

  .decision-wrap {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }

  .decision-text {
    font-weight: 700;
    text-transform: uppercase;
    font-size: 0.75rem;
  }

  .reasoning-text {
    color: rgba(255, 255, 255, 0.8);
    line-height: 1.4;
  }

  .row-buy .decision-text { color: #4ade80; }
  .row-exit .decision-text { color: #f87171; }
  .row-heartbeat .decision-text { color: var(--color-text-dim); }

  .empty-state {
    text-align: center;
    padding: 40px !important;
    color: var(--color-text-dim);
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--color-text-dim);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.75rem;
    cursor: pointer;
  }

  .btn-ghost:hover {
    background: rgba(255, 255, 255, 0.05);
    color: white;
  }
</style>
