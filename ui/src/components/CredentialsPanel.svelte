<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { CreateCredentialRequest, CredentialEnvironment, CredentialSummary } from "../lib/types";

  export let credentials: CredentialSummary[] = [];

  const dispatch = createEventDispatcher<{
    submit: CreateCredentialRequest;
  }>();

  let label = "Primary Alpaca";
  let api_key = "";
  let api_secret = "";
  let environment: CredentialEnvironment = "paper";
  let use_for_data = true;
  let use_for_trading = true;

  function submit() {
    dispatch("submit", {
      label,
      api_key,
      api_secret,
      environment,
      use_for_data,
      use_for_trading,
    });
    api_key = "";
    api_secret = "";
  }
</script>

<section class="panel">
  <div class="panel-header">
    <div>
      <p>Broker Credentials</p>
      <h2>Yahoo first, Alpaca optional</h2>
    </div>
  </div>

  <div class="warning">
    Yahoo is the default market-data source. Alpaca keys are stored in the app database, and the backend supports an app master key via <code>AUTO_STONKS_MASTER_KEY</code>.
  </div>

  <form class="credential-form" on:submit|preventDefault={submit}>
  <div class="form-grid">
    <label>
      <span>Label</span>
      <input id="credential-label" name="label" bind:value={label} />
    </label>
    <label>
      <span>Environment</span>
      <select id="credential-environment" name="environment" bind:value={environment}>
        <option value="paper">Paper</option>
        <option value="live">Live</option>
      </select>
    </label>
    <label>
      <span>API key</span>
      <input id="credential-api-key" name="api_key" bind:value={api_key} />
    </label>
    <label>
      <span>API secret</span>
      <input id="credential-api-secret" name="api_secret" type="password" bind:value={api_secret} />
    </label>
  </div>

  <div class="toggles">
    <label><input id="credential-use-for-data" name="use_for_data" type="checkbox" bind:checked={use_for_data} /> Use for data</label>
    <label><input id="credential-use-for-trading" name="use_for_trading" type="checkbox" bind:checked={use_for_trading} /> Use for trading</label>
  </div>

  <button type="submit">Store credential</button>
  </form>

  <div class="credential-list">
    {#each credentials as credential}
      <article>
        <strong>{credential.label}</strong>
        <span>{credential.environment} · {credential.masked_key}</span>
        <small>
          {credential.use_for_data ? "data" : ""}{credential.use_for_data && credential.use_for_trading ? " + " : ""}
          {credential.use_for_trading ? "trading" : ""}
        </small>
      </article>
    {/each}
  </div>
</section>

<style>
  .panel {
    padding: 1.2rem;
    border-radius: 26px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: linear-gradient(180deg, rgba(20, 18, 33, 0.96), rgba(12, 10, 22, 0.92));
  }

  .panel-header p,
  .panel-header h2 {
    margin: 0;
  }

  .panel-header p {
    color: rgba(221, 233, 255, 0.64);
  }

  .panel-header h2 {
    margin-top: 0.25rem;
    color: white;
  }

  .warning {
    margin: 1rem 0;
    padding: 0.9rem 1rem;
    border-radius: 18px;
    background: rgba(255, 186, 85, 0.08);
    border: 1px solid rgba(255, 186, 85, 0.18);
    color: rgba(255, 232, 198, 0.92);
    line-height: 1.45;
  }

  .form-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.8rem;
  }

  .credential-form {
    display: contents;
  }

  label {
    display: grid;
    gap: 0.35rem;
  }

  span,
  small {
    color: rgba(221, 233, 255, 0.7);
  }

  input,
  select,
  button {
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(8, 11, 19, 0.88);
    color: white;
    padding: 0.8rem 0.9rem;
    font: inherit;
  }

  button {
    margin-top: 0.9rem;
    cursor: pointer;
    background: linear-gradient(135deg, #f3b24f, #f9d477);
    color: #1a1203;
    font-weight: 700;
  }

  .toggles {
    display: flex;
    gap: 1rem;
    margin-top: 0.8rem;
    color: rgba(221, 233, 255, 0.8);
  }

  .toggles label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .credential-list {
    display: grid;
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .credential-list article {
    display: grid;
    gap: 0.15rem;
    padding: 0.85rem 0.95rem;
    border-radius: 18px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .credential-list strong {
    color: white;
  }

  @media (max-width: 720px) {
    .form-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
