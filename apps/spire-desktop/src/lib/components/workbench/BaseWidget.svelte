<!--
  SPIRE - Base Widget Shell

  Standard wrapper for every widget on the workbench canvas.
  Provides a consistent header bar with title, optional extra
  controls (via the `header-extra` slot), and a close button.
  The widget body is rendered through the default slot.

  Props:
    title      - Header text shown in the widget bar
    widgetId   - Unique instance ID (used for close dispatch)
    closable   - Whether to show the × close button (default: true)

  Events:
    close      - Dispatched when the close button is clicked

  @component
  @param {string} title Header title rendered in the widget chrome.
  @param {string} widgetId Stable widget instance identifier emitted with `close`.
  @param {boolean} closable Whether the close affordance is shown.
  @fires close Emitted with `widgetId` when the close button is clicked.
-->
<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";

  export let title: string = "Widget";
  export let widgetId: string = "";
  export let closable: boolean = true;

  const dispatch = createEventDispatcher<{ close: string }>();

  function handleClose(): void {
    dispatch("close", widgetId);
  }
</script>

<div class="base-widget">
  <header class="widget-header">
    <span class="widget-title">{title}</span>
    <div class="widget-header-extra">
      <slot name="header-extra" />
    </div>
    {#if closable}
      <button
        class="widget-close"
        on:click={handleClose}
        aria-label="Close {title}"
        use:tooltip={{ text: "Close widget" }}
      >&times;</button>
    {/if}
  </header>
  <div class="widget-body">
    <slot />
  </div>
</div>

<style>
  .base-widget {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  .widget-header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 1.6rem;
  }
  .widget-title {
    font-size: 0.72rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    flex-shrink: 1;
  }
  .widget-header-extra {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    justify-content: flex-end;
    min-width: 0;
  }
  .widget-close {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary);
    font-size: 0.9rem;
    cursor: pointer;
    padding: 0 0.2rem;
    line-height: 1;
    flex-shrink: 0;
    font-family: var(--font-mono);
  }
  .widget-close:hover {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }
  .widget-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem;
    min-height: 0;
  }
</style>
