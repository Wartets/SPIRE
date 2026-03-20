<script lang="ts">
  export let title: string;
  export let subtitle = "";
  export let detail = "";
</script>

<article class="citation-card">
  <h5>{title}</h5>
  {#if subtitle}
    <p class="subtitle">{subtitle}</p>
  {/if}
  {#if detail}
    <p class="detail">{detail}</p>
  {/if}
</article>

<style>
  .citation-card {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: 0.35rem 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  h5 {
    margin: 0;
    font-size: 0.74rem;
    color: var(--fg-primary);
    font-family: var(--font-mono);
  }

  .subtitle,
  .detail {
    margin: 0;
    color: var(--fg-secondary);
    font-size: 0.67rem;
    font-family: var(--font-mono);
  }
</style>
