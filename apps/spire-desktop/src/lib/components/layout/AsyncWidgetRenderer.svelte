<script lang="ts">
  import { loadWidgetComponent } from "$lib/core/registry/WidgetRegistry";
  import UnknownWidget from "$lib/components/shared/UnknownWidget.svelte";

  export let widgetType: string;

  $: componentPromise = loadWidgetComponent(widgetType);
</script>

{#await componentPromise}
  <div class="widget-loading">Loading…</div>
{:then WidgetComponent}
  {#if WidgetComponent}
    <svelte:component this={WidgetComponent} />
  {:else}
    <UnknownWidget {widgetType} />
  {/if}
{:catch}
  <UnknownWidget {widgetType} />
{/await}

<style>
  .widget-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    color: var(--color-text-muted);
    font-size: 0.75rem;
    font-family: var(--font-mono);
  }
</style>
