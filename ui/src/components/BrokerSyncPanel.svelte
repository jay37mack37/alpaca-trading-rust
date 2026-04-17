<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { StrategyDetailResponse } from "../lib/types";

  export let detail: StrategyDetailResponse | null = null;
  export let loading = false;

  const dispatch = createEventDispatcher<{ sync: { strategyId: string } }>();

  function money(value: number | null | undefined) {
    return value == null ? "—" : `$${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <p>Alpaca Sync</p>
      <h2>{detail?.strategy.name ?? "Select a strategy"}</h2>
    </div>
    {#if detail}
      <button type="button" on:click={() => dispatch("sync", { strategyId: detail.strategy.id })}>
        Sync now
      </button>
    {/if}
  </div>

  {#if loading}
    <div class="empty">Loading broker state…</div>
  {:else if !detail}
    <div class="empty">Choose a strategy to inspect its broker-linked state.</div>
  {:else if !detail.strategy.credential_id}
    <div class="empty">This strategy is not linked to an Alpaca credential yet.</div>
  {:else if !detail.broker_sync}
    <div class="empty">No synced Alpaca snapshot yet. Trigger a sync after linking the credential.</div>
  {:else}
    <div class="account-grid">
      <article>
        <span>Environment</span>
        <strong>{detail.broker_sync.environment}</strong>
      </article>
      <article>
        <span>Equity</span>
        <strong>{money(detail.broker_sync.account?.equity)}</strong>
      </article>
      <article>
        <span>Buying power</span>
        <strong>{money(detail.broker_sync.account?.buying_power)}</strong>
      </article>
      <article>
        <span>Status</span>
        <strong>{detail.broker_sync.account?.status ?? "—"}</strong>
      </article>
    </div>

    <div class="meta">
      Last synced {new Date(detail.broker_sync.synced_at).toLocaleString()}
    </div>

    <div class="tables">
      <article>
        <h3>Broker positions</h3>
        <table>
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Qty</th>
              <th>Value</th>
              <th>UPL</th>
            </tr>
          </thead>
          <tbody>
            {#each detail.broker_sync.positions as position}
              <tr>
                <td>{position.symbol}</td>
                <td>{position.quantity.toFixed(3)}</td>
                <td>{money(position.market_value)}</td>
                <td>{money(position.unrealized_pl)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </article>

      <article>
        <h3>Broker orders</h3>
        <table>
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Status</th>
              <th>Qty</th>
              <th>Filled</th>
            </tr>
          </thead>
          <tbody>
            {#each detail.broker_sync.orders.slice(0, 10) as order}
              <tr>
                <td>{order.symbol ?? "—"}</td>
                <td>{order.status ?? "—"}</td>
                <td>{order.quantity?.toFixed(3) ?? "—"}</td>
                <td>{order.filled_qty?.toFixed(3) ?? "—"}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </article>
    </div>

    <div class="flags">
      <span>Pattern day trader: {detail.broker_sync.account?.pattern_day_trader ? "yes" : "no"}</span>
      <span>Trading blocked: {detail.broker_sync.account?.trading_blocked ? "yes" : "no"}</span>
      <span>Transfers blocked: {detail.broker_sync.account?.transfers_blocked ? "yes" : "no"}</span>
      <span>Account blocked: {detail.broker_sync.account?.account_blocked ? "yes" : "no"}</span>
    </div>
  {/if}
</section>

<style>
  .panel {
    padding: 1.2rem;
    border-radius: 26px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: linear-gradient(180deg, rgba(14, 27, 39, 0.96), rgba(8, 14, 22, 0.92));
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: baseline;
  }

  .panel-header p,
  .meta,
  .flags span {
    color: rgba(221, 233, 255, 0.65);
  }

  .panel-header p,
  .panel-header h2 {
    margin: 0;
  }

  .panel-header h2 {
    margin-top: 0.2rem;
    color: white;
  }

  button {
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: linear-gradient(135deg, #5fc0ff, #7de6d1);
    color: #04131b;
    padding: 0.75rem 0.95rem;
    font: inherit;
    font-weight: 700;
    cursor: pointer;
  }

  .empty {
    margin-top: 1rem;
    padding: 1rem;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
    color: rgba(221, 233, 255, 0.72);
  }

  .account-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.8rem;
    margin-top: 1rem;
  }

  .account-grid article {
    padding: 0.95rem;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .account-grid span {
    color: rgba(221, 233, 255, 0.6);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .account-grid strong {
    display: block;
    margin-top: 0.25rem;
    color: white;
  }

  .meta {
    margin: 0.9rem 0 0;
  }

  .tables {
    display: grid;
    gap: 1rem;
    margin-top: 1rem;
  }

  .tables article {
    padding: 1rem;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  h3 {
    margin-top: 0;
    color: white;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    text-align: left;
    padding: 0.55rem 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    color: rgba(228, 238, 255, 0.82);
  }

  th {
    color: rgba(221, 233, 255, 0.56);
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  .flags {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem;
    margin-top: 1rem;
  }

  @media (max-width: 900px) {
    .account-grid,
    .flags {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @media (max-width: 640px) {
    .account-grid,
    .flags {
      grid-template-columns: 1fr;
    }
  }
</style>
