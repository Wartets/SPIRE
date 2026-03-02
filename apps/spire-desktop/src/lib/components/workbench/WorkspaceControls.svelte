<!--
  SPIRE — Workspace Controls Component

  Compact inline controls for saving, loading, and resetting
  the workspace.  Renders as a horizontal row of buttons
  designed to sit inside the workbench toolbox bar.
-->
<script lang="ts">
  import {
    downloadWorkspace,
    importFromFile,
    resetWorkspace,
    hasAutoSave,
    autoRestore,
  } from "$lib/services/workspaceManager";
  import { appendLog } from "$lib/stores/physicsStore";

  // --- Save dialog ---
  let showSaveDialog = false;
  let workspaceName = "";

  function handleSave(): void {
    const name = workspaceName.trim() || "workspace";
    downloadWorkspace(name);
    showSaveDialog = false;
    workspaceName = "";
  }

  function cancelSave(): void {
    showSaveDialog = false;
    workspaceName = "";
  }

  // --- Load via hidden file input ---
  let fileInput: HTMLInputElement;

  function triggerFileLoad(): void {
    fileInput?.click();
  }

  async function handleFileSelected(event: Event): Promise<void> {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    await importFromFile(file);
    // Reset the input so the same file can be re-imported
    input.value = "";
  }

  // --- Reset confirmation ---
  let confirmReset = false;

  function handleReset(): void {
    if (!confirmReset) {
      confirmReset = true;
      setTimeout(() => { confirmReset = false; }, 3000);
      return;
    }
    resetWorkspace();
    confirmReset = false;
  }

  // --- Auto-restore prompt ---
  let showRestorePrompt = false;

  function checkAutoSave(): void {
    if (typeof window !== "undefined" && hasAutoSave()) {
      showRestorePrompt = true;
    }
  }

  function handleRestore(): void {
    autoRestore();
    showRestorePrompt = false;
  }

  function dismissRestore(): void {
    showRestorePrompt = false;
  }

  // Check on mount (called from parent via bind)
  export { checkAutoSave };
</script>

<!-- Hidden file input for workspace import -->
<input
  type="file"
  accept=".json,.spire.json"
  class="hidden-input"
  bind:this={fileInput}
  on:change={handleFileSelected}
/>

<div class="workspace-controls">
  <!-- Save -->
  {#if showSaveDialog}
    <div class="save-inline">
      <input
        type="text"
        class="name-input"
        bind:value={workspaceName}
        placeholder="Workspace name"
        on:keydown={(e) => e.key === "Enter" && handleSave()}
        on:keydown={(e) => e.key === "Escape" && cancelSave()}
      />
      <button class="ctrl-btn save" on:click={handleSave}>Export</button>
      <button class="ctrl-btn cancel" on:click={cancelSave}>&times;</button>
    </div>
  {:else}
    <button
      class="ctrl-btn"
      on:click={() => (showSaveDialog = true)}
      title="Export workspace to .spire.json"
    >
      Save
    </button>
  {/if}

  <!-- Load -->
  <button class="ctrl-btn" on:click={triggerFileLoad} title="Import workspace from .spire.json">
    Load
  </button>

  <!-- Reset -->
  <button
    class="ctrl-btn"
    class:danger={confirmReset}
    on:click={handleReset}
    title={confirmReset ? "Click again to confirm reset" : "Reset workspace to defaults"}
  >
    {confirmReset ? "Confirm?" : "Reset All"}
  </button>
</div>

<!-- Auto-restore banner -->
{#if showRestorePrompt}
  <div class="restore-banner">
    <span class="restore-text">Previous session found.</span>
    <button class="ctrl-btn restore" on:click={handleRestore}>Restore</button>
    <button class="ctrl-btn dismiss" on:click={dismissRestore}>Dismiss</button>
  </div>
{/if}

<style>
  .workspace-controls {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .hidden-input {
    display: none;
  }
  .ctrl-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    padding: 0.2rem 0.5rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .ctrl-btn:hover {
    color: var(--fg-primary);
    border-color: var(--border-focus);
  }
  .ctrl-btn.save {
    color: var(--hl-success);
    border-color: var(--hl-success);
  }
  .ctrl-btn.cancel {
    color: var(--fg-secondary);
    padding: 0.2rem 0.35rem;
  }
  .ctrl-btn.danger {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }
  .ctrl-btn.restore {
    color: var(--hl-symbol);
    border-color: var(--hl-symbol);
  }
  .ctrl-btn.dismiss {
    color: var(--fg-secondary);
  }
  .save-inline {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .name-input {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.2rem 0.4rem;
    font-size: 0.68rem;
    font-family: var(--font-mono);
    width: 10rem;
  }
  .name-input:focus {
    border-color: var(--border-focus);
    outline: none;
  }
  .restore-banner {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0.4rem;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    font-size: 0.72rem;
  }
  .restore-text {
    color: var(--fg-secondary);
  }
</style>
