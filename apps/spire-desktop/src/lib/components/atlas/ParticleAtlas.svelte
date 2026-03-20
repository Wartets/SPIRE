<script lang="ts">
  import { theoreticalModel, appendLog } from "$lib/stores/physicsStore";
  import { categorizeParticles } from "$lib/core/physics/taxonomy";
  import type { Field } from "$lib/types/spire";
  import ParticleCell from "$lib/components/atlas/ParticleCell.svelte";
  import CustomParticleModal from "$lib/components/atlas/CustomParticleModal.svelte";
  import PeriodicTable from "$lib/components/atlas/PeriodicTable.svelte";
  import ParticleViewer from "$lib/components/atlas/ParticleViewer.svelte";
  import IsotopeDrawer from "$lib/components/atlas/IsotopeDrawer.svelte";
  import PeriodicInspector from "$lib/components/atlas/PeriodicInspector.svelte";
  import {
    atlasSelectionRequest,
    submitAtlasSelection,
    clearAtlasSelectionRequest,
  } from "$lib/stores/atlasSelectionStore";
  import { initialIdsInput, finalIdsInput } from "$lib/stores/workspaceInputsStore";
  import { broadcastSelection } from "$lib/stores/selectionBus";
  import {
    atlasPeriodicState,
    atlasPeriodicSelectedIsotopes,
    clearAtlasPeriodicSelection,
    selectAtlasPeriodicElement,
    selectAtlasPeriodicIsotope,
    setAtlasPeriodicSelectionFromIsotope,
  } from "$lib/stores/atlasPeriodicStore";
  import {
    listCompositeReferenceLibrary,
    type CompositeQuarkContent,
  } from "$lib/core/physics/composites";
  import { listAtlasReferenceParticles } from "$lib/core/physics/atlasReferenceParticles";
  import PdgCatalog from "$lib/components/atlas/PdgCatalog.svelte";

  const logAtlas = (message: string): void => {
    appendLog(message, { category: "Atlas" });
  };

  type AtlasMode = "taxonomy" | "periodic" | "pdg";

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

  $: taxonomy = categorizeParticles($theoreticalModel);
  $: referenceComposites = listCompositeReferenceLibrary();
  $: atlasReferenceSeedParticles = listAtlasReferenceParticles();
  $: modelReferenceSeedParticles = $theoreticalModel?.fields ?? [];
  $: atlasReferenceParticles = mergeReferenceParticleLibraries(atlasReferenceSeedParticles, modelReferenceSeedParticles);
  $: taxonomyFields = taxonomy.groups.flatMap((group) => group.buckets.flatMap((bucket) => bucket.particles));
  $: periodicSelectionLabel = $atlasPeriodicState.selectedIsotope
    ? `${$atlasPeriodicState.selectedIsotope.symbol}-${$atlasPeriodicState.selectedIsotope.A}`
    : $atlasPeriodicState.selectedElement
      ? `${$atlasPeriodicState.selectedElement.symbol} (${$atlasPeriodicState.selectedElement.name})`
      : "none";
  $: periodicSelectedStats = $atlasPeriodicState.selectedElement
    ? $atlasPeriodicState.isotopeStats[$atlasPeriodicState.selectedElement.Z] ?? null
    : null;
  $: periodicSelectedType = $atlasPeriodicState.selectedIsotope
    ? "isotope"
    : $atlasPeriodicState.selectedElement
      ? "element"
      : "none";
  $: periodicSelectedNeutrons = $atlasPeriodicState.selectedIsotope
    ? $atlasPeriodicState.selectedIsotope.A - $atlasPeriodicState.selectedIsotope.Z
    : null;
  $: periodicNaturalIsotopeLabel = periodicSelectedStats?.naturalA
    ? `${$atlasPeriodicState.selectedElement?.symbol ?? ""}-${periodicSelectedStats.naturalA}`
    : "n/a";
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

  function referenceDomainFor(field: Field): "sm" | "hadron" | "bsm" {
    const id = field.id.toLowerCase();
    if ([
      "gamma", "g", "z", "w+", "w-", "h", "u", "d", "s", "c", "b", "t",
      "e-", "e+", "mu-", "tau-", "nu_e", "nu_mu", "nu_tau",
    ].includes(id)) {
      return "sm";
    }
    if ([
      "p", "n", "pi+", "pi0", "k+", "k0", "lambda0", "sigma+", "sigma0", "sigma-",
      "xi0", "xi-", "omega-", "rho0", "phi_meson", "j_psi", "upsilon", "x3872",
      "z_c3900", "theta_plus",
    ].includes(id)) {
      return "hadron";
    }
    return "bsm";
  }

  function mergeReferenceParticleLibraries(primary: Field[], secondary: Field[]): Field[] {
    const merged = new Map<string, Field>();

    const push = (field: Field): void => {
      const key = field.id.trim().toLowerCase();
      const existing = merged.get(key);
      if (!existing) {
        merged.set(key, {
          ...field,
          interactions: [...new Set(field.interactions)],
        });
        return;
      }

      const interactions = Array.from(new Set([...existing.interactions, ...field.interactions]));
      const preferExistingMass = Number.isFinite(existing.mass) && Math.abs(existing.mass) > 0;
      const preferExistingWidth = Number.isFinite(existing.width) && existing.width >= 0;

      merged.set(key, {
        ...field,
        ...existing,
        mass: preferExistingMass ? existing.mass : field.mass,
        width: preferExistingWidth ? existing.width : field.width,
        interactions,
      });
    };

    primary.forEach(push);
    secondary.forEach(push);

    return [...merged.values()].sort((a, b) => Math.abs(a.mass) - Math.abs(b.mass) || a.name.localeCompare(b.name));
  }

  function referenceInteractionFilterToType(
    filter: typeof referenceInteractionFilter,
  ): Field["interactions"][number] {
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

  function triggerFlash(id: string): void {
    flashId = id;
    setTimeout(() => {
      if (flashId === id) flashId = null;
    }, 820);
  }

  function clearTaxonomySelection(): void {
    selectedParticle = null;
    selectedReferenceComposite = null;
  }

  function appendToReaction(target: "initial" | "final", id: string, label: string): void {
    if (target === "initial") {
      initialIdsInput.update((previous) => [...previous, id]);
    } else {
      finalIdsInput.update((previous) => [...previous, id]);
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

  function openRecentElement(Z: number): void {
    const element = $atlasPeriodicState.recentElements.find((entry) => entry.Z === Z) ?? null;
    if (!element) return;
    selectAtlasPeriodicElement(element);
  }

  function openRecentIsotope(Z: number, A: number): void {
    const isotope = $atlasPeriodicState.recentIsotopes.find((entry) => entry.Z === Z && entry.A === A) ?? null;
    if (!isotope) return;
    setAtlasPeriodicSelectionFromIsotope(isotope);
  }
</script>

<div class="atlas">
  <header class="atlas-header">
    <h3>Particle Atlas</h3>
    <div class="controls">
      <button class:active={mode === "taxonomy"} on:click={() => (mode = "taxonomy")}>Taxonomy</button>
      <button class:active={mode === "periodic"} on:click={() => (mode = "periodic")}>Periodic</button>
      <button class:active={mode === "pdg"} on:click={() => (mode = "pdg")}>PDG Catalog</button>
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
        <PeriodicTable />
      </div>

      <aside class="viewer-col periodic-side-col">
        <section class="group periodic-console">
          <h4>Selection Console <span>({periodicSelectionLabel})</span></h4>
          <div class="reference-summary">
            The periodic workspace keeps the selection summary, isotope catalogue, and inspector visible in one scientific console.
          </div>
          <div class="taxonomy-stats taxonomy-console-stats periodic-console-status">
            <article><span>Selected type</span><strong>{periodicSelectedType}</strong></article>
            <article><span>Current selection</span><strong>{periodicSelectionLabel}</strong></article>
            <article><span>Selection mode</span><strong>inspect</strong></article>
          </div>
          <div class="periodic-console-board">
            <div class="periodic-console-pane">
              <div class="taxonomy-stats periodic-stats">
                <article><span>Selected element</span><strong>{$atlasPeriodicState.selectedElement?.symbol ?? "-"}</strong></article>
                <article><span>Selected isotope</span><strong>{$atlasPeriodicState.selectedIsotope ? `${$atlasPeriodicState.selectedIsotope.symbol}-${$atlasPeriodicState.selectedIsotope.A}` : "-"}</strong></article>
                <article><span>Recent elements</span><strong>{$atlasPeriodicState.recentElements.length}</strong></article>
                <article><span>Recent isotopes</span><strong>{$atlasPeriodicState.recentIsotopes.length}</strong></article>
              </div>
            </div>

            <div class="periodic-console-pane periodic-dossier">
              <div class="bucket-title">Selected dossier</div>
              <div class="periodic-dossier-grid">
                <div><span>Natural isotope</span><strong>{periodicNaturalIsotopeLabel}</strong></div>
                <div><span>Known isotopes</span><strong>{periodicSelectedStats?.count ?? 0}</strong></div>
                <div><span>Stable isotopes</span><strong>{periodicSelectedStats?.stable ?? 0}</strong></div>
                <div><span>Neutrons (selected)</span><strong>{periodicSelectedNeutrons ?? "-"}</strong></div>
              </div>
            </div>
          </div>

          <div class="selection-grid">
            <div>
              <div class="bucket-title">Recent elements</div>
              <div class="chip-list">
                {#if $atlasPeriodicState.recentElements.length === 0}
                  <span class="chip-muted">No element selected yet.</span>
                {:else}
                  {#each $atlasPeriodicState.recentElements as element (`recent-el-${element.Z}`)}
                    <button class="chip" on:click={() => openRecentElement(element.Z)}>{element.symbol} · Z{element.Z}</button>
                  {/each}
                {/if}
              </div>
            </div>

            <div>
              <div class="bucket-title">Recent isotopes</div>
              <div class="chip-list">
                {#if $atlasPeriodicState.recentIsotopes.length === 0}
                  <span class="chip-muted">No isotope selected yet.</span>
                {:else}
                  {#each $atlasPeriodicState.recentIsotopes as isotope (`recent-iso-${isotope.Z}-${isotope.A}`)}
                    <button class="chip" on:click={() => openRecentIsotope(isotope.Z, isotope.A)}>{isotope.symbol}-{isotope.A}</button>
                  {/each}
                {/if}
              </div>
            </div>
          </div>


          <div class="periodic-inline-catalogue">
            <div class="bucket-title">Isotope catalogue</div>
            {#if $atlasPeriodicState.selectedElement}
              <IsotopeDrawer
                element={$atlasPeriodicState.selectedElement}
                isotopes={$atlasPeriodicSelectedIsotopes}
                selectedIsotope={$atlasPeriodicState.selectedIsotope?.isotopeData ?? null}
                compact={true}
                showHeader={false}
                on:isotopeSelect={(event: CustomEvent<{ isotope: import("$lib/core/physics/nuclearDataLoader").IsotopeData }>) => {
                  if ($atlasPeriodicState.selectedElement) {
                    selectAtlasPeriodicIsotope($atlasPeriodicState.selectedElement, event.detail.isotope);
                  }
                }}
                on:close={clearAtlasPeriodicSelection}
              />
            {:else}
              <div class="viewer-empty">Select an element in the table to open its isotope catalogue here.</div>
            {/if}
          </div>
        </section>

        <section class="group periodic-console periodic-inspector-window">
          <h4>Inspector Window</h4>
          {#if $atlasPeriodicState.selectedElement || $atlasPeriodicState.selectedIsotope}
            <PeriodicInspector
              element={$atlasPeriodicState.selectedElement}
              isotope={$atlasPeriodicState.selectedIsotope}
              on:addToReaction={handleViewerAddToReaction}
            />
          {:else}
            <div class="viewer-empty">Select an element in the table to inspect the atomic model and the full scientific dossier here.</div>
          {/if}
        </section>
      </aside>
    </div>
  {:else if mode === "pdg"}
    <PdgCatalog />
  {:else}
    <section class="taxonomy-control-band">
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

      <div class="taxonomy-control-grid">
        <label class="taxonomy-field taxonomy-field--search">
          <span>Search</span>
          <input class="taxonomy-search" type="search" bind:value={taxonomyQuery} placeholder="Search particles by name, id, or symbol..." />
        </label>
        <label class="taxonomy-field">
          <span>Field filter</span>
          <select class="taxonomy-select" bind:value={taxonomyFilter}>
            <option value="all">All fields</option>
            <option value="charged">Charged</option>
            <option value="neutral">Neutral</option>
            <option value="colored">Colored</option>
            <option value="unstable">Unstable</option>
          </select>
        </label>
        <label class="taxonomy-field">
          <span>Sort</span>
          <select class="taxonomy-select" bind:value={taxonomySort}>
            <option value="mass">Mass ladder</option>
            <option value="alpha">Alphabetical</option>
            <option value="charge">|Charge|</option>
          </select>
        </label>
        <label class="taxonomy-field">
          <span>Catalogue domain</span>
          <select class="taxonomy-select" bind:value={referenceDomainFilter}>
            <option value="all">All domains</option>
            <option value="sm">Standard Model</option>
            <option value="hadron">Hadronic</option>
            <option value="bsm">BSM / exotic</option>
          </select>
        </label>
        <label class="taxonomy-field">
          <span>Catalogue interaction</span>
          <select class="taxonomy-select" bind:value={referenceInteractionFilter}>
            <option value="all">Any interaction</option>
            <option value="strong">Strong</option>
            <option value="weak">Weak</option>
            <option value="electromagnetic">Electromagnetic</option>
            <option value="yukawa">Yukawa</option>
          </select>
        </label>
      </div>

      <div class="taxonomy-stats">
        <article><span>Taxonomy scope</span><strong>{taxonomyFilterLabel}</strong></article>
        <article><span>Sort mode</span><strong>{taxonomySortLabel}</strong></article>
        <article><span>Reference particles</span><strong>{visibleAtlasReferenceParticles.length}</strong></article>
        <article><span>Ref. SM / hadron / BSM</span><strong>{referenceStats.sm}/{referenceStats.hadron}/{referenceStats.bsm}</strong></article>
        <article><span>Charged / neutral</span><strong>{taxonomyStats.charged}/{taxonomyStats.neutral}</strong></article>
        <article><span>Colored / unstable</span><strong>{taxonomyStats.colored}/{taxonomyStats.unstable}</strong></article>
        <article><span>Interaction rich</span><strong>{taxonomyStats.interactionRich}</strong></article>
        <article><span>Selection mode</span><strong>{$atlasSelectionRequest.pending ? "picker" : "inspect"}</strong></article>
      </div>
    </section>

    <div class="taxonomy-layout" class:taxonomy-layout--empty={filteredTaxonomyGroups.length === 0}>
      <aside class="taxonomy-catalogue-col">
        <section class="group reference-group">
          <h4>Reference Particle Catalogue <span>({visibleAtlasReferenceParticles.length}/{atlasReferenceParticles.length})</span></h4>
          <div class="reference-summary">Merged atlas + active-model particle libraries for broader comparison, inspection, and reaction-building access.</div>
          <div class="grid">
            {#each visibleAtlasReferenceParticles as particle (`ref-${particle.id}`)}
              <ParticleCell
                particle={particle}
                selectable={false}
                flashing={flashId === particle.id}
                on:quickAdd={handleQuickAdd}
                on:select={(event) => handleReferenceParticleSelect(event.detail)}
              />
            {/each}
          </div>
        </section>

        <section class="group reference-group">
          <h4>Composite Reference Library <span>({referenceComposites.length})</span></h4>
          <div class="reference-summary">Supplemental atlas states with built-in valence content and decay presets.</div>
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
      </aside>

      {#if filteredTaxonomyGroups.length > 0}
        <div class="taxonomy-grid-col">
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
                          on:select={(event) => handleParticleSelect(event.detail)}
                        />
                      {/each}
                    </div>
                  </div>
                {/each}
              </section>
            {/if}
          {/each}
        </div>
      {/if}

      <aside class="viewer-col viewer-col--taxonomy">
        <section class="group taxonomy-console">
          <h4>Selection Console</h4>
          <div class="reference-summary">The taxonomy workspace keeps the active selection and the scientific inspector visible in one rail.</div>
          {#if filteredTaxonomyGroups.length === 0}
            <div class="viewer-empty taxonomy-empty-hint">No particles match the current taxonomy filters.</div>
          {/if}
          <div class="taxonomy-stats taxonomy-console-stats">
            <article><span>Selected type</span><strong>{selectedParticle ? "field" : selectedReferenceComposite ? "reference" : "none"}</strong></article>
            <article><span>Current selection</span><strong>{selectedParticle?.symbol ?? selectedReferenceComposite?.label ?? "-"}</strong></article>
            <article><span>Selection mode</span><strong>{$atlasSelectionRequest.pending ? "picker" : "inspect"}</strong></article>
          </div>
          <div class="periodic-actions">
            <button on:click={clearTaxonomySelection}>Clear active selection</button>
          </div>
        </section>

        <section class="group taxonomy-console taxonomy-inspector-window">
          <h4>Inspector Window</h4>
          {#if selectedParticle}
            <ParticleViewer particle={selectedParticle} on:addToReaction={handleViewerAddToReaction} on:decaySelect={handleDecaySelect} />
          {:else if selectedReferenceComposite}
            <ParticleViewer referenceComposite={selectedReferenceComposite} on:addToReaction={handleViewerAddToReaction} on:decaySelect={handleDecaySelect} />
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

  .taxonomy-control-band {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 0.38rem 0.42rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .taxonomy-controls-copy h4 {
    margin: 0 0 0.12rem;
  }

  .taxonomy-controls-copy p,
  .taxonomy-field span,
  .taxonomy-search,
  .taxonomy-select,
  .taxonomy-stats span,
  .taxonomy-stats strong,
  .reference-summary,
  .reference-card,
  .chip,
  .chip-muted,
  .viewer-empty,
  .bucket-title,
  h4,
  h4 span {
    font-family: var(--font-mono);
  }

  .taxonomy-controls-copy p {
    margin: 0;
    font-size: 0.61rem;
    color: var(--fg-secondary);
    line-height: 1.35;
  }

  .taxonomy-control-grid {
    display: grid;
    grid-template-columns: minmax(13rem, 1.5fr) repeat(4, minmax(7.2rem, 0.68fr));
    gap: 0.25rem;
  }

  .taxonomy-field {
    display: flex;
    flex-direction: column;
    gap: 0.12rem;
    min-width: 0;
  }

  .taxonomy-field span {
    font-size: 0.5rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--fg-secondary);
  }

  .taxonomy-search,
  .taxonomy-select {
    background: var(--bg-inset);
    border: 1px solid var(--border);
    color: var(--fg-primary);
    font-size: 0.58rem;
    padding: 0.16rem 0.26rem;
  }

  .taxonomy-stats {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    gap: 0.24rem;
  }

  .taxonomy-stats article {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: 0.2rem 0.24rem;
    display: flex;
    flex-direction: column;
    gap: 0.06rem;
    min-width: 0;
  }

  .taxonomy-stats span {
    font-size: 0.49rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-secondary);
  }

  .taxonomy-stats strong {
    font-size: 0.62rem;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .taxonomy-layout {
    display: grid;
    grid-template-columns: minmax(22rem, 1.06fr) minmax(0, 1.16fr) minmax(18rem, 0.56fr);
    gap: 0.38rem;
    flex: 1 1 0;
    min-height: 0;
    overflow: hidden;
  }

  .taxonomy-layout.taxonomy-layout--empty {
    grid-template-columns: minmax(24rem, 1.68fr) minmax(20rem, 0.86fr);
  }

  .periodic-layout {
    display: grid;
    grid-template-columns: minmax(0, 1.68fr) minmax(20rem, 0.86fr);
    gap: 0.35rem;
    flex: 1 1 0;
    min-height: 0;
    overflow: hidden;
  }

  .periodic-table-col,
  .taxonomy-catalogue-col,
  .taxonomy-grid-col,
  .viewer-col--taxonomy,
  .periodic-side-col {
    min-height: 0;
  }

  .periodic-table-col,
  .viewer-col--taxonomy,
  .periodic-side-col {
    display: flex;
    flex-direction: column;
  }

  .taxonomy-catalogue-col,
  .taxonomy-grid-col,
  .viewer-col {
    overflow-y: auto;
    overflow-x: hidden;
  }

  .taxonomy-catalogue-col,
  .taxonomy-grid-col,
  .viewer-col--taxonomy,
  .periodic-side-col {
    display: flex;
    flex-direction: column;
    gap: 0.28rem;
  }

  .taxonomy-catalogue-col > .group:first-child {
    position: sticky;
    top: 0;
    z-index: 2;
  }

  .periodic-console,
  .taxonomy-console {
    background: var(--bg-surface);
  }

  .taxonomy-console-stats {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .taxonomy-inspector-window,
  .periodic-inspector-window {
    flex: 1 1 auto;
    min-height: 0;
  }

  .periodic-inline-catalogue {
    margin-top: 0.1rem;
    border-top: 1px dashed rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.24);
    padding-top: 0.22rem;
  }

  .selection-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.22rem;
    margin-bottom: 0.24rem;
  }

  .periodic-console-board {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.24rem;
    margin-bottom: 0.24rem;
  }

  .periodic-console-pane {
    min-width: 0;
  }

  .periodic-stats {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.2rem;
  }

  .periodic-console-status {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    margin-bottom: 0.24rem;
  }

  .periodic-dossier {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: 0.24rem;
  }

  .periodic-dossier-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.2rem 0.34rem;
  }

  .periodic-dossier-grid div {
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
    min-width: 0;
    border-bottom: 1px dashed rgba(var(--color-text-muted-rgb, 136, 136, 136), 0.2);
    padding-bottom: 0.1rem;
  }

  .periodic-dossier-grid span,
  .periodic-dossier-grid strong {
    font-family: var(--font-mono);
  }

  .periodic-dossier-grid span {
    font-size: 0.48rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .periodic-dossier-grid strong {
    font-size: 0.62rem;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    font-size: 0.56rem;
    padding: 0.12rem 0.26rem;
    cursor: pointer;
  }

  .chip:hover,
  .periodic-actions button:hover,
  .reference-card:hover {
    border-color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.08);
  }

  .chip-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
  }

  .chip-muted {
    font-size: 0.58rem;
    color: var(--fg-secondary);
    font-style: italic;
  }

  .viewer-empty {
    border: 1px dashed var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    font-size: 0.74rem;
    font-style: italic;
    padding: 0.45rem;
  }

  .taxonomy-empty-hint {
    margin-bottom: 0.24rem;
  }

  .group {
    border: 1px solid var(--border);
    padding: 0.3rem;
    background: var(--bg-surface);
  }

  .reference-group {
    border-style: dashed;
  }

  .reference-summary {
    font-size: 0.58rem;
    color: var(--fg-secondary);
    margin-bottom: 0.22rem;
    line-height: 1.35;
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
    cursor: pointer;
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
    margin: 0 0 0.18rem;
    font-size: 0.68rem;
    text-transform: uppercase;
    color: var(--fg-primary);
  }

  h4 span {
    color: var(--fg-secondary);
    font-weight: 400;
  }

  .bucket {
    margin-bottom: 0.3rem;
  }

  .bucket:last-child {
    margin-bottom: 0;
  }

  .bucket-title {
    font-size: 0.58rem;
    color: var(--fg-secondary);
    margin-bottom: 0.18rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 0.2rem;
  }

  @media (max-width: 1440px) {
    .taxonomy-control-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }

    .taxonomy-stats {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }

    .taxonomy-layout {
      grid-template-columns: minmax(18rem, 0.98fr) minmax(0, 1.06fr) minmax(17rem, 0.58fr);
    }
  }

  @media (max-width: 1240px) {
    .taxonomy-layout,
    .periodic-layout {
      grid-template-columns: 1fr;
    }

    .selection-grid {
      grid-template-columns: 1fr;
    }

    .periodic-console-board {
      grid-template-columns: 1fr;
    }

    .periodic-console-status {
      grid-template-columns: 1fr;
    }

    .viewer-col {
      max-height: 48%;
    }
  }

  @media (max-width: 840px) {
    .taxonomy-control-grid,
    .taxonomy-stats,
    .taxonomy-console-stats,
    .periodic-dossier-grid,
    .periodic-stats {
      grid-template-columns: 1fr;
    }
  }
</style>
