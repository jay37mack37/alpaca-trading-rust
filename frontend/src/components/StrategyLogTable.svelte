<script lang="ts">
  import { onMount, tick } from "svelte";
  import { prettyTime } from "../lib/format";

  export let logs: Array<{
    time: string;
    symbol: string;
    source: string;
    math_edge: string;
    ai_score: string;
    decision: string;
    narrative: string;
    was_executed?: boolean;
  }> = [];

  let logBodyEl: HTMLDivElement;
  let isPaused = false;
  let showScrollToLive = false;
  let prevLogCount = 0;

  // Pagination & Filtering
  let selectedCategory = "all";
  let selectedStrategy = "all";
  let currentPage = 1;
  const pageSize = 20;


  const categories = [
    { id: "all", label: "All Events" },
    { id: "executed", label: "Executed", color: "#22c55e" },
    { id: "signal", label: "Buy Signals", color: "#60a5fa" },
    { id: "scan", label: "Scans", color: "rgba(255,255,255,0.4)" },
    { id: "protection", label: "Protection", color: "#f87171" },
    { id: "exit", label: "Exits", color: "#fbbf24" },
  ];

  function getCategory(decision: string): string {
    const d = decision.toLowerCase();
    if (["signal", "buy"].includes(d)) return "signal";
    if (["scan", "heartbeat", "hold", "scanning"].includes(d)) return "scan";
    if (["protection", "guard", "risk"].includes(d)) return "protection";
    if (["exit", "sell"].includes(d)) return "exit";
    return "other";
  }

  $: uniqueStrategies = Array.from(new Set(logs.map(l => l.source))).sort();

  $: filteredLogs = logs.filter(l => {
    let matchesCat = false;
    if (selectedCategory === "all") matchesCat = true;
    else if (selectedCategory === "executed") matchesCat = l.was_executed === true;
    else matchesCat = getCategory(l.decision) === selectedCategory;
    
    const matchesStrat = selectedStrategy === "all" || l.source === selectedStrategy;
    return matchesCat && matchesStrat;
  });

  $: totalPages = Math.ceil(filteredLogs.length / pageSize) || 1;
  $: paginatedLogs = filteredLogs.slice(
    (currentPage - 1) * pageSize,
    currentPage * pageSize
  );

  function setPage(p: number) {
    currentPage = Math.max(1, Math.min(p, totalPages));
    isPaused = true; // Auto-pause when interacting with history
  }

  function scrollToBottom(smooth = false) {
    if (!logBodyEl || isPaused) return;
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
    selectedCategory = "all";
    currentPage = 1;
    scrollToBottom(true);
  }

  $: {
    const currentCount = logs.length;
    if (currentCount > prevLogCount && !isPaused && selectedCategory === "all") {
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
    <div class="header-left">
      <p>System Events</p>
      <h2>Intelligence Feed</h2>
    </div>
    
    <div class="filter-bar">
      <div class="filter-group">
        <span class="filter-label">Type:</span>
        {#each categories as cat}
          <button 
            class="filter-btn" 
            class:active={selectedCategory === cat.id}
            on:click={() => { selectedCategory = cat.id; currentPage = 1; }}
            style="--cat-color: {cat.color}"
          >
            {cat.label}
          </button>
        {/each}
      </div>

      <div class="filter-separator"></div>

      <div class="filter-group">
        <span class="filter-label">STRATEGY:</span>
        <select bind:value={selectedStrategy} class="strat-select" on:change={() => currentPage = 1}>
          <option value="all">All Engines</option>
          {#each uniqueStrategies as strat}
            <option value={strat}>{strat}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="header-actions">
      <div class="kronos-legend">
        <span class="legend-item"><span class="dot bull"></span> Bullish (&gt;0.6)</span>
        <span class="legend-item"><span class="dot neut"></span> Neutral (0.4-0.6)</span>
        <span class="legend-item"><span class="dot bear"></span> Bearish (&lt;0.4)</span>
      </div>
      {#if showScrollToLive || isPaused}
        <button type="button" class="btn-live" on:click={resumeLive}>
          🔴 Resume Live
        </button>
      {/if}
      <button type="button" class="btn-ghost" on:click={() => (logs = [])}>🗑 Clear</button>
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
      {#if paginatedLogs.length === 0}
        <div class="empty-state">No matching events in current window...</div>
      {:else}
        {#each paginatedLogs as log (log.time + log.symbol + log.decision + log.narrative)}
          <div
            class="log-row"
            class:row-signal={['signal', 'buy'].includes(log.decision.toLowerCase())}
            class:row-protection={['protection', '0dte'].some(s => log.decision.toLowerCase().includes(s))}
            class:row-exit={['exit', 'sell'].includes(log.decision.toLowerCase())}
            class:row-executed={log.was_executed || log.decision.toLowerCase() === 'executed'}
            class:row-haggle={log.decision.toLowerCase() === 'haggle'}
            class:row-scan={['scan', 'heartbeat', 'hold', 'scanning'].includes(log.decision.toLowerCase())}
          >
            <div class="col-time timestamp">{prettyTime(log.time)}</div>
            <div class="col-type type-cell">{log.source}</div>
            <div class="col-symbol symbol-cell"><strong>{log.symbol}</strong></div>
            <div class="col-edge edge-cell">{log.math_edge}</div>
            <div class="col-kronos kronos-cell">{log.ai_score}</div>
            <div class="col-decision decision-cell">
              <div class="decision-wrap">
                <span class="decision-text">{log.decision}:</span>
                <span class="reasoning-text">{log.narrative}</span>
                {#if log.was_executed}
                  <span class="executed-badge">EXECUTED</span>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <div class="pagination-footer">
      <div class="page-info">
        Page <strong>{currentPage}</strong> of {totalPages}
        <span class="count-pill">{filteredLogs.length} events</span>
      </div>
      <div class="page-controls">
        <button class="page-btn" disabled={currentPage === 1} on:click={() => setPage(currentPage - 1)}>
          &larr; Prev
        </button>
        <button class="page-btn" disabled={currentPage === totalPages} on:click={() => setPage(currentPage + 1)}>
          Next &rarr;
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .log-row.row-signal { background: rgba(59, 130, 246, 0.15); border-left: 4px solid #60a5fa; }
  .log-row.row-protection { background: rgba(239, 68, 68, 0.15); border-left: 4px solid #f87171; }
  .log-row.row-exit { background: rgba(251, 191, 36, 0.15); border-left: 4px solid #fbbf24; }
  .log-row.row-executed { background: rgba(34, 197, 94, 0.15); border-left: 4px solid #22c55e; }
  .log-row.row-haggle { background: rgba(168, 85, 247, 0.15); border-left: 4px solid #c084fc; }
  .log-row.row-scan { background: rgba(255, 255, 255, 0.03); border-left: 4px solid rgba(255,255,255,0.2); }

  .decision-wrap { display: flex; gap: 8px; align-items: baseline; }
  .decision-text { font-weight: 700; text-transform: uppercase; font-size: 0.75rem; min-width: 60px; }
  .reasoning-text { opacity: 0.8; font-style: italic; }
  .executed-badge {
    background: #22c55e;
    color: white;
    font-size: 0.6rem;
    font-weight: 800;
    padding: 2px 6px;
    border-radius: 4px;
    margin-left: auto;
    box-shadow: 0 0 10px rgba(34, 197, 94, 0.4);
    animation: glow 2s infinite alternate;
  }
  @keyframes glow {
    from { box-shadow: 0 0 5px rgba(34, 197, 94, 0.4); }
    to { box-shadow: 0 0 15px rgba(34, 197, 94, 0.8); }
  }

  .type-cell { font-weight: 600; color: rgba(255,255,255,0.7); font-size: 0.7rem; }
  .edge-cell, .kronos-cell { font-family: var(--font-mono); }

  .log-panel {
    background: rgba(13, 17, 23, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex: 1;
    min-height: 0;
    backdrop-filter: blur(10px);
  }

  .panel-header {
    padding: 12px 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
    background: rgba(255, 255, 255, 0.02);
  }

  .filter-bar {
    display: flex;
    gap: 16px;
    background: rgba(0, 0, 0, 0.2);
    padding: 6px 12px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    align-items: center;
  }

  .filter-group {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .filter-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    color: var(--color-text-dim);
    margin-right: 4px;
    font-weight: 700;
  }

  .filter-separator {
    width: 1px;
    height: 16px;
    background: rgba(255, 255, 255, 0.1);
  }

  .strat-select {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: white;
    border-radius: 4px;
    font-size: 0.75rem;
    padding: 2px 8px;
    cursor: pointer;
    outline: none;
  }

  .strat-select option {
    background-color: #0d1117;
    color: white;
  }

  .filter-btn {
    background: transparent;
    border: none;
    color: var(--color-text-dim);
    padding: 4px 12px;
    border-radius: 6px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .filter-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: white;
  }

  .filter-btn.active {
    background: var(--cat-color);
    color: white;
    font-weight: 600;
  }

  .panel-header h2 {
    font-size: 1rem;
    font-weight: 600;
    margin: 0;
  }

  .panel-header p {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-text-dim);
    margin: 0 0 2px 0;
  }

  .log-container {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .log-header {
    display: grid;
    grid-template-columns: 100px 80px 100px 120px 120px 1fr;
    padding: 10px 20px;
    background: rgba(0, 0, 0, 0.2);
    color: var(--color-text-dim);
    font-weight: 500;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    flex-shrink: 0;
  }

  .log-body {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.15) transparent;
    min-height: 300px;
  }

  .pagination-footer {
    padding: 10px 20px;
    background: rgba(0, 0, 0, 0.3);
    border-top: 1px solid rgba(255, 255, 255, 0.08);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.8rem;
  }

  .count-pill {
    margin-left: 12px;
    background: rgba(255, 255, 255, 0.1);
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.7rem;
    color: var(--color-text-dim);
  }

  .page-controls {
    display: flex;
    gap: 8px;
  }

  .page-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: white;
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .page-btn:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }

  .page-btn:not(:disabled):hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .log-row {
    display: grid;
    grid-template-columns: 100px 80px 100px 120px 120px 1fr;
    font-size: 0.85rem;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    align-items: center;
  }

  .log-row > div {
    padding: 8px 20px;
  }

  .timestamp {
    color: var(--color-text-dim);
    font-family: var(--font-mono);
    font-size: 0.8rem;
  }

  .empty-state {
    text-align: center;
    padding: 40px !important;
    color: var(--color-text-dim);
    font-style: italic;
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: var(--color-text-dim);
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.7rem;
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
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.7rem;
    cursor: pointer;
    font-weight: 600;
    animation: pulse-live 1.5s ease-in-out infinite;
  }

  .btn-live:hover {
    background: rgba(239, 68, 68, 0.25);
  }

  .kronos-legend {
    display: flex;
    gap: 12px;
    margin-right: 16px;
    font-size: 0.65rem;
    color: var(--color-text-dim);
    background: rgba(0, 0, 0, 0.2);
    padding: 4px 12px;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .legend-item .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .dot.bull { background: #22c55e; box-shadow: 0 0 5px #22c55e; }
  .dot.neut { background: #94a3b8; }
  .dot.bear { background: #ef4444; box-shadow: 0 0 5px #ef4444; }

  @keyframes pulse-live {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }

  .executed-summary {
    background: linear-gradient(90deg, rgba(34, 197, 94, 0.08) 0%, rgba(34, 197, 94, 0.03) 100%);
    border-bottom: 1px solid rgba(34, 197, 94, 0.2);
    padding: 8px 20px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    z-index: 10;
  }

  .summary-header {
    font-size: 0.6rem;
    font-weight: 800;
    color: #22c55e;
    letter-spacing: 0.05em;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 2px;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    background: #22c55e;
    border-radius: 50%;
    animation: pulse-green 2s infinite;
  }

  @keyframes pulse-green {
    0% { transform: scale(1); box-shadow: 0 0 0 0 rgba(34, 197, 94, 0.7); }
    70% { transform: scale(1.1); box-shadow: 0 0 0 6px rgba(34, 197, 94, 0); }
    100% { transform: scale(1); box-shadow: 0 0 0 0 rgba(34, 197, 94, 0); }
  }

  .summary-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .summary-item {
    display: flex;
    gap: 12px;
    font-size: 0.7rem;
    align-items: center;
  }

  .s-time { color: rgba(255,255,255,0.3); font-family: var(--font-mono); font-size: 0.65rem; }
  .s-sym { font-weight: 800; color: #22c55e; min-width: 40px; }
  .s-desc { color: rgba(255,255,255,0.7); font-style: italic; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
