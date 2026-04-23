<script lang="ts">
  import { onMount, tick } from "svelte";

  export let logs: Array<{
    time: string;
    symbol: string;
    source: string;
    math_edge: string;
    ai_score: string;
    decision: string;
    narrative: string;
  }> = [];

  let logBodyEl: HTMLDivElement;
  let isPaused = false;
  let showScrollToLive = false;
  let prevLogCount = 0;

  function getDecisionTone(decision: string) {
    const d = decision.toLowerCase();
    if (d === "buy") return "positive";
    if (d === "sell") return "negative";
    if (d === "hold" || d === "skip") return "neutral";
    return "neutral";
  }

  function scrollToBottom(smooth = false) {
    if (!logBodyEl) return;
    logBodyEl.scrollTo({
      top: logBodyEl.scrollHeight,
      behavior: smooth ? "smooth" : "auto",
    });
  }

  function handleScroll() {
    if (!logBodyEl) return;
    const { scrollTop, scrollHeight, clientHeight } = logBodyEl;
    const nearBottom = scrollHeight - scrollTop - clientHeight < 50;
    if (nearBottom) {
      isPaused = false;
      showScrollToLive = false;
    } else {
      isPaused = true;
      showScrollToLive = true;
    }
  }

  function resumeLive() {
    isPaused = false;
    showScrollToLive = false;
    scrollToBottom(true);
  }

  $: {
    const currentCount = logs.length;
    if (currentCount > prevLogCount && !isPaused) {
      prevLogCount = currentCount;
      tick().then(() => scrollToBottom(true));
    } else {
      prevLogCount = currentCount;
    }
  }

  onMount(() => {
    tick().then(() => scrollToBottom());
  });
</script>

<div class="log-panel">
  <div class="panel-header">
    <div>
      <p>System Events</p>
      <h2>Intelligence Feed</h2>
    </div>
    <div class="header-actions">
      {#if showScrollToLive}
        <button type="button" class="btn-live" on:click={resumeLive}>
          🔴 Scroll to Live
        </button>
      {/if}
      <button type="button" class="btn-ghost" on:click={() => (logs = [])}>🗑 Clear Feed</button>
    </div>
  </div>

  <div class="log-container">
    <div class="log-header">
      <div class="col-time">Time</div>
      <div class="col-type">Src</div>
      <div class="col-symbol">Sym</div>
      <div class="col-edge">Math Context</div>
      <div class="col-kronos">Kronos</div>
      <div class="col-decision">Audit Trail</div>
    </div>

    <div class="log-body" bind:this={logBodyEl} on:scroll={handleScroll}>
      {#if logs.length === 0}
        <div class="empty-state">Waiting for engine cycles...</div>
      {:else}
        {#each logs as log (log.time + log.symbol + log.decision + log.narrative)}
          <div
            class="log-row"
            class:row-signal={['signal', 'buy', 'sell'].includes(log.decision.toLowerCase())}
            class:row-protection={log.decision.toLowerCase() === 'protection'}
            class:row-exit={log.decision.toLowerCase() === 'exit'}
            class:row-haggle={log.decision.toLowerCase() === 'haggle'}
            class:row-scan={['scan', 'heartbeat', 'hold', 'scanning'].includes(log.decision.toLowerCase())}
          >
            <div class="col-time timestamp">{log.time.split(" ")[1]}</div>
            <div class="col-type type-cell">{log.source}</div>
            <div class="col-symbol symbol-cell"><strong>{log.symbol}</strong></div>
            <div class="col-edge edge-cell">{log.math_edge}</div>
            <div class="col-kronos kronos-cell">{log.ai_score}</div>
            <div class="col-decision decision-cell">
              <div class="decision-wrap">
                <span class="decision-text">{log.decision}:</span>
                <span class="reasoning-text">{log.narrative}</span>
              </div>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .log-row.row-signal { background: rgba(59, 130, 246, 0.15); border-left: 4px solid #60a5fa; }
  .log-row.row-protection { background: rgba(239, 68, 68, 0.15); border-left: 4px solid #f87171; }
  .log-row.row-exit { background: rgba(251, 191, 36, 0.15); border-left: 4px solid #fbbf24; }
  .log-row.row-haggle { background: rgba(168, 85, 247, 0.15); border-left: 4px solid #c084fc; }
  .log-row.row-scan { background: rgba(255, 255, 255, 0.03); border-left: 4px solid rgba(255,255,255,0.2); }

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
    flex: 1;
    min-height: 0;
  }

  .panel-header {
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
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

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .log-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    height: 500px;
  }

  .log-header {
    display: grid;
    grid-template-columns: 100px 80px 100px 120px 120px 1fr;
    padding: 10px 20px;
    background: rgba(0, 0, 0, 0.2);
    color: var(--color-text-dim);
    font-weight: 500;
    font-size: 0.85rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }

  .log-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
  }

  .log-body::-webkit-scrollbar {
    width: 8px;
  }

  .log-body::-webkit-scrollbar-track {
    background: transparent;
  }

  .log-body::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.15);
    border-radius: 4px;
  }

  .log-body::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.25);
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

  .btn-live {
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.75rem;
    cursor: pointer;
    font-weight: 600;
    animation: pulse-live 1.5s ease-in-out infinite;
  }

  .btn-live:hover {
    background: rgba(239, 68, 68, 0.25);
  }

  @keyframes pulse-live {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }
</style>
