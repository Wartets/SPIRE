<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Field } from "$lib/types/spire";
  import {
    resolveCompositeState,
    assignColorSinglet,
    resolveDecayChannels,
    type DecayChannelPreset,
  } from "$lib/core/physics/composites";
  import type { IsotopeData, ElementData } from "$lib/core/physics/nuclearDataLoader";
  import { formatHalfLife, bindingEnergyPerNucleon } from "$lib/core/physics/nuclearDataLoader";
  import {
    synthesizeMoleculeIsotopes,
    type MoleculeSynthesisResult,
  } from "$lib/core/physics/moleculeSynthesis";
  import CompositionSvg from "$lib/components/atlas/CompositionSvg.svelte";
  import DecayTree from "$lib/components/atlas/DecayTree.svelte";
  import MoleculePreview3D from "$lib/components/atlas/MoleculePreview3D.svelte";

  interface DecaySelectEvent {
    parentId: string;
    finalStateIds: string[];
    label: string;
  }

  /** Standard particle view (exclusive with `isotope`). */
  export let particle: Field | null = null;
  export let element: ElementData | null = null;

  /**
   * Nuclear isotope view.  When set, renders nuclear droplet + isotope decay
   * tree instead of the hadronic schematic.
   */
  export let isotope: {
    Z: number;
    A: number;
    symbol: string;
    name: string;
    isotopeData: IsotopeData;
    element: ElementData;
  } | null = null;

  type ViewerMode = "schematic" | "quantum" | "feynman";

  const dispatch = createEventDispatcher<{
    decaySelect: DecaySelectEvent;
    isotopeSynthesisSelect: {
      Z: number;
      A: number;
      symbol: string;
      name: string;
      isotopeData: IsotopeData;
      element: ElementData;
    };
  }>();
  const HBAR_GEV_S = 6.582119569e-25;

  let mode: ViewerMode = "schematic";
  let activeIdentity = "";

  let moleculeFormula = "H2O";
  let synthesisBusy = false;
  let synthesisError = "";
  let synthesisResult: MoleculeSynthesisResult | null = null;

  let selectedDecayIndex = 0;
  let vertexLayout: "s-channel" | "t-channel" | "split" = "split";
  let showCoupling = true;
  let emphasizeInteraction = true;
  let autoSynthesisIdentity = "";

  // reset tab only when the selected item identity changes
  $: {
    const identity = particle
      ? `particle:${particle.id}`
      : isotope
        ? `isotope:${isotope.symbol}-${isotope.A}`
        : element
          ? `element:${element.symbol}`
        : "";
    if (identity !== activeIdentity) {
      activeIdentity = identity;
      mode = "schematic";
      selectedDecayIndex = 0;
    }
  }

  $: currentElement = isotope?.element ?? element;

  $: {
    const nextIdentity = isotope
      ? `mol:${isotope.symbol}-${isotope.A}`
      : currentElement
        ? `mol:${currentElement.symbol}`
        : "";
    if (nextIdentity && nextIdentity !== autoSynthesisIdentity) {
      autoSynthesisIdentity = nextIdentity;
      if (currentElement && moleculeFormula.trim() === "H2O") {
        moleculeFormula = currentElement.symbol === "H" ? "H2" : `${currentElement.symbol}2`;
      }
    }
  }

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

  async function runSynthesis(): Promise<void> {
    synthesisBusy = true;
    synthesisError = "";
    try {
      synthesisResult = await synthesizeMoleculeIsotopes(moleculeFormula);
    } catch (error) {
      synthesisResult = null;
      synthesisError = error instanceof Error ? error.message : "Unable to synthesize isotopes.";
    } finally {
      synthesisBusy = false;
    }
  }

  function elementPhaseHint(el: ElementData): string {
    if (["H", "N", "O", "F", "Cl", "He", "Ne", "Ar", "Kr", "Xe", "Rn"].includes(el.symbol)) return "gas";
    if (["Br", "Hg"].includes(el.symbol)) return "liquid";
    return "solid";
  }

  function elementBlock(el: ElementData): string {
    if (el.group == null) return "f";
    if (el.group <= 2) return "s";
    if (el.group <= 12) return "d";
    return "p";
  }

  function selectSynthesisIsotope(symbol: string, A: number): void {
    if (!synthesisResult) return;
    const rec = synthesisResult.recommendations.find((r) => r.symbol === symbol);
    if (!rec) return;
    const isotopeChoice = rec.allCandidates.find((iso) => iso.A === A) ?? null;
    if (!isotopeChoice) return;
    dispatch("isotopeSynthesisSelect", {
      Z: rec.element.Z,
      A: isotopeChoice.A,
      symbol: rec.element.symbol,
      name: rec.element.name,
      isotopeData: isotopeChoice,
      element: rec.element,
    });
  }

  $: composite   = particle ? resolveCompositeState(particle) : null;
  $: valence     = composite ? assignColorSinglet(composite.valence) : [];
  $: decayChannels = particle ? resolveDecayChannels(particle) : [];
  $: selectedDecayIndex = Math.min(selectedDecayIndex, Math.max(0, decayChannels.length - 1));
  $: dominantDecay = decayChannels[selectedDecayIndex] ?? null;
  $: qn = particle?.quantum_numbers;
  $: jpc = qn
    ? `${toSpinLabel(qn.spin)}^{${qn.parity === "Even" ? "+" : "-"}${qn.charge_conjugation === "Even" ? "+" : qn.charge_conjugation === "Odd" ? "-" : "?"}}`
    : "n/a";
  $: isospin = qn ? `${(qn.weak_isospin / 2).toFixed(1)}` : "n/a";

  // Nuclear-mode derived values
  $: isoN = isotope ? isotope.A - isotope.Z : 0;
  $: isoBA = isotope ? bindingEnergyPerNucleon(isotope.Z, isotope.A) : 0;
  $: isoHL = isotope ? formatHalfLife(isotope.isotopeData.half_life_s) : "";
  $: isotopeDecayArg = isotope
    ? { Z: isotope.Z, A: isotope.A, symbol: isotope.symbol, isotope: isotope.isotopeData }
    : null;

  // Display name / symbol for the header
  $: displaySymbol = particle
    ? particle.symbol
    : isotope
      ? `${isotope.symbol}-${isotope.A}`
      : currentElement
        ? currentElement.symbol
        : "";
  $: displayName   = particle
    ? `${particle.name} · ${particle.id}`
    : isotope
      ? `${isotope.name}, Z=${isotope.Z}`
      : currentElement
        ? `${currentElement.name} · Z=${currentElement.Z}`
        : "";
  $: isotopeStabilityTag = isotope
    ? isotope.isotopeData.half_life_s === null
      ? "stable"
      : isotope.isotopeData.half_life_s > 3.156e7
        ? "long-lived"
        : "radioactive"
    : "";
  $: synthesisTokens = synthesisResult?.tokens.map((token) => ({ symbol: token.symbol, count: token.count })) ?? [];
