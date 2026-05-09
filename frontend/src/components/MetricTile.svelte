<script lang="ts">
  export let label: string;
  export let value: string;
  export let detail = "";
  export let tone: "neutral" | "positive" | "negative" = "neutral";
</script>

<article class={`metric glass-panel metric--${tone}`}>
  <p>{label}</p>
  <h3 class="font-mono">{value}</h3>
  {#if detail}
    <span>{detail}</span>
  {/if}
</article>

<style>
  .metric {
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    justify-content: center;
  }

  p {
    margin: 0;
    font-size: 0.65rem;
    font-weight: 800;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: #64748b;
  }

  h3 {
    margin: 0.5rem 0 0.25rem;
    font-size: 1.75rem;
    font-weight: 800;
    line-height: 1;
    color: var(--text-primary);
  }

  span {
    color: #475569;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .metric--positive h3 {
    color: #4ade80;
  }

  .metric--negative h3 {
    color: #f87171;
  }
</style>
