<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type {
    ChargeConjugation,
    ColorRepresentation,
    Field,
    InteractionType,
    Parity,
    WeakMultiplet,
  } from "$lib/types/spire";
  import { upsertModelField, appendLog } from "$lib/stores/physicsStore";

  const logAtlas = (message: string): void => {
    appendLog(message, { category: "Atlas" });
  };

  export let open = false;

  const dispatch = createEventDispatcher<{ close: void; created: Field }>();

  type TemplateId = "blank" | "electron" | "photon" | "up" | "neutral_scalar" | "z_prime";

  interface ParticleTemplate {
    id: TemplateId;
    label: string;
    apply: () => void;
  }

  const INTERACTION_OPTIONS: InteractionType[] = [
    "Electromagnetic",
    "WeakCC",
    "WeakNC",
    "Strong",
    "Yukawa",
    "ScalarSelf",
  ];

  let selectedTemplate: TemplateId = "blank";
  let id = "custom_x";
  let name = "Custom X";
  let symbol = "X";
  let mass = 1;
  let width = 0;
  let charge = 0;
  let spin = 0;
  let color: ColorRepresentation = "Singlet";
  let weakIsospin = 0;
  let hypercharge = 0;
  let baryonNumber = 0;
  let leptonElectron = 0;
  let leptonMuon = 0;
  let leptonTau = 0;
  let parity: Parity = "Even";
  let chargeConjugation: ChargeConjugation = "Undefined";
  let weakMultipletMode: "Singlet" | "DoubletUp" | "DoubletDown" | "Triplet" = "Singlet";
  let weakTripletIndex = 0;
  let interactions: InteractionType[] = [];
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

  function normalizeId(raw: string): string {
    const trimmed = raw.trim().toLowerCase().replace(/\s+/g, "_");
    return trimmed.startsWith("custom_") ? trimmed : `custom_${trimmed}`;
  }

  function hasInteraction(type: InteractionType): boolean {
    return interactions.includes(type);
  }

  function toggleInteraction(type: InteractionType): void {
    if (hasInteraction(type)) {
      interactions = interactions.filter((entry) => entry !== type);
    } else {
      interactions = [...interactions, type];
    }
  }

  function setTemplate(templateId: TemplateId): void {
    selectedTemplate = templateId;
    templates.find((template) => template.id === templateId)?.apply();
  }

  function weakMultipletValue(): WeakMultiplet {
    if (weakMultipletMode === "Triplet") {
      return { Triplet: weakTripletIndex };
    }
    return weakMultipletMode;
  }

  function applyBlankTemplate(): void {
    id = "custom_x";
    name = "Custom X";
    symbol = "X";
    mass = 1;
    width = 0;
    charge = 0;
    spin = 0;
    color = "Singlet";
    weakIsospin = 0;
    hypercharge = 0;
    baryonNumber = 0;
    leptonElectron = 0;
    leptonMuon = 0;
    leptonTau = 0;
    parity = "Even";
    chargeConjugation = "Undefined";
    weakMultipletMode = "Singlet";
    weakTripletIndex = 0;
    interactions = [];
  }

  function applyElectronTemplate(): void {
    id = "custom_e_minus";
    name = "Custom Electron-like";
    symbol = "e*";
    mass = 5.11e-4;
    width = 0;
    charge = -1;
    spin = 1;
    color = "Singlet";
    weakIsospin = -1;
    hypercharge = -1;
    baryonNumber = 0;
    leptonElectron = 1;
    leptonMuon = 0;
    leptonTau = 0;
    parity = "Odd";
    chargeConjugation = "Undefined";
    weakMultipletMode = "DoubletDown";
    weakTripletIndex = 0;
    interactions = ["Electromagnetic", "WeakNC", "WeakCC"];
  }

  function applyPhotonTemplate(): void {
    id = "custom_gamma_like";
    name = "Custom Photon-like";
    symbol = "γ*";
    mass = 0;
    width = 0;
    charge = 0;
    spin = 2;
    color = "Singlet";
    weakIsospin = 0;
    hypercharge = 0;
    baryonNumber = 0;
    leptonElectron = 0;
    leptonMuon = 0;
    leptonTau = 0;
    parity = "Odd";
    chargeConjugation = "Odd";
    weakMultipletMode = "Singlet";
    weakTripletIndex = 0;
    interactions = ["Electromagnetic"];
  }

  function applyUpTemplate(): void {
    id = "custom_u_like";
    name = "Custom Up-like Quark";
    symbol = "u*";
    mass = 0.002;
    width = 0;
    charge = 2 / 3;
    spin = 1;
    color = "Triplet";
    weakIsospin = 1;
    hypercharge = 1;
    baryonNumber = 1;
    leptonElectron = 0;
    leptonMuon = 0;
    leptonTau = 0;
    parity = "Odd";
    chargeConjugation = "Undefined";
    weakMultipletMode = "DoubletUp";
    weakTripletIndex = 0;
    interactions = ["Strong", "Electromagnetic", "WeakNC", "WeakCC"];
  }

  function applyNeutralScalarTemplate(): void {
    id = "custom_s0";
    name = "Neutral Scalar";
    symbol = "S⁰";
    mass = 80;
    width = 0.05;
    charge = 0;
    spin = 0;
    color = "Singlet";
    weakIsospin = 0;
    hypercharge = 0;
    baryonNumber = 0;
    leptonElectron = 0;
    leptonMuon = 0;
    leptonTau = 0;
    parity = "Even";
    chargeConjugation = "Even";
    weakMultipletMode = "Singlet";
    weakTripletIndex = 0;
    interactions = ["Yukawa", "ScalarSelf", "WeakNC"];
  }

  function applyZPrimeTemplate(): void {
    id = "custom_z_prime";
    name = "Z Prime";
    symbol = "Z'";
    mass = 2000;
    width = 30;
    charge = 0;
    spin = 2;
    color = "Singlet";
    weakIsospin = 0;
    hypercharge = 0;
    baryonNumber = 0;
    leptonElectron = 0;
    leptonMuon = 0;
    leptonTau = 0;
    parity = "Odd";
    chargeConjugation = "Undefined";
    weakMultipletMode = "Singlet";
    weakTripletIndex = 0;
    interactions = ["WeakNC"];
  }

  const templates: ParticleTemplate[] = [
    { id: "blank", label: "Blank", apply: applyBlankTemplate },
    { id: "electron", label: "Electron-like", apply: applyElectronTemplate },
    { id: "photon", label: "Photon-like", apply: applyPhotonTemplate },
    { id: "up", label: "Up-quark-like", apply: applyUpTemplate },
    { id: "neutral_scalar", label: "Neutral scalar", apply: applyNeutralScalarTemplate },
    { id: "z_prime", label: "Z' boson", apply: applyZPrimeTemplate },
  ];

  function validate(): string {
    if (!id.trim() || !name.trim() || !symbol.trim()) {
      return "ID, name, and symbol are required.";
    }
    if (!Number.isFinite(mass) || mass < 0) {
      return "Mass must be a non-negative number.";
    }
    if (!Number.isFinite(width) || width < 0) {
      return "Width must be a non-negative number.";
    }
    if (!Number.isFinite(spin) || spin < 0 || !Number.isInteger(spin)) {
      return "2s (spin encoding) must be a non-negative integer.";
    }
    if (weakMultipletMode === "Triplet" && ![-1, 0, 1].includes(weakTripletIndex)) {
      return "Triplet index must be -1, 0, or +1.";
    }
    return "";
  }

  function createParticle(): void {
    error = validate();
    if (error) return;

    const field: Field = {
      id: normalizeId(id),
      name: name.trim(),
      symbol: symbol.trim(),
      mass,
      width,
      quantum_numbers: {
        electric_charge: charge,
        weak_isospin: weakIsospin,
        hypercharge,
        baryon_number: baryonNumber,
        lepton_numbers: {
          electron: leptonElectron,
          muon: leptonMuon,
          tau: leptonTau,
        },
        spin,
        parity,
        charge_conjugation: chargeConjugation,
        color,
        weak_multiplet: weakMultipletValue(),
      },
      interactions: [...new Set(interactions)],
    };

    upsertModelField(field);
    logAtlas(`Custom particle upserted: ${field.id} (${field.symbol})`);
    dispatch("created", field);
    close();
  }

  $: weakMultipletLabel = weakMultipletMode === "Triplet" ? `Triplet(${weakTripletIndex})` : weakMultipletMode;
  $: liveSummary = `${symbol || "?"} · Q=${charge} · 2s=${spin} · color=${color} · interactions=${interactions.length}`;
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-backdrop" role="button" tabindex="0" aria-label="Close custom particle modal" on:click={close} on:keydown={handleBackdropKeydown}>
    <div class="modal" role="dialog" tabindex="-1" aria-label="Create custom particle" on:click|stopPropagation>
      <header class="modal-header">
        <h3>Custom Particle Forge</h3>
        <span class="summary">{liveSummary}</span>
      </header>

      <section class="preset-band">
        <span>Template</span>
        <div class="preset-list">
          {#each templates as template (template.id)}
            <button
              type="button"
              class:active={selectedTemplate === template.id}
              on:click={() => setTemplate(template.id)}
            >{template.label}</button>
          {/each}
        </div>
      </section>

      <section class="section">
        <h4>Identity</h4>
        <div class="grid grid-3">
          <label>ID <input bind:value={id} /></label>
          <label>Name <input bind:value={name} /></label>
          <label>Symbol <input bind:value={symbol} /></label>
        </div>
      </section>

      <section class="section">
        <h4>Dynamics</h4>
        <div class="grid grid-4">
          <label>Mass (GeV) <input type="number" bind:value={mass} min="0" step="0.001" /></label>
          <label>Width (GeV) <input type="number" bind:value={width} min="0" step="0.0001" /></label>
          <label>Electric charge Q <input type="number" bind:value={charge} step="0.333333" /></label>
          <label>2s (spin) <input type="number" bind:value={spin} min="0" step="1" /></label>
        </div>
      </section>

      <section class="section">
        <h4>Quantum numbers</h4>
        <div class="grid grid-4">
          <label>Color
            <select bind:value={color}>
              <option value="Singlet">Singlet</option>
              <option value="Triplet">Triplet</option>
              <option value="AntiTriplet">AntiTriplet</option>
              <option value="Octet">Octet</option>
            </select>
          </label>
          <label>Parity
            <select bind:value={parity}>
              <option value="Even">Even</option>
              <option value="Odd">Odd</option>
            </select>
          </label>
          <label>C-conjugation
            <select bind:value={chargeConjugation}>
              <option value="Undefined">Undefined</option>
              <option value="Even">Even</option>
              <option value="Odd">Odd</option>
            </select>
          </label>
          <label>Weak multiplet
            <select bind:value={weakMultipletMode}>
              <option value="Singlet">Singlet</option>
              <option value="DoubletUp">DoubletUp</option>
              <option value="DoubletDown">DoubletDown</option>
              <option value="Triplet">Triplet</option>
            </select>
          </label>
        </div>

        <div class="grid grid-4">
          <label>T3 (weak isospin) <input type="number" bind:value={weakIsospin} step="0.5" /></label>
          <label>Y (hypercharge) <input type="number" bind:value={hypercharge} step="0.5" /></label>
          <label>B (baryon number x3) <input type="number" bind:value={baryonNumber} step="1" /></label>
          <label>Triplet index
            <input type="number" bind:value={weakTripletIndex} min="-1" max="1" step="1" disabled={weakMultipletMode !== "Triplet"} />
          </label>
        </div>

        <div class="grid grid-3">
          <label>Lₑ <input type="number" bind:value={leptonElectron} step="1" /></label>
          <label>Lμ <input type="number" bind:value={leptonMuon} step="1" /></label>
          <label>Lτ <input type="number" bind:value={leptonTau} step="1" /></label>
        </div>
      </section>

      <section class="section">
        <h4>Interactions</h4>
        <div class="interaction-pills">
          {#each INTERACTION_OPTIONS as interaction (interaction)}
            <button
              type="button"
              class="pill"
              class:pill--active={hasInteraction(interaction)}
              on:click={() => toggleInteraction(interaction)}
            >{interaction}</button>
          {/each}
        </div>
      </section>

      <section class="section section--preview">
        <h4>Preview</h4>
        <div class="preview-grid">
          <div><span>Normalized ID</span><strong>{normalizeId(id)}</strong></div>
          <div><span>Weak multiplet</span><strong>{weakMultipletLabel}</strong></div>
          <div><span>Mass/Width</span><strong>{mass} / {width} GeV</strong></div>
          <div><span>Interactions</span><strong>{interactions.join(", ") || "none"}</strong></div>
        </div>
      </section>

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="actions">
        <button on:click={close}>Cancel</button>
        <button class="primary" on:click={createParticle}>Create / Update</button>
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
    padding: 0.65rem;
  }

  .modal {
    width: min(1080px, 96vw);
    max-height: calc(100vh - 1.3rem);
    overflow-y: auto;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-primary);
    padding: 0.7rem;
    font-family: var(--font-mono);
    display: grid;
    gap: 0.5rem;
  }

  .modal-header {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.35rem;
    display: grid;
    gap: 0.14rem;
  }

  h3 {
    margin: 0;
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--hl-symbol);
  }

  .summary {
    font-size: 0.64rem;
    color: var(--fg-secondary);
  }

  .preset-band {
    display: grid;
    gap: 0.22rem;
  }

  .preset-band > span {
    font-size: 0.57rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-secondary);
  }

  .preset-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .preset-list button,
  .pill,
  .actions button {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    padding: 0.16rem 0.34rem;
    cursor: pointer;
  }

  .preset-list button.active,
  .pill--active,
  .actions button.primary {
    border-color: var(--hl-symbol);
    color: var(--hl-symbol);
    background: color-mix(in srgb, var(--bg-inset) 76%, rgba(var(--color-accent-rgb), 0.18));
  }

  .section {
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.18);
    background: var(--bg-inset);
    padding: 0.4rem;
    display: grid;
    gap: 0.3rem;
  }

  h4 {
    margin: 0;
    font-size: 0.62rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--fg-secondary);
  }

  .grid {
    display: grid;
    gap: 0.32rem;
  }

  .grid-3 {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .grid-4 {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }

  label {
    display: grid;
    gap: 0.14rem;
    font-size: 0.62rem;
    color: var(--fg-secondary);
  }

  input,
  select {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    padding: 0.2rem 0.28rem;
    font-family: var(--font-mono);
    font-size: 0.67rem;
  }

  .interaction-pills {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .section--preview {
    background: color-mix(in srgb, var(--bg-inset) 80%, rgba(var(--color-accent-rgb), 0.08));
  }

  .preview-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.42rem;
  }

  .preview-grid div {
    display: grid;
    gap: 0.08rem;
    border-bottom: 1px dashed rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.24);
    padding-bottom: 0.12rem;
    min-width: 0;
  }

  .preview-grid span {
    font-size: 0.56rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-secondary);
  }

  .preview-grid strong {
    font-size: 0.66rem;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.28rem;
    margin-top: 0.1rem;
  }

  .error {
    margin: 0;
    color: var(--hl-error);
    font-size: 0.68rem;
    border: 1px solid color-mix(in srgb, var(--hl-error) 45%, var(--border));
    background: color-mix(in srgb, var(--bg-inset) 75%, rgba(var(--color-error-rgb, 255, 82, 82), 0.12));
    padding: 0.24rem 0.3rem;
  }

  @media (max-width: 980px) {
    .grid-4,
    .grid-3,
    .preview-grid {
      grid-template-columns: 1fr 1fr;
    }
  }

  @media (max-width: 680px) {
    .grid-4,
    .grid-3,
    .preview-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