</script>

<div class="viewer">
  <header class="viewer-header">
    <div class="title-block">
      <strong>{displaySymbol}</strong>
      <span>{displayName}</span>
    </div>
    <div class="mode-tabs">
      <button class:active={mode === "schematic"} on:click={() => (mode = "schematic")}>Schematic</button>
      {#if particle}
        <button class:active={mode === "quantum"} on:click={() => (mode = "quantum")}>Quantum Numbers</button>
        <button class:active={mode === "feynman"} on:click={() => (mode = "feynman")}>Feynman Vertex</button>
      {:else if isotope || currentElement}
        <button class:active={mode === "quantum"} on:click={() => (mode = "quantum")}>{isotope ? "Nuclear Data" : "Element Data"}</button>
        <button class:active={mode === "feynman"} on:click={() => (mode = "feynman")}>Molecule Lab</button>
      {/if}
    </div>
  </header>

  {#if isotope || currentElement}
    <!-- ── Nuclear / isotope mode ─────────────────────────── -->
    {#if mode === "schematic"}
      {#if isotope}
        <CompositionSvg
          mode="nucleus"
          protons={isotope.Z}
          neutrons={isoN}
          label={`${isotope.symbol}-${isotope.A}: ${isotope.Z}p ${isoN}n`}
          valence={[]}
        />
        <section class="decay-section">
          <h5>Nuclear Decay Channels</h5>
          <DecayTree isotopeDecay={isotopeDecayArg} />
        </section>
      {/if}

      {#if currentElement}
        <section class="element-overview">
          <h5>Selected Element</h5>
          <div class="overview-grid">
            <div><span>Symbol</span><strong>{currentElement.symbol}</strong></div>
            <div><span>Name</span><strong>{currentElement.name}</strong></div>
            <div><span>Z</span><strong>{currentElement.Z}</strong></div>
            <div><span>Atomic mass</span><strong>{currentElement.atomic_mass.toFixed(3)}</strong></div>
            <div><span>Block</span><strong>{elementBlock(currentElement)}-block</strong></div>
            <div><span>Phase @ STP</span><strong>{elementPhaseHint(currentElement)}</strong></div>
          </div>
        </section>
        <MoleculePreview3D
          formula={synthesisResult?.formula ?? moleculeFormula}
          tokens={synthesisTokens}
          highlightSymbol={currentElement.symbol}
        />
      {/if}
    {:else if mode === "quantum"}
      <section class="qn-card">
        {#if isotope}
          <div><span>Nuclide</span><strong>{isotope.symbol}-{isotope.A}</strong></div>
          <div><span>Z (protons)</span><strong>{isotope.Z}</strong></div>
          <div><span>N (neutrons)</span><strong>{isoN}</strong></div>
          <div><span>A (mass number)</span><strong>{isotope.A}</strong></div>
          <div><span>Spin / Parity</span><strong>{isotope.isotopeData.spin_parity ?? "n/a"}</strong></div>
          <div><span>Half-life</span><strong>{isoHL}</strong></div>
          <div><span>B/A (MeV)</span><strong>{isoBA.toFixed(3)}</strong></div>
          <div><span>Abundance</span><strong>{isotope.isotopeData.abundance_percent != null ? `${isotope.isotopeData.abundance_percent}%` : "n/a"}</strong></div>
          <div><span>Mass excess (keV)</span><strong>{isotope.isotopeData.mass_excess_kev?.toFixed(1) ?? "n/a"}</strong></div>
          <div><span>Stability class</span><strong>{isotopeStabilityTag}</strong></div>
        {/if}
        {#if currentElement}
          <div><span>Category</span><strong>{currentElement.category}</strong></div>
          <div><span>Period / Group</span><strong>{currentElement.period} / {currentElement.group ?? "f-block"}</strong></div>
          <div><span>Electron config</span><strong class="mono">{currentElement.electron_configuration}</strong></div>
          <div><span>Atomic mass</span><strong>{currentElement.atomic_mass.toFixed(4)}</strong></div>
          <div><span>Block</span><strong>{elementBlock(currentElement)}-block</strong></div>
          <div><span>Phase @ STP</span><strong>{elementPhaseHint(currentElement)}</strong></div>
        {/if}
      </section>
      {#if isotope}
        <section class="isotope-preview">
          <h5>Selected Nuclide Preview</h5>
          <div class="preview-grid">
            <div class="preview-item">
              <span>Nuclide</span>
              <strong>{isotope.symbol}-{isotope.A}</strong>
            </div>
            <div class="preview-item">
              <span>N/Z ratio</span>
              <strong>{(isoN / Math.max(1, isotope.Z)).toFixed(3)}</strong>
            </div>
            <div class="preview-item preview-wide">
              <span>Mass excess</span>
              <strong>{isotope.isotopeData.mass_excess_kev?.toFixed(1) ?? "n/a"} keV</strong>
            </div>
          </div>
          <div class="nucleon-bar" role="img" aria-label="proton-neutron composition">
            <span class="bar proton" style={`width:${(isotope.Z / Math.max(1, isotope.A)) * 100}%`}>p {isotope.Z}</span>
            <span class="bar neutron" style={`width:${(isoN / Math.max(1, isotope.A)) * 100}%`}>n {isoN}</span>
          </div>
        </section>
      {/if}
    {:else}
      <section class="synthesis-card">
        <h5>Isotope synthesis from molecule</h5>
        <div class="synthesis-controls">
          <input
            type="text"
            bind:value={moleculeFormula}
            class="synthesis-input"
            placeholder="Molecule formula, e.g. C6H12O6"
          />
          <button on:click={runSynthesis} disabled={synthesisBusy}>
            {synthesisBusy ? "Synthesizing…" : "Synthesize"}
          </button>
        </div>
        {#if synthesisError}
          <p class="error-msg">{synthesisError}</p>
        {/if}
        {#if synthesisResult}
          <div class="synth-summary">
            <span>Natural mass estimate A≈{synthesisResult.estimatedNaturalMass.toFixed(1)}</span>
            <span>Enriched mass estimate A≈{synthesisResult.estimatedEnrichedMass.toFixed(1)}</span>
          </div>
          <MoleculePreview3D
            formula={synthesisResult.formula}
            tokens={synthesisTokens}
            highlightSymbol={currentElement?.symbol ?? null}
          />
          <ul class="synth-list">
            {#each synthesisResult.recommendations as rec (`${rec.symbol}-${rec.count}`)}
              <li>
                <div class="synth-row">
                  <strong>{rec.symbol}<sub>{rec.count}</sub></strong>
                  <span>{rec.element.name}</span>
                  <span>
                    natural: {rec.natural ? `${rec.symbol}-${rec.natural.A}` : "n/a"}
                    · enriched: {rec.enriched ? `${rec.symbol}-${rec.enriched.A}` : "n/a"}
                  </span>
                </div>
                <div class="synth-actions">
                  {#if rec.natural}
                    <button on:click={() => selectSynthesisIsotope(rec.symbol, rec.natural!.A)}>Use {rec.symbol}-{rec.natural.A}</button>
                  {/if}
                  {#if rec.enriched && rec.enriched?.A !== rec.natural?.A}
                    <button on:click={() => selectSynthesisIsotope(rec.symbol, rec.enriched!.A)}>Use {rec.symbol}-{rec.enriched.A}</button>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}

  {:else if particle}
    <!-- ── Standard hadronic / EW particle mode ──────────── -->
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
        <div><span>Electric charge Q</span><strong>{qn?.electric_charge}</strong></div>
        <div><span>Baryon number B</span><strong>{qn?.baryon_number}</strong></div>
        <div><span>Lepton numbers</span><strong>{qn?.lepton_numbers.electron}/{qn?.lepton_numbers.muon}/{qn?.lepton_numbers.tau}</strong></div>
        <div><span>Color rep</span><strong>{qn?.color}</strong></div>
        <div><span>Weak isospin T3</span><strong>{qn?.weak_isospin}</strong></div>
        <div><span>Hypercharge Y</span><strong>{qn?.hypercharge}</strong></div>
      </section>
    {:else}
      <section class="feynman">
        <div class="feynman-controls">
          <label>
            Channel
            <select bind:value={selectedDecayIndex}>
              {#if decayChannels.length === 0}
                <option value={0}>No decay presets</option>
              {:else}
                {#each decayChannels as ch, idx (`${ch.finalStateIds.join("+")}-${idx}`)}
                  <option value={idx}>{ch.finalStateIds.join(" + ")} ({(ch.branchingRatio * 100).toFixed(2)}%)</option>
                {/each}
              {/if}
            </select>
          </label>
          <label>
            Layout
            <select bind:value={vertexLayout}>
              <option value="split">Split</option>
              <option value="s-channel">s-channel</option>
              <option value="t-channel">t-channel</option>
            </select>
          </label>
          <label class="check"><input type="checkbox" bind:checked={showCoupling} /> Coupling text</label>
          <label class="check"><input type="checkbox" bind:checked={emphasizeInteraction} /> Emphasize interaction</label>
        </div>
        <svg viewBox="0 0 520 210" role="img" aria-label="Dominant vertex diagram">
          {#if vertexLayout === "split"}
            <line class="line" x1="48" y1="105" x2="250" y2="105" />
            <line class="line" x1="250" y1="105" x2="440" y2="56" />
            <line class="line" x1="250" y1="105" x2="440" y2="154" />
          {:else if vertexLayout === "s-channel"}
            <line class="line" x1="48" y1="62" x2="210" y2="105" />
            <line class="line" x1="48" y1="148" x2="210" y2="105" />
            <line class="line" x1="210" y1="105" x2="440" y2="105" />
          {:else}
            <line class="line" x1="48" y1="56" x2="250" y2="56" />
            <line class="line" x1="250" y1="56" x2="440" y2="154" />
            <line class="line" x1="48" y1="154" x2="250" y2="154" />
          {/if}
          <text class="lbl" x="30" y="105">{particle.symbol}</text>
          <circle
            class="vertex"
            class:vertex-strong={emphasizeInteraction && dominantDecay?.interaction === "strong"}
            class:vertex-weak={emphasizeInteraction && dominantDecay?.interaction === "weak"}
            class:vertex-em={emphasizeInteraction && dominantDecay?.interaction === "em"}
            cx="250"
            cy={vertexLayout === "s-channel" ? 105 : vertexLayout === "t-channel" ? 56 : 105}
            r="4.8"
          />

          {#if dominantDecay}
            <text class="lbl" x="452" y="56">{dominantDecay.finalStateIds[0] ?? "f1"}</text>
            <text class="lbl" x="452" y="154">{dominantDecay.finalStateIds[1] ?? "f2"}</text>
            {#if showCoupling}
              <text class="coupling" x="260" y="94">
                g_{dominantDecay.interaction} · BR={(dominantDecay.branchingRatio * 100).toFixed(2)}%
              </text>
            {/if}
          {:else}
            <text class="lbl" x="452" y="56">f₁</text>
            <text class="lbl" x="452" y="154">f₂</text>
            {#if showCoupling}
              <text class="coupling" x="260" y="94">g_eff</text>
            {/if}
          {/if}
        </svg>
        <p class="hint">Minimal effective vertex for the dominant listed process.</p>
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

  .isotope-preview,
  .element-overview,
  .synthesis-card {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .element-overview h5,
  .isotope-preview h5,
  .synthesis-card h5 {
    margin: 0 0 0.35rem;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.28rem 0.65rem;
  }

  .overview-grid > div {
    display: flex;
    justify-content: space-between;
    gap: 0.35rem;
    border-bottom: 1px dashed rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.22);
    padding-bottom: 0.14rem;
  }

  .overview-grid span,
  .overview-grid strong {
    font-family: var(--font-mono);
    font-size: 0.68rem;
  }

  .overview-grid span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .overview-grid strong {
    color: var(--color-text-primary, var(--fg-primary));
    text-align: right;
  }

  .preview-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.3rem;
  }

  .preview-item {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    font-family: var(--font-mono);
    font-size: 0.67rem;
  }

  .preview-item span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .preview-item strong {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .preview-wide {
    grid-column: 1 / -1;
  }

  .nucleon-bar {
    margin-top: 0.4rem;
    display: flex;
    height: 1.2rem;
    border: 1px solid var(--color-border, var(--border));
    overflow: hidden;
  }

  .bar {
    font-family: var(--font-mono);
    font-size: 0.62rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    white-space: nowrap;
  }

  .bar.proton {
    background: color-mix(in srgb, #ff5f56 70%, var(--color-bg-inset, var(--bg-inset)));
  }

  .bar.neutron {
    background: color-mix(in srgb, #8e8e93 70%, var(--color-bg-inset, var(--bg-inset)));
  }

  .synthesis-controls {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.25rem;
  }

  .synthesis-input,
  .feynman-controls select,
  .feynman-controls input {
    background: var(--color-bg-surface, var(--bg-surface));
    border: 1px solid var(--color-border, var(--border));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.67rem;
    padding: 0.18rem 0.28rem;
  }

  .synthesis-controls button,
  .synth-actions button {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.64rem;
    padding: 0.17rem 0.35rem;
    cursor: pointer;
  }

  .error-msg {
    margin: 0.35rem 0 0;
    color: var(--hl-error);
    font-family: var(--font-mono);
    font-size: 0.66rem;
  }

  .synth-summary {
    margin-top: 0.35rem;
    display: flex;
    gap: 0.6rem;
    flex-wrap: wrap;
    font-family: var(--font-mono);
    font-size: 0.64rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .synth-list {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .synth-list li {
    border: 1px solid var(--color-border, var(--border));
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    gap: 0.24rem;
  }

  .synth-row {
    display: grid;
    gap: 0.16rem;
    font-family: var(--font-mono);
    font-size: 0.64rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .synth-row strong {
    color: var(--color-accent, var(--hl-symbol));
    font-size: 0.72rem;
  }

  .synth-actions {
    display: flex;
    gap: 0.22rem;
    flex-wrap: wrap;
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

  .feynman-controls {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.25rem 0.45rem;
    margin-bottom: 0.25rem;
    font-family: var(--font-mono);
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

  .vertex-strong {
    fill: var(--color-success, var(--hl-success));
  }

  .vertex-weak {
    fill: var(--hl-value);
  }

  .vertex-em {
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

  .mono {
    font-family: var(--font-mono);
    font-size: 0.65rem;
    word-break: break-all;
  }
</style>
