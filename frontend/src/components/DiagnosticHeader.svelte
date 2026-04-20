<script lang="ts">
  import { prettyMoneyStrict } from "../lib/format";

  export let alpacaStatus: "idle" | "connecting" | "live" | "reconnecting" = "idle";
  export let kronosStatus: "active" | "standby" | "off" = "active";
  export let kronosLatency: number = 0;
  export let buyingPower: number = 0;

  function getStatusColor(status: string) {
    if (status === "live" || status === "active") return "#4ade80";
    if (status === "connecting" || status === "reconnecting" || status === "standby") return "#fbbf24";
    return "#f87171";
  }
</script>

<div class="diagnostic-header">
  <div class="stat-item">
    <span class="label">Alpaca Feed</span>
    <div class="value-row">
      <div class="indicator" style="background: {getStatusColor(alpacaStatus)}"></div>
      <span class="value">{alpacaStatus.toUpperCase()}</span>
    </div>
  </div>

  <div class="stat-item">
    <span class="label">Kronos AI</span>
    <div class="value-row">
      <div class="indicator" style="background: {getStatusColor(kronosStatus)}"></div>
      <span class="value">{kronosStatus.toUpperCase()}</span>
      {#if kronosStatus === "active"}
        <span class="latency">{kronosLatency}ms</span>
      {/if}
    </div>
  </div>

  <div class="stat-item">
    <span class="label">Buying Power</span>
    <span class="value money">{prettyMoneyStrict(buyingPower)}</span>
  </div>
</div>

<style>
  .diagnostic-header {
    display: flex;
    gap: 2rem;
    padding: 12px 24px;
    background: rgba(0, 0, 0, 0.4);
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(10px);
    font-size: 0.85rem;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 1px;
  }

  .value-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .indicator {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    box-shadow: 0 0 10px currentColor;
  }

  .value {
    color: white;
    font-weight: 600;
  }

  .latency {
    color: rgba(255, 255, 255, 0.4);
    font-size: 0.75rem;
    font-weight: normal;
  }

  .money {
    color: #4ade80;
    font-family: inherit;
  }
</style>
