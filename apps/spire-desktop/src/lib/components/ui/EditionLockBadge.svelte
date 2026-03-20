<script lang="ts">
  export let edition: string | null = null;
  export let stale = false;
  export let mismatch = false;
</script>

<span class="edition-lock-badge" class:stale={stale} class:mismatch={mismatch}>
  <span class="label">PDG lock</span>
  <strong>{edition ?? "unlocked"}</strong>
</span>

<style>
  .edition-lock-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    padding: 0.12rem 0.4rem;
    font-size: 0.66rem;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .edition-lock-badge strong {
    color: var(--fg-primary);
    text-transform: none;
    letter-spacing: normal;
  }

  .edition-lock-badge.stale,
  .edition-lock-badge.mismatch {
    border-color: var(--hl-error);
    color: var(--hl-error);
  }

  .edition-lock-badge.stale strong,
  .edition-lock-badge.mismatch strong {
    color: var(--hl-error);
  }
</style>
