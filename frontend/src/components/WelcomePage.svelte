<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import {
    api,
    setApiToken,
    clearApiToken,
    apiTokenConfigured,
    fetchSetupStatus,
    writeEnvToken,
  } from "../lib/api";
  import type { CredentialEnvironment } from "../lib/types";

  const dispatch = createEventDispatcher<{ complete: void }>();

  type Step = "token" | "credentials" | "done";

  let step: Step = apiTokenConfigured() ? "credentials" : "token";
  let tokenInput = "";
  let tokenError = "";
  let tokenSaving = false;
  let backendOnline = false;
  let backendPolling = true;

  let label = "Alpaca Paper";
  let apiKey = "";
  let apiSecret = "";
  let environment: CredentialEnvironment = "paper";
  let useForData = true;
  let useForTrading = true;
  let credError = "";
  let credSaving = false;

  // Poll backend health
  let healthInterval: ReturnType<typeof setInterval>;
  $: {
    if (backendPolling) {
      healthInterval = setInterval(checkHealth, 3000);
    }
  }

  async function checkHealth() {
    try {
      const res = await fetch("/api/health");
      backendOnline = res.ok;
      if (backendOnline && backendPolling) {
        backendPolling = false;
        clearInterval(healthInterval);
      }
    } catch {
      backendOnline = false;
    }
  }

  // Initial check
  void checkHealth();

  async function connectToken() {
    tokenError = "";
    tokenSaving = true;
    const token = tokenInput.trim();

    if (!token) {
      tokenError = "Please enter the API token from the backend console output.";
      tokenSaving = false;
      return;
    }

    try {
      // Store in localStorage so the API layer picks it up immediately
      setApiToken(token);

      // Persist to frontend/.env via the backend
      const result = await writeEnvToken(token);

      // Check setup status (this now works because token is in localStorage)
      const status = await fetchSetupStatus();

      if (status.has_credentials) {
        step = "done";
      } else {
        step = "credentials";
      }
    } catch (err) {
      clearApiToken();
      const msg = err instanceof Error ? err.message : "Connection failed";
      if (msg.includes("401") || msg.toLowerCase().includes("invalid")) {
        tokenError = "Invalid token. Check the backend console output and try again.";
      } else if (msg.includes("Failed to fetch") || msg.includes("NetworkError")) {
        tokenError = "Cannot reach the backend. Make sure it's running on port 8080.";
      } else {
        tokenError = msg;
      }
    } finally {
      tokenSaving = false;
    }
  }

  async function saveCredentials() {
    credError = "";
    credSaving = true;

    try {
      await api.createCredential({
        label,
        api_key: apiKey.trim(),
        api_secret: apiSecret.trim(),
        environment,
        use_for_data: useForData,
        use_for_trading: useForTrading,
      });
      step = "done";
    } catch (err) {
      credError = err instanceof Error ? err.message : "Failed to save credentials";
    } finally {
      credSaving = false;
    }
  }

  function skipCredentials() {
    step = "done";
  }

  function launchDashboard() {
    dispatch("complete");
  }

  function resetSetup() {
    clearApiToken();
    step = "token";
    tokenInput = "";
    tokenError = "";
  }
</script>

