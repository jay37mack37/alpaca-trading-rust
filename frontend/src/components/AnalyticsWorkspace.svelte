<script lang="ts">
  import { api } from "../lib/api";
  import type { DataProvider } from "../lib/types";

  let symbols = "";
  let providers: DataProvider[] = ["yahoo", "alpaca"];
  let selectedProvider: DataProvider = "yahoo";
  let loading = false;
  let results: any = null;
  let error = "";

  async function runAnalysis() {
    loading = true;
    error = "";
    try {
      // In a real implementation, this would call a backend endpoint
      // that triggers the Python scripts we restored.
      // For now, we simulate the results based on the symbols provided.
      await new Promise(r => setTimeout(r, 1500));
      
      const symbolList = symbols.split(",").map(s => s.trim().toUpperCase()).filter(Boolean);
      results = {
        timestamp: new Date().toISOString(),
        symbols: symbolList.length > 0 ? symbolList : ["SPY", "AAPL", "MSFT"],
        patterns: [
          { symbol: "SPY", pattern: "Double Bottom", confidence: 0.85, direction: "bullish" },
          { symbol: "AAPL", pattern: "Head and Shoulders", confidence: 0.72, direction: "bearish" },
          { symbol: "MSFT", pattern: "Bull Flag", confidence: 0.65, direction: "bullish" }
        ]
      };
    } catch (err) {
      error = err instanceof Error ? err.message : "Analysis failed";
    } finally {
      loading = false;
    }
  }
</script>

<div class="analytics-layout">
  <section class="controls-card">
    <div class="card-header">
      <p class="eyebrow">Pattern Engine</p>
      <h2>Analytics Workspace</h2>
    </div>

    <div class="controls-grid">
      <div class="input-group">
        <label for="ana-symbols">Symbols</label>
        <input id="ana-symbols" bind:value={symbols} placeholder="SPY, AAPL, MSFT..." />
        <p class="help">Comma separated list. Leave empty for active watchlist.</p>
      </div>

      <div class="input-group">
        <label for="ana-provider">Data Source</label>
        <select id="ana-provider" bind:value={selectedProvider}>
          {#each providers as p}
            <option value={p}>{p.toUpperCase()}</option>
          {/each}
        </select>
      </div>

      <div class="actions">
        <button type="button" class="btn-primary" on:click={runAnalysis} disabled={loading}>
          {loading ? "Analyzing..." : "Run Pattern Analysis"}
        </button>
      </div>
    </div>
  </section>

  {#if error}
    <div class="banner error">{error}</div>
  {/if}

  {#if results}
    <section class="results-grid">
      {#each results.patterns as pattern}
        <article class="pattern-card">
          <header>
            <span class="symbol">{pattern.symbol}</span>
            <span class="confidence">{(pattern.confidence * 100).toFixed(0)}% Conf.</span>
          </header>
          <div class="pattern-name">{pattern.pattern}</div>
          <footer class:bullish={pattern.direction === "bullish"} class:bearish={pattern.direction === "bearish"}>
            {pattern.direction.toUpperCase()}
          </footer>
        </article>
      {/each}
    </section>
  {/if}
</div>

<style>
  .analytics-layout {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .controls-card {
    background: rgba(13, 17, 23, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 12px;
    padding: 24px;
  }

  .card-header {
    margin-bottom: 24px;
  }

  .card-header h2 {
    margin: 0;
    font-size: 1.5rem;
  }

  .eyebrow {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--color-text-dim);
    margin: 0 0 4px 0;
  }

  .controls-grid {
    display: grid;
    grid-template-columns: 1fr 200px auto;
    gap: 20px;
    align-items: flex-end;
  }

  .input-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .input-group label {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--color-text-dim);
  }

  .help {
    font-size: 0.7rem;
    color: var(--color-text-dim);
    margin: 4px 0 0 0;
  }

  input, select {
    background: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 10px 14px;
    color: white;
    font-size: 0.9rem;
  }

  .btn-primary {
    background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
    color: white;
    border: none;
    padding: 12px 24px;
    border-radius: 6px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(37, 99, 235, 0.3);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .results-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }

  .pattern-card {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 12px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .pattern-card header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .symbol {
    font-weight: 700;
    font-size: 1.1rem;
  }

  .confidence {
    font-size: 0.75rem;
    color: var(--color-text-dim);
    background: rgba(255, 255, 255, 0.05);
    padding: 2px 6px;
    border-radius: 4px;
  }

  .pattern-name {
    font-size: 1.2rem;
    font-weight: 500;
  }

  footer {
    font-size: 0.75rem;
    font-weight: 800;
    letter-spacing: 0.05em;
    padding-top: 8px;
    border-top: 1px solid rgba(255, 255, 255, 0.04);
  }

  footer.bullish { color: #4ade80; }
  footer.bearish { color: #f87171; }

  .banner {
    padding: 12px 20px;
    border-radius: 8px;
    font-size: 0.9rem;
  }

  .banner.error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    color: #f87171;
  }
</style>
