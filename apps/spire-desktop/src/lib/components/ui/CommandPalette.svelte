<!--
  SPIRE — Command Palette

  A globally accessible modal overlay triggered via Ctrl+K / Cmd+K.
  Provides fuzzy search across all registered commands, groups results
  by category, and supports full keyboard navigation (arrows, Enter,
  Escape).

  This component is presentation-only: it knows nothing about physics
  or specific application features.  All available commands are sourced
  from the CommandRegistry store.
-->
<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    commands,
    closePalette,
    fuzzySearchCommands,
  } from "$lib/core/services/CommandRegistry";
  import type { Command } from "$lib/core/services/CommandRegistry";

  let query = "";
  let selectedIndex = 0;
  let inputEl: HTMLInputElement;

  // ── Filtered & Grouped Commands ──────────────────────────
  $: filtered = fuzzySearchCommands(query, $commands);

  // Group filtered results by category for visual separation
  interface CommandGroup {
    category: string;
    items: Command[];
  }
  $: groups = buildGroups(filtered);

  function buildGroups(cmds: Command[]): CommandGroup[] {
    const map = new Map<string, Command[]>();
    for (const cmd of cmds) {
      const existing = map.get(cmd.category);
      if (existing) {
        existing.push(cmd);
      } else {
        map.set(cmd.category, [cmd]);
      }
    }
    const result: CommandGroup[] = [];
    for (const [category, items] of map) {
      result.push({ category, items });
    }
    return result;
  }

  // Flat index → command mapping for keyboard navigation
  $: flatItems = filtered;

  // ── Keyboard Navigation ──────────────────────────────────
  function handleKeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        selectedIndex = (selectedIndex + 1) % Math.max(flatItems.length, 1);
        scrollSelectedIntoView();
        break;
      case "ArrowUp":
        event.preventDefault();
        selectedIndex =
          (selectedIndex - 1 + flatItems.length) % Math.max(flatItems.length, 1);
        scrollSelectedIntoView();
        break;
      case "Enter":
        event.preventDefault();
        if (flatItems[selectedIndex]) {
          invokeCommand(flatItems[selectedIndex]);
        }
        break;
      case "Escape":
        event.preventDefault();
        closePalette();
        break;
    }
  }

  function invokeCommand(cmd: Command): void {
    closePalette();
    // Execute after a microtask so the palette closes first
    requestAnimationFrame(() => cmd.execute());
  }

  function handleItemClick(cmd: Command): void {
    invokeCommand(cmd);
  }

  function handleBackdropClick(): void {
    closePalette();
  }

  // Reset selection when query changes
  $: if (query !== undefined) {
    selectedIndex = 0;
  }

  // Scroll the selected item into view
  function scrollSelectedIntoView(): void {
    tick().then(() => {
      const el = document.querySelector(".palette-item.selected");
      if (el) {
        el.scrollIntoView({ block: "nearest" });
      }
    });
  }

  // ── Focus Management ─────────────────────────────────────
  onMount(async () => {
    await tick();
    inputEl?.focus();
  });

  // Compute a running flat index to correlate grouped items
  // with the selectedIndex.
  function flatIndex(groups: CommandGroup[], groupIdx: number, itemIdx: number): number {
    let idx = 0;
    for (let g = 0; g < groupIdx; g++) {
      idx += groups[g].items.length;
    }
    return idx + itemIdx;
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="palette-backdrop" on:click={handleBackdropClick}>
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="palette-container" on:click|stopPropagation={() => {}}>
    <!-- Search Input -->
    <div class="palette-input-row">
      <span class="palette-prompt">&gt;</span>
      <input
        bind:this={inputEl}
        bind:value={query}
        on:keydown={handleKeydown}
        class="palette-input"
        type="text"
        placeholder="Type a command..."
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <!-- Results -->
    <div class="palette-results">
      {#if flatItems.length === 0}
        <div class="palette-empty">No matching commands</div>
      {:else}
        {#each groups as group, gi}
          <div class="palette-group">
            <div class="palette-category">{group.category}</div>
            {#each group.items as cmd, ii}
              <!-- svelte-ignore a11y-click-events-have-key-events -->
              <!-- svelte-ignore a11y-no-static-element-interactions -->
              <div
                class="palette-item"
                class:selected={flatIndex(groups, gi, ii) === selectedIndex}
                on:click={() => handleItemClick(cmd)}
                on:mouseenter={() => {
                  selectedIndex = flatIndex(groups, gi, ii);
                }}
              >
                <span class="palette-item-title">{cmd.title}</span>
                {#if cmd.shortcut}
                  <span class="palette-item-shortcut">{cmd.shortcut}</span>
                {/if}
              </div>
            {/each}
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  /* ── Backdrop ─────────────────────────────────────────────── */
  .palette-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    justify-content: center;
    padding-top: 12vh;
  }

  /* ── Container ────────────────────────────────────────────── */
  .palette-container {
    width: min(560px, 90vw);
    max-height: 60vh;
    background: var(--bg-primary);
    border: 1px solid var(--border-focus);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
    align-self: flex-start;
  }

  /* ── Input Row ────────────────────────────────────────────── */
  .palette-input-row {
    display: flex;
    align-items: center;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid var(--border);
    background: var(--bg-surface);
    gap: 0.5rem;
    flex-shrink: 0;
  }
  .palette-prompt {
    color: var(--hl-symbol);
    font-size: 1rem;
    font-weight: 700;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }
  .palette-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--fg-accent);
    font-family: var(--font-mono);
    font-size: 0.88rem;
    caret-color: var(--hl-symbol);
  }
  .palette-input::placeholder {
    color: var(--fg-secondary);
  }

  /* ── Results ──────────────────────────────────────────────── */
  .palette-results {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .palette-empty {
    padding: 1.5rem 1rem;
    text-align: center;
    color: var(--fg-secondary);
    font-size: 0.78rem;
    font-style: italic;
  }

  /* ── Category Headers ─────────────────────────────────────── */
  .palette-group {
    border-bottom: 1px solid var(--border);
  }
  .palette-group:last-child {
    border-bottom: none;
  }
  .palette-category {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-secondary);
    padding: 0.4rem 0.8rem 0.15rem;
    font-family: var(--font-mono);
    background: var(--bg-inset);
    user-select: none;
  }

  /* ── Command Items ────────────────────────────────────────── */
  .palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.4rem 0.8rem 0.4rem 1.4rem;
    cursor: pointer;
    user-select: none;
    transition: background 0.05s;
  }
  .palette-item:hover,
  .palette-item.selected {
    background: var(--bg-surface);
  }
  .palette-item.selected {
    border-left: 2px solid var(--hl-symbol);
    padding-left: calc(1.4rem - 2px);
  }
  .palette-item-title {
    font-size: 0.8rem;
    color: var(--fg-primary);
    font-family: var(--font-mono);
  }
  .palette-item.selected .palette-item-title {
    color: var(--fg-accent);
  }
  .palette-item-shortcut {
    font-size: 0.62rem;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    padding: 0.1rem 0.35rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    letter-spacing: 0.02em;
    white-space: nowrap;
  }
</style>
