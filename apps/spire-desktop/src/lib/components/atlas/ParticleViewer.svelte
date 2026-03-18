<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Field } from "$lib/types/spire";
  import {
    resolveCompositeState,
    assignColorSinglet,
    resolveDecayChannels,
    type CompositeQuarkContent,
  } from "$lib/core/physics/composites";
  import CompositionSvg from "$lib/components/atlas/CompositionSvg.svelte";
  import DecayTree from "$lib/components/atlas/DecayTree.svelte";
  import MathRenderer from "$lib/components/math/MathRenderer.svelte";
  import { formatQuantumFraction } from "$lib/core/physics/fractionFormat";

  interface DecaySelectEvent {
    parentId: string;
    finalStateIds: string[];
    label: string;
  }

  export let particle: Field | null = null;
  export let referenceComposite: CompositeQuarkContent | null = null;

  type ViewerMode = "schematic" | "quantum" | "feynman";

  const dispatch = createEventDispatcher<{
    decaySelect: DecaySelectEvent;
    addToReaction: {
      target: "initial" | "final";
      id: string;
      label: string;
    };
  }>();

  const HBAR_GEV_S = 6.582119569e-25;

  let mode: ViewerMode = "schematic";
  let activeIdentity = "";
  let selectedDecayIndex = 0;
  let symbolicLatexMode: "rendered" | "raw" = "rendered";
  let vertexLayout: "s-channel" | "t-channel" | "split" = "split";
  let showCoupling = true;
  let showInteractionLegend = true;
  let showMomentumLabels = true;
  let showArrowheads = true;
  let finalArrangement: "fan" | "balanced" = "fan";
  let lineFlavor: "fermion" | "boson" | "scalar" = "fermion";
  let labelMode: "symbol" | "id" | "pretty" = "pretty";
  let propagatorStyle: "decay" | "exchange" | "contact" = "decay";

  $: {
    const identity = particle
      ? `particle:${particle.id}`
      : referenceComposite
        ? `reference:${referenceComposite.label}`
        : "";
    if (identity !== activeIdentity) {
      activeIdentity = identity;
      mode = "schematic";
      selectedDecayIndex = 0;
    }
  }

  function toSpinLabel(twiceSpin: number): string {
    if (!Number.isFinite(twiceSpin)) return "n/a";
    return twiceSpin % 2 === 0 ? `${twiceSpin / 2}` : `${twiceSpin}/2`;
  }

  function toSpinLatex(twiceSpin: number): string {
    if (!Number.isFinite(twiceSpin)) return "\\text{n/a}";
    if (twiceSpin % 2 === 0) return `${twiceSpin / 2}`;
    return `\\frac{${twiceSpin}}{2}`;
  }

  function formatMaybe(value: number | null | undefined, digits = 6): string {
    if (value === null || value === undefined || !Number.isFinite(value)) return "n/a";
    if (Math.abs(value) >= 1e3 || (Math.abs(value) > 0 && Math.abs(value) < 1e-3)) return value.toExponential(3);
    return value.toFixed(Math.min(digits, 6));
  }

  function formatQuantum(value: number | null | undefined, signed = true): string {
    if (value === null || value === undefined) return "n/a";
    return formatQuantumFraction(value, { signed });
  }

  function inferLifetime(width: number): string {
    if (!Number.isFinite(width) || width <= 0) return "stable / undefined";
    return `${(HBAR_GEV_S / width).toExponential(3)} s`;
  }

  function emitDecaySelect(event: CustomEvent<DecaySelectEvent>): void {
    dispatch("decaySelect", event.detail);
  }

  function reactionAddId(): string | null {
    if (particle) return particle.id;
    if (referenceComposite) return referenceComposite.particleIds[0] ?? null;
    return null;
  }

  function reactionAddLabel(): string {
    if (particle) return `${particle.symbol} (${particle.id})`;
    if (referenceComposite) return `${referenceComposite.label} (${referenceComposite.particleIds[0] ?? "reference"})`;
    return "selection";
  }

  function emitAddToReaction(target: "initial" | "final"): void {
    const id = reactionAddId();
    if (!id) return;
    dispatch("addToReaction", { target, id, label: reactionAddLabel() });
  }

  function prettyParticleId(id: string): string {
    return id.replace(/_/g, " ").replace(/\bbar\b/g, "bar");
  }

  function weakMultipletLabel(field: Field | null): string {
    if (!field) return "n/a";
    return typeof field.quantum_numbers.weak_multiplet === "string"
      ? field.quantum_numbers.weak_multiplet
      : `Triplet(${field.quantum_numbers.weak_multiplet.Triplet})`;
  }

  function normalizedMetric(value: number, min: number, max: number): number {
    if (!Number.isFinite(value)) return 0;
    if (Math.abs(max - min) < 1e-9) return 0;
    return Math.max(0, Math.min(1, (value - min) / (max - min)));
  }

  function signedMetric(value: number, extent: number): number {
    if (!Number.isFinite(value) || extent <= 0) return 0;
    return Math.max(-1, Math.min(1, value / extent));
  }

  function branchLabel(id: string): string {
    if (labelMode === "id") return id;
    if (labelMode === "symbol") return id.replace(/_bar$/, "bar");
    return prettyParticleId(id);
  }

  function chargeDescriptor(charge: number): string {
    if (Math.abs(charge) < 1e-12) return "neutral";
    return charge > 0 ? "positive" : "negative";
  }

  function symmetrySignature(field: Field | null): string {
    if (!field) return "n/a";
    const qn = field.quantum_numbers;
    const parity = qn.parity === "Even" ? "+" : "-";
    const chargeConjugation = qn.charge_conjugation === "Even"
      ? "+"
      : qn.charge_conjugation === "Odd"
        ? "-"
        : "?";
    return `J^PC ${toSpinLabel(qn.spin)}^${parity}${chargeConjugation}`;
  }

  $: composite = particle ? resolveCompositeState(particle) : null;
  $: activeComposite = composite ?? referenceComposite;
  $: valence = activeComposite ? assignColorSinglet(activeComposite.valence) : [];
  $: decayChannels = particle ? resolveDecayChannels(particle) : referenceComposite ? resolveDecayChannels(referenceComposite.particleIds[0]) : [];
  $: selectedDecayIndex = Math.min(selectedDecayIndex, Math.max(0, decayChannels.length - 1));
  $: dominantDecay = decayChannels[selectedDecayIndex] ?? null;
  $: qn = particle?.quantum_numbers;
  $: jpc = qn
    ? `${toSpinLabel(qn.spin)}^{${qn.parity === "Even" ? "+" : "-"}${qn.charge_conjugation === "Even" ? "+" : qn.charge_conjugation === "Odd" ? "-" : "?"}}`
    : "n/a";
  $: isospin = qn ? formatQuantum(qn.weak_isospin / 2, true) : "n/a";
  $: spinParityLatex = qn
    ? `J^{PC} = ${toSpinLatex(qn.spin)}^{${qn.parity === "Even" ? "+" : "-"}${qn.charge_conjugation === "Even" ? "+" : qn.charge_conjugation === "Odd" ? "-" : "?"}}`
    : "";
  $: isospinLatex = qn ? `I = ${formatMaybe(qn.weak_isospin / 2, 2)}` : "";
  $: displaySymbol = particle ? particle.symbol : referenceComposite?.label ?? "";
  $: displayName = particle ? `${particle.name} | ${particle.id}` : referenceComposite ? `${referenceComposite.kind} reference state | ${referenceComposite.particleIds[0]}` : "";
  $: feynmanFinalStateIds = dominantDecay?.finalStateIds?.length ? dominantDecay.finalStateIds : ["f1", "f2"];
  $: feynmanFinalPoints = feynmanFinalStateIds.map((_, index) => {
    const count = feynmanFinalStateIds.length;
    const startY = finalArrangement === "fan" ? 34 : 56;
    const endY = finalArrangement === "fan" ? 176 : 154;
    const y = count === 1 ? 105 : startY + (index * (endY - startY)) / Math.max(1, count - 1);
    return { x: 446, y };
  });
  $: mediatorLabel = dominantDecay ? (propagatorStyle === "contact" ? `${dominantDecay.interaction} contact` : `${particle?.symbol ?? "V"}`) : particle?.symbol ?? "V";
  $: dominantInteraction = dominantDecay?.interaction ?? "weak";
  $: fieldMetrics = particle
    ? [
        { label: "log10 m", text: formatMaybe(particle.mass), className: "metric-mass", normalized: normalizedMetric(Math.log10(Math.max(particle.mass, 1e-12)), -12, 3) },
        { label: "log10 width", text: formatMaybe(particle.width), className: "metric-width", normalized: normalizedMetric(Math.log10(Math.max(particle.width, 1e-18)), -18, 1) },
        { label: "Q", text: formatQuantum(particle.quantum_numbers.electric_charge, true), className: "metric-charge", normalized: signedMetric(particle.quantum_numbers.electric_charge, 3) },
        { label: "spin", text: toSpinLabel(particle.quantum_numbers.spin), className: "metric-spin", normalized: normalizedMetric(particle.quantum_numbers.spin / 2, 0, 3) },
        { label: "T3", text: formatQuantum(particle.quantum_numbers.weak_isospin / 2, true), className: "metric-isospin", normalized: signedMetric(particle.quantum_numbers.weak_isospin / 2, 2) },
        { label: "Y", text: formatQuantum(particle.quantum_numbers.hypercharge, true), className: "metric-hypercharge", normalized: signedMetric(particle.quantum_numbers.hypercharge, 4) },
      ]
    : [];
  $: interactionChannels = particle
    ? [
        { label: "Strong", active: particle.interactions.includes("Strong") },
        { label: "EM", active: particle.interactions.includes("Electromagnetic") },
        { label: "Weak NC", active: particle.interactions.includes("WeakNC") },
        { label: "Weak CC", active: particle.interactions.includes("WeakCC") },
        { label: "Yukawa", active: particle.interactions.includes("Yukawa") },
      ]
    : [];
