<!--
  SPIRE — Global Context Menu

  Custom right-click context menu that replaces the browser default.
  Subscribes to `contextMenuStore` for position, visibility, and
  menu items.  Rendered as a fixed-position overlay; dismissed on
  click-outside, Escape, or item activation.

  This component should be placed once in the root layout.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    contextMenu,
    hideContextMenu,
  } from "$lib/stores/contextMenuStore";
  import type { ContextMenuItem } from "$lib/stores/contextMenuStore";

  function handleItemClick(item: ContextMenuItem): void {
    if (item.disabled) return;
    hideContextMenu();
    item.action();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" && $contextMenu.visible) {
      event.preventDefault();
      hideContextMenu();
    }
  }

  function handleClickOutside(): void {
    if ($contextMenu.visible) {
      hideContextMenu();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

{#if $contextMenu.visible}
  <!-- Invisible backdrop to catch outside clicks -->
  <div
    class="ctx-backdrop"
    on:mousedown={handleClickOutside}
    role="presentation"
  ></div>

  <!-- Context menu panel -->
  <div
    class="ctx-menu"
    style="left: {$contextMenu.x}px; top: {$contextMenu.y}px;"
    role="menu"
    aria-label="Context menu"
  >
    {#each $contextMenu.items as item (item.id)}
      {#if item.separator}
        <div class="ctx-separator" role="separator"></div>
      {/if}
      <button
        class="ctx-item"
        class:disabled={item.disabled}
        on:click={() => handleItemClick(item)}
        role="menuitem"
        disabled={item.disabled}
      >
        {#if item.icon}
          <span class="ctx-icon">{item.icon}</span>
        {/if}
        <span class="ctx-label">{item.label}</span>
        {#if item.shortcut}
          <span class="ctx-shortcut">{item.shortcut}</span>
        {/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9998;
  }

  .ctx-menu {
    position: fixed;
    z-index: 9999;
    min-width: 180px;
    max-width: 280px;
    background: var(--bg-primary, #121212);
    border: 1px solid var(--border, #333);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    padding: 0.2rem 0;
    font-family: var(--font-mono);
  }

  .ctx-separator {
    height: 1px;
    background: var(--border, #333);
    margin: 0.15rem 0;
  }

  .ctx-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.6rem;
    background: none;
    border: none;
    color: var(--fg-primary, #e8e8e8);
    font-size: 0.75rem;
    font-family: var(--font-mono);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    transition: background 0.08s;
  }

  .ctx-item:hover:not(.disabled) {
    background: var(--bg-surface, #1a1a1a);
    color: var(--fg-accent, #fff);
  }

  .ctx-item.disabled {
    color: var(--fg-secondary, #888);
    cursor: default;
    opacity: 0.5;
  }

  .ctx-icon {
    width: 1.2em;
    text-align: center;
    font-size: 0.82rem;
    flex-shrink: 0;
  }

  .ctx-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ctx-shortcut {
    font-size: 0.62rem;
    color: var(--fg-secondary, #888);
    margin-left: auto;
    flex-shrink: 0;
  }
</style>
