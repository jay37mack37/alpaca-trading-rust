<script lang="ts">
  import SvelteVirtualList from "@humanspeak/svelte-virtual-list";

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
      <h2>Intelligence Feed</h2>
    </div>
    <button type="button" class="btn-ghost" on:click={() => (logs = [])}>🗑 Clear Feed</button>
  </div>

  <div class="log-container">
    <div class="log-header">
      <div class="col-time">Time</div>
      <div class="col-type">Type</div>
      <div class="col-symbol">Symbol</div>
      <div class="col-edge">Math Edge %</div>
      <div class="col-kronos">Kronos Score</div>
      <div class="col-decision">Decision / Reasoning</div>
    </div>

    <div class="log-body">
      {#if logs.length === 0}
        <div class="empty-state">Waiting for engine cycles...</div>
      {:else}
        <SvelteVirtualList items={logs} viewportClass="log-viewport">
          {#snippet renderItem(log)}
            <div
              class="log-row"
              class:row-buy={log.decision.toLowerCase() === 'buy'}
              class:row-exit={log.decision.toLowerCase() === 'exit' || log.decision.toLowerCase() === 'skip'}
              class:row-heartbeat={log.decision.toLowerCase() === 'heartbeat' || log.decision.toLowerCase() === 'hold'}
            >
              <div class="col-time timestamp">{log.time.split(" ")[1]}</div>
              <div class="col-type type-cell">{(log as any).type ?? 'DRIFT'}</div>
              <div class="col-symbol symbol-cell"><strong>{log.symbol}</strong></div>
              <div class="col-edge edge-cell">{log.math_edge}</div>
              <div class="col-kronos kronos-cell">{log.kronos_score}</div>
              <div class="col-decision decision-cell">
                <div class="decision-wrap">
                  <span class="decision-text">{log.decision}:</span>
                  <span class="reasoning-text">{log.reasoning}</span>
                </div>
              </div>
            </div>
          {/snippet}
        </SvelteVirtualList>
      {/if}
    </div>
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
    display: flex;
    flex-direction: column;
    height: 500px;
  }

  .log-header {
    display: grid;
    grid-template-columns: 100px 80px 100px 120px 120px 1fr;
    padding: 12px 20px;
    background: rgba(0, 0, 0, 0.2);
    color: var(--color-text-dim);
    font-weight: 500;
    font-size: 0.85rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .log-body {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  :global(.log-viewport) {
    height: 100% !important;
  }

  .log-row {
    display: grid;
    grid-template-columns: 100px 80px 100px 120px 120px 1fr;
    font-size: 0.85rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    align-items: center;
  }

  .log-row > div {
    padding: 10px 20px;
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
