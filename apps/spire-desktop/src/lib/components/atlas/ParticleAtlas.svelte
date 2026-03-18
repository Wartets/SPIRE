<script lang="ts">
  import { theoreticalModel } from "$lib/stores/physicsStore";
  import { categorizeParticles } from "$lib/core/physics/taxonomy";
  import type { Field } from "$lib/types/spire";
  import ParticleCell from "$lib/components/atlas/ParticleCell.svelte";
  import CustomParticleModal from "$lib/components/atlas/CustomParticleModal.svelte";
  import PeriodicTable from "$lib/components/atlas/PeriodicTable.svelte";
  import ParticleViewer from "$lib/components/atlas/ParticleViewer.svelte";
  import TaxonomyGuidePanel from "$lib/components/atlas/TaxonomyGuidePanel.svelte";
  import {
    atlasSelectionRequest,
    submitAtlasSelection,
    clearAtlasSelectionRequest,
  } from "$lib/stores/atlasSelectionStore";
  import { appendLog } from "$lib/stores/physicsStore";
  import { initialIdsInput, finalIdsInput } from "$lib/stores/workspaceInputsStore";
  import { broadcastSelection, selectionBus } from "$lib/stores/selectionBus";
  import {
    getElementByZ,
    type IsotopeData,
    type ElementData,
  } from "$lib/core/physics/nuclearDataLoader";
  import { listCompositeReferenceLibrary, type CompositeQuarkContent } from "$lib/core/physics/composites";
  import { listAtlasReferenceParticles } from "$lib/core/physics/atlasReferenceParticles";

  const logAtlas = (message: string): void => {
    appendLog(message, { category: "Atlas" });
  };

  type AtlasMode = "taxonomy" | "periodic";

  let mode: AtlasMode = "taxonomy";
  let customOpen = false;
  let selectedParticle: Field | null = null;
  let selectedReferenceComposite: CompositeQuarkContent | null = null;
  let flashId: string | null = null;
  let taxonomyQuery = "";
  let taxonomyFilter: "all" | "charged" | "neutral" | "colored" | "unstable" = "all";
  let taxonomySort: "mass" | "alpha" | "charge" = "mass";
  let referenceDomainFilter: "all" | "sm" | "hadron" | "bsm" = "all";
  let referenceInteractionFilter: "all" | "strong" | "weak" | "electromagnetic" | "yukawa" = "all";
  let taxonomyNavigatorOpen = false;
  let atlasControlsOpen = true;
  let recentElements: ElementData[] = [];
  let recentIsotopes: {
    Z: number; A: number; symbol: string; name: string;
    isotopeData: IsotopeData; element: ElementData;
  }[] = [];
  let selectedElementForViewer: ElementData | null = null;
  let selectedIsotopeForViewer: {
    Z: number; A: number; symbol: string; name: string;
    isotopeData: IsotopeData; element: ElementData;
  } | null = null;

  $: taxonomy = categorizeParticles($theoreticalModel);
  $: referenceComposites = listCompositeReferenceLibrary();
  $: atlasReferenceParticles = listAtlasReferenceParticles();
  $: visibleAtlasReferenceParticles = atlasReferenceParticles
    .filter((particle) => {
      const query = taxonomyQuery.trim().toLowerCase();
      const matchesQuery = !query || [particle.name, particle.id, particle.symbol].some((value) => value.toLowerCase().includes(query));
      if (!matchesQuery) return false;

      const domain = referenceDomainFor(particle);
      if (referenceDomainFilter !== "all" && domain !== referenceDomainFilter) return false;

      if (referenceInteractionFilter !== "all") {
        const target = referenceInteractionFilterToType(referenceInteractionFilter);
        return particle.interactions.includes(target);
      }

      return true;
    })
    .sort((a, b) => {
      if (taxonomySort === "alpha") return a.name.localeCompare(b.name) || a.id.localeCompare(b.id);
      if (taxonomySort === "charge") {
        return (
          Math.abs(b.quantum_numbers.electric_charge) - Math.abs(a.quantum_numbers.electric_charge) ||
          a.name.localeCompare(b.name)
        );
      }
      return Math.abs(a.mass) - Math.abs(b.mass) || a.name.localeCompare(b.name);
    });
  $: taxonomyFields = taxonomy.groups.flatMap((group) => group.buckets.flatMap((bucket) => bucket.particles));
  $: periodicSelectionLabel = selectedIsotopeForViewer
    ? `${selectedIsotopeForViewer.symbol}-${selectedIsotopeForViewer.A}`
    : selectedElementForViewer
      ? `${selectedElementForViewer.symbol} (${selectedElementForViewer.name})`
      : "none";
  $: referenceStats = {
    sm: atlasReferenceParticles.filter((particle) => referenceDomainFor(particle) === "sm").length,
    hadron: atlasReferenceParticles.filter((particle) => referenceDomainFor(particle) === "hadron").length,
    bsm: atlasReferenceParticles.filter((particle) => referenceDomainFor(particle) === "bsm").length,
  };
  $: taxonomyStats = {
    charged: taxonomyFields.filter((field) => Math.abs(field.quantum_numbers.electric_charge) > 1e-12).length,
    neutral: taxonomyFields.filter((field) => Math.abs(field.quantum_numbers.electric_charge) <= 1e-12).length,
    colored: taxonomyFields.filter((field) => field.quantum_numbers.color !== "Singlet").length,
    unstable: taxonomyFields.filter((field) => Number.isFinite(field.width) && field.width > 0).length,
    interactionRich: taxonomyFields.filter((field) => field.interactions.length >= 2).length,
  };
  $: taxonomyFilterLabel = {
    all: "all fields",
    charged: "charged fields",
    neutral: "neutral fields",
    colored: "colored states",
    unstable: "unstable states",
  }[taxonomyFilter];
  $: taxonomySortLabel = {
    mass: "mass ladder",
    alpha: "alphabetical",
    charge: "charge magnitude",
  }[taxonomySort];
  $: filteredTaxonomyGroups = taxonomy.groups
    .map((group) => ({
      ...group,
      buckets: group.buckets
        .map((bucket) => ({
          ...bucket,
          particles: [...bucket.particles]
            .filter((particle) => {
              const query = taxonomyQuery.trim().toLowerCase();
              const matchesQuery = !query || [particle.name, particle.id, particle.symbol].some((value) => value.toLowerCase().includes(query));
              if (!matchesQuery) return false;
              switch (taxonomyFilter) {
                case "charged":
                  return Math.abs(particle.quantum_numbers.electric_charge) > 1e-12;
                case "neutral":
                  return Math.abs(particle.quantum_numbers.electric_charge) <= 1e-12;
                case "colored":
                  return particle.quantum_numbers.color !== "Singlet";
                case "unstable":
                  return Number.isFinite(particle.width) && particle.width > 0;
                default:
                  return true;
              }
            })
            .sort((a, b) => {
              if (taxonomySort === "alpha") {
                return a.name.localeCompare(b.name) || a.id.localeCompare(b.id);
              }
              if (taxonomySort === "charge") {
                return (
                  Math.abs(b.quantum_numbers.electric_charge) - Math.abs(a.quantum_numbers.electric_charge) ||
                  a.name.localeCompare(b.name)
                );
              }
              return Math.abs(a.mass) - Math.abs(b.mass) || a.name.localeCompare(b.name);
            }),
        }))
        .filter((bucket) => bucket.particles.length > 0),
    }))
    .filter((group) => group.buckets.length > 0);

  // Subscribe to selectionBus to capture isotope selections from PeriodicTable
  $: {
    const payload = $selectionBus;
    if (payload?.type === "ISOTOPE_SELECTED" && payload.data) {
      const d = payload.data;
      const el = getElementByZ(d.Z);
      if (el) {
        selectedElementForViewer = el;
        selectedIsotopeForViewer = {
          Z: d.Z,
          A: d.A,
          symbol: d.symbol,
          name: d.name,
          isotopeData: d.isotope,
          element: el,
        };
        pushRecentElement(el);
        pushRecentIsotope({
          Z: d.Z,
          A: d.A,
          symbol: d.symbol,
          name: d.name,
          isotopeData: d.isotope,
          element: el,
        });
      }
    }
    if (payload?.type === "ELEMENT_SELECTED" && payload.data?.Z) {
      selectedElementForViewer = payload.data;
      selectedIsotopeForViewer = null;
      pushRecentElement(payload.data);
    }
  }

  function referenceDomainFor(field: Field): "sm" | "hadron" | "bsm" {
    const id = field.id.toLowerCase();
    if (["gamma", "g", "z", "w+", "w-", "h", "u", "d", "s", "c", "b", "t", "e-", "e+", "mu-", "tau-", "nu_e", "nu_mu", "nu_tau"].includes(id)) {
      return "sm";
    }
    if (["p", "n", "pi+", "pi0", "k+", "k0", "lambda0", "sigma+", "sigma0", "sigma-", "xi0", "xi-", "omega-", "rho0", "phi_meson", "j_psi", "upsilon", "x3872", "z_c3900", "theta_plus"].includes(id)) {
      return "hadron";
    }
    return "bsm";
  }

  function referenceInteractionFilterToType(filter: typeof referenceInteractionFilter): Field["interactions"][number] {
    switch (filter) {
      case "strong":
        return "Strong";
      case "weak":
        return "WeakNC";
      case "yukawa":
        return "Yukawa";
      default:
        return "Electromagnetic";
    }
  }

  function pushRecentElement(element: ElementData): void {
    recentElements = [element, ...recentElements.filter((entry) => entry.Z !== element.Z)].slice(0, 8);
  }

  function pushRecentIsotope(item: {
    Z: number; A: number; symbol: string; name: string;
    isotopeData: IsotopeData; element: ElementData;
  }): void {
    recentIsotopes = [
      item,
      ...recentIsotopes.filter((entry) => !(entry.Z === item.Z && entry.A === item.A)),
    ].slice(0, 8);
  }

  function triggerFlash(id: string): void {
    flashId = id;
    setTimeout(() => { if (flashId === id) flashId = null; }, 820);
  }

  function clearTaxonomySelection(): void {
    selectedParticle = null;
    selectedReferenceComposite = null;
  }

  function appendToReaction(target: "initial" | "final", id: string, label: string): void {
    if (target === "initial") {
      initialIdsInput.update((prev) => [...prev, id]);
    } else {
      finalIdsInput.update((prev) => [...prev, id]);
    }
    logAtlas(`${label} appended to Reaction Workspace ${target} state.`);
  }

  function handleParticleSelect(field: Field): void {
    if ($atlasSelectionRequest.pending && $atlasSelectionRequest.target) {
      submitAtlasSelection($atlasSelectionRequest.target, field.id);
      logAtlas(`Atlas picker selected ${field.id} for ${$atlasSelectionRequest.target} state.`);
      return;
    }
    selectedParticle = field;
    selectedReferenceComposite = null;
    broadcastSelection({ type: "PARTICLE_SELECTED", data: field });
    triggerFlash(field.id);
    logAtlas(`Atlas particle inspected: ${field.id}`);
  }

  function handleReferenceParticleSelect(field: Field): void {
    selectedParticle = field;
    selectedReferenceComposite = null;
    broadcastSelection({ type: "PARTICLE_SELECTED", data: field });
    triggerFlash(field.id);
    logAtlas(`Atlas reference particle inspected: ${field.id}`);
  }

  function handleReferenceCompositeSelect(entry: CompositeQuarkContent): void {
    selectedParticle = null;
    selectedReferenceComposite = entry;
    logAtlas(`Atlas reference composite inspected: ${entry.label}`);
  }

  function handleDecaySelect(
    event: CustomEvent<{ parentId: string; finalStateIds: string[]; label: string }>,
  ): void {
    const detail = event.detail;
    initialIdsInput.set([detail.parentId]);
    finalIdsInput.set([...detail.finalStateIds]);
    logAtlas(`Decay branch loaded into Reaction Workspace: ${detail.label}`);
  }

  function handleQuickAdd(
    event: CustomEvent<{ particle: Field; target: "initial" | "final" }>,
  ): void {
    if ($atlasSelectionRequest.pending) return;
    const { particle, target } = event.detail;
    appendToReaction(target, particle.id, `${particle.symbol} (${particle.id})`);
    triggerFlash(particle.id);
  }

  function handleViewerAddToReaction(
    event: CustomEvent<{ target: "initial" | "final"; id: string; label: string }>,
  ): void {
    appendToReaction(event.detail.target, event.detail.id, event.detail.label);
  }

  function handleSynthesisIsotopeSelect(
    event: CustomEvent<{
      Z: number;
      A: number;
      symbol: string;
      name: string;
      isotopeData: IsotopeData;
      element: ElementData;
    }>,
  ): void {
    const d = event.detail;
    selectedElementForViewer = d.element;
    selectedIsotopeForViewer = {
      Z: d.Z,
      A: d.A,
      symbol: d.symbol,
      name: d.name,
      isotopeData: d.isotopeData,
      element: d.element,
    };
    broadcastSelection({
      type: "ISOTOPE_SELECTED",
      data: {
        Z: d.Z,
        A: d.A,
        symbol: d.symbol,
        name: d.name,
        isotope: d.isotopeData,
      },
    });
    logAtlas(`Synthesis suggestion selected: ${d.symbol}-${d.A}`);
  }

  function handleSynthesisElementSelect(event: CustomEvent<ElementData>): void {
    selectedElementForViewer = event.detail;
    selectedIsotopeForViewer = null;
    pushRecentElement(event.detail);
    broadcastSelection({ type: "ELEMENT_SELECTED", data: event.detail });
    logAtlas(`Synthesis element selected: ${event.detail.symbol}`);
  }

  function handleElementSelect(event: CustomEvent<ElementData>): void {
    selectedElementForViewer = event.detail;
    selectedIsotopeForViewer = null;
    pushRecentElement(event.detail);
    logAtlas(`Element details opened: ${event.detail.name} (Z=${event.detail.Z})`);
  }

  function openRecentElement(element: ElementData): void {
    selectedElementForViewer = element;
    selectedIsotopeForViewer = null;
    pushRecentElement(element);
  }

  function openRecentIsotope(item: {
    Z: number; A: number; symbol: string; name: string;
    isotopeData: IsotopeData; element: ElementData;
  }): void {
    selectedElementForViewer = item.element;
    selectedIsotopeForViewer = item;
    pushRecentElement(item.element);
    pushRecentIsotope(item);
  }
