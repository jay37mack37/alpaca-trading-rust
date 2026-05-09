<script lang="ts">
  import type { PositionSummary, BrokerPositionSummary } from "../lib/types";
  import PositionCard from "./PositionCard.svelte";
  import BrokerPositionCard from "./BrokerPositionCard.svelte";

  import { api } from "../lib/api";

  export let positions: PositionSummary[] = [];
  export let brokerPositions: BrokerPositionSummary[] = [];
  export let strategies: any[] = [];

  let isSellingAll = false;

  async function handleSellAll() {
    if (!confirm("Are you sure you want to FORCE LIQUIDATE ALL positions? This will hit Alpaca directly and close everything visible, even if the bot's metadata is lost.")) return;
    isSellingAll = true;
    try {
      await api.liquidateAllBrokerPositions();
    } catch (err) {
      console.error(err);
      alert("Failed to force liquidate all positions. Check logs.");
    } finally {
      isSellingAll = false;
    }
  }

  function handleFlattened(event: CustomEvent<string>) {
    const symbol = event.detail;
    // Note: Parent should ideally update the positions array via the WebSocket/State
    // but we can optimistically filter here if needed.
  }
</script>

<section class="positions-section">
  <div class="section-header">
    <div class="header-main">
        <h2>Live Deployments</h2>
        <span class="count">{positions.length + brokerPositions.length} Total</span>
        {#if positions.length > 0 || brokerPositions.length > 0}
          <button class="sell-all-btn" on:click={handleSellAll} disabled={isSellingAll}>
            {isSellingAll ? 'FLATTENING...' : 'SELL ALL'}
          </button>
        {/if}
    </div>
    <p>Real-time surveillance of tracked strategies and manual broker positions.</p>
  </div>
  
  <div class="positions-scroll">
    {#if positions.length === 0 && brokerPositions.length === 0}
      <div class="empty-positions">
        <p>No active deployments or broker positions found.</p>
      </div>
    {:else}
      {#each positions as position (position.strategy_id + position.instrument_symbol)}
        <PositionCard {position} strategyName={strategies.find(s => s.id === position.strategy_id)?.name || position.strategy_id} on:flattened={handleFlattened} />
      {/each}

      {#each brokerPositions as brokerPos (brokerPos.symbol)}
          {#if !positions.some(p => p.instrument_symbol === brokerPos.symbol)}
              <BrokerPositionCard position={brokerPos} />
          {/if}
      {/each}
    {/if}
  </div>
</section>

<style>
  .positions-section {
    margin-top: 1rem;
    margin-bottom: 3rem;
  }

  .section-header {
    margin-bottom: 1.5rem;
    border-left: 3px solid #ef4444;
    padding-left: 1rem;
  }

  .header-main {
    display: flex;
    align-items: baseline;
    gap: 1rem;
  }

  .section-header h2 {
    font-size: 1.25rem;
    font-weight: 600;
    margin-bottom: 0.25rem;
    color: white;
  }

  .count {
    font-size: 0.75rem;
    color: #ef4444;
    font-weight: 800;
    text-transform: uppercase;
    background: rgba(239, 68, 68, 0.1);
    padding: 0.1rem 0.5rem;
    border-radius: 4px;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }

  .section-header p {
    font-size: 0.85rem;
    color: rgba(221, 233, 255, 0.6);
  }

  .sell-all-btn {
    margin-left: auto;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.4);
    color: #f87171;
    font-size: 0.7rem;
    font-weight: 800;
    letter-spacing: 0.05em;
    padding: 0.25rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .sell-all-btn:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.3);
    color: white;
  }

  .sell-all-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .positions-scroll {
    display: flex;
    gap: 1.5rem;
    overflow-x: auto;
    padding: 0.5rem 0.25rem 1.5rem 0.25rem;
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.1) transparent;
  }

  .positions-scroll::-webkit-scrollbar {
    height: 6px;
  }

  .positions-scroll::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 3px;
  }

  .empty-positions {
    padding: 3rem;
    text-align: center;
    color: rgba(255, 255, 255, 0.3);
    font-style: italic;
    font-size: 0.9rem;
    width: 100%;
    background: rgba(0, 0, 0, 0.2);
    border-radius: 8px;
    border: 1px dashed rgba(255, 255, 255, 0.1);
  }
</style>
