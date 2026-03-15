<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Field } from "$lib/types/spire";
  import {
    resolveCompositeState,
    assignColorSinglet,
    resolveDecayChannels,
    type DecayChannelPreset,
  } from "$lib/core/physics/composites";
  import CompositionSvg from "$lib/components/atlas/CompositionSvg.svelte";
  import DecayTree from "$lib/components/atlas/DecayTree.svelte";

  interface DecaySelectEvent {
    parentId: string;
    finalStateIds: string[];
    label: string;
  }

  export let particle: Field;

  type ViewerMode = "schematic" | "quantum" | "feynman";

  const dispatch = createEventDispatcher<{ decaySelect: DecaySelectEvent }>();
  const HBAR_GEV_S = 6.582119569e-25;

  let mode: ViewerMode = "schematic";

  function toSpinLabel(twiceSpin: number): string {
    if (!Number.isFinite(twiceSpin)) return "n/a";
    return twiceSpin % 2 === 0 ? `${twiceSpin / 2}` : `${twiceSpin}/2`;
  }

  function formatMaybe(v: number | null | undefined, digits = 6): string {
    if (v === null || v === undefined || !Number.isFinite(v)) return "n/a";
    if (Math.abs(v) >= 1e3 || (Math.abs(v) > 0 && Math.abs(v) < 1e-3)) {
      return v.toExponential(3);
    }
    return v.toFixed(Math.min(digits, 6));
  }

  function inferLifetime(width: number): string {
    if (!Number.isFinite(width) || width <= 0) return "stable / undefined";
    return `${(HBAR_GEV_S / width).toExponential(3)} s`;
  }

  function emitDecaySelect(e: CustomEvent<DecaySelectEvent>): void {
    dispatch("decaySelect", e.detail);
  }

  $: composite = resolveCompositeState(particle);
  $: valence = composite ? assignColorSinglet(composite.valence) : [];
  $: decayChannels = resolveDecayChannels(particle);
  $: dominantDecay = decayChannels.length > 0 ? decayChannels[0] : null;
  $: qn = particle.quantum_numbers;
  $: jpc = `${toSpinLabel(qn.spin)}^{${qn.parity === "Even" ? "+" : "-"}${qn.charge_conjugation === "Even" ? "+" : qn.charge_conjugation === "Odd" ? "-" : "?"}}`;
  $: isospin = `${(qn.weak_isospin / 2).toFixed(1)}`;
</script>

