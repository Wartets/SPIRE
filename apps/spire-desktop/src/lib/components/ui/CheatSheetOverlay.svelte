<!--
  SPIRE - Keyboard Shortcut Cheat Sheet Overlay

  Full-screen, typographically clean overlay showing all active
  keyboard shortcuts grouped by category.  Triggered by Ctrl+/ (or
  Cmd+/ on Mac).  Includes a search bar that filters by command
  name or key combination.

  Context-aware highlighting: the category matching the current
  active context (e.g. "Canvas" when on the infinite canvas) is
  visually emphasised.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    activeKeybindings,
    cheatSheetOpen,
    formatBindingLabel,
    type Keybinding,
    type KeybindCategory,
  } from "$lib/core/services/ShortcutService";
  import { getCommand } from "$lib/core/services/CommandRegistry";
  import { viewMode } from "$lib/stores/layoutStore";

  let searchQuery = "";
  let inputEl: HTMLInputElement | undefined;

  // Focus the search input when the overlay opens
  onMount(() => {
    requestAnimationFrame(() => inputEl?.focus());
  });

  // ── Resolve command titles ──
  interface DisplayBinding {
    commandId: string;
    title: string;
    keyLabel: string;
    category: KeybindCategory;
  }

  function buildDisplay(bindings: Keybinding[]): DisplayBinding[] {
    return bindings
      .filter((kb) => kb.key) // skip unbound
      .map((kb) => {
        const cmd = getCommand(kb.commandId);
        return {
          commandId: kb.commandId,
          title: cmd?.title ?? kb.commandId.split(".").pop() ?? kb.commandId,
          keyLabel: formatBindingLabel(kb),
          category: kb.category,
        };
      });
  }

  $: allDisplay = buildDisplay($activeKeybindings);

  // ── Filter ──
  $: filtered = searchQuery.trim()
    ? allDisplay.filter((d) => {
        const q = searchQuery.toLowerCase();
        return (
          d.title.toLowerCase().includes(q) ||
          d.keyLabel.toLowerCase().includes(q) ||
          d.commandId.toLowerCase().includes(q)
        );
      })
    : allDisplay;

  // ── Group by category ──
  const CATEGORY_ORDER: KeybindCategory[] = [
    "File",
    "Edit",
    "View",
    "Canvas",
    "Notebook",
    "Navigation",
    "Help",
  ];

  $: grouped = CATEGORY_ORDER
    .map((cat) => ({
      category: cat,
      items: filtered.filter((d) => d.category === cat),
    }))
    .filter((g) => g.items.length > 0);

  // ── Context-aware highlighting ──
  $: activeContext = $viewMode === "canvas" ? "Canvas" : "View";

  function close(): void {
    cheatSheetOpen.set(false);
  }

  function handleBackdrop(event: MouseEvent): void {
    if ((event.target as HTMLElement).classList.contains("cs-overlay")) {
      close();
    }
  }

  function handleKey(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
    }
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="cs-overlay" on:mousedown={handleBackdrop} on:keydown={handleKey}>
  <div class="cs-panel" role="dialog" aria-label="Keyboard Shortcuts">
    <!-- Header -->
    <header class="cs-header">
      <h2 class="cs-title">Keyboard Shortcuts</h2>
      <input
        class="cs-search"
        type="text"
        placeholder="Search shortcuts…"
        bind:value={searchQuery}
        bind:this={inputEl}
      />
      <button class="cs-close" on:click={close} aria-label="Close">✕</button>
    </header>

    <!-- Grid -->
    <div class="cs-grid">
      {#each grouped as group (group.category)}
        <section
          class="cs-category"
          class:highlighted={group.category === activeContext}
        >
          <h3 class="cs-cat-title">{group.category}</h3>
          <div class="cs-list">
            {#each group.items as item (item.commandId)}
              <div class="cs-row">
                <span class="cs-cmd">{item.title}</span>
                <kbd class="cs-kbd">{item.keyLabel}</kbd>
              </div>
            {/each}
          </div>
        </section>
      {/each}

      {#if grouped.length === 0}
        <p class="cs-empty">No shortcuts match "{searchQuery}"</p>
      {/if}
    </div>

    <!-- Footer -->
    <footer class="cs-footer">
      <span class="cs-footer-hint">Press <kbd>Escape</kbd> to close</span>
      <span class="cs-footer-hint">Customise in Command Palette → "Keyboard Shortcuts"</span>
    </footer>
  </div>
</div>

<style>
  .cs-overlay {
    position: fixed;
    inset: 0;
    z-index: 12000;
    background: rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    backdrop-filter: blur(3px);
  }

  .cs-panel {
    width: 90vw;
    max-width: 860px;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-base);
    border: 1px solid var(--border, #333);
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.8);
    overflow: hidden;
  }

  /* ── Header ── */
  .cs-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.8rem;
    border-bottom: 1px solid var(--border, #333);
    background: var(--color-bg-surface);
    flex-shrink: 0;
  }

  .cs-title {
    margin: 0;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-accent, var(--color-text-primary));
    white-space: nowrap;
  }

  .cs-search {
    flex: 1;
    background: var(--color-bg-inset);
    border: 1px solid var(--border, #333);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    padding: 0.25rem 0.5rem;
    outline: none;
  }
  .cs-search:focus {
    border-color: var(--border-focus, var(--color-text-muted));
  }

  .cs-close {
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    font-family: var(--font-mono);
    font-size: 0.72rem;
    cursor: pointer;
    padding: 0.15rem 0.4rem;
  }
  .cs-close:hover {
    color: var(--color-text-primary);
    border-color: var(--fg-secondary, var(--color-text-muted));
  }

  /* ── Grid ── */
  .cs-grid {
    flex: 1;
    overflow-y: auto;
    padding: 0.6rem 0.8rem;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 0.8rem;
    align-content: start;
  }

  .cs-category {
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border, #333);
    background: var(--color-bg-surface);
  }

  .cs-category.highlighted {
    border-color: var(--hl-value, #d4a017);
    background: rgba(212, 160, 23, 0.05);
  }

  .cs-cat-title {
    margin: 0 0 0.35rem 0;
    font-size: 0.62rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--fg-secondary, var(--color-text-muted));
  }

  .cs-category.highlighted .cs-cat-title {
    color: var(--hl-value, #d4a017);
  }

  .cs-list {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .cs-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 0.2rem 0;
  }

  .cs-cmd {
    font-size: 0.7rem;
    color: var(--color-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .cs-kbd {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    color: var(--color-accent);
    background: var(--color-bg-inset);
    border: 1px solid var(--border, #333);
    padding: 0.1rem 0.35rem;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .cs-empty {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--fg-secondary, var(--color-text-muted));
    font-style: italic;
    font-size: 0.75rem;
    padding: 2rem 0;
  }

  /* ── Footer ── */
  .cs-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.35rem 0.8rem;
    border-top: 1px solid var(--border, #333);
    background: var(--color-bg-surface);
    flex-shrink: 0;
  }

  .cs-footer-hint {
    font-size: 0.55rem;
    color: var(--fg-secondary, var(--color-text-muted));
  }

  .cs-footer-hint kbd {
    font-family: var(--font-mono);
    font-size: 0.52rem;
    padding: 0.05rem 0.25rem;
    border: 1px solid var(--border, #333);
    background: var(--color-bg-inset);
    color: var(--color-text-primary);
  }
</style>
