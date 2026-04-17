<script lang="ts">
  export let label: string;
  export let value: string;
  export let detail = "";
  export let tone: "neutral" | "positive" | "negative" = "neutral";
</script>

<article class={`metric metric--${tone}`}>
  <p>{label}</p>
  <h3>{value}</h3>
  {#if detail}
    <span>{detail}</span>
  {/if}
</article>

<style>
  .metric {
    border-radius: 22px;
    padding: 1.1rem 1.2rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: linear-gradient(180deg, rgba(18, 23, 39, 0.94), rgba(10, 14, 25, 0.86));
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.03);
  }

  p {
    margin: 0;
    font-size: 0.8rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: rgba(221, 233, 255, 0.56);
  }

  h3 {
    margin: 0.45rem 0 0.2rem;
    font-size: clamp(1.15rem, 2vw, 1.8rem);
    line-height: 1.1;
    color: white;
  }

  span {
    color: rgba(221, 233, 255, 0.7);
    font-size: 0.92rem;
  }

  .metric--positive h3 {
    color: #70f7b1;
  }

  .metric--negative h3 {
    color: #ff7f7f;
  }
</style>