<div class="viewer">
  <header class="viewer-header">
    <div class="title-block">
      <strong>{particle.symbol}</strong>
      <span>{particle.name} · {particle.id}</span>
    </div>
    <div class="mode-tabs">
      <button class:active={mode === "schematic"} on:click={() => (mode = "schematic")}>Schematic</button>
      <button class:active={mode === "quantum"} on:click={() => (mode = "quantum")}>Quantum Numbers</button>
      <button class:active={mode === "feynman"} on:click={() => (mode = "feynman")}>Feynman Vertex</button>
    </div>
  </header>

  {#if mode === "schematic"}
    {#if composite}
      <CompositionSvg valence={valence} label={`${composite.label}: ${composite.valence.join(" ")}`} />
    {:else}
      <div class="empty">No valence-composition preset is registered for this particle.</div>
    {/if}

    <section class="decay-section">
      <h5>Decay Modes</h5>
      <DecayTree
        parentId={particle.id}
        channels={decayChannels}
        on:select={emitDecaySelect}
      />
    </section>
  {:else if mode === "quantum"}
    <section class="qn-card">
      <div><span>Mass</span><strong>{formatMaybe(particle.mass)} GeV</strong></div>
      <div><span>Width</span><strong>{formatMaybe(particle.width)} GeV</strong></div>
      <div><span>Lifetime</span><strong>{inferLifetime(particle.width)}</strong></div>
      <div><span>J^PC</span><strong>{jpc}</strong></div>
      <div><span>I</span><strong>{isospin}</strong></div>
      <div><span>G-parity</span><strong>n/a</strong></div>
      <div><span>Electric charge Q</span><strong>{qn.electric_charge}</strong></div>
      <div><span>Baryon number B</span><strong>{qn.baryon_number}</strong></div>
      <div><span>Lepton numbers</span><strong>{qn.lepton_numbers.electron}/{qn.lepton_numbers.muon}/{qn.lepton_numbers.tau}</strong></div>
      <div><span>Color rep</span><strong>{qn.color}</strong></div>
      <div><span>Weak isospin T3</span><strong>{qn.weak_isospin}</strong></div>
      <div><span>Hypercharge Y</span><strong>{qn.hypercharge}</strong></div>
    </section>
  {:else}
    <section class="feynman">
      <svg viewBox="0 0 520 210" role="img" aria-label="Dominant vertex diagram">
        <line class="line" x1="48" y1="105" x2="250" y2="105" />
        <text class="lbl" x="30" y="105">{particle.symbol}</text>

        <line class="line" x1="250" y1="105" x2="440" y2="56" />
        <line class="line" x1="250" y1="105" x2="440" y2="154" />

        <circle class="vertex" cx="250" cy="105" r="4.8" />

        {#if dominantDecay}
          <text class="lbl" x="452" y="56">{dominantDecay.finalStateIds[0] ?? "f1"}</text>
          <text class="lbl" x="452" y="154">{dominantDecay.finalStateIds[1] ?? "f2"}</text>
          <text class="coupling" x="260" y="94">
            g_{dominantDecay.interaction} · BR={(dominantDecay.branchingRatio * 100).toFixed(2)}%
          </text>
        {:else}
          <text class="lbl" x="452" y="56">f₁</text>
          <text class="lbl" x="452" y="154">f₂</text>
          <text class="coupling" x="260" y="94">g_eff</text>
        {/if}
      </svg>
      <p class="hint">Minimal effective vertex for the dominant listed process.</p>
    </section>
  {/if}
</div>

<style>
  .viewer {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    padding: 0.45rem;
    min-height: 0;
  }

  .viewer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-bottom: 1px solid var(--color-border, var(--border));
    padding-bottom: 0.3rem;
  }

  .title-block {
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
  }

  .title-block strong {
    font-family: var(--font-mono);
    font-size: 0.88rem;
    color: var(--color-accent, var(--hl-symbol));
  }

  .title-block span {
    font-family: var(--font-mono);
    font-size: 0.69rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .mode-tabs {
    display: flex;
    gap: 0.22rem;
  }

  .mode-tabs button {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-muted, var(--fg-secondary));
    padding: 0.2rem 0.42rem;
    font-size: 0.67rem;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  .mode-tabs button.active {
    border-color: var(--color-accent, var(--hl-symbol));
    color: var(--color-text-primary, var(--fg-primary));
  }

  .decay-section h5 {
    margin: 0 0 0.25rem;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .qn-card {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.7rem;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .qn-card > div {
    display: flex;
    justify-content: space-between;
    gap: 0.35rem;
    border-bottom: 1px dashed rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.25);
    padding-bottom: 0.14rem;
  }

  .qn-card span,
  .qn-card strong {
    font-family: var(--font-mono);
    font-size: 0.69rem;
  }

  .qn-card span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .qn-card strong {
    color: var(--color-text-primary, var(--fg-primary));
    font-weight: 600;
    text-align: right;
  }

  .feynman {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.25rem;
  }

  .feynman svg {
    width: 100%;
    height: auto;
    display: block;
  }

  .line {
    stroke: var(--color-text-primary, var(--fg-primary));
    stroke-width: 1.15;
  }

  .vertex {
    fill: var(--color-accent, var(--hl-symbol));
  }

  .lbl,
  .coupling {
    font-family: var(--font-mono);
    fill: var(--color-text-primary, var(--fg-primary));
    dominant-baseline: middle;
  }

  .lbl {
    font-size: 12px;
  }

  .coupling {
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 10px;
  }

  .hint,
  .empty {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .empty {
    border: 1px dashed var(--color-border, var(--border));
    padding: 0.45rem;
    background: var(--color-bg-inset, var(--bg-inset));
  }
</style>
