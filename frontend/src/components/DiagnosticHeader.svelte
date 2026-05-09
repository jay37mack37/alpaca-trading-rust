<script lang="ts">
  import { prettyMoneyStrict } from "../lib/format";

  export let alpacaActive: boolean = false;
  export let kronosActive: boolean = false;
  export let optionsActive: boolean = false;
  export let kronosLatency: number = 0;
  export let buyingPower: number = 0;

  function getStatusColor(active: boolean) {
    return active ? "var(--accent-cyan)" : "var(--accent-coral)";
  }
</script>

<div class="diagnostic-header glass-panel">
  <div class="stat-item">
    <span class="label">Broker</span>
    <div class="value-row">
      <div class="indicator" style="background: {getStatusColor(alpacaActive)}; box-shadow: 0 0 12px {getStatusColor(alpacaActive)}"></div>
      <span class="value">{alpacaActive ? "ALPACA LIVE" : "DISCONNECTED"}</span>
    </div>
  </div>

  <div class="stat-item">
    <span class="label">Kronos Intelligence</span>
    <div class="value-row">
      <div class="indicator" style="background: {getStatusColor(kronosActive)}; box-shadow: 0 0 12px {getStatusColor(kronosActive)}"></div>
      <span class="value">{kronosActive ? "SYNCED" : "OFFLINE"}</span>
      {#if kronosActive}
        <span class="latency font-mono">{kronosLatency}ms</span>
      {/if}
    </div>
  </div>

  <div class="stat-item">
    <span class="label">Market Data</span>
    <div class="value-row">
      <div class="indicator" style="background: {getStatusColor(optionsActive)}; box-shadow: 0 0 12px {getStatusColor(optionsActive)}"></div>
      <span class="value">{optionsActive ? "READY" : "STALE"}</span>
    </div>
  </div>

  <div class="stat-item">
    <span class="label">Buying Power</span>
    <span class="value money font-mono">{prettyMoneyStrict(buyingPower)}</span>
  </div>

  <div class="stat-item version-item">
    <span class="label">Terminal</span>
    <span class="value font-mono">v0.2.0-PRO</span>
  </div>
</div>

<style>
  .diagnostic-header {
    display: flex;
    gap: 3rem;
    padding: 0.75rem 2rem;
    background: rgba(0, 0, 0, 0.6);
    border-radius: 0;
    border-top: none;
    border-left: none;
    border-right: none;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    backdrop-filter: blur(20px);
    z-index: 100;
    position: sticky;
    top: 0;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  .label {
    color: #475569;
    font-size: 0.6rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.1rem;
    margin-bottom: 0.25rem;
  }

  .value-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .indicator {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .value {
    color: var(--text-primary);
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .latency {
    color: #475569;
    font-size: 0.7rem;
  }

  .money {
    color: var(--accent-cyan);
  }

  .version-item {
    margin-left: auto;
    text-align: right;
  }
</style>
