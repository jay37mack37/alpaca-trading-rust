<script lang="ts">
  import type { OptionContractSnapshot } from "../lib/types";

  export let options: OptionContractSnapshot[] = [];

  $: calls = options.filter((contract) => contract.option_type === "call").slice(0, 8);
  $: puts = options.filter((contract) => contract.option_type === "put").slice(0, 8);

  function formatNumber(value: number | null) {
    return value == null ? "—" : value.toLocaleString();
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <p>Options Tracker</p>
      <h2>Top chain liquidity snapshot</h2>
    </div>
    <span>{options.length} contracts cached</span>
  </div>

  <div class="options-grid">
    <article>
      <h3>Calls</h3>
      <table>
        <thead>
          <tr>
            <th>Strike</th>
            <th>Last</th>
            <th>OI</th>
            <th>Volume</th>
          </tr>
        </thead>
        <tbody>
          {#each calls as contract}
            <tr>
              <td>{contract.strike.toFixed(2)}</td>
              <td>{formatNumber(contract.last)}</td>
              <td>{formatNumber(contract.open_interest)}</td>
              <td>{formatNumber(contract.volume)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </article>

    <article>
      <h3>Puts</h3>
      <table>
        <thead>
          <tr>
            <th>Strike</th>
            <th>Last</th>
            <th>OI</th>
            <th>Volume</th>
          </tr>
        </thead>
        <tbody>
          {#each puts as contract}
            <tr>
              <td>{contract.strike.toFixed(2)}</td>
              <td>{formatNumber(contract.last)}</td>
              <td>{formatNumber(contract.open_interest)}</td>
              <td>{formatNumber(contract.volume)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </article>
  </div>
</section>

<style>
  .panel {
    padding: 1.2rem;
    border-radius: 26px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: linear-gradient(180deg, rgba(16, 24, 21, 0.96), rgba(9, 13, 11, 0.92));
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .panel-header p,
  .panel-header span {
    margin: 0;
    color: rgba(221, 233, 255, 0.64);
  }

  .panel-header h2 {
    margin: 0.2rem 0 0;
    color: white;
  }

  .options-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
  }

  article {
    padding: 1rem;
    border-radius: 20px;
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

  @media (max-width: 720px) {
    .options-grid {
      grid-template-columns: 1fr;
    }
  }
</style>