</script>

<div class="atlas">
  <header class="atlas-header">
    <h3>Particle Atlas</h3>
    <div class="controls">
      <button class:active={mode === "taxonomy"} on:click={() => (mode = "taxonomy")}>Taxonomy</button>
      <button class:active={mode === "periodic"} on:click={() => (mode = "periodic")}>Periodic</button>
      <button class="accent" on:click={() => (customOpen = true)}>+ Custom Particle</button>
    </div>
  </header>

  {#if $atlasSelectionRequest.pending && $atlasSelectionRequest.target}
    <div class="picker-banner">
      Picker mode: select a particle to append to
      <strong>{$atlasSelectionRequest.target}</strong>
      state.
      <button on:click={clearAtlasSelectionRequest}>Cancel</button>
    </div>
  {/if}

  {#if mode === "periodic"}
    <div class="periodic-layout">
      <div class="periodic-table-col">
        <PeriodicTable
          on:elementSelect={handleElementSelect}
          on:clearInfo={() => {
            selectedElementForViewer = null;
            selectedIsotopeForViewer = null;
          }}
        />
      </div>

      <aside class="viewer-col periodic-side-col">
        <section class="group periodic-console">
          <h4>Selection Console <span>({periodicSelectionLabel})</span></h4>
          <div class="reference-summary">The periodic workspace keeps inspector and selection controls visible at all times.</div>
          <div class="taxonomy-stats periodic-stats">
            <article><span>Selected element</span><strong>{selectedElementForViewer?.symbol ?? "—"}</strong></article>
            <article><span>Selected isotope</span><strong>{selectedIsotopeForViewer ? `${selectedIsotopeForViewer.symbol}-${selectedIsotopeForViewer.A}` : "—"}</strong></article>
            <article><span>Recent elements</span><strong>{recentElements.length}</strong></article>
            <article><span>Recent isotopes</span><strong>{recentIsotopes.length}</strong></article>
          </div>
          <div class="periodic-actions">
            <button
              on:click={() => {
                selectedElementForViewer = null;
                selectedIsotopeForViewer = null;
              }}
            >Clear active selection</button>
          </div>
        </section>

        <section class="group periodic-console">
          <h4>Quick Selection Windows</h4>
          <div class="selection-grid">
            <div>
              <div class="bucket-title">Recent elements</div>
              <div class="chip-list">
                {#if recentElements.length === 0}
                  <span class="chip-muted">No element selected yet.</span>
                {:else}
                  {#each recentElements as element (`recent-el-${element.Z}`)}
                    <button class="chip" on:click={() => openRecentElement(element)}>{element.symbol} · Z{element.Z}</button>
                  {/each}
                {/if}
              </div>
            </div>
            <div>
              <div class="bucket-title">Recent isotopes</div>
              <div class="chip-list">
                {#if recentIsotopes.length === 0}
                  <span class="chip-muted">No isotope selected yet.</span>
                {:else}
                  {#each recentIsotopes as isotope (`recent-iso-${isotope.Z}-${isotope.A}`)}
                    <button class="chip" on:click={() => openRecentIsotope(isotope)}>{isotope.symbol}-{isotope.A}</button>
                  {/each}
                {/if}
              </div>
            </div>
          </div>
        </section>

        <section class="group periodic-console">
          <h4>Inspector Window</h4>
          {#if selectedElementForViewer || selectedIsotopeForViewer}
          <ParticleViewer
            element={selectedElementForViewer}
            isotope={selectedIsotopeForViewer}
            on:addToReaction={handleViewerAddToReaction}
            on:elementSynthesisSelect={handleSynthesisElementSelect}
            on:isotopeSynthesisSelect={handleSynthesisIsotopeSelect}
          />
          {:else}
            <div class="viewer-empty">Select an element in the table to open isotopes, molecule isotope synthesis, and advanced inspector controls here.</div>
          {/if}
        </section>
      </aside>
    </div>
  {:else}
    <div class="taxonomy-overview-toggles">
      <button class="taxonomy-overview-toggle" aria-expanded={taxonomyNavigatorOpen} on:click={() => (taxonomyNavigatorOpen = !taxonomyNavigatorOpen)}>
        <span>Taxonomy Navigator</span>
        <strong>{taxonomyNavigatorOpen ? "Hide" : "Show"}</strong>
      </button>
      <button class="taxonomy-overview-toggle" aria-expanded={atlasControlsOpen} on:click={() => (atlasControlsOpen = !atlasControlsOpen)}>
        <span>Atlas Controls</span>
        <strong>{atlasControlsOpen ? "Hide" : "Show"}</strong>
      </button>
    </div>

    {#if taxonomyNavigatorOpen || atlasControlsOpen}
    <section class="taxonomy-overview">
      {#if taxonomyNavigatorOpen}
      <TaxonomyGuidePanel
        total={taxonomy.total}
        charged={taxonomyStats.charged}
        neutral={taxonomyStats.neutral}
        colored={taxonomyStats.colored}
        unstable={taxonomyStats.unstable}
        interactionRich={taxonomyStats.interactionRich}
        referenceCount={visibleAtlasReferenceParticles.length}
        activeFilter={taxonomyFilterLabel}
        activeSort={taxonomySortLabel}
      />
      {/if}

      {#if atlasControlsOpen}
      <section class="taxonomy-controls-card">
        <div class="taxonomy-controls-copy">
          <h4>Atlas Controls</h4>
          <p>
            {#if $theoreticalModel}
              {taxonomy.total} fields are available in the active model, alongside the extended reference catalogue.
            {:else}
              No active model is loaded, so the atlas is operating in curated reference mode for comparison and inspection.
            {/if}
          </p>
        </div>

        <div class="taxonomy-toolbar">
      <input
        class="taxonomy-search"
        type="search"
        bind:value={taxonomyQuery}
        placeholder="Search particles by name, id, or symbol…"
      />
      <select class="taxonomy-select" bind:value={taxonomyFilter}>
        <option value="all">All fields</option>
        <option value="charged">Charged</option>
        <option value="neutral">Neutral</option>
        <option value="colored">Colored</option>
        <option value="unstable">Unstable</option>
      </select>
      <select class="taxonomy-select" bind:value={taxonomySort}>
        <option value="mass">Sort by mass</option>
        <option value="alpha">Sort alphabetically</option>
        <option value="charge">Sort by |charge|</option>
      </select>
      <select class="taxonomy-select" bind:value={referenceDomainFilter}>
        <option value="all">Reference domain: all</option>
        <option value="sm">Reference domain: SM</option>
        <option value="hadron">Reference domain: hadronic</option>
        <option value="bsm">Reference domain: BSM / exotic</option>
      </select>
      <select class="taxonomy-select" bind:value={referenceInteractionFilter}>
        <option value="all">Reference interaction: any</option>
        <option value="strong">Reference interaction: strong</option>
        <option value="weak">Reference interaction: weak</option>
        <option value="electromagnetic">Reference interaction: electromagnetic</option>
        <option value="yukawa">Reference interaction: Yukawa</option>
      </select>
        </div>

        <div class="taxonomy-stats">
          <article><span>Reference particles</span><strong>{visibleAtlasReferenceParticles.length}</strong></article>
          <article><span>Ref. SM / hadron / BSM</span><strong>{referenceStats.sm}/{referenceStats.hadron}/{referenceStats.bsm}</strong></article>
          <article><span>Selection mode</span><strong>{$atlasSelectionRequest.pending ? "picker" : "inspect"}</strong></article>
        </div>
      </section>
      {/if}
    </section>
    {/if}

    <div class="taxonomy-layout">
      <div class="taxonomy-grid-col">
        {#if filteredTaxonomyGroups.length === 0}
          <div class="viewer-empty">No particles match the current taxonomy filters.</div>
        {/if}
        {#each filteredTaxonomyGroups as group (group.key)}
          {#if group.count > 0}
            <section class="group">
              <h4>{group.label} <span>({group.count})</span></h4>

              {#each group.buckets as bucket (bucket.key)}
                <div class="bucket">
                  <div class="bucket-title">{bucket.label}</div>
                  <div class="grid">
                    {#each bucket.particles as particle (particle.id)}
                      <ParticleCell
                        particle={particle}
                        selectable={$atlasSelectionRequest.pending}
                        flashing={flashId === particle.id}
                        on:quickAdd={handleQuickAdd}
                        on:select={(e) => handleParticleSelect(e.detail)}
                      />
                    {/each}
                  </div>
                </div>
              {/each}
            </section>
          {/if}
        {/each}

        <section class="group reference-group">
          <h4>Reference Particle Catalogue <span>({visibleAtlasReferenceParticles.length}/{atlasReferenceParticles.length})</span></h4>
          <div class="reference-summary">Extended atlas particle set (SM + hadrons + BSM templates) for richer schematics and configurable Feynman inspection.</div>
          <div class="grid">
            {#each visibleAtlasReferenceParticles as particle (`ref-${particle.id}`)}
              <ParticleCell
                particle={particle}
                selectable={false}
                flashing={flashId === particle.id}
                on:quickAdd={handleQuickAdd}
                on:select={(e) => handleReferenceParticleSelect(e.detail)}
              />
            {/each}
          </div>
        </section>

        <section class="group reference-group">
          <h4>Composite Reference Library <span>({referenceComposites.length})</span></h4>
          <div class="reference-summary">Supplemental atlas states with built-in schematic valence content and decay presets.</div>
          <div class="reference-grid">
            {#each referenceComposites as entry (`${entry.label}-${entry.kind}`)}
              <button class="reference-card" on:click={() => handleReferenceCompositeSelect(entry)}>
                <strong>{entry.label}</strong>
                <span>{entry.kind}</span>
                <em>{entry.valence.join(" · ")}</em>
              </button>
            {/each}
          </div>
        </section>
      </div>

      <aside class="viewer-col viewer-col--taxonomy">
        <section class="group taxonomy-console">
          <h4>Selection Console</h4>
          <div class="reference-summary">The taxonomy workspace keeps the active selection, workspace actions, and scientific viewer together in one rail.</div>
          <div class="taxonomy-stats taxonomy-console-stats">
            <article><span>Selected type</span><strong>{selectedParticle ? "field" : selectedReferenceComposite ? "reference" : "none"}</strong></article>
            <article><span>Current selection</span><strong>{selectedParticle?.symbol ?? selectedReferenceComposite?.label ?? "â€”"}</strong></article>
            <article><span>Selection mode</span><strong>{$atlasSelectionRequest.pending ? "picker" : "inspect"}</strong></article>
          </div>
          <div class="periodic-actions">
            <button on:click={clearTaxonomySelection}>Clear active selection</button>
          </div>
        </section>

        <section class="group taxonomy-console taxonomy-inspector-window">
          <h4>Inspector Window</h4>
          {#if selectedParticle}
            <ParticleViewer
              particle={selectedParticle}
              on:addToReaction={handleViewerAddToReaction}
              on:decaySelect={handleDecaySelect}
              on:elementSynthesisSelect={handleSynthesisElementSelect}
            />
          {:else if selectedReferenceComposite}
            <ParticleViewer
              referenceComposite={selectedReferenceComposite}
              on:addToReaction={handleViewerAddToReaction}
              on:decaySelect={handleDecaySelect}
              on:elementSynthesisSelect={handleSynthesisElementSelect}
            />
          {:else}
            <div class="viewer-empty">Select a particle card to inspect composition, decays, and reaction-workspace actions here.</div>
          {/if}
        </section>
      </aside>
    </div>
  {/if}

  <CustomParticleModal
    open={customOpen}
    on:close={() => (customOpen = false)}
    on:created={() => {
      customOpen = false;
      mode = "taxonomy";
    }}
  />
</div>

<style>
  .atlas {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .atlas-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.25rem;
  }

  h3 {
    margin: 0;
    font-size: 0.92rem;
    color: var(--fg-accent);
  }

  .controls {
    display: flex;
    gap: 0.25rem;
  }

  .controls button {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    padding: 0.2rem 0.45rem;
    font-size: 0.68rem;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  .controls button.active {
    color: var(--fg-primary);
    border-color: var(--hl-symbol);
  }

  .controls button.accent {
    color: var(--hl-symbol);
    border-color: var(--hl-symbol);
  }

  .picker-banner {
    border: 1px solid var(--hl-value);
    background: rgba(var(--color-accent-rgb), 0.08);
    color: var(--fg-primary);
    padding: 0.35rem 0.45rem;
    font-size: 0.72rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .picker-banner button {
    margin-left: auto;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    padding: 0.15rem 0.35rem;
    cursor: pointer;
  }

  .taxonomy-overview {
    display: grid;
    grid-template-columns: minmax(0, 1.15fr) minmax(18rem, 0.85fr);
    gap: 0.45rem;
  }

  .taxonomy-overview-toggles {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .taxonomy-overview-toggle {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-primary);
    padding: 0.2rem 0.38rem;
    display: inline-flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    min-width: 14rem;
    font-family: var(--font-mono);
    font-size: 0.64rem;
    cursor: pointer;
  }

  .taxonomy-overview-toggle strong {
    color: var(--fg-secondary);
    font-weight: 500;
  }

  .taxonomy-controls-card {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.42rem;
  }

  .taxonomy-controls-copy h4 {
    margin: 0 0 0.14rem;
  }

  .taxonomy-controls-copy p {
    margin: 0;
    font-size: 0.66rem;
    color: var(--fg-secondary);
    line-height: 1.45;
    font-family: var(--font-mono);
  }

  .taxonomy-toolbar,
  .taxonomy-stats {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .taxonomy-search,
  .taxonomy-select {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    font-size: 0.68rem;
    padding: 0.22rem 0.35rem;
    font-family: var(--font-mono);
  }

  .taxonomy-search {
    flex: 1;
    min-width: 16rem;
  }

  .taxonomy-stats article {
    min-width: 8rem;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: 0.35rem 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
  }

  .taxonomy-stats span,
  .taxonomy-stats strong {
    font-family: var(--font-mono);
  }

  .taxonomy-stats span {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-secondary);
  }

  .taxonomy-stats strong {
    font-size: 0.86rem;
    color: var(--fg-primary);
  }

  .taxonomy-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.1fr) minmax(0, 0.9fr);
    gap: 0.45rem;
    flex: 1 1 0;
    min-height: 0;
    overflow: hidden;
  }

  .periodic-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 40%);
    gap: 0.45rem;
    flex: 1 1 0;
    min-height: 0;
    overflow: hidden;
  }

  .periodic-table-col {
    min-height: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .taxonomy-grid-col {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-height: 0;
    overflow-y: auto;
  }

  .viewer-col {
    min-height: 0;
    overflow-y: auto;
  }

  .viewer-col--taxonomy {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .taxonomy-console {
    background: var(--bg-surface);
  }

  .taxonomy-console-stats article {
    min-width: 7.4rem;
  }

  .taxonomy-inspector-window {
    flex: 1 1 auto;
    min-height: 0;
  }

  .periodic-side-col {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding-right: 0.1rem;
  }

  .periodic-console {
    background: var(--bg-surface);
  }

  .periodic-stats {
    margin-bottom: 0.3rem;
  }

  .periodic-stats article {
    min-width: 7.2rem;
  }

  .periodic-actions {
    display: flex;
    justify-content: flex-end;
  }

  .periodic-actions button,
  .chip {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 0.62rem;
    padding: 0.14rem 0.32rem;
    cursor: pointer;
  }

  .chip:hover,
  .periodic-actions button:hover {
    border-color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.08);
  }

  .selection-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.35rem;
  }

  .chip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .chip-muted {
    font-size: 0.64rem;
    color: var(--fg-secondary);
    font-style: italic;
    font-family: var(--font-mono);
  }

  .viewer-empty {
    border: 1px dashed var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    font-size: 0.74rem;
    font-style: italic;
    padding: 0.45rem;
  }

  .group {
    border: 1px solid var(--border);
    padding: 0.35rem;
    background: var(--bg-surface);
  }

  .reference-group {
    border-style: dashed;
  }

  .reference-summary {
    font-size: 0.66rem;
    color: var(--fg-secondary);
    margin-bottom: 0.35rem;
  }

  .reference-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 0.28rem;
  }

  .reference-card {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    align-items: flex-start;
    text-align: left;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-primary);
    padding: 0.35rem 0.4rem;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  .reference-card:hover {
    border-color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.08);
  }

  .reference-card strong {
    color: var(--hl-symbol);
    font-size: 0.72rem;
  }

  .reference-card span,
  .reference-card em {
    font-size: 0.61rem;
    color: var(--fg-secondary);
  }

  h4 {
    margin: 0 0 0.25rem;
    font-size: 0.76rem;
    text-transform: uppercase;
    color: var(--fg-primary);
  }

  h4 span {
    color: var(--fg-secondary);
    font-weight: 400;
  }

  .bucket {
    margin-bottom: 0.45rem;
  }

  .bucket:last-child {
    margin-bottom: 0;
  }

  .bucket-title {
    font-size: 0.69rem;
    color: var(--fg-secondary);
    margin-bottom: 0.2rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 0.25rem;
  }

  @media (max-width: 1240px) {
    .taxonomy-overview-toggles {
      flex-direction: column;
    }

    .taxonomy-overview-toggle {
      min-width: 0;
    }

    .taxonomy-overview {
      grid-template-columns: 1fr;
    }

    .taxonomy-layout {
      grid-template-columns: 1fr;
    }

    .periodic-layout {
      grid-template-columns: 1fr;
    }

    .selection-grid {
      grid-template-columns: 1fr;
    }

    .viewer-col {
      max-height: 42%;
    }

    .viewer-col--taxonomy {
      overflow-y: auto;
    }
  }
</style>
