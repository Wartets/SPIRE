<script lang="ts">
  export let widgetType: string;
  export let compact: boolean = false;
</script>

<div class="unknown-widget" class:compact>
  <span class="title">Unknown widget</span>
  <span class="type">{widgetType}</span>
</div>

<style>
  .unknown-widget {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: var(--hl-error);
    font-size: 0.75rem;
  }

  .unknown-widget.compact {
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--fg-secondary);
    font-style: italic;
    height: 100%;
    gap: 0.1rem;
  }

  .title {
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .type {
    font-family: var(--font-mono);
    word-break: break-word;
  }
</style>
