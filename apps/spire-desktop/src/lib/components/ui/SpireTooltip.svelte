<script lang="ts">
  export let text = "";
  export let top = 0;
  export let left = 0;
  export let visible = false;
  export let maxWidth = 360;
</script>

{#if visible}
  <div
    class="spire-tooltip"
    role="tooltip"
    style={`top:${top}px;left:${left}px;max-width:${maxWidth}px;`}
  >
    {text}
  </div>
{/if}

<style>
  .spire-tooltip {
    position: fixed;
    z-index: 14000;
    pointer-events: none;
    padding: 0.35rem 0.5rem;
    border: 1px solid var(--border-color, var(--color-border));
    background: var(--color-bg-base);
    color: var(--color-text-primary);
    font-family: var(--font-mono, "Fira Code", monospace);
    font-size: 0.7rem;
    line-height: 1.35;
    white-space: normal;
  }
</style>
