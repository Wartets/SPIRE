<script lang="ts">
  import { theoreticalModel } from "$lib/stores/physicsStore";
  import { categorizeParticles } from "$lib/core/physics/taxonomy";
  import type { Field } from "$lib/types/spire";
  import ParticleCell from "$lib/components/atlas/ParticleCell.svelte";
  import CustomParticleModal from "$lib/components/atlas/CustomParticleModal.svelte";
  import PeriodicTable from "$lib/components/atlas/PeriodicTable.svelte";
  import ParticleViewer from "$lib/components/atlas/ParticleViewer.svelte";
  import {
    atlasSelectionRequest,
    submitAtlasSelection,
    clearAtlasSelectionRequest,
  } from "$lib/stores/atlasSelectionStore";
  import { appendLog } from "$lib/stores/physicsStore";
  import { initialIdsInput, finalIdsInput } from "$lib/stores/workspaceInputsStore";

  type AtlasMode = "taxonomy" | "periodic";

  let mode: AtlasMode = "taxonomy";
  let customOpen = false;
  let selectedParticle: Field | null = null;

  $: taxonomy = categorizeParticles($theoreticalModel);

  function handleParticleSelect(field: Field): void {
    if ($atlasSelectionRequest.pending && $atlasSelectionRequest.target) {
      submitAtlasSelection($atlasSelectionRequest.target, field.id);
      appendLog(`Atlas picker selected ${field.id} for ${$atlasSelectionRequest.target} state.`);
      return;
    }
    selectedParticle = field;
    appendLog(`Atlas particle inspected: ${field.id}`);
  }

  function handleDecaySelect(
    event: CustomEvent<{ parentId: string; finalStateIds: string[]; label: string }>,
  ): void {
    const detail = event.detail;
    initialIdsInput.set([detail.parentId]);
    finalIdsInput.set([...detail.finalStateIds]);
    appendLog(`Decay branch loaded into Reaction Workspace: ${detail.label}`);
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
    <PeriodicTable />
  {:else if !$theoreticalModel}
    <p class="empty">Load a model to build the particle taxonomy.</p>
  {:else}
    <div class="summary">{taxonomy.total} total fields in active model</div>

    <div class="taxonomy-layout">
      <div class="taxonomy-grid-col">
        {#each taxonomy.groups as group (group.key)}
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
                        on:select={(e) => handleParticleSelect(e.detail)}
                      />
                    {/each}
                  </div>
                </div>
              {/each}
            </section>
          {/if}
        {/each}
      </div>

      <aside class="viewer-col">
        {#if selectedParticle}
          <ParticleViewer particle={selectedParticle} on:decaySelect={handleDecaySelect} />
        {:else}
          <div class="viewer-empty">Select a particle to open the schematic composition viewer.</div>
        {/if}
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
    overflow: auto;
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

  .summary {
    font-size: 0.72rem;
    color: var(--fg-secondary);
  }

  .taxonomy-layout {
    display: grid;
    grid-template-columns: minmax(360px, 1.1fr) minmax(300px, 0.9fr);
    gap: 0.45rem;
    min-height: 0;
  }

  .taxonomy-grid-col {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-height: 0;
  }

  .viewer-col {
    min-height: 0;
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

  .empty {
    color: var(--fg-secondary);
    font-style: italic;
    font-size: 0.8rem;
  }

  @media (max-width: 1240px) {
    .taxonomy-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
