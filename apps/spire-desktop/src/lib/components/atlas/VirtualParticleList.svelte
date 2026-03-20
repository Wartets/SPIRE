<script lang="ts">
  import type { PdgParticleRecord } from '$lib/types/spire';
  import { selectedPdgParticle } from '$lib/stores/pdgStore';

  export let items: PdgParticleRecord[] = [];
  export let selectedId: number | null = null;

  // Virtual scrolling: only render visible items
  let container: HTMLElement;
  let scrollTop = 0;
  const itemHeight = 40; // px per particle row
  const visibleCount = 15; // items visible at once

  $: startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - 2);
  $: endIndex = Math.min(items.length, startIndex + visibleCount + 4);
  $: visibleItems = items.slice(startIndex, endIndex);
  $: offsetY = startIndex * itemHeight;
  $: totalHeight = items.length * itemHeight;

  function handleScroll(event: Event) {
    const target = event.target as HTMLElement;
    scrollTop = target.scrollTop;
  }

  function selectParticle(particle: PdgParticleRecord) {
    selectedPdgParticle.set(particle);
  }

  function formatMass(mass: number | null | undefined): string {
    if (mass === null || mass === undefined) {
      return '—';
    }
    if (mass < 1) {
      return mass.toFixed(4);
    }
    if (mass < 10) {
      return mass.toFixed(2);
    }
    return Math.round(mass).toString();
  }

  function formatCharge(charge: number): string {
    if (charge === 0) return '0';
    if (charge > 0) return '+' + Math.abs(charge);
    return '−' + Math.abs(charge);
  }

  function getMassValue(particle: PdgParticleRecord): number | undefined {
    return particle.mass?.value;
  }
</script>

<div class="virtual-list" bind:this={container} on:scroll={handleScroll}>
  <div class="virtual-spacer" style="height: {totalHeight}px; position: relative;">
    <div style="transform: translateY({offsetY}px);">
      {#each visibleItems as particle (particle.pdg_id)}
        <div
          class="particle-row"
          class:selected={selectedId === particle.pdg_id}
          on:click={() => selectParticle(particle)}
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectParticle(particle)}
          role="button"
          tabindex="0"
        >
          <span class="pdg-id">{particle.pdg_id}</span>
          <span class="symbol">{particle.label?.slice(0, 2) ?? '—'}</span>
          <span class="name">{particle.label ?? 'Unknown'}</span>
          <span class="charge">Q:{formatCharge(particle.quantum_numbers.charge ?? 0)}</span>
          <span class="mass">M:{formatMass(getMassValue(particle))}</span>
        </div>
      {/each}
    </div>
  </div>
  {#if items.length === 0}
    <div class="empty-state">
      <p>No particles match current filters</p>
    </div>
  {/if}
</div>

<style>
  .virtual-list {
    flex: 1;
    overflow-y: auto;
    border: 1px solid var(--outline, #ddd);
    border-radius: 4px;
    background: var(--surface, #fff);
    max-height: 500px;
  }

  .virtual-spacer {
    width: 100%;
  }

  .particle-row {
    display: grid;
    grid-template-columns: 50px 50px 1fr 70px 100px;
    gap: 0.5rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--outline-variant, #eee);
    cursor: pointer;
    transition: background-color 0.15s ease;
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .particle-row:hover {
    background: var(--surface-variant, #f5f5f5);
  }

  .particle-row.selected {
    background: var(--primary-container, #e3f2fd);
    color: var(--on-primary-container, #0d47a1);
    font-weight: 500;
  }

  .pdg-id {
    font-family: monospace;
    font-weight: 600;
    color: var(--primary, #1976d2);
  }

  .symbol {
    font-weight: 600;
    color: var(--on-surface, #000);
  }

  .name {
    color: var(--on-surface-variant, #666);
    text-overflow: ellipsis;
    overflow: hidden;
  }

  .charge {
    font-family: monospace;
    text-align: center;
    font-size: 0.85rem;
    color: var(--on-surface-variant, #666);
  }

  .mass {
    font-family: monospace;
    text-align: right;
    font-size: 0.85rem;
    color: var(--on-surface-variant, #666);
  }

  .empty-state {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--on-surface-variant, #999);
  }
</style>
