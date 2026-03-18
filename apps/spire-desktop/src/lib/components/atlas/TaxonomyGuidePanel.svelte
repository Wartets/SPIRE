<script lang="ts">
  export let total = 0;
  export let charged = 0;
  export let neutral = 0;
  export let colored = 0;
  export let unstable = 0;
  export let interactionRich = 0;
  export let referenceCount = 0;
  export let activeFilter = "all";
  export let activeSort = "mass";
</script>

<section class="taxonomy-guide">
  <div class="guide-hero">
    <div>
      <h4>Taxonomy Navigator</h4>
      <p>Browse particles by family, refine by quantum features, and keep the scientific inspector visible while you compare states.</p>
    </div>
    <div class="hero-stats">
      <span>{total} states</span>
      <span>{referenceCount} references</span>
    </div>
  </div>

  <div class="guide-grid">
    <article>
      <h5>How To Read</h5>
      <p>The cards emphasize charge, spin, color representation, baryon content, and interaction coverage so the main taxonomy differences stay visible while scanning.</p>
    </article>
    <article>
      <h5>Quick Actions</h5>
      <p>Click to inspect. Use <kbd>Shift</kbd> to append to the initial state, or <kbd>Alt</kbd>/<kbd>Ctrl</kbd> to append to the final state.</p>
    </article>
    <article>
      <h5>Current Scope</h5>
      <p>Filter: <strong>{activeFilter}</strong> · Sort: <strong>{activeSort}</strong>. Use the inspector rail to compare decay structure and quantum invariants without losing place.</p>
    </article>
  </div>

  <div class="metric-row">
    <span><strong>{charged}</strong> charged</span>
    <span><strong>{neutral}</strong> neutral</span>
    <span><strong>{colored}</strong> colored</span>
    <span><strong>{unstable}</strong> unstable</span>
    <span><strong>{interactionRich}</strong> multi-interaction</span>
  </div>
</section>

<style>
  .taxonomy-guide {
    border: 1px solid var(--color-border, var(--border));
    background: color-mix(in srgb, var(--color-bg-surface, var(--bg-surface)) 82%, rgba(var(--color-accent-rgb), 0.05));
    padding: 0.48rem 0.52rem;
    display: flex;
    flex-direction: column;
    gap: 0.42rem;
  }

  .guide-hero,
  .guide-grid,
  .metric-row {
    display: flex;
    gap: 0.4rem;
  }

  .guide-hero {
    align-items: flex-start;
    justify-content: space-between;
  }

  .guide-hero h4,
  .guide-hero p,
  .guide-grid h5,
  .guide-grid p,
  .metric-row span,
  .metric-row strong,
  .hero-stats span,
  kbd {
    font-family: var(--font-mono);
  }

  .guide-hero h4 {
    margin: 0;
    font-size: 0.76rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  .guide-hero p {
    margin: 0.12rem 0 0;
    max-width: 54rem;
    font-size: 0.64rem;
    color: var(--color-text-muted, var(--fg-secondary));
    line-height: 1.45;
  }

  .hero-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 0.22rem;
    justify-content: flex-end;
  }

  .hero-stats span,
  .metric-row span {
    border: 1px solid rgba(var(--color-accent-rgb), 0.18);
    background: rgba(var(--color-accent-rgb), 0.06);
    color: var(--color-text-primary, var(--fg-primary));
    font-size: 0.58rem;
    padding: 0.08rem 0.24rem;
  }

  .guide-grid {
    flex-wrap: wrap;
  }

  .guide-grid article {
    flex: 1 1 14rem;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.15);
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.34rem 0.38rem;
  }

  .guide-grid h5 {
    margin: 0 0 0.14rem;
    font-size: 0.6rem;
    color: var(--color-accent, var(--hl-symbol));
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .guide-grid p {
    margin: 0;
    font-size: 0.61rem;
    color: var(--color-text-muted, var(--fg-secondary));
    line-height: 1.45;
  }

  .metric-row {
    flex-wrap: wrap;
  }

  .metric-row strong {
    color: var(--color-accent, var(--hl-symbol));
    font-weight: 700;
  }

  kbd {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    color: var(--color-text-primary, var(--fg-primary));
    padding: 0.02rem 0.18rem;
    font-size: 0.58rem;
  }
</style>
