<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { ColorRepresentation, Field, InteractionType } from "$lib/types/spire";
  import { upsertModelField, appendLog } from "$lib/stores/physicsStore";

  const logAtlas = (message: string): void => {
    appendLog(message, { category: "Atlas" });
  };

  export let open = false;

  const dispatch = createEventDispatcher<{ close: void; created: Field }>();

  let id = "custom_x";
  let name = "Custom X";
  let symbol = "X";
  let mass = 1;
  let width = 0;
  let charge = 0;
  let spin = 0;
  let color: ColorRepresentation = "Singlet";
  let interactionCsv = "";
  let error = "";

  function close(): void {
    dispatch("close");
  }

  function handleBackdropKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" || event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      close();
    }
  }

  function parseInteractions(raw: string): InteractionType[] {
    const allowed: InteractionType[] = [
      "Electromagnetic",
      "WeakCC",
      "WeakNC",
      "Strong",
      "Yukawa",
      "ScalarSelf",
    ];
    const set = new Set<InteractionType>();
    for (const token of raw.split(",").map((x) => x.trim()).filter(Boolean)) {
      if ((allowed as string[]).includes(token)) {
        set.add(token as InteractionType);
      }
    }
    return [...set];
  }

  function normalizeId(raw: string): string {
    const trimmed = raw.trim().toLowerCase().replace(/\s+/g, "_");
    return trimmed.startsWith("custom_") ? trimmed : `custom_${trimmed}`;
  }

  function createParticle(): void {
    error = "";
    if (!id.trim() || !name.trim() || !symbol.trim()) {
      error = "ID, name, and symbol are required.";
      return;
    }
    if (!Number.isFinite(mass) || mass < 0) {
      error = "Mass must be a non-negative number.";
      return;
    }
    if (!Number.isFinite(width) || width < 0) {
      error = "Width must be a non-negative number.";
      return;
    }

    const field: Field = {
      id: normalizeId(id),
      name: name.trim(),
      symbol: symbol.trim(),
      mass,
      width,
      quantum_numbers: {
        electric_charge: charge,
        weak_isospin: 0,
        hypercharge: 0,
        baryon_number: 0,
        lepton_numbers: {
          electron: 0,
          muon: 0,
          tau: 0,
        },
        spin,
        parity: "Even",
        charge_conjugation: "Undefined",
        color,
        weak_multiplet: "Singlet",
      },
      interactions: parseInteractions(interactionCsv),
    };

    upsertModelField(field);
    logAtlas(`Custom particle upserted: ${field.id} (${field.symbol})`);
    dispatch("created", field);
    close();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" role="button" tabindex="0" aria-label="Close custom particle modal" on:click={close} on:keydown={handleBackdropKeydown}>
    <div class="modal" role="dialog" tabindex="-1" aria-label="Create custom particle" on:click|stopPropagation>
      <h3>Create Custom Particle</h3>

      <div class="grid">
        <label>ID <input bind:value={id} /></label>
        <label>Name <input bind:value={name} /></label>
        <label>Symbol <input bind:value={symbol} /></label>
        <label>Mass (GeV) <input type="number" bind:value={mass} min="0" step="0.01" /></label>
        <label>Width (GeV) <input type="number" bind:value={width} min="0" step="0.01" /></label>
        <label>Electric Charge <input type="number" bind:value={charge} step="0.333333" /></label>
        <label>2s (spin encoding) <input type="number" bind:value={spin} min="0" step="1" /></label>
        <label>
          Color
          <select bind:value={color}>
            <option value="Singlet">Singlet</option>
            <option value="Triplet">Triplet</option>
            <option value="AntiTriplet">AntiTriplet</option>
            <option value="Octet">Octet</option>
          </select>
        </label>
      </div>

      <label class="full">
        Interactions (comma-separated)
        <input bind:value={interactionCsv} placeholder="Electromagnetic, WeakNC" />
      </label>

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="actions">
        <button on:click={close}>Cancel</button>
        <button class="primary" on:click={createParticle}>Create</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 15000;
  }

  .modal {
    width: min(780px, 90vw);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-primary);
    padding: 0.75rem;
    font-family: var(--font-mono);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.45rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
    font-size: 0.7rem;
    color: var(--fg-secondary);
  }

  input, select {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.28rem;
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }

  .full {
    margin-top: 0.45rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.35rem;
    margin-top: 0.6rem;
  }

  button {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.28rem 0.5rem;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  button.primary {
    border-color: var(--hl-symbol);
    color: var(--hl-symbol);
  }

  .error {
    margin: 0.35rem 0 0;
    color: var(--hl-error);
    font-size: 0.72rem;
  }
</style>
