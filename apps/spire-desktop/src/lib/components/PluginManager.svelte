<!--
  SPIRE - Plugin Manager

  Widget for loading, inspecting, and unloading WASM extension plugins.
  Provides a file picker dialog (via Tauri's dialog API) for selecting
  `.wasm` files, displays a list of loaded plugins with their metadata
  and capabilities, and offers per-plugin unload controls.

  The plugin system runs in a sandboxed wasmtime runtime with fuel
  metering and no WASI access.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { loadPlugin, listActivePlugins, unloadPlugin } from "$lib/api";
  import type { PluginInfo } from "$lib/api";

  // ── State ──
  let plugins: PluginInfo[] = [];
  let loading = false;
  let error = "";
  let successMsg = "";

  /** Whether the Tauri dialog API is available. */
  let hasTauriDialog = false;

  onMount(async () => {
    // Check if Tauri dialog is available (only in desktop mode)
    try {
      const dialog = await import("@tauri-apps/api/dialog");
      hasTauriDialog = !!dialog.open;
    } catch {
      hasTauriDialog = false;
    }
    await refreshPlugins();
  });

  async function refreshPlugins(): Promise<void> {
    try {
      plugins = await listActivePlugins();
    } catch (e: any) {
      console.warn("[SPIRE] Failed to list plugins:", e);
    }
  }

  async function handleLoadPlugin(): Promise<void> {
    loading = true;
    error = "";
    successMsg = "";

    try {
      let filePath: string | null = null;

      if (hasTauriDialog) {
        const { open } = await import("@tauri-apps/api/dialog");
        const result = await open({
          title: "Select WASM Plugin",
          filters: [{ name: "WASM Modules", extensions: ["wasm"] }],
          multiple: false,
        });
        filePath = typeof result === "string" ? result : null;
      } else {
        // Fallback for non-Tauri environments: prompt for path
        filePath = window.prompt("Enter the path to the .wasm plugin file:");
      }

      if (!filePath) {
        loading = false;
        return;
      }

      const info = await loadPlugin(filePath);
      successMsg = `Loaded "${info.name}" v${info.version}`;
      await refreshPlugins();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  async function handleUnload(name: string): Promise<void> {
    loading = true;
    error = "";
    successMsg = "";

    try {
      await unloadPlugin(name);
      successMsg = `Unloaded "${name}"`;
      await refreshPlugins();
    } catch (e: any) {
      error = e?.message ?? String(e);
    } finally {
      loading = false;
    }
  }

  function capabilityLabel(cap: string): string {
    const map: Record<string, string> = {
      KinematicCut: "Kinematic Cut",
      CustomObservable: "Custom Observable",
      CustomMatrixElement: "Matrix Element",
      AnalysisPostProcess: "Post-Processor",
    };
    return map[cap] ?? cap;
  }
</script>

<div class="plugin-manager">
  <!-- Header / Controls -->
  <div class="pm-controls">
    <button
      class="pm-load-btn"
      on:click={handleLoadPlugin}
      disabled={loading}
    >
      {#if loading}
        Loading…
      {:else}
        ＋ Load Plugin (.wasm)
      {/if}
    </button>
    <button
      class="pm-refresh-btn"
      on:click={refreshPlugins}
      disabled={loading}
      use:tooltip={{ text: "Refresh plugin list" }}
    >↻</button>
  </div>

  <!-- Feedback messages -->
  {#if error}
    <div class="pm-msg pm-error">{error}</div>
  {/if}
  {#if successMsg}
    <div class="pm-msg pm-success">{successMsg}</div>
  {/if}

  <!-- Info bar -->
  <div class="pm-info">
    <span class="pm-count">{plugins.length} plugin{plugins.length !== 1 ? "s" : ""} loaded</span>
    <span class="pm-badge">WASM Sandbox</span>
  </div>

  <!-- Plugin list -->
  {#if plugins.length === 0}
    <div class="pm-empty">
      <div class="pm-empty-icon">🧩</div>
      <p>No plugins loaded</p>
      <p class="pm-empty-hint">
        Load a <code>.wasm</code> plugin to extend SPIRE with custom
        kinematic cuts, observables, or matrix elements.
      </p>
    </div>
  {:else}
    <div class="pm-list">
      {#each plugins as plugin (plugin.name)}
        <div class="pm-card">
          <div class="pm-card-header">
            <span class="pm-card-name">{plugin.name}</span>
            <span class="pm-card-version">v{plugin.version}</span>
            <button
              class="pm-unload-btn"
              on:click={() => handleUnload(plugin.name)}
              use:tooltip={{ text: "Unload plugin" }}
              disabled={loading}
            >✕</button>
          </div>
          <div class="pm-card-desc">{plugin.description}</div>
          <div class="pm-card-meta">
            <span class="pm-card-author">by {plugin.author}</span>
          </div>
          <div class="pm-card-caps">
            {#each plugin.capabilities as cap}
              <span class="pm-cap-tag">{capabilityLabel(cap)}</span>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .plugin-manager {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-family: var(--font-mono, monospace);
    color: var(--fg, var(--color-text-primary));
  }

  .pm-controls {
    display: flex;
    gap: 0.5rem;
  }

  .pm-load-btn {
    flex: 1;
    padding: 0.5rem 1rem;
    background: var(--accent, #4fc3f7);
    color: var(--bg, #1e1e1e);
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 600;
    font-size: 0.85rem;
    transition: opacity 0.15s;
  }
  .pm-load-btn:hover:not(:disabled) {
    opacity: 0.85;
  }
  .pm-load-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .pm-refresh-btn {
    padding: 0.5rem 0.7rem;
    background: var(--bg-elevated, #2a2a2a);
    color: var(--fg-muted, var(--color-text-muted));
    border: 1px solid var(--border, #333);
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
  }
  .pm-refresh-btn:hover:not(:disabled) {
    color: var(--fg, var(--color-text-primary));
    border-color: var(--fg-muted, var(--color-text-muted));
  }

  .pm-msg {
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .pm-error {
    background: rgba(244, 67, 54, 0.15);
    color: var(--hl-error, #f44336);
    border: 1px solid rgba(244, 67, 54, 0.3);
  }
  .pm-success {
    background: rgba(76, 175, 80, 0.15);
    color: var(--hl-success, var(--color-success));
    border: 1px solid rgba(76, 175, 80, 0.3);
  }

  .pm-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: var(--fg-muted, var(--color-text-muted));
    padding: 0.25rem 0;
    border-bottom: 1px solid var(--border, #333);
  }

  .pm-badge {
    padding: 0.15rem 0.4rem;
    background: rgba(79, 195, 247, 0.15);
    color: var(--accent, #4fc3f7);
    border-radius: 3px;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .pm-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    gap: 0.5rem;
    color: var(--fg-muted, var(--color-text-muted));
    text-align: center;
    padding: 2rem 1rem;
  }
  .pm-empty-icon {
    font-size: 2.5rem;
    opacity: 0.5;
  }
  .pm-empty-hint {
    font-size: 0.75rem;
    max-width: 260px;
    line-height: 1.5;
  }
  .pm-empty-hint code {
    background: var(--bg-elevated, #2a2a2a);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
    font-size: 0.72rem;
  }

  .pm-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow-y: auto;
    flex: 1;
  }

  .pm-card {
    background: var(--bg-elevated, #2a2a2a);
    border: 1px solid var(--border, #333);
    border-radius: 6px;
    padding: 0.6rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .pm-card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .pm-card-name {
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--fg, var(--color-text-primary));
  }

  .pm-card-version {
    font-size: 0.7rem;
    color: var(--fg-muted, var(--color-text-muted));
    background: var(--bg-surface, #1e1e1e);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }

  .pm-unload-btn {
    margin-left: auto;
    background: none;
    border: 1px solid var(--border, #333);
    color: var(--fg-muted, var(--color-text-muted));
    border-radius: 4px;
    padding: 0.15rem 0.4rem;
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.15s;
  }
  .pm-unload-btn:hover:not(:disabled) {
    color: var(--hl-error, #f44336);
    border-color: var(--hl-error, #f44336);
  }

  .pm-card-desc {
    font-size: 0.78rem;
    color: var(--fg-muted, var(--color-text-muted));
    line-height: 1.4;
  }

  .pm-card-meta {
    font-size: 0.7rem;
    color: var(--fg-dim, var(--color-text-muted));
  }

  .pm-card-caps {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
    margin-top: 0.15rem;
  }

  .pm-cap-tag {
    padding: 0.12rem 0.4rem;
    background: rgba(156, 39, 176, 0.15);
    color: #ce93d8;
    border-radius: 3px;
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
</style>
