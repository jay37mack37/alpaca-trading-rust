<script lang="ts">
  import type { PositionSummary } from "../lib/types";
  import PositionCard from "./PositionCard.svelte";

  export let positions: PositionSummary[] = [];

  function handleFlattened(event: CustomEvent<string>) {
    const symbol = event.detail;
    // Note: Parent should ideally update the positions array via the WebSocket/State
    // but we can optimistically filter here if needed.
  }
</script>

{#if positions.length > 0}
<section class="positions-section">
  <div class="section-header">
    <div class="header-main">
        <h2>Open Combat Positions</h2>
        <span class="count">{positions.length} Active</span>
    </div>
    <p>Real-time surveillance of live deployments and risk parameters.</p>
  </div>
  
  <div class="positions-scroll">
    {#each positions as position (position.strategy_id + position.instrument_symbol)}
      <PositionCard {position} on:flattened={handleFlattened} />
    {/each}
  </div>
</section>
{/if}

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
</style>