<div class="welcome-shell">
  <div class="welcome-card">
    <div class="welcome-header">
      <p class="welcome-eyebrow">AutoStonks Algo Suite</p>
      <h1>Welcome</h1>
    </div>

    <!-- Backend status indicator -->
    <div class="backend-indicator" class:online={backendOnline} class:offline={!backendOnline}>
      <span class="indicator-dot"></span>
      <span>{backendOnline ? "Backend online" : "Waiting for backend..."}</span>
    </div>

    {#if step === "token"}
      <!-- Step 1: API Token -->
      <section class="step">
        <div class="step-header">
          <span class="step-badge">1</span>
          <div>
            <h2>Connect to Backend</h2>
            <p>Paste the API token printed by the backend on first startup.</p>
          </div>
        </div>

        <label class="field">
          <span class="field-label">API Token</span>
          <input
            type="text"
            bind:value={tokenInput}
            placeholder="e.g. 6f974436cb24058c..."
            on:keydown={(e) => e.key === "Enter" && void connectToken()}
            disabled={!backendOnline || tokenSaving}
          />
        </label>

        {#if tokenError}
          <div class="step-error">{tokenError}</div>
        {/if}

        <button
          class="step-button primary"
          on:click={() => void connectToken()}
          disabled={!backendOnline || tokenSaving || !tokenInput.trim()}
        >
          {tokenSaving ? "Connecting..." : "Connect"}
        </button>

        {#if !backendOnline}
          <p class="step-hint">Start the backend with <code>start.bat</code> or <code>./start.sh</code>, then return here.</p>
        {/if}
      </section>

    {:else if step === "credentials"}
      <!-- Step 2: Alpaca Credentials -->
      <section class="step">
        <div class="step-header">
          <span class="step-badge done">1</span>
          <div>
            <h3 style="margin:0;opacity:0.5">Backend connected</h3>
          </div>
        </div>

        <div class="step-divider"></div>

        <div class="step-header">
          <span class="step-badge">2</span>
          <div>
            <h2>Add Alpaca Keys</h2>
            <p>Optional — connect your Alpaca paper trading account.</p>
          </div>
        </div>

        <label class="field">
          <span class="field-label">Label</span>
          <input type="text" bind:value={label} placeholder="Alpaca Paper" />
        </label>

        <label class="field">
          <span class="field-label">API Key</span>
          <input type="text" bind:value={apiKey} placeholder="PK..." />
        </label>

        <label class="field">
          <span class="field-label">API Secret</span>
          <input type="password" bind:value={apiSecret} placeholder="Your secret key" />
        </label>

        <label class="field">
          <span class="field-label">Environment</span>
          <select bind:value={environment}>
            <option value="paper">Paper</option>
            <option value="live">Live</option>
          </select>
        </label>

        <div class="field-row">
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={useForData} />
            <span>Use for market data</span>
          </label>
          <label class="checkbox-label">
            <input type="checkbox" bind:checked={useForTrading} />
            <span>Use for trading</span>
          </label>
        </div>

        {#if credError}
          <div class="step-error">{credError}</div>
        {/if}

        <div class="step-actions">
          <button class="step-button" on:click={skipCredentials} disabled={credSaving}>
            Skip
          </button>
          <button
            class="step-button primary"
            on:click={() => void saveCredentials()}
            disabled={credSaving || !apiKey.trim() || !apiSecret.trim()}
          >
            {credSaving ? "Saving..." : "Save & Continue"}
          </button>
        </div>

        <button class="reset-link" on:click={resetSetup}>Change API token</button>
      </section>

    {:else}
      <!-- Step 3: Done -->
      <section class="step">
        <div class="step-header">
          <span class="step-badge done">1</span>
          <div>
            <h3 style="margin:0;opacity:0.5">Backend connected</h3>
          </div>
        </div>
        <div class="step-header">
          <span class="step-badge done">2</span>
          <div>
            <h3 style="margin:0;opacity:0.5">Alpaca keys configured</h3>
          </div>
        </div>

        <div class="step-divider"></div>

        <div class="done-message">
          <h2>Setup Complete</h2>
          <p>Your API token has been saved to <code>frontend/.env</code>. Your current session is already active. If you restart the Vite dev server later, the token will be picked up automatically.</p>
        </div>

        <button class="step-button primary" on:click={launchDashboard}>
          Launch Dashboard
        </button>
      </section>
    {/if}
  </div>
</div>

<style>
  .welcome-shell {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: calc(100vh - 6rem);
    padding: 2rem 1rem;
  }

  .welcome-card {
    width: min(520px, 100%);
    padding: 2.5rem;
    border-radius: 26px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background:
      radial-gradient(circle at top left, rgba(49, 104, 255, 0.12), transparent 40%),
      linear-gradient(180deg, rgba(18, 23, 39, 0.97), rgba(10, 14, 25, 0.95));
  }

  .welcome-header {
    margin-bottom: 1.5rem;
  }

  .welcome-eyebrow {
    margin: 0 0 0.25rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    font-size: 0.82rem;
    color: rgba(221, 233, 255, 0.62);
  }

  .welcome-header h1 {
    margin: 0;
    font-size: 2.4rem;
    line-height: 1;
  }

  /* Backend indicator */
  .backend-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.65rem 1rem;
    border-radius: 12px;
    margin-bottom: 1.5rem;
    font-size: 0.88rem;
    font-weight: 500;
  }

  .backend-indicator.online {
    background: rgba(72, 199, 142, 0.1);
    border: 1px solid rgba(72, 199, 142, 0.2);
    color: #48c78e;
  }

  .backend-indicator.offline {
    background: rgba(255, 183, 77, 0.08);
    border: 1px solid rgba(255, 183, 77, 0.18);
    color: #ffb74d;
  }

  .indicator-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
  }

  .backend-indicator.online .indicator-dot {
    box-shadow: 0 0 6px currentColor;
  }

  /* Steps */
  .step {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .step-header {
    display: flex;
    gap: 0.85rem;
    align-items: flex-start;
  }

  .step-header h2 {
    margin: 0;
    font-size: 1.3rem;
  }

  .step-header p {
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
    color: rgba(221, 233, 255, 0.55);
  }

  .step-badge {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: 2px solid rgba(108, 193, 255, 0.5);
    color: #6cc1ff;
    font-size: 0.85rem;
    font-weight: 700;
    flex-shrink: 0;
  }

  .step-badge.done {
    border-color: rgba(72, 199, 142, 0.5);
    color: #48c78e;
    background: rgba(72, 199, 142, 0.1);
  }

  .step-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 0.5rem 0;
  }

  /* Fields */
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .field-label {
    font-size: 0.8rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: rgba(221, 233, 255, 0.5);
  }

  .field input,
  .field select {
    padding: 0.75rem 0.9rem;
    border-radius: 12px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    color: #edf3ff;
    font-size: 0.95rem;
    outline: none;
    transition: border-color 160ms ease;
  }

  .field input:focus,
  .field select:focus {
    border-color: rgba(108, 193, 255, 0.5);
  }

  .field input::placeholder {
    color: rgba(221, 233, 255, 0.28);
  }

  .field input:disabled {
    opacity: 0.5;
  }

  .field-row {
    display: flex;
    gap: 1.5rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.88rem;
    color: rgba(221, 233, 255, 0.7);
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    accent-color: #6cc1ff;
  }

  /* Buttons */
  .step-button {
    padding: 0.8rem 1.5rem;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.06);
    color: rgba(236, 243, 255, 0.8);
    font-weight: 600;
    cursor: pointer;
    transition: all 160ms ease;
  }

  .step-button:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.18);
  }

  .step-button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .step-button.primary {
    background: linear-gradient(135deg, #3a7bfd, #6be7ff);
    color: #061020;
    border: none;
    font-weight: 700;
  }

  .step-button.primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .step-actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  /* Error and hints */
  .step-error {
    padding: 0.65rem 0.9rem;
    border-radius: 12px;
    background: rgba(255, 112, 112, 0.1);
    border: 1px solid rgba(255, 112, 112, 0.2);
    color: #ffb1b1;
    font-size: 0.88rem;
  }

  .step-hint {
    margin: 0;
    font-size: 0.82rem;
    color: rgba(221, 233, 255, 0.4);
  }

  .step-hint code {
    color: rgba(108, 193, 255, 0.7);
    background: rgba(108, 193, 255, 0.08);
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
  }

  .reset-link {
    align-self: flex-start;
    padding: 0;
    border: none;
    background: none;
    color: rgba(108, 193, 255, 0.6);
    font-size: 0.82rem;
    cursor: pointer;
    text-decoration: underline;
  }

  .reset-link:hover {
    color: rgba(108, 193, 255, 0.9);
  }

  /* Done */
  .done-message {
    padding: 1.2rem;
    border-radius: 16px;
    background: rgba(72, 199, 142, 0.06);
    border: 1px solid rgba(72, 199, 142, 0.15);
  }

  .done-message h2 {
    margin: 0 0 0.5rem;
    color: #48c78e;
  }

  .done-message p {
    margin: 0;
    font-size: 0.88rem;
    color: rgba(221, 233, 255, 0.6);
    line-height: 1.5;
  }

  .done-message code {
    color: rgba(108, 193, 255, 0.8);
    background: rgba(108, 193, 255, 0.08);
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
  }
</style>