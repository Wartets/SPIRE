<!--
  SPIRE - Keybind Customisation Panel

  Table-based UI for viewing, filtering, and reassigning keyboard
  shortcuts.  Includes:
    - Grouped display by category
    - Search/filter bar
    - Key capture modal for recording new bindings
    - Conflict detection and resolution (Swap / Unbind / Cancel)
    - Reset to defaults

  Accessible from the Command Palette via "Keyboard Shortcuts".
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import {
    activeKeybindings,
    formatBindingLabel,
    formatKeyLabel,
    normaliseKeyEvent,
    detectConflict,
    updateBinding,
    swapBindings,
    unbindCommand,
    resetBinding,
    resetAllBindings,
    DEFAULT_KEYBINDINGS,
    type Keybinding,
    type KeybindCategory,
    type ConflictInfo,
  } from "$lib/core/services/ShortcutService";
  import { getCommand } from "$lib/core/services/CommandRegistry";

  export let onClose: (() => void) | undefined = undefined;

  let searchQuery = "";

  // ── Display data ──
  interface DisplayRow {
    binding: Keybinding;
    title: string;
    keyLabel: string;
    isDefault: boolean;
  }

  function buildRows(bindings: Keybinding[]): DisplayRow[] {
    return bindings.map((kb) => {
      const cmd = getCommand(kb.commandId);
      const def = DEFAULT_KEYBINDINGS.find((d) => d.commandId === kb.commandId);
      const isDefault = def ? def.key === kb.key && def.chordKey === kb.chordKey : true;
      return {
        binding: kb,
        title: cmd?.title ?? kb.commandId,
        keyLabel: kb.key ? formatBindingLabel(kb) : "(unbound)",
        isDefault,
      };
    });
  }

  $: allRows = buildRows($activeKeybindings);

  $: filtered = searchQuery.trim()
    ? allRows.filter((r) => {
        const q = searchQuery.toLowerCase();
        return (
          r.title.toLowerCase().includes(q) ||
          r.keyLabel.toLowerCase().includes(q) ||
          r.binding.commandId.toLowerCase().includes(q) ||
          r.binding.category.toLowerCase().includes(q)
        );
      })
    : allRows;

  const CATEGORY_ORDER: KeybindCategory[] = [
    "File", "Edit", "View", "Canvas", "Notebook", "Navigation", "Help",
  ];

  $: grouped = CATEGORY_ORDER
    .map((cat) => ({
      category: cat,
      rows: filtered.filter((r) => r.binding.category === cat),
    }))
    .filter((g) => g.rows.length > 0);

  // ── Key Capture State ──
  let capturing: Keybinding | null = null;
  let capturedKey = "";
  let capturedChordKey = "";
  let captureStep: "first" | "second" | "done" = "first";
  let chordMode = false;

  function startCapture(kb: Keybinding): void {
    capturing = kb;
    capturedKey = "";
    capturedChordKey = "";
    captureStep = "first";
    chordMode = false;
    conflictInfo = null;
  }

  function cancelCapture(): void {
    capturing = null;
    conflictInfo = null;
  }

  function handleCaptureKeydown(event: KeyboardEvent): void {
    if (!capturing) return;
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      cancelCapture();
      return;
    }

    const normalised = normaliseKeyEvent(event);
    if (!normalised) return; // Lone modifier

    if (captureStep === "first") {
      capturedKey = normalised;
      // After capturing the first key, briefly wait — if user continues
      // pressing modifiers, we enter chord mode.  For simplicity, provide
      // a "Record chord" button and also auto-detect.
      captureStep = "done";
    } else if (captureStep === "second") {
      capturedChordKey = normalised;
      captureStep = "done";
    }
  }

  function enableChordCapture(): void {
    chordMode = true;
    captureStep = "second";
    capturedChordKey = "";
  }

  function confirmCapture(): void {
    if (!capturing || !capturedKey) return;
    const chord = chordMode && capturedChordKey ? capturedChordKey : undefined;

    // Check for conflicts
    const conflict = detectConflict(capturedKey, chord, capturing.commandId);
    if (conflict) {
      conflictInfo = conflict;
      return;
    }

    updateBinding(capturing.commandId, capturedKey, chord);
    capturing = null;
    conflictInfo = null;
  }

  // ── Conflict Resolution ──
  let conflictInfo: ConflictInfo | null = null;

  function resolveSwap(): void {
    if (!capturing || !conflictInfo) return;
    swapBindings(capturing.commandId, conflictInfo.existingCommandId);
    capturing = null;
    conflictInfo = null;
  }

  function resolveUnbind(): void {
    if (!capturing || !conflictInfo) return;
    unbindCommand(conflictInfo.existingCommandId);
    const chord = chordMode && capturedChordKey ? capturedChordKey : undefined;
    updateBinding(capturing.commandId, capturedKey, chord);
    capturing = null;
    conflictInfo = null;
  }

  function resolveCancel(): void {
    conflictInfo = null;
  }

  function getConflictTitle(commandId: string): string {
    const cmd = getCommand(commandId);
    return cmd?.title ?? commandId;
  }

  /** Auto-focus action for the capture modal so it receives keyboard events. */
  function autoFocus(node: HTMLElement): void {
    tick().then(() => node.focus());
  }
</script>

<div class="kp-container">
  <!-- Header -->
  <header class="kp-header">
    <h2 class="kp-title">Keyboard Shortcuts</h2>
    <input
      class="kp-search"
      type="text"
      placeholder="Filter shortcuts…"
      bind:value={searchQuery}
    />
    <button class="kp-reset-all" on:click={resetAllBindings} use:tooltip={{ text: "Reset all to defaults" }}>
      Reset All
    </button>
    {#if onClose}
      <button class="kp-close" on:click={onClose}>✕</button>
    {/if}
  </header>

  <!-- Table -->
  <div class="kp-body">
    {#each grouped as group (group.category)}
      <div class="kp-group">
        <h3 class="kp-group-title">{group.category}</h3>
        <table class="kp-table">
          <thead>
            <tr>
              <th class="kp-th-cmd">Command</th>
              <th class="kp-th-key">Shortcut</th>
              <th class="kp-th-actions"></th>
            </tr>
          </thead>
          <tbody>
            {#each group.rows as row (row.binding.commandId)}
              <tr class="kp-row" class:modified={!row.isDefault}>
                <td class="kp-cell-cmd">{row.title}</td>
                <td class="kp-cell-key">
                  <kbd class="kp-kbd">{row.keyLabel}</kbd>
                </td>
                <td class="kp-cell-actions">
                  <button
                    class="kp-btn"
                    on:click={() => startCapture(row.binding)}
                    use:tooltip={{ text: "Edit shortcut" }}
                  >✎</button>
                  {#if !row.isDefault}
                    <button
                      class="kp-btn kp-btn-reset"
                      on:click={() => resetBinding(row.binding.commandId)}
                      use:tooltip={{ text: "Reset to default" }}
                    >↺</button>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/each}

    {#if grouped.length === 0}
      <p class="kp-empty">No shortcuts match "{searchQuery}"</p>
    {/if}
  </div>
</div>

<!-- Key Capture Modal -->
{#if capturing}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="kp-modal-backdrop" on:mousedown={cancelCapture}>
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="kp-modal" tabindex="-1" use:autoFocus on:mousedown|stopPropagation on:keydown={handleCaptureKeydown}>
      <h3 class="kp-modal-title">
        Set shortcut for: <strong>{getConflictTitle(capturing.commandId)}</strong>
      </h3>

      {#if !conflictInfo}
        <div class="kp-modal-body">
          {#if captureStep === "first" || (captureStep === "done" && !chordMode)}
            <p class="kp-modal-hint">
              {#if capturedKey}
                Captured: <kbd class="kp-kbd-lg">{formatKeyLabel(capturedKey)}</kbd>
              {:else}
                Press the desired key combination…
              {/if}
            </p>
          {/if}

          {#if captureStep === "second"}
            <p class="kp-modal-hint">
              First key: <kbd class="kp-kbd-lg">{formatKeyLabel(capturedKey)}</kbd>
              <br />
              {#if capturedChordKey}
                Second key: <kbd class="kp-kbd-lg">{formatKeyLabel(capturedChordKey)}</kbd>
              {:else}
                Now press the second key…
              {/if}
            </p>
          {/if}

          <div class="kp-modal-actions">
            {#if capturedKey && captureStep === "done" && !chordMode}
              <button class="kp-btn kp-btn-chord" on:click={enableChordCapture}>
                + Add Chord (2nd key)
              </button>
            {/if}
            <button
              class="kp-btn kp-btn-confirm"
              disabled={!capturedKey}
              on:click={confirmCapture}
            >
              Confirm
            </button>
            <button class="kp-btn" on:click={cancelCapture}>Cancel</button>
          </div>
        </div>
      {:else}
        <!-- Conflict resolution -->
        <div class="kp-modal-body">
          <p class="kp-conflict-msg">
            <kbd class="kp-kbd-lg">
              {formatKeyLabel(capturedKey)}{capturedChordKey ? ` → ${formatKeyLabel(capturedChordKey)}` : ""}
            </kbd>
            is already bound to
            <strong>{getConflictTitle(conflictInfo.existingCommandId)}</strong>.
          </p>
          <div class="kp-modal-actions">
            <button class="kp-btn kp-btn-confirm" on:click={resolveSwap}>
              Swap Bindings
            </button>
            <button class="kp-btn kp-btn-warn" on:click={resolveUnbind}>
              Unbind Existing
            </button>
            <button class="kp-btn" on:click={resolveCancel}>Cancel</button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .kp-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-base);
    color: var(--color-text-primary);
    font-family: var(--font-mono);
  }

  /* ── Header ── */
  .kp-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    border-bottom: 1px solid var(--border, #333);
    background: var(--color-bg-surface);
    flex-shrink: 0;
  }
  .kp-title {
    margin: 0;
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-accent, var(--color-text-primary));
    white-space: nowrap;
  }
  .kp-search {
    flex: 1;
    background: var(--color-bg-inset);
    border: 1px solid var(--border, #333);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.68rem;
    padding: 0.2rem 0.4rem;
    outline: none;
  }
  .kp-search:focus { border-color: var(--border-focus, var(--color-text-muted)); }
  .kp-reset-all, .kp-close {
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    font-family: var(--font-mono);
    font-size: 0.6rem;
    cursor: pointer;
    padding: 0.15rem 0.4rem;
  }
  .kp-reset-all:hover, .kp-close:hover {
    color: var(--fg-primary);
    border-color: var(--fg-secondary, var(--color-text-muted));
  }

  /* ── Body ── */
  .kp-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.4rem 0.6rem;
  }
  .kp-group { margin-bottom: 0.6rem; }
  .kp-group-title {
    margin: 0 0 0.25rem 0;
    font-size: 0.58rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--fg-secondary, var(--color-text-muted));
  }
  .kp-table {
    width: 100%;
    border-collapse: collapse;
  }
  .kp-table th {
    text-align: left;
    font-size: 0.52rem;
    font-weight: 500;
    color: var(--fg-secondary, var(--color-text-muted));
    padding: 0.15rem 0.3rem;
    border-bottom: 1px solid var(--border, #333);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .kp-th-cmd { width: 50%; }
  .kp-th-key { width: 35%; }
  .kp-th-actions { width: 15%; }

  .kp-row { transition: background 0.06s; }
  .kp-row:hover { background: var(--color-bg-surface); }
  .kp-row.modified .kp-kbd {
    border-color: var(--hl-value, #d4a017);
    color: var(--hl-value, #d4a017);
  }

  .kp-cell-cmd, .kp-cell-key, .kp-cell-actions {
    padding: 0.22rem 0.3rem;
    font-size: 0.68rem;
    border-bottom: 1px solid rgba(51, 51, 51, 0.4);
  }
  .kp-cell-cmd { color: var(--color-text-primary); }
  .kp-cell-actions {
    display: flex;
    gap: 0.2rem;
    justify-content: flex-end;
  }

  .kp-kbd {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    color: var(--color-accent);
    background: var(--color-bg-inset);
    border: 1px solid var(--border, #333);
    padding: 0.08rem 0.3rem;
    white-space: nowrap;
  }

  .kp-btn {
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    font-family: var(--font-mono);
    font-size: 0.58rem;
    cursor: pointer;
    padding: 0.1rem 0.3rem;
  }
  .kp-btn:hover {
    color: var(--fg-primary);
    border-color: var(--fg-secondary);
  }
  .kp-btn-reset { color: var(--hl-value, #d4a017); }
  .kp-btn-confirm {
    color: var(--color-success);
    border-color: var(--color-success);
  }
  .kp-btn-warn {
    color: var(--color-error);
    border-color: var(--color-error);
  }
  .kp-btn-chord { color: var(--color-accent); }
  .kp-btn:disabled { opacity: 0.35; cursor: default; }

  .kp-empty {
    text-align: center;
    color: var(--fg-secondary, var(--color-text-muted));
    font-style: italic;
    font-size: 0.72rem;
    padding: 1.5rem 0;
  }

  /* ── Capture Modal ── */
  .kp-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 13000;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .kp-modal {
    background: var(--color-bg-base);
    border: 1px solid var(--border, #333);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.7);
    padding: 0.8rem 1rem;
    min-width: 340px;
    max-width: 480px;
  }
  .kp-modal-title {
    margin: 0 0 0.5rem 0;
    font-size: 0.72rem;
    color: var(--fg-primary);
    font-weight: 500;
  }
  .kp-modal-body { display: flex; flex-direction: column; gap: 0.5rem; }
  .kp-modal-hint {
    font-size: 0.68rem;
    color: var(--fg-secondary, var(--color-text-muted));
    margin: 0;
  }
  .kp-modal-actions {
    display: flex;
    gap: 0.35rem;
    justify-content: flex-end;
    margin-top: 0.3rem;
  }
  .kp-kbd-lg {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-accent);
    background: var(--color-bg-inset);
    border: 1px solid var(--border, #333);
    padding: 0.1rem 0.4rem;
  }
  .kp-conflict-msg {
    font-size: 0.68rem;
    color: var(--color-error);
    margin: 0;
    line-height: 1.6;
  }
</style>
