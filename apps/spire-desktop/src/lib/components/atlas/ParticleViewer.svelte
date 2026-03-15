<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Field } from "$lib/types/spire";
  import {
    resolveCompositeState,
    assignColorSinglet,
    resolveDecayChannels,
    type CompositeQuarkContent,
    type DecayChannelPreset,
  } from "$lib/core/physics/composites";
  import type { IsotopeData, ElementData } from "$lib/core/physics/nuclearDataLoader";
  import { formatHalfLife, bindingEnergyPerNucleon } from "$lib/core/physics/nuclearDataLoader";
  import {
    synthesizeMoleculeIsotopes,
    type MoleculeSynthesisResult,
    type MoleculeIsotopologueProfile,
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
  export let referenceComposite: CompositeQuarkContent | null = null;
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
  let selectedIsotopologueId = "natural";

  let selectedDecayIndex = 0;
  let vertexLayout: "s-channel" | "t-channel" | "split" = "split";
  let showCoupling = true;
  let emphasizeInteraction = true;
  let showMomentumLabels = true;
  let showArrowheads = true;
  let showInteractionLegend = true;
  let incomingLegs: "single" | "pair" = "single";
  let finalArrangement: "fan" | "balanced" = "fan";
  let lineFlavor: "fermion" | "boson" | "scalar" = "fermion";
  let labelMode: "symbol" | "id" | "pretty" = "pretty";
  let propagatorStyle: "decay" | "exchange" | "contact" = "decay";
  let fieldSchematicStyle: "profile" | "orbitals" | "charge-map" = "profile";
  let autoSynthesisIdentity = "";

  // reset tab only when the selected item identity changes
  $: {
    const identity = particle
      ? `particle:${particle.id}`
      : referenceComposite
        ? `reference:${referenceComposite.label}`
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
      selectedIsotopologueId = synthesisResult.isotopologues[0]?.id ?? "natural";
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

  function selectIsotopologue(profileId: string): void {
    selectedIsotopologueId = profileId;
  }

  function prettyParticleId(id: string): string {
    return id.replace(/_/g, " ").replace(/\bbar\b/g, "bar");
  }

  function fieldNodeColor(label: string): string {
    if (label.includes("Q")) return "var(--hl-value)";
    if (label.includes("spin")) return "var(--hl-symbol)";
    if (label.includes("color")) return "var(--hl-success)";
    return "#c792ea";
  }

  function branchLabel(id: string): string {
    if (labelMode === "id") return id;
    if (labelMode === "symbol") return id.replace(/_bar$/, "̅");
    return prettyParticleId(id);
  }

  function chargeDescriptor(charge: number): string {
    if (Math.abs(charge) < 1e-12) return "neutral";
    return charge > 0 ? "positive" : "negative";
  }

  $: composite   = particle ? resolveCompositeState(particle) : null;
  $: activeComposite = composite ?? referenceComposite;
  $: valence     = activeComposite ? assignColorSinglet(activeComposite.valence) : [];
  $: decayChannels = particle ? resolveDecayChannels(particle) : referenceComposite ? resolveDecayChannels(referenceComposite.particleIds[0]) : [];
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
    : referenceComposite
      ? referenceComposite.label
    : isotope
      ? `${isotope.symbol}-${isotope.A}`
      : currentElement
        ? currentElement.symbol
        : "";
  $: displayName   = particle
    ? `${particle.name} · ${particle.id}`
    : referenceComposite
      ? `${referenceComposite.kind} reference state · ${referenceComposite.particleIds[0]}`
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
  $: selectedIsotopologue = synthesisResult?.isotopologues.find((profile) => profile.id === selectedIsotopologueId) ?? synthesisResult?.isotopologues[0] ?? null;
  $: previewTokens = selectedIsotopologue
    ? selectedIsotopologue.components.map((component) => ({
        symbol: component.symbol,
        count: component.count,
        isotopeA: component.isotope?.A ?? null,
      }))
    : synthesisTokens;
  $: orbitNodes = particle
    ? [
        { label: `Q ${particle.quantum_numbers.electric_charge}`, x: 62, y: 44 },
        { label: `spin ${toSpinLabel(particle.quantum_numbers.spin)}`, x: 170, y: 36 },
        { label: `color ${particle.quantum_numbers.color}`, x: 208, y: 112 },
        { label: `T3 ${particle.quantum_numbers.weak_isospin}`, x: 142, y: 172 },
        { label: `${particle.interactions.length} int.`, x: 54, y: 154 },
      ]
    : [];
  $: feynmanFinalStateIds = dominantDecay?.finalStateIds?.length ? dominantDecay.finalStateIds : ["f1", "f2"];
  $: feynmanFinalPoints = feynmanFinalStateIds.map((_, index) => {
    const count = feynmanFinalStateIds.length;
    const startY = finalArrangement === "fan" ? 34 : 56;
    const endY = finalArrangement === "fan" ? 176 : 154;
    const y = count === 1 ? 105 : startY + (index * (endY - startY)) / Math.max(1, count - 1);
    return { x: 446, y };
  });
  $: mediatorLabel = dominantDecay
    ? propagatorStyle === "contact"
      ? `${dominantDecay.interaction} contact`
      : `${particle?.symbol ?? "V"}`
    : particle?.symbol ?? "V";
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
          tokens={previewTokens}
          highlightSymbol={currentElement.symbol}
          subtitle={selectedIsotopologue ? `${selectedIsotopologue.name} · A≈${selectedIsotopologue.estimatedMass.toFixed(1)}` : "Element-centered molecular scene"}
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
          <div class="isotopologue-grid">
            {#each synthesisResult.isotopologues as profile (profile.id)}
              <button
                class="isotopologue-card"
                class:isotopologue-card--active={selectedIsotopologue?.id === profile.id}
                on:click={() => selectIsotopologue(profile.id)}
              >
                <strong>{profile.name}</strong>
                <span>{profile.description}</span>
                <em>A≈{profile.estimatedMass.toFixed(1)}</em>
              </button>
            {/each}
          </div>
          <MoleculePreview3D
            formula={synthesisResult.formula}
            tokens={previewTokens}
            highlightSymbol={currentElement?.symbol ?? null}
            subtitle={selectedIsotopologue ? `${selectedIsotopologue.name} · ${selectedIsotopologue.description}` : "Isotopologue preview"}
          />
          {#if selectedIsotopologue}
            <section class="isotopologue-details">
              <h5>Molecule-level isotope synthesis</h5>
              <div class="isotopologue-components">
                {#each selectedIsotopologue.components as component (`${selectedIsotopologue.id}-${component.symbol}`)}
                  <article>
                    <strong>{#if component.isotope?.A}<sup>{component.isotope.A}</sup>{/if}{component.symbol}<sub>{component.count}</sub></strong>
                    <span>{component.element.name}</span>
                    <em>{component.isotope ? `${component.symbol}-${component.isotope.A}` : "natural mass proxy"}</em>
                    {#if component.isotope}
                      <button on:click={() => selectSynthesisIsotope(component.symbol, component.isotope!.A)}>
                        Inspect {component.symbol}-{component.isotope.A}
                      </button>
                    {/if}
                  </article>
                {/each}
              </div>
            </section>
          {/if}
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

  {:else if particle || referenceComposite}
    <!-- ── Standard hadronic / EW particle mode ──────────── -->
    {#if mode === "schematic"}
      {#if activeComposite}
        <CompositionSvg valence={valence} label={`${activeComposite.label}: ${activeComposite.valence.join(" ")}`} />
      {:else}
        <section class="field-schematic">
          <div class="field-schematic-header">
            <h5>Field Profile Schematic</h5>
            <select bind:value={fieldSchematicStyle}>
              <option value="profile">Profile</option>
              <option value="orbitals">Orbit map</option>
              <option value="charge-map">Charge map</option>
            </select>
          </div>
          <svg viewBox="0 0 260 210" role="img" aria-label={`Field profile schematic for ${particle?.name ?? 'selected field'}`}>
            <defs>
              <radialGradient id="field-core" cx="35%" cy="30%">
                <stop offset="0%" stop-color="rgba(255,255,255,0.92)" />
                <stop offset="100%" stop-color="rgba(255,255,255,0)" />
              </radialGradient>
            </defs>
            {#if fieldSchematicStyle !== "charge-map"}
              <circle class="orbit" cx="130" cy="104" r="64" />
              <circle class="orbit orbit-secondary" cx="130" cy="104" r="88" />
            {/if}
            {#each orbitNodes as node, index (`${node.label}-${index}`)}
              <line class="field-link" x1="130" y1="104" x2={node.x} y2={node.y} />
              <circle class="field-node" cx={node.x} cy={node.y} r={fieldSchematicStyle === "charge-map" ? 18 : 14} style={`--node-accent:${fieldNodeColor(node.label)}`} />
              <text class="field-node-label" x={node.x} y={node.y}>{node.label}</text>
            {/each}
            <circle class="field-core" cx="130" cy="104" r="28" />
            <circle class="field-core-shine" cx="130" cy="104" r="26" />
            <text class="field-core-label" x="130" y="103">{particle?.symbol ?? "?"}</text>
            <text class="field-core-sub" x="130" y="122">{chargeDescriptor(particle?.quantum_numbers.electric_charge ?? 0)}</text>
          </svg>
          <div class="field-summary-grid">
            <div><span>Interactions</span><strong>{particle?.interactions.join(", ") || "n/a"}</strong></div>
            <div><span>Weak multiplet</span><strong>{particle ? (typeof particle.quantum_numbers.weak_multiplet === "string" ? particle.quantum_numbers.weak_multiplet : `Triplet(${particle.quantum_numbers.weak_multiplet.Triplet})`) : "n/a"}</strong></div>
            <div><span>Color rep</span><strong>{particle?.quantum_numbers.color ?? "n/a"}</strong></div>
            <div><span>Parity</span><strong>{particle?.quantum_numbers.parity ?? "n/a"}</strong></div>
          </div>
        </section>
      {/if}

      <section class="decay-section">
        <h5>Decay Modes</h5>
        <DecayTree
          parentId={particle?.id ?? referenceComposite?.particleIds[0] ?? "reference"}
          channels={decayChannels}
          on:select={emitDecaySelect}
        />
      </section>
    {:else if mode === "quantum"}
      {#if particle}
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
        <div><span>Interactions</span><strong class="mono">{particle.interactions.join(", ") || "n/a"}</strong></div>
        <div><span>Weak multiplet</span><strong>{typeof qn?.weak_multiplet === "string" ? qn?.weak_multiplet : qn ? `Triplet(${qn.weak_multiplet.Triplet})` : "n/a"}</strong></div>
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
          <label>
            Incoming legs
            <select bind:value={incomingLegs}>
              <option value="single">Single</option>
              <option value="pair">Pair</option>
            </select>
          </label>
          <label>
            Final arrangement
            <select bind:value={finalArrangement}>
              <option value="fan">Fan</option>
              <option value="balanced">Balanced</option>
            </select>
          </label>
          <label>
            Propagator
            <select bind:value={propagatorStyle}>
              <option value="decay">Decay</option>
              <option value="exchange">Exchange</option>
              <option value="contact">Contact</option>
            </select>
          </label>
          <label>
            Line flavor
            <select bind:value={lineFlavor}>
              <option value="fermion">Fermion-like</option>
              <option value="boson">Boson-like</option>
              <option value="scalar">Scalar-like</option>
            </select>
          </label>
          <label>
            Labels
            <select bind:value={labelMode}>
              <option value="pretty">Pretty</option>
              <option value="symbol">Symbolic</option>
              <option value="id">Raw id</option>
            </select>
          </label>
          <label class="check"><input type="checkbox" bind:checked={showCoupling} /> Coupling text</label>
          <label class="check"><input type="checkbox" bind:checked={emphasizeInteraction} /> Emphasize interaction</label>
          <label class="check"><input type="checkbox" bind:checked={showMomentumLabels} /> Momentum labels</label>
          <label class="check"><input type="checkbox" bind:checked={showArrowheads} /> Flow arrows</label>
          <label class="check"><input type="checkbox" bind:checked={showInteractionLegend} /> Interaction legend</label>
        </div>
        <svg viewBox="0 0 520 210" role="img" aria-label="Dominant vertex diagram">
          <defs>
            <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="7" refY="3" orient="auto">
              <path d="M0,0 L8,3 L0,6 z" fill="currentColor"></path>
            </marker>
          </defs>
          {#if vertexLayout === "split"}
            {#if incomingLegs === "pair"}
              <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="82" x2="250" y2="104" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
              <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="128" x2="250" y2="106" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            {:else}
              <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="105" x2="250" y2="105" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            {/if}
            {#each feynmanFinalPoints as point}
              <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="250" y1="105" x2={point.x} y2={point.y} marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            {/each}
          {:else if vertexLayout === "s-channel"}
            <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="62" x2="210" y2="105" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="148" x2="210" y2="105" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="210" y1="105" x2="440" y2="105" class:line--mediator={propagatorStyle !== 'contact'} />
          {:else}
            <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="56" x2="250" y2="56" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="48" y1="154" x2="250" y2="154" marker-end={showArrowheads ? 'url(#arrowhead)' : undefined} />
            {#each feynmanFinalPoints as point}
              <line class="line" class:line--boson={lineFlavor === 'boson'} class:line--scalar={lineFlavor === 'scalar'} x1="250" y1="56" x2={point.x} y2={point.y} class:line--mediator={propagatorStyle !== 'contact'} />
            {/each}
          {/if}
          <text class="lbl" x="30" y="105">{branchLabel(particle?.id ?? referenceComposite?.particleIds[0] ?? "state")}</text>
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
            {#each feynmanFinalStateIds as stateId, idx (`${stateId}-${idx}`)}
              <text class="lbl" x="456" y={feynmanFinalPoints[idx]?.y ?? 105}>{branchLabel(stateId)}</text>
            {/each}
            {#if showCoupling}
              <text class="coupling" x="260" y="94">
                g_{dominantDecay.interaction} · BR={(dominantDecay.branchingRatio * 100).toFixed(2)}%
              </text>
            {/if}
            <text class="mediator" x="260" y="118">{mediatorLabel}</text>
          {:else}
            <text class="lbl" x="452" y="56">f₁</text>
            <text class="lbl" x="452" y="154">f₂</text>
            {#if showCoupling}
              <text class="coupling" x="260" y="94">g_eff</text>
            {/if}
          {/if}
          {#if showMomentumLabels}
            <text class="momentum" x="122" y="92">p_in</text>
            {#each feynmanFinalPoints as point, idx (`momentum-${idx}`)}
              <text class="momentum" x={point.x - 32} y={point.y - 10}>p_{idx + 1}</text>
            {/each}
          {/if}
        </svg>
          {#if showInteractionLegend && dominantDecay}
          <div class="feynman-legend">
            <span><i class="legend-dot legend-dot--interaction"></i>{dominantDecay.interaction} interaction</span>
            <span><i class="legend-dot legend-dot--branch"></i>BR {(dominantDecay.branchingRatio * 100).toFixed(2)}%</span>
            <span><i class="legend-dot legend-dot--layout"></i>{vertexLayout} / {propagatorStyle}</span>
          </div>
        {/if}
        {#if dominantDecay}
          <div class="channel-stats">
            <div><span>Parent</span><strong>{particle?.id ?? referenceComposite?.particleIds[0] ?? "state"}</strong></div>
            <div><span>Final multiplicity</span><strong>{feynmanFinalStateIds.length}</strong></div>
            <div><span>Interaction</span><strong>{dominantDecay.interaction}</strong></div>
            <div><span>Branching ratio</span><strong>{(dominantDecay.branchingRatio * 100).toFixed(3)}%</strong></div>
          </div>
        {/if}
        <p class="hint">Minimal effective vertex for the dominant listed process{referenceComposite ? " in the atlas reference library" : ""}.</p>
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

  .isotopologue-grid {
    margin-top: 0.4rem;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 0.3rem;
  }

  .isotopologue-card {
    display: flex;
    flex-direction: column;
    gap: 0.16rem;
    align-items: flex-start;
    text-align: left;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    color: var(--color-text-primary, var(--fg-primary));
    padding: 0.34rem 0.4rem;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  .isotopologue-card strong {
    font-size: 0.68rem;
  }

  .isotopologue-card span,
  .isotopologue-card em {
    font-size: 0.61rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .isotopologue-card--active {
    border-color: var(--color-accent, var(--hl-symbol));
    background: color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 10%, var(--color-bg-surface, var(--bg-surface)));
  }

  .isotopologue-details {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
    margin-top: 0.35rem;
  }

  .isotopologue-details h5,
  .field-schematic h5 {
    margin: 0 0 0.35rem;
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-text-primary, var(--fg-primary));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .isotopologue-components {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 0.32rem;
  }

  .isotopologue-components article {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-surface, var(--bg-surface));
    padding: 0.3rem;
    display: flex;
    flex-direction: column;
    gap: 0.14rem;
    font-family: var(--font-mono);
  }

  .isotopologue-components strong {
    color: var(--color-accent, var(--hl-symbol));
  }

  .isotopologue-components span,
  .isotopologue-components em {
    font-size: 0.62rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .isotopologue-components button {
    margin-top: 0.1rem;
    align-self: flex-start;
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.61rem;
    padding: 0.16rem 0.3rem;
    cursor: pointer;
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

  .field-schematic {
    border: 1px solid var(--color-border, var(--border));
    background: var(--color-bg-inset, var(--bg-inset));
    padding: 0.45rem;
  }

  .field-schematic-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
    margin-bottom: 0.35rem;
  }

  .field-schematic-header select {
    background: var(--color-bg-surface, var(--bg-surface));
    border: 1px solid var(--color-border, var(--border));
    color: var(--color-text-primary, var(--fg-primary));
    font-family: var(--font-mono);
    font-size: 0.64rem;
    padding: 0.16rem 0.25rem;
  }

  .field-schematic svg {
    width: 100%;
    display: block;
    background: radial-gradient(circle at top, rgba(255,255,255,0.05), transparent 50%);
    border: 1px solid rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.15);
  }

  .orbit {
    fill: none;
    stroke: rgba(255,255,255,0.12);
    stroke-width: 1.2;
  }

  .orbit-secondary {
    stroke-dasharray: 5 4;
  }

  .field-link {
    stroke: rgba(255,255,255,0.16);
    stroke-width: 1.2;
  }

  .field-node {
    fill: color-mix(in srgb, var(--node-accent) 72%, var(--color-bg-inset, var(--bg-inset)));
    stroke: color-mix(in srgb, var(--node-accent) 84%, black);
    stroke-width: 1;
  }

  .field-node-label,
  .field-core-label,
  .field-core-sub {
    font-family: var(--font-mono);
    text-anchor: middle;
  }

  .field-node-label {
    font-size: 7px;
    fill: rgba(17, 20, 30, 0.9);
    dominant-baseline: middle;
  }

  .field-core {
    fill: color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 72%, var(--color-bg-inset, var(--bg-inset)));
    stroke: color-mix(in srgb, var(--color-accent, var(--hl-symbol)) 84%, black);
    stroke-width: 1.2;
  }

  .field-core-shine {
    fill: url(#field-core);
  }

  .field-core-label {
    font-size: 18px;
    font-weight: 700;
    fill: rgba(12, 14, 22, 0.9);
  }

  .field-core-sub {
    font-size: 7px;
    fill: rgba(12, 14, 22, 0.86);
  }

  .field-summary-grid {
    margin-top: 0.35rem;
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.28rem 0.5rem;
  }

  .field-summary-grid > div {
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
  }

  .field-summary-grid span,
  .field-summary-grid strong {
    font-family: var(--font-mono);
  }

  .field-summary-grid span {
    font-size: 0.58rem;
    color: var(--color-text-muted, var(--fg-secondary));
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .field-summary-grid strong {
    font-size: 0.67rem;
    color: var(--color-text-primary, var(--fg-primary));
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
    color: var(--color-text-primary, var(--fg-primary));
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

  .mediator,
  .momentum {
    font-family: var(--font-mono);
    fill: var(--color-text-muted, var(--fg-secondary));
  }

  .mediator {
    font-size: 10px;
  }

  .momentum {
    font-size: 9px;
  }

  .feynman-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem 0.8rem;
    margin-top: 0.25rem;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .channel-stats {
    margin-top: 0.25rem;
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
    font-family: var(--font-mono);
    font-size: 0.59rem;
  }

  .channel-stats span {
    color: var(--color-text-muted, var(--fg-secondary));
  }

  .channel-stats strong {
    color: var(--color-text-primary, var(--fg-primary));
  }

  .legend-dot {
    display: inline-block;
    width: 0.55rem;
    height: 0.55rem;
    margin-right: 0.22rem;
    border: 1px solid currentColor;
  }

  .legend-dot--interaction {
    color: var(--hl-symbol);
  }

  .legend-dot--branch {
    color: var(--hl-value);
  }

  .legend-dot--layout {
    color: var(--hl-success);
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
