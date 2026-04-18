<script lang="ts">
  export let logs: Array<{
    time: string;
    symbol: string;
    math_edge: string;
    kronos_score: string;
    decision: string;
    reasoning: string;
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
      <h2>Strategy Live Log</h2>
    </div>
    <button type="button" class="btn-ghost" on:click={() => (logs = [])}>🗑 Clear View</button>
  </div>

  <div class="log-container">
    <table class="log-table">
      <thead>
        <tr>
          <th>Time</th>
          <th>Symbol</th>
          <th>Math/Edge</th>
          <th>Kronos Score</th>
          <th>Decision</th>
          <th>Reasoning</th>
        </tr>
      </thead>
      <tbody>
        {#if logs.length === 0}
          <tr>
            <td colspan="6" class="empty-state">No log entries yet. Active strategies will report here.</td>
          </tr>
        {:else}
          {#each logs as log}
            <tr>
              <td class="timestamp">{log.time.split(" ")[1]}</td>
              <td><strong>{log.symbol}</strong></td>
              <td>{log.math_edge}</td>
              <td>{log.kronos_score}</td>
              <td>
                <span class="tag" class:tag--positive={getDecisionTone(log.decision) === "positive"} class:tag--negative={getDecisionTone(log.decision) === "negative"}>
                  {log.decision}
                </span>
              </td>
              <td class="reasoning">{log.reasoning}</td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>
</div>

<style>
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
    padding: 10px 20px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .timestamp {
    color: var(--color-text-dim);
    font-family: var(--font-mono);
  }

  .reasoning {
    color: var(--color-text-dim);
    font-style: italic;
  }

  .empty-state {
    text-align: center;
    padding: 40px !important;
    color: var(--color-text-dim);
  }

  .tag {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    background: rgba(255, 255, 255, 0.05);
  }

  .tag--positive {
    background: rgba(34, 197, 94, 0.1);
    color: #4ade80;
  }

  .tag--negative {
    background: rgba(239, 68, 68, 0.1);
    color: #f87171;
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
