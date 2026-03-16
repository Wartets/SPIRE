<!--
  GpuToggle.svelte - Hardware Backend Toggle

  Queries the backend for available compute hardware and renders a
  toggle switch between CPU and GPU integration.  When GPU acceleration
  is not compiled-in or no adapter is available, the toggle is disabled
  with a descriptive tooltip.

  ## Usage

  ```svelte
  <script>
    import GpuToggle from "./GpuToggle.svelte";
    let backend: "cpu" | "gpu" = "cpu";
  </script>

  <GpuToggle bind:selected={backend} />
  ```
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { queryHardwareBackends, type HardwareReport } from "$lib/api";

  /** The currently selected backend ("cpu" or "gpu"). Bindable. */
  export let selected: "cpu" | "gpu" = "cpu";

  let report: HardwareReport | null = null;
  let loading = true;
  let error = "";

  onMount(async () => {
    try {
      report = await queryHardwareBackends();

      // If GPU is available, default to it; otherwise stay on CPU.
      if (report.gpu_adapter_available) {
        // Don't override an explicit user selection.
        if (selected === "cpu") {
          selected = "cpu"; // keep default
        }
      } else {
        selected = "cpu";
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      selected = "cpu";
    } finally {
      loading = false;
    }
  });

  function toggle() {
    if (!report?.gpu_adapter_available) return;
    selected = selected === "cpu" ? "gpu" : "cpu";
  }

  $: gpuAvailable = report?.gpu_adapter_available ?? false;
  $: tooltipText = loading
    ? "Checking hardware…"
    : !report?.gpu_feature_compiled
      ? "GPU support not compiled (build with --features gpu)"
      : !report?.gpu_adapter_available
        ? "No compatible GPU adapter found"
        : report?.adapter_name
          ? `GPU: ${report.adapter_name}`
          : "GPU available";
</script>

<div class="gpu-toggle" use:tooltip={{ text: tooltipText }}>
  <span class="label">Backend:</span>

  <button
    class="toggle-btn"
    class:active={selected === "cpu"}
    on:click={() => (selected = "cpu")}
    disabled={loading}
  >
    <svg class="icon" viewBox="0 0 16 16" width="14" height="14">
      <rect x="3" y="3" width="10" height="10" rx="1" fill="none" stroke="currentColor" stroke-width="1.2" />
      <line x1="6" y1="1" x2="6" y2="3" stroke="currentColor" stroke-width="1.2" />
      <line x1="10" y1="1" x2="10" y2="3" stroke="currentColor" stroke-width="1.2" />
      <line x1="6" y1="13" x2="6" y2="15" stroke="currentColor" stroke-width="1.2" />
      <line x1="10" y1="13" x2="10" y2="15" stroke="currentColor" stroke-width="1.2" />
    </svg>
    CPU
  </button>

  <button
    class="toggle-btn"
    class:active={selected === "gpu"}
    class:disabled={!gpuAvailable}
    on:click={toggle}
    disabled={loading || !gpuAvailable}
  >
    <svg class="icon" viewBox="0 0 16 16" width="14" height="14">
      <rect x="1" y="4" width="14" height="8" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.2" />
      <rect x="3" y="6" width="4" height="4" rx="0.5" fill="currentColor" opacity="0.5" />
      <rect x="9" y="6" width="4" height="4" rx="0.5" fill="currentColor" opacity="0.5" />
    </svg>
    GPU
    {#if !gpuAvailable && !loading}
      <span class="badge">N/A</span>
    {/if}
  </button>

  {#if error}
    <span class="error" use:tooltip={{ text: error }}>!</span>
  {/if}
</div>

<style>
  .gpu-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
    font-family: var(--font-mono, "JetBrains Mono", monospace);
  }

  .label {
    color: var(--text-muted, var(--color-text-muted));
    font-weight: 500;
    user-select: none;
  }

  .toggle-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border: 1px solid var(--border-color, #444);
    border-radius: 4px;
    background: var(--bg-secondary, #1e1e2e);
    color: var(--text-normal, #cdd6f4);
    cursor: pointer;
    transition: all 0.15s ease;
    font-size: 0.7rem;
    font-family: inherit;
    line-height: 1;
  }

  .toggle-btn:hover:not(:disabled) {
    border-color: var(--accent, #89b4fa);
  }

  .toggle-btn.active {
    background: var(--accent, #89b4fa);
    color: var(--bg-primary, #1e1e2e);
    border-color: var(--accent, #89b4fa);
    font-weight: 600;
  }

  .toggle-btn.disabled,
  .toggle-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .icon {
    flex-shrink: 0;
  }

  .badge {
    font-size: 0.55rem;
    padding: 0 3px;
    border-radius: 2px;
    background: var(--warning, #fab387);
    color: var(--bg-primary, #1e1e2e);
    font-weight: 700;
  }

  .error {
    color: var(--error, #f38ba8);
    cursor: help;
  }
</style>
