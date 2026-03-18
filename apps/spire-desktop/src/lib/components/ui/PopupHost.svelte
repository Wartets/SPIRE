<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import {
    popupState,
    closePopup,
    resolvePopup,
    setPopupInputValue,
  } from "$lib/stores/popupStore";

  let panelEl: HTMLDivElement | null = null;
  let stopKey: (() => void) | null = null;

  function actionClass(variant: string | undefined): string {
    if (variant === "primary") return "popup-btn popup-btn-primary";
    if (variant === "danger") return "popup-btn popup-btn-danger";
    if (variant === "ghost") return "popup-btn popup-btn-ghost";
    return "popup-btn";
  }

  function toneClass(tone: string): string {
    if (tone === "info") return "popup-panel popup-info";
    if (tone === "warn") return "popup-panel popup-warn";
    if (tone === "danger") return "popup-panel popup-danger";
    if (tone === "success") return "popup-panel popup-success";
    return "popup-panel";
  }

  async function focusDefaultAction(): Promise<void> {
    await tick();
    if (!panelEl) return;
    const preferred = panelEl.querySelector<HTMLButtonElement>("button[data-autofocus='true']");
    if (preferred) {
      preferred.focus();
      return;
    }
    const fallback = panelEl.querySelector<HTMLButtonElement>(".popup-actions button");
    fallback?.focus();
  }

  $: if ($popupState.visible) {
    focusDefaultAction();
  }

  function handleWindowKey(event: KeyboardEvent): void {
    if (!$popupState.visible) return;
    if (event.key === "Escape") {
      event.preventDefault();
      closePopup();
      return;
    }

    if (event.key === "Enter") {
      const autofocusAction = $popupState.actions.find((a) => a.autofocus);
      if (autofocusAction) {
        event.preventDefault();
        resolvePopup(autofocusAction.id);
      }
    }
  }

  $: {
    if ($popupState.visible && !stopKey) {
      window.addEventListener("keydown", handleWindowKey, true);
      stopKey = () => window.removeEventListener("keydown", handleWindowKey, true);
    }
    if (!$popupState.visible && stopKey) {
      stopKey();
      stopKey = null;
    }
  }

  onDestroy(() => {
    stopKey?.();
  });
</script>

{#if $popupState.visible}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="popup-overlay" role="button" tabindex="-1" aria-label="Close popup" on:mousedown={closePopup}>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      bind:this={panelEl}
      class={toneClass($popupState.tone)}
      style="max-width: min({$popupState.maxWidth}px, calc(100vw - 1rem));"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-label={$popupState.title}
      on:mousedown|stopPropagation
    >
      <div class="popup-head">
        <h3>{$popupState.title}</h3>
      </div>

      {#if $popupState.message}
        <p class="popup-message">{$popupState.message}</p>
      {/if}

      {#if $popupState.input}
        <label class="popup-input-wrap">
          <span class="popup-input-label">{$popupState.input.label}</span>
          {#if $popupState.input.multiline}
            <textarea
              class="popup-input"
              rows={$popupState.input.rows ?? 5}
              placeholder={$popupState.input.placeholder ?? ""}
              value={$popupState.inputValue}
              on:input={(event) => setPopupInputValue((event.currentTarget as HTMLTextAreaElement).value)}
            ></textarea>
          {:else}
            <input
              class="popup-input"
              type="text"
              placeholder={$popupState.input.placeholder ?? ""}
              value={$popupState.inputValue}
              on:input={(event) => setPopupInputValue((event.currentTarget as HTMLInputElement).value)}
            />
          {/if}
        </label>
      {/if}

      {#if $popupState.meta.length > 0}
        <div class="popup-meta">
          {#each $popupState.meta as row}
            <div class="meta-row">
              <span class="meta-label">{row.label}</span>
              <span class="meta-value">{row.value}</span>
            </div>
          {/each}
        </div>
      {/if}

      {#if $popupState.details.length > 0}
        <ul class="popup-details">
          {#each $popupState.details as detail}
            <li>{detail}</li>
          {/each}
        </ul>
      {/if}

      <div class="popup-actions">
        {#each $popupState.actions as action}
          <button
            class={actionClass(action.variant)}
            data-autofocus={action.autofocus ? "true" : "false"}
            on:click={() => resolvePopup(action.id)}
          >
            {action.label}
          </button>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .popup-overlay {
    position: fixed;
    inset: 0;
    z-index: 13000;
    background: rgba(0, 0, 0, 0.52);
    display: grid;
    place-items: center;
    padding: 0.5rem;
  }

  .popup-panel {
    width: 100%;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    box-shadow: 0 16px 42px rgba(0, 0, 0, 0.45);
    padding: 0.55rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: calc(100vh - 1rem);
    overflow-y: auto;
  }

  .popup-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.35rem;
  }

  .popup-head h3 {
    margin: 0;
    font-size: 0.85rem;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-accent);
  }

  .popup-message {
    margin: 0;
    font-size: 0.76rem;
    color: var(--fg-primary);
    line-height: 1.45;
  }

  .popup-meta {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: 0.35rem 0.45rem;
    display: grid;
    gap: 0.2rem;
  }

  .popup-input-wrap {
    display: grid;
    gap: 0.18rem;
  }

  .popup-input-label {
    color: var(--fg-secondary);
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-family: var(--font-mono);
  }

  .popup-input {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    padding: 0.3rem 0.38rem;
    resize: vertical;
    min-height: 2rem;
  }

  .popup-input:focus {
    outline: none;
    border-color: var(--border-focus, var(--hl-symbol));
  }

  .meta-row {
    display: grid;
    grid-template-columns: 7.5rem minmax(0, 1fr);
    gap: 0.45rem;
    align-items: baseline;
  }

  .meta-label {
    color: var(--fg-secondary);
    font-size: 0.66rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .meta-value {
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.72rem;
    overflow-wrap: anywhere;
  }

  .popup-details {
    margin: 0;
    padding-left: 1rem;
    color: var(--fg-secondary);
    font-size: 0.7rem;
    line-height: 1.5;
    display: grid;
    gap: 0.15rem;
  }

  .popup-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.35rem;
    border-top: 1px solid var(--border);
    padding-top: 0.45rem;
    flex-wrap: wrap;
  }

  .popup-btn {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0.2rem 0.55rem;
    cursor: pointer;
  }

  .popup-btn:hover {
    border-color: var(--border-focus);
    color: var(--fg-primary);
  }

  .popup-btn-primary {
    border-color: color-mix(in srgb, var(--color-accent) 70%, var(--border));
    color: var(--fg-accent);
  }

  .popup-btn-danger {
    border-color: color-mix(in srgb, var(--color-error) 75%, var(--border));
    color: var(--hl-error);
  }

  .popup-btn-ghost {
    opacity: 0.85;
  }

  .popup-info .popup-head h3 { color: var(--fg-accent); }
  .popup-warn .popup-head h3 { color: var(--hl-value); }
  .popup-danger .popup-head h3 { color: var(--hl-error); }
  .popup-success .popup-head h3 { color: var(--hl-success); }
</style>
