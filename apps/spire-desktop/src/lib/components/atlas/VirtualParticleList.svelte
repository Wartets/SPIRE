<script lang="ts">
    import { createEventDispatcher } from 'svelte';
    import type { PdgParticleRecord } from '$lib/types/spire';
    import { selectedPdgParticle } from '$lib/stores/pdgStore';
    import { draggableParticle } from '$lib/utils/dnd';
    import { formatQuantumFraction } from '$lib/core/physics/fractionFormat';
    import { tooltip } from '$lib/actions/tooltip';

    export let items: PdgParticleRecord[] = [];
    export let selectedId: number | null = null;
    export let initialScrollTop = 0;

    const dispatch = createEventDispatcher<{ scrollchange: { scrollTop: number } }>();

    let container: HTMLElement;
    let scrollTop = 0;
    const itemHeight = 40;
    const visibleCount = 15;

    $: if (container && initialScrollTop > 0 && Math.abs(container.scrollTop - initialScrollTop) > 2) {
        container.scrollTop = initialScrollTop;
        scrollTop = initialScrollTop;
    }

    $: startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - 2);
    $: endIndex = Math.min(items.length, startIndex + visibleCount + 4);
    $: visibleItems = items.slice(startIndex, endIndex);
    $: offsetY = startIndex * itemHeight;
    $: totalHeight = items.length * itemHeight;

    function handleScroll(event: Event) {
        const target = event.target as HTMLElement;
        scrollTop = target.scrollTop;
        dispatch('scrollchange', { scrollTop });
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
        return formatQuantumFraction(charge, { signed: true, maxDenominator: 24, tolerance: 1e-5 }).replace('-', '−');
    }

    function formatSpin(spin: string | null | undefined): string {
        if (spin === null || spin === undefined) {
            return '—';
        }
        const asNumber = Number(spin);
        if (Number.isFinite(asNumber)) {
            return formatQuantumFraction(asNumber, { signed: false, maxDenominator: 24, tolerance: 1e-5 });
        }
        return String(spin);
    }

    function getSpinValue(particle: PdgParticleRecord): string | undefined {
        if (particle.quantum_numbers.spin_j !== undefined) {
            return particle.quantum_numbers.spin_j;
        }

        const qn = particle.quantum_numbers as unknown as Record<string, unknown>;
        const legacySpin = qn.spin ?? qn.spinJ ?? qn.j;
        return typeof legacySpin === 'string' || typeof legacySpin === 'number'
            ? String(legacySpin)
            : undefined;
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
                    use:draggableParticle={{
                        type: 'pdg-particle',
                        pdgId: particle.pdg_id,
                        label: particle.label,
                        edition: particle.provenance.edition,
                        sourceId: particle.provenance.source_id,
                    }}
                    on:click={() => selectParticle(particle)}
                    on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectParticle(particle)}
                    role="button"
                    tabindex="0"
                >
                    <span class="pdg-id" use:tooltip={{ text: `PDG ID: ${particle.pdg_id}` }}>{particle.pdg_id}</span>
                    <span class="symbol" use:tooltip={{ text: `Symbol: ${particle.label?.split(' ')[0] ?? '—'}` }}>{particle.label?.split(' ')[0] ?? '—'}</span>
                    <span class="name" use:tooltip={{ text: `Particle: ${particle.label ?? 'Unknown'}` }}>{particle.label ?? 'Unknown'}</span>
                    <span class="charge" use:tooltip={{ text: `Charge Q=${formatCharge(particle.quantum_numbers.charge ?? 0)}` }}>{formatCharge(particle.quantum_numbers.charge ?? 0)}</span>
                    <span class="spin" use:tooltip={{ text: `Spin J=${formatSpin(getSpinValue(particle))}` }}>{formatSpin(getSpinValue(particle))}</span>
                    <span class="mass" use:tooltip={{ text: `Mass ${formatMass(getMassValue(particle))} MeV` }}>{formatMass(getMassValue(particle))}</span>
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
        border: 1px solid var(--border);
        background: var(--bg-inset);
        max-height: 500px;
    }

    .virtual-spacer {
        width: 100%;
    }

    .particle-row {
        display: grid;
        grid-template-columns: var(--pdg-catalog-columns, minmax(48px, 0.56fr) minmax(54px, 0.66fr) minmax(140px, 2.2fr) minmax(56px, 0.6fr) minmax(56px, 0.6fr) minmax(88px, 0.95fr));
        gap: 0.3rem;
        align-items: center;
        min-height: 40px;
        padding: 0.32rem 0.5rem;
        border-bottom: 1px solid var(--border);
        cursor: pointer;
        font-size: 0.56rem;
        font-family: var(--font-mono);
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        transition: none !important;
        animation: none !important;
    }

    .particle-row:hover {
        background: color-mix(in srgb, var(--bg-surface) 78%, var(--bg-inset));
    }

    .particle-row.selected {
        background: color-mix(in srgb, var(--hl-symbol) 14%, var(--bg-inset));
        border-left: 2px solid var(--hl-symbol);
        padding-left: calc(0.5rem - 1px);
    }

    .particle-row:active {
        transform: none;
        filter: none;
    }

    :global(.particle-row[data-dragging='true']) {
        opacity: 0.55;
    }

    .pdg-id {
        font-family: var(--font-mono);
        font-weight: 700;
        color: var(--hl-symbol);
        text-align: center;
        font-variant-numeric: tabular-nums;
    }

    .symbol {
        font-weight: 700;
        color: var(--fg-primary);
        text-align: center;
    }

    .name {
        color: var(--fg-secondary);
        text-overflow: ellipsis;
        overflow: hidden;
        padding-right: 0.2rem;
    }

    .charge {
        font-family: var(--font-mono);
        text-align: center;
        font-size: 0.53rem;
        color: var(--fg-secondary);
        font-variant-numeric: tabular-nums;
    }

    .spin {
        font-family: var(--font-mono);
        text-align: center;
        font-size: 0.53rem;
        color: var(--fg-secondary);
        font-variant-numeric: tabular-nums;
    }

    .mass {
        font-family: var(--font-mono);
        text-align: center;
        font-size: 0.53rem;
        color: var(--hl-value);
        font-variant-numeric: tabular-nums;
    }

    .empty-state {
        padding: 2rem 1rem;
        text-align: center;
        color: var(--fg-secondary);
        font-family: var(--font-mono);
        border-top: 1px solid var(--border);
    }
</style>
