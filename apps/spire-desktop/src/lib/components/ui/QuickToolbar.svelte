<!--
  SPIRE — Quick Toolbar

  A compact floating row of buttons displaying pinned commands from
  the CommandRegistry.  Each button shows the command's icon (or a
  truncated title) and its shortcut hint.  Clicking a button invokes
  the command immediately.

  This component is entirely data-driven: it simply maps over the
  `pinnedCommands` derived store.  It knows nothing about the nature
  of the commands it displays.
-->
<script lang="ts">
  import {
    pinnedCommands,
    openPalette,
  } from "$lib/core/services/CommandRegistry";
  import type { Command } from "$lib/core/services/CommandRegistry";

  function invoke(cmd: Command): void {
    cmd.execute();
  }
</script>

{#if $pinnedCommands.length > 0}
  <div class="quick-toolbar">
    {#each $pinnedCommands as cmd (cmd.id)}
      <button
        class="qt-button"
        on:click={() => invoke(cmd)}
        title="{cmd.title}{cmd.shortcut ? ` (${cmd.shortcut})` : ''}"
      >
        {#if cmd.icon}
          <span class="qt-icon">{cmd.icon}</span>
        {/if}
        <span class="qt-label">{cmd.title}</span>
      </button>
    {/each}

    <span class="qt-divider"></span>

    <button
      class="qt-button qt-palette-btn"
      on:click={() => openPalette()}
      title="Command Palette (Ctrl+K)"
    >
      <span class="qt-label">Ctrl+K</span>
    </button>
  </div>
{/if}

<style>
  .quick-toolbar {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.4rem;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    flex-shrink: 1;
    min-width: 0;
    overflow-x: auto;
    flex-wrap: wrap;
    max-height: 3.2rem;
    scrollbar-width: none;
    -ms-overflow-style: none;
  }
  .quick-toolbar::-webkit-scrollbar { display: none; }

  .qt-button {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.5rem;
    border: 1px solid transparent;
    background: none;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 0.68rem;
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.1s, color 0.1s;
  }
  .qt-button:hover {
    border-color: var(--border-focus);
    color: var(--fg-accent);
  }

  .qt-icon {
    font-size: 0.78rem;
  }

  .qt-label {
    letter-spacing: 0.02em;
  }

  .qt-divider {
    width: 1px;
    height: 1rem;
    background: var(--border);
    flex-shrink: 0;
  }

  .qt-palette-btn {
    color: var(--hl-symbol);
    border: 1px solid var(--border);
    background: var(--bg-inset);
    font-size: 0.62rem;
  }
  .qt-palette-btn:hover {
    border-color: var(--hl-symbol);
  }

  /* ── Responsive: collapse labels, keep icons ─────────────── */
  @media (max-width: 768px) {
    .quick-toolbar {
      flex-wrap: nowrap;
      overflow-x: auto;
      max-height: none;
    }
    .qt-label { display: none; }
    .qt-button { padding: 0.2rem 0.3rem; }
    .qt-palette-btn .qt-label { display: inline; }
  }
</style>