</script>

<div class="viewer">
  <header class="viewer-header">
    <div class="title-block">
      <strong>{displaySymbol}</strong>
      <span>{displayName}</span>
    </div>
    <div class="viewer-actions">
      <div class="mode-tabs">
        <button class:active={mode === "schematic"} on:click={() => (mode = "schematic")}>Schematic</button>
        <button class:active={mode === "quantum"} on:click={() => (mode = "quantum")}>Quantum Numbers</button>
        <button class:active={mode === "feynman"} on:click={() => (mode = "feynman")}>Feynman Vertex</button>
      </div>
      {#if reactionAddId()}
        <div class="reaction-actions">
          <button class="reaction-add" on:click={() => emitAddToReaction("initial")}>+ initial</button>
          <button class="reaction-add" on:click={() => emitAddToReaction("final")}>+ final</button>
        </div>
      {/if}
    </div>
  </header>

  {#if particle || referenceComposite}
    {#if mode === "schematic"}
      {#if activeComposite}
        <CompositionSvg valence={valence} label={`${activeComposite.label}: ${activeComposite.valence.join(" ")}`} />
      {:else}
        <section class="field-schematic">
          <div class="field-schematic-header">
            <h5>Field State Dossier</h5>
            <span class="field-state-tag">{chargeDescriptor(particle?.quantum_numbers.electric_charge ?? 0)}</span>
          </div>
          <div class="field-dossier-hero">
            <div>
              <strong>{particle?.symbol ?? "?"}</strong>
              <span>{particle?.name ?? "Selected field"} | {particle?.id ?? "field"}</span>
            </div>
            <div class="field-dossier-tags">
              <span>{symmetrySignature(particle)}</span>
              <span>{particle?.quantum_numbers.color ?? "colorless"}</span>
              <span>{weakMultipletLabel(particle)}</span>
            </div>
          </div>
          <div class="field-dossier-grid">
            <article>
              <h6>Mass And Lifetime</h6>
              <div class="field-dossier-values">
                <div><span>Mass</span><strong>{formatMaybe(particle?.mass)} GeV</strong></div>
                <div><span>Width</span><strong>{formatMaybe(particle?.width)} GeV</strong></div>
                <div><span>Lifetime</span><strong>{inferLifetime(particle?.width ?? NaN)}</strong></div>
              </div>
            </article>
            <article>
              <h6>Gauge Charges</h6>
              <div class="field-dossier-values">
                <div><span>Electric charge</span><strong>{formatQuantum(particle?.quantum_numbers.electric_charge, true)}</strong></div>
                <div><span>Weak isospin</span><strong>{formatQuantum(particle?.quantum_numbers.weak_isospin, true)}</strong></div>
                <div><span>Hypercharge</span><strong>{formatQuantum(particle?.quantum_numbers.hypercharge, true)}</strong></div>
              </div>
            </article>
            <article>
              <h6>Conserved Numbers</h6>
              <div class="field-dossier-values">
                <div><span>Baryon number</span><strong>{formatQuantum(particle?.quantum_numbers.baryon_number, true)}</strong></div>
                <div><span>Lepton e/mu/t</span><strong>{particle ? `${particle.quantum_numbers.lepton_numbers.electron}/${particle.quantum_numbers.lepton_numbers.muon}/${particle.quantum_numbers.lepton_numbers.tau}` : "n/a"}</strong></div>
                <div><span>Spin</span><strong>{toSpinLabel(particle?.quantum_numbers.spin ?? NaN)}</strong></div>
              </div>
            </article>
            <article>
              <h6>Interaction Channels</h6>
              <div class="field-interaction-badges">
                {#each interactionChannels as channel (`field-int-${channel.label}`)}
                  <span class:field-interaction-badge--active={channel.active} class="field-interaction-badge">{channel.label}</span>
                {/each}
              </div>
            </article>
          </div>
          <div class="field-summary-grid">
            <div><span>Interactions</span><strong>{particle?.interactions.join(", ") || "n/a"}</strong></div>
            <div><span>Weak multiplet</span><strong>{weakMultipletLabel(particle)}</strong></div>
            <div><span>Color rep</span><strong>{particle?.quantum_numbers.color ?? "n/a"}</strong></div>
            <div><span>Parity</span><strong>{particle?.quantum_numbers.parity ?? "n/a"}</strong></div>
          </div>
        </section>
      {/if}

      <section class="decay-section">
        <h5>Decay Modes</h5>
        <DecayTree parentId={particle?.id ?? referenceComposite?.particleIds[0] ?? "reference"} channels={decayChannels} on:select={emitDecaySelect} />
      </section>
    {:else if mode === "quantum"}
      {#if particle}
        <section class="symbolic-card">
          <div class="symbolic-header">
            <h5>Symbolic Quantum Labels</h5>
            <button on:click={() => (symbolicLatexMode = symbolicLatexMode === "rendered" ? "raw" : "rendered")}>
              {symbolicLatexMode === "rendered" ? "Raw LaTeX" : "Rendered Math"}
            </button>
          </div>
          <MathRenderer latex={spinParityLatex} mode={symbolicLatexMode} block={true} />
          <MathRenderer latex={isospinLatex} mode={symbolicLatexMode} block={true} />
        </section>
        <section class="qn-card">
          <div><span>Mass</span><strong>{formatMaybe(particle.mass)} GeV</strong></div>
          <div><span>Width</span><strong>{formatMaybe(particle.width)} GeV</strong></div>
          <div><span>Lifetime</span><strong>{inferLifetime(particle.width)}</strong></div>
          <div><span>J^PC</span><strong>{jpc}</strong></div>
          <div><span>I</span><strong>{isospin}</strong></div>
          <div><span>Electric charge Q</span><strong>{formatQuantum(qn?.electric_charge, true)}</strong></div>
          <div><span>Baryon number B</span><strong>{formatQuantum(qn?.baryon_number, true)}</strong></div>
          <div><span>Lepton numbers</span><strong>{qn?.lepton_numbers.electron}/{qn?.lepton_numbers.muon}/{qn?.lepton_numbers.tau}</strong></div>
          <div><span>Color rep</span><strong>{qn?.color}</strong></div>
          <div><span>Weak isospin T3</span><strong>{formatQuantum(qn?.weak_isospin, true)}</strong></div>
          <div><span>Hypercharge Y</span><strong>{formatQuantum(qn?.hypercharge, true)}</strong></div>
          <div><span>Interactions</span><strong class="mono">{particle.interactions.join(", ") || "n/a"}</strong></div>
          <div><span>Weak multiplet</span><strong>{weakMultipletLabel(particle)}</strong></div>
        </section>
      {:else if referenceComposite}
        <section class="qn-card">
          <div><span>Reference kind</span><strong>{referenceComposite.kind}</strong></div>
          <div><span>Alias count</span><strong>{referenceComposite.particleIds.length}</strong></div>
          <div><span>Primary id</span><strong>{referenceComposite.particleIds[0]}</strong></div>
          <div><span>Valence pattern</span><strong class="mono">{referenceComposite.valence.join(" ")}</strong></div>
          <div><span>Schematic coverage</span><strong>atlas reference</strong></div>
          <div><span>Decay presets</span><strong>{decayChannels.length}</strong></div>
        </section>
      {/if}
    {:else}
      <section class="feynman">
        <div class="feynman-controls">
          <label>Channel<select bind:value={selectedDecayIndex}>{#if decayChannels.length === 0}<option value={0}>No decay presets</option>{:else}{#each decayChannels as channel, index (`${channel.finalStateIds.join("+")}-${index}`)}<option value={index}>{branchLabel(channel.finalStateIds.join(" + "))}</option>{/each}{/if}</select></label>
          <label>Layout<select bind:value={vertexLayout}><option value="split">Split</option><option value="s-channel">s-channel</option><option value="t-channel">t-channel</option></select></label>
          <label>Propagator<select bind:value={propagatorStyle}><option value="decay">Decay</option><option value="exchange">Exchange</option><option value="contact">Contact</option></select></label>
          <label>Labels<select bind:value={labelMode}><option value="pretty">Pretty</option><option value="symbol">Symbol</option><option value="id">ID</option></select></label>
          <label>Outgoing<select bind:value={finalArrangement}><option value="fan">Fan</option><option value="balanced">Balanced</option></select></label>
          <label>Line flavor<select bind:value={lineFlavor}><option value="fermion">Fermion</option><option value="boson">Boson</option><option value="scalar">Scalar</option></select></label>
          <label class="check"><input type="checkbox" bind:checked={showCoupling} />Coupling</label>
          <label class="check"><input type="checkbox" bind:checked={showMomentumLabels} />Momenta</label>
          <label class="check"><input type="checkbox" bind:checked={showArrowheads} />Arrowheads</label>
          <label class="check"><input type="checkbox" bind:checked={showInteractionLegend} />Legend</label>
        </div>
        <svg viewBox="0 0 480 210" role="img" aria-label="Effective vertex layout">
          {#if showArrowheads}
            <defs>
              <pattern id="feynman-grid" width="20" height="20" patternUnits="userSpaceOnUse">
                <path d="M 20 0 L 0 0 0 20" fill="none" stroke="rgba(255,255,255,0.05)" stroke-width="1" />
              </pattern>
              <filter id="feynman-glow" x="-40%" y="-40%" width="180%" height="180%">
                <feGaussianBlur stdDeviation="1.2" result="blur" />
                <feMerge>
                  <feMergeNode in="blur" />
                  <feMergeNode in="SourceGraphic" />
                </feMerge>
              </filter>
              <marker id="vertex-arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
                <path d="M 0 0 L 8 4 L 0 8 z" fill="currentColor" />
              </marker>
            </defs>
          {/if}

          <rect x="1" y="1" width="478" height="208" rx="6" class="diagram-bg" />
          <rect x="1" y="1" width="478" height="208" rx="6" fill="url(#feynman-grid)" />

          <g class={`diagram-layer diagram-layer--${dominantInteraction}`} filter="url(#feynman-glow)">
            <line class={`line line--${lineFlavor}`} x1="40" y1="105" x2="170" y2="105" marker-end={showArrowheads ? "url(#vertex-arrow)" : undefined} />
            <line class={`line line--mediator ${propagatorStyle === "exchange" ? "line--boson" : propagatorStyle === "contact" ? "line--scalar" : ""}`} x1="170" y1="105" x2="250" y2={vertexLayout === "t-channel" ? 56 : 105} marker-end={showArrowheads ? "url(#vertex-arrow)" : undefined} />
            {#each feynmanFinalPoints as point, index (`f-out-${index}`)}
              <line class={`line line--${lineFlavor}`} x1="250" y1={vertexLayout === "t-channel" ? 56 : 105} x2={point.x} y2={point.y} marker-end={showArrowheads ? "url(#vertex-arrow)" : undefined} />
            {/each}
          </g>

          <circle class="vertex" cx="170" cy="105" r="4.8" />
          <circle class="vertex" cx="250" cy={vertexLayout === "t-channel" ? 56 : 105} r="4.8" />

          {#if dominantDecay}
            {#each feynmanFinalStateIds as stateId, index (`f-label-${stateId}-${index}`)}
              <text class="lbl" x="456" y={feynmanFinalPoints[index]?.y ?? 105}>{branchLabel(stateId)}</text>
            {/each}
            {#if showCoupling}
              <text class="coupling" x="260" y="94">g_{dominantDecay.interaction} | BR={(dominantDecay.branchingRatio * 100).toFixed(2)}%</text>
            {/if}
            <text class="mediator" x="260" y="118">{mediatorLabel}</text>
          {/if}

          {#if showMomentumLabels}
            <text class="momentum" x="122" y="92">p_in</text>
            {#each feynmanFinalPoints as point, index (`momentum-${index}`)}
              <text class="momentum" x={point.x - 32} y={point.y - 10}>p_{index + 1}</text>
            {/each}
          {/if}
        </svg>
        <div class="feynman-summary">Dominant vertex view for the active atlas field or reference state.</div>
        {#if showInteractionLegend && dominantDecay}
          <div class="feynman-legend">
            <span>interaction: {dominantDecay.interaction}</span>
            <span>BR {(dominantDecay.branchingRatio * 100).toFixed(2)}%</span>
            <span>{vertexLayout} / {propagatorStyle}</span>
          </div>
        {/if}
        <div class="channel-stats">
          <div><span>Parent</span><strong>{particle?.id ?? referenceComposite?.particleIds[0] ?? "state"}</strong></div>
          <div><span>Final multiplicity</span><strong>{feynmanFinalStateIds.length}</strong></div>
          <div><span>Interaction</span><strong>{dominantDecay?.interaction ?? "n/a"}</strong></div>
          <div><span>Branching ratio</span><strong>{dominantDecay ? `${(dominantDecay.branchingRatio * 100).toFixed(3)}%` : "n/a"}</strong></div>
        </div>
      </section>
    {/if}
  {:else}
    <div class="empty">No item selected.</div>
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

  .title-block strong,
  .title-block span,
  .mode-tabs button,
  .reaction-add,
  .field-state-tag,
  .field-dossier-hero strong,
  .field-dossier-hero span,
  .field-dossier-values span,
  .field-dossier-values strong,
  .field-interaction-badge,
  .field-summary-grid span,
  .field-summary-grid strong,
  .symbolic-header h5,
  .symbolic-header button,
  .qn-card span,
  .qn-card strong,
  .feynman-controls,
  .channel-stats span,
  .channel-stats strong,
  .empty {
    font-family: var(--font-mono);
  }

  .title-block strong {
    font-size: 0.88rem;
    color: var(--color-accent, var(--hl-symbol));
  }

  .title-block span {
    font-size: 0.69rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .viewer-actions {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
    align-items: flex-end;
  }

  .mode-tabs,
  .reaction-actions {
    display: flex;
    gap: 0.22rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .mode-tabs button,
  .reaction-add,
  .symbolic-header button,
  .feynman-controls select,
  .feynman-controls input {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-primary, var(--fg-primary));
    padding: 0.18rem 0.4rem;
    font-size: 0.62rem;
    cursor: pointer;
  }

  .mode-tabs button.active,
  .reaction-add {
    border-color: var(--color-accent, var(--hl-symbol));
  }

  .field-schematic,
  .symbolic-card,
  .feynman {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .field-schematic-header,
  .symbolic-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    margin-bottom: 0.35rem;
  }

  .field-state-tag {
    border: 1px solid rgba(var(--color-accent-rgb), 0.2);
    background: rgba(var(--color-accent-rgb), 0.07);
    color: var(--color-text-primary, var(--fg-primary));
    font-size: 0.58rem;
    padding: 0.08rem 0.22rem;
  }

  .field-dossier-hero {
    display: flex;
    justify-content: space-between;
    gap: 0.45rem;
    padding: 0.34rem 0.38rem;
    border: 1px solid rgba(var(--color-accent-rgb), 0.16);
    background: var(--color-bg-surface, var(--bg-surface));
    margin-bottom: 0.34rem;
  }

  .field-dossier-hero strong {
    display: block;
    font-size: 0.92rem;
    color: var(--color-text-primary, var(--fg-primary));
  }

  .field-dossier-hero span {
    font-size: 0.62rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .field-dossier-tags {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.18rem;
    align-content: flex-start;
  }

  .field-dossier-tags span {
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.2);
    background: var(--color-bg-surface, var(--bg-surface));
    padding: 0.08rem 0.22rem;
  }

  .field-dossier-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.32rem;
    margin-bottom: 0.34rem;
  }

  .field-dossier-grid article {
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.16);
    background: var(--color-bg-surface, var(--bg-surface));
    padding: 0.32rem 0.35rem;
  }

  .field-dossier-grid h6,
  .decay-section h5,
  .symbolic-header h5 {
    margin: 0 0 0.22rem;
    font-size: 0.58rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-accent, var(--hl-symbol));
  }

  .field-dossier-values {
    display: grid;
    gap: 0.18rem;
  }

  .field-dossier-values > div {
    display: flex;
    justify-content: space-between;
    gap: 0.28rem;
  }

  .field-dossier-values span,
  .field-summary-grid span {
    font-size: 0.58rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .field-dossier-values strong,
  .field-summary-grid strong {
    font-size: 0.64rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-align: right;
  }

  .field-interaction-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .field-interaction-badge {
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.2);
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-muted, var(--fg-secondary));
    font-size: 0.58rem;
    padding: 0.08rem 0.22rem;
  }

  .field-interaction-badge--active {
    border-color: rgba(var(--color-accent-rgb), 0.28);
    background: rgba(var(--color-accent-rgb), 0.09);
    color: var(--color-text-primary, var(--fg-primary));
  }

  .field-summary-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.28rem 0.5rem;
  }

  .field-summary-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
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

  .feynman-controls {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.45rem;
    margin-bottom: 0.25rem;
    font-size: 0.64rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .feynman-controls label {
    display: flex;
    flex-direction: column;
    gap: 0.13rem;
  }

  .feynman-controls .check {
    flex-direction: row;
    align-items: center;
    gap: 0.33rem;
  }

  .feynman-summary {
    font-family: var(--font-mono);
    font-size: 0.64rem;
    color: var(--color-text-muted, var(--fg-secondary));
    margin-bottom: 0.25rem;
  }

  .feynman svg {
    width: 100%;
    height: auto;
    display: block;
    margin-bottom: 0.25rem;
  }

  .diagram-bg {
    fill: color-mix(in srgb, var(--color-bg-surface, var(--bg-surface)) 85%, #06080f 15%);
    stroke: rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.22);
  }

  .diagram-layer {
    color: var(--color-accent, var(--hl-symbol));
  }

  .diagram-layer--weak {
    color: #64b5f6;
  }

  .diagram-layer--strong {
    color: #ef9a9a;
  }

  .diagram-layer--em {
    color: #ffd54f;
  }

  .line {
    stroke: currentColor;
    stroke-width: 1.35;
    color: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .line--mediator {
    stroke-dasharray: 5 3;
  }

  .line--boson {
    stroke-dasharray: 2 2;
  }

  .line--scalar {
    stroke-width: 1.6;
    opacity: 0.86;
  }

  .vertex {
    fill: var(--color-accent, var(--hl-symbol));
    stroke: rgba(255, 255, 255, 0.55);
    stroke-width: 0.6;
  }

  .lbl,
  .coupling,
  .mediator,
  .momentum {
    font-family: var(--font-mono);
    fill: var(--color-text-primary, var(--fg-primary));
    dominant-baseline: middle;
  }

  .lbl {
    font-size: 12px;
  }

  .coupling,
  .mediator,
  .momentum {
    fill: var(--color-text-muted, var(--fg-secondary));
    font-size: 10px;
  }

  .feynman-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 0.8rem;
    margin-top: 0.25rem;
    margin-bottom: 0.25rem;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .channel-stats {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.2rem 0.45rem;
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.2);
    padding: 0.25rem 0.3rem;
    background: rgba(var(--color-bg-surface-rgb, 18, 20, 28), 0.42);
  }

  .channel-stats > div {
    display: flex;
    justify-content: space-between;
    gap: 0.25rem;
  }

  .channel-stats span,
  .channel-stats strong {
    font-size: 0.59rem;
  }

  .channel-stats span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .channel-stats strong {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .empty {
    border: 1px dashed var(--color-border, var(--border));
    padding: 0.45rem;
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-muted, var(--fg-secondary));
    font-size: 0.7rem;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 0.65rem;
    word-break: break-all;
  }

  @media (max-width: 920px) {
    .field-dossier-grid,
    .field-summary-grid,
    .qn-card,
    .feynman-controls,
    .channel-stats {
      grid-template-columns: 1fr;
    }
  }
</style>
