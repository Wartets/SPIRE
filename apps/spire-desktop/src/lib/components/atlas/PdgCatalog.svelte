<script lang="ts">
    import { onMount, onDestroy, tick } from 'svelte';
    import {
        pdgCatalogMode,
        pdgSearchResults,
        chargeFilter,
        spinFilter,
        massRange,
        pdgFacetStats,
        pdgMetadata,
        pdgMetadataError,
        refreshPdgMetadata,
        pdgSortMode,
        pdgEditionFilter,
        pdgOnlyWithMeasurements,
        pdgAvailableEditions,
        pdgCatalogHistory,
        pdgLiveFetchState,
        type PdgSortMode,
    } from '$lib/stores/pdgStore';
    import PdgSearch from './PdgSearch.svelte';
    import VirtualParticleList from './VirtualParticleList.svelte';
    import PdgDetails from './PdgDetails.svelte';
    import SpireNumberInput from '$lib/components/ui/SpireNumberInput.svelte';
    import { formatQuantumFraction } from '$lib/core/physics/fractionFormat';
    import { tooltip } from '$lib/actions/tooltip';
    import { selectedPdgParticle } from '$lib/stores/pdgStore';
    import { getWidgetUiSnapshot, setWidgetUiSnapshot } from '$lib/stores/workspaceStore';

    let selectedCharges: number[] = [];
    let selectedSpins: string[] = [];
    let rootEl: HTMLElement | null = null;
    let controlBandEl: HTMLElement | null = null;
    let detailScroller: HTMLElement | null = null;
    let listScrollTop = 0;
    let detailScrollTop = 0;
    let pdgUiKey = 'pdg-catalog';
    let persistRaf: number | null = null;
    let controlBandWidth = 0;
    let controlObserver: ResizeObserver | null = null;
    let minMassValue = 0;
    let maxMassValue = 1000000;

    const sortOptions: Array<{ value: PdgSortMode; label: string }> = [
        { value: 'pdg_asc', label: 'PDG ID ↑' },
        { value: 'pdg_desc', label: 'PDG ID ↓' },
        { value: 'alpha', label: 'Alphabetical' },
        { value: 'mass_asc', label: 'Mass ↑' },
        { value: 'mass_desc', label: 'Mass ↓' },
        { value: 'charge_abs', label: '|Q| ↓' },
    ];

    $: selectedHistory = $selectedPdgParticle
        ? ($pdgCatalogHistory[$selectedPdgParticle.pdg_id] ?? [$selectedPdgParticle])
        : [];

    $: chargeEntries = Object.entries($pdgFacetStats.chargeDistribution)
        .map(([charge, count]) => ({ charge, count }))
        .sort((a, b) => b.count - a.count || Number(a.charge) - Number(b.charge));

    $: spinEntries = Object.entries($pdgFacetStats.spinDistribution)
        .map(([spin, count]) => ({ spin, count }))
        .sort((a, b) => b.count - a.count || a.spin.localeCompare(b.spin));

    $: isWideLayout = controlBandWidth >= 1200;
    $: isCompactLayout = controlBandWidth > 0 && controlBandWidth < 980;
    $: facetCap = isCompactLayout ? 4 : controlBandWidth > 0 && controlBandWidth < 1240 ? 6 : 8;
    $: visibleChargeEntries = getVisibleChargeEntries(facetCap);
    $: visibleSpinEntries = getVisibleSpinEntries(facetCap);

    $: activeFilterCount = selectedCharges.length
        + selectedSpins.length
        + ($pdgOnlyWithMeasurements ? 1 : 0)
        + ($pdgEditionFilter !== 'latest' ? 1 : 0)
        + ($pdgSortMode !== 'pdg_asc' ? 1 : 0)
        + (minMassValue > 0 || maxMassValue < 1000000 ? 1 : 0);

    $: isMassFiltered = minMassValue > 0 || maxMassValue < 1000000;
    $: showMassControls = isMassFiltered || $pdgSortMode === 'mass_asc' || $pdgSortMode === 'mass_desc' || isWideLayout;
    $: showChargeFacet = $pdgCatalogMode === 'full_catalog' || selectedCharges.length > 0 || $pdgSortMode === 'charge_abs' || isWideLayout;
    $: showSpinFacet = $pdgCatalogMode === 'full_catalog' || selectedSpins.length > 0 || isWideLayout;
    $: showFacetRow = showChargeFacet || showSpinFacet;
    $: showSummaryRow = activeFilterCount > 0 || Boolean($selectedPdgParticle) || isWideLayout;

    $: isViewingOlderEdition = Boolean(
        $selectedPdgParticle
        && $pdgMetadata?.edition
        && $selectedPdgParticle.provenance.edition !== $pdgMetadata.edition,
    );

    $: if ($pdgAvailableEditions.length > 0 && $pdgEditionFilter !== 'latest' && !$pdgAvailableEditions.includes($pdgEditionFilter)) {
        pdgEditionFilter.set('latest');
    }

    $: if (Number.isFinite(minMassValue) && Number.isFinite(maxMassValue)) {
        applyMassFilters();
    }

    function toggleChargeFilter(charge: number) {
        if (selectedCharges.includes(charge)) {
            selectedCharges = selectedCharges.filter((c) => c !== charge);
        } else {
            selectedCharges = [...selectedCharges, charge];
        }
        chargeFilter.set(selectedCharges.length > 0 ? selectedCharges : 'all');
    }

    function toggleSpinFilter(spin: string) {
        if (selectedSpins.includes(spin)) {
            selectedSpins = selectedSpins.filter((s) => s !== spin);
        } else {
            selectedSpins = [...selectedSpins, spin];
        }
        spinFilter.set(selectedSpins.length > 0 ? selectedSpins : 'all');
    }

    function getVisibleChargeEntries(cap: number): Array<{ charge: string; count: number }> {
        const selectedSet = new Set(selectedCharges.map(String));
        const selected = chargeEntries.filter((entry) => selectedSet.has(entry.charge));
        const top = chargeEntries.slice(0, cap);
        const merged = [...selected, ...top];
        return merged.filter((entry, index, arr) => arr.findIndex((x) => x.charge === entry.charge) === index);
    }

    function getVisibleSpinEntries(cap: number): Array<{ spin: string; count: number }> {
        const selectedSet = new Set(selectedSpins);
        const selected = spinEntries.filter((entry) => selectedSet.has(entry.spin));
        const top = spinEntries.slice(0, cap);
        const merged = [...selected, ...top];
        return merged.filter((entry, index, arr) => arr.findIndex((x) => x.spin === entry.spin) === index);
    }

    function applyMassFilters() {
        const min = Number(minMassValue);
        const max = Number(maxMassValue);
        massRange.set({
            min: Number.isFinite(min) ? Math.max(0, min) : 0,
            max: Number.isFinite(max) ? Math.max(0, max) : 1000000,
        });
    }

    function chargeLabel(charge: string): string {
        const numeric = Number(charge);
        return formatQuantumFraction(numeric, { signed: true, maxDenominator: 24, tolerance: 1e-5 }).replace('-', '−');
    }

    function resetFilters() {
        selectedCharges = [];
        selectedSpins = [];
        chargeFilter.set('all');
        spinFilter.set('all');
        pdgOnlyWithMeasurements.set(false);
        pdgSortMode.set('pdg_asc');
        pdgEditionFilter.set('latest');
        minMassValue = 0;
        maxMassValue = 1000000;
        massRange.set({ min: 0, max: 1000000 });
    }

    function switchMode(mode: 'core_sm' | 'full_catalog') {
        pdgCatalogMode.set(mode);
    }

    function persistUiSnapshot(): void {
        if (persistRaf !== null) {
            cancelAnimationFrame(persistRaf);
        }
        persistRaf = requestAnimationFrame(() => {
            setWidgetUiSnapshot(pdgUiKey, {
                listScrollTop,
                detailScrollTop,
            });
        });
    }

    function handleDetailScroll(event: Event): void {
        const target = event.currentTarget as HTMLElement;
        detailScrollTop = target.scrollTop;
        persistUiSnapshot();
    }

    onMount(() => {
        const canvasWidgetId = rootEl?.closest('[data-canvas-item-id]')?.getAttribute('data-canvas-item-id');
        if (canvasWidgetId) {
            pdgUiKey = `pdg-catalog:${canvasWidgetId}`;
        }

        const snapshot = getWidgetUiSnapshot<{ listScrollTop?: number; detailScrollTop?: number }>(pdgUiKey);
        if (snapshot) {
            listScrollTop = typeof snapshot.listScrollTop === 'number' ? snapshot.listScrollTop : 0;
            detailScrollTop = typeof snapshot.detailScrollTop === 'number' ? snapshot.detailScrollTop : 0;
        }

        void refreshPdgMetadata();

        if (controlBandEl) {
            controlBandWidth = controlBandEl.clientWidth;
            controlObserver = new ResizeObserver((entries) => {
                const entry = entries[0];
                if (!entry) return;
                controlBandWidth = entry.contentRect.width;
            });
            controlObserver.observe(controlBandEl);
        }

        minMassValue = Math.max(0, Math.floor($pdgFacetStats.massRange.min));
        maxMassValue = Math.max(1, Math.ceil($pdgFacetStats.massRange.max));
        applyMassFilters();

        void tick().then(() => {
            if (detailScroller && detailScrollTop > 0) {
                detailScroller.scrollTop = detailScrollTop;
            }
        });
    });

    onDestroy(() => {
        controlObserver?.disconnect();
        controlObserver = null;
        if (persistRaf !== null) {
            cancelAnimationFrame(persistRaf);
            persistRaf = null;
        }
        setWidgetUiSnapshot(pdgUiKey, {
            listScrollTop,
            detailScrollTop,
        });
    });
</script>

<div class="pdg-shell" bind:this={rootEl}>
    <section class="pdg-control-band" bind:this={controlBandEl}>
        <div class="pdg-controls-copy">
            <h4 use:tooltip={{ text: 'Primary controls for browsing and filtering PDG particles' }}>Browse Controls</h4>
            <p use:tooltip={{ text: 'Control visibility adapts automatically to active sort/filter state and available width' }}>Adaptive filters and sorting for the PDG catalogue.</p>
        </div>

        <div class="pdg-control-grid">
            <label class="pdg-field pdg-field--mode">
                <span use:tooltip={{ text: 'Switch between reduced core set and full catalogue' }}>Mode</span>
                <div class="mode-buttons mode-buttons--compact" role="group" aria-label="Catalog mode">
                    <button
                        class:active={$pdgCatalogMode === 'core_sm'}
                        on:click={() => switchMode('core_sm')}
                        use:tooltip={{ text: 'Focused Standard Model subset' }}
                    >Core</button>
                    <button
                        class:active={$pdgCatalogMode === 'full_catalog'}
                        on:click={() => switchMode('full_catalog')}
                        use:tooltip={{ text: 'Complete PDG catalog mode' }}
                    >Full</button>
                </div>
            </label>

            <label class="pdg-field pdg-field--search">
                <span use:tooltip={{ text: 'Name, symbol, or PDG ID' }}>Search</span>
                <PdgSearch />
            </label>

            <label class="pdg-field">
                <span use:tooltip={{ text: 'Ordering of results' }}>Sort</span>
                <select bind:value={$pdgSortMode} aria-label="Sort particles">
                    {#each sortOptions as option}
                        <option value={option.value}>{option.label}</option>
                    {/each}
                </select>
            </label>

            <label class="pdg-field">
                <span use:tooltip={{ text: 'PDG release snapshot' }}>Edition</span>
                <select bind:value={$pdgEditionFilter} aria-label="Edition view">
                    <option value="latest">Latest</option>
                    {#each $pdgAvailableEditions as edition}
                        <option value={edition}>{edition}</option>
                    {/each}
                </select>
            </label>

            <label class="pdg-field pdg-field--check" use:tooltip={{ text: 'Require measured mass/width/lifetime' }}>
                <span use:tooltip={{ text: 'Data quality filters' }}>Quality</span>
                <label class="spire-check spire-check--compact">
                    <input type="checkbox" bind:checked={$pdgOnlyWithMeasurements} />
                    <span>Measured only</span>
                </label>
            </label>

            {#if showMassControls}
                <div class="pdg-field pdg-field--mass" aria-label="Mass window filter">
                    <span use:tooltip={{ text: 'Filter visible particles by mass interval in MeV' }}>Mass (MeV)</span>
                    <div class="mass-inline">
                        <label class="mass-field" use:tooltip={{ text: 'Minimum mass in MeV' }}>
                            <span>min</span>
                            <SpireNumberInput bind:value={minMassValue} min={0} step={1} ariaLabel="Minimum mass filter" />
                        </label>
                        <label class="mass-field" use:tooltip={{ text: 'Maximum mass in MeV' }}>
                            <span>max</span>
                            <SpireNumberInput bind:value={maxMassValue} min={0} step={1} ariaLabel="Maximum mass filter" />
                        </label>
                    </div>
                </div>
            {/if}

            <div class="pdg-field pdg-field--reset">
                <span use:tooltip={{ text: 'Reset active sort and filter settings' }}>Actions</span>
                <button class="reset-button" on:click={resetFilters} use:tooltip={{ text: 'Reset all filters and sorting' }}>Reset</button>
            </div>
        </div>

        {#if showFacetRow}
            <div class="pdg-facet-row">
                {#if showChargeFacet}
                    <section class="facet-panel" data-kind="charge">
                        <h5 use:tooltip={{ text: 'Electric charge filters' }}>Charge</h5>
                        <div class="facet-chip-row">
                            {#each visibleChargeEntries as entry (`charge-inline-${entry.charge}`)}
                                <label class="spire-check chip-check">
                                    <input
                                        type="checkbox"
                                        checked={selectedCharges.includes(Number(entry.charge))}
                                        on:change={() => toggleChargeFilter(Number(entry.charge))}
                                    />
                                    <span use:tooltip={{ text: `Charge ${chargeLabel(entry.charge)} — ${entry.count} particle(s)` }}>Q={chargeLabel(entry.charge)} · {entry.count}</span>
                                </label>
                            {/each}
                        </div>
                    </section>
                {/if}

                {#if showSpinFacet}
                    <section class="facet-panel" data-kind="spin">
                        <h5 use:tooltip={{ text: 'Spin quantum number filters' }}>Spin</h5>
                        <div class="facet-chip-row">
                            {#each visibleSpinEntries as entry (`spin-inline-${entry.spin}`)}
                                <label class="spire-check chip-check">
                                    <input
                                        type="checkbox"
                                        checked={selectedSpins.includes(entry.spin)}
                                        on:change={() => toggleSpinFilter(entry.spin)}
                                    />
                                    <span use:tooltip={{ text: `Spin J=${entry.spin} — ${entry.count} particle(s)` }}>J={entry.spin} · {entry.count}</span>
                                </label>
                            {/each}
                        </div>
                    </section>
                {/if}
            </div>
        {/if}

        {#if showSummaryRow}
            <div class="pdg-stats">
                <article use:tooltip={{ text: 'Number of particles currently visible after filters' }}><span>Results</span><strong>{$pdgSearchResults.length}</strong></article>
                <article use:tooltip={{ text: 'Current ordering strategy for the list' }}><span>Sort mode</span><strong>{$pdgSortMode}</strong></article>
                <article use:tooltip={{ text: 'PDG edition snapshot currently displayed' }}><span>Edition</span><strong>{$pdgEditionFilter}</strong></article>
                <article use:tooltip={{ text: 'Count of active filters and non-default sort options' }}><span>Active filters</span><strong>{activeFilterCount}</strong></article>
                <article use:tooltip={{ text: 'Currently selected particle in the list' }}><span>Selection</span><strong>{$selectedPdgParticle?.label ?? $selectedPdgParticle?.pdg_id ?? 'None'}</strong></article>
            </div>
        {/if}
    </section>

    <div class="pdg-main-grid" class:pdg-main-grid--single={!$selectedPdgParticle}>
        <section class="pdg-pane pdg-pane--list">
            <h4>Particle Catalog <span>({$pdgSearchResults.length})</span></h4>
            <div class="catalog-header" aria-hidden="true">
                <span use:tooltip={{ text: 'PDG Monte Carlo particle identifier' }}>ID</span>
                <span use:tooltip={{ text: 'Short particle symbol' }}>SYM</span>
                <span use:tooltip={{ text: 'Particle label / canonical name' }}>PARTICLE</span>
                <span use:tooltip={{ text: 'Electric charge Q (in e units)' }}>Q</span>
                <span use:tooltip={{ text: 'Spin quantum number J' }}>J</span>
                <span use:tooltip={{ text: 'Mass value (MeV)' }}>MASS</span>
            </div>
            <VirtualParticleList
                items={$pdgSearchResults}
                selectedId={$selectedPdgParticle?.pdg_id ?? null}
                initialScrollTop={listScrollTop}
                on:scrollchange={(event) => {
                    listScrollTop = event.detail.scrollTop;
                    persistUiSnapshot();
                }}
            />
        </section>

        {#if $selectedPdgParticle}
            <main class="pdg-pane pdg-pane--detail" bind:this={detailScroller} on:scroll={handleDetailScroll}>
                {#if $pdgMetadataError}
                    <div class="edition-banner error" role="status">
                        <span>Metadata unavailable: {$pdgMetadataError}</span>
                    </div>
                {:else if isViewingOlderEdition}
                    <div class="edition-banner" role="status">
                        <span>
                            Showing {$selectedPdgParticle?.provenance.edition}; latest is {$pdgMetadata?.edition}. Compare below.
                        </span>
                    </div>
                {/if}

                <PdgDetails
                    particle={$selectedPdgParticle}
                    history={selectedHistory}
                    liveFetchActive={$pdgLiveFetchState.active && $pdgLiveFetchState.pdgId === $selectedPdgParticle.pdg_id}
                    liveSourceId={$pdgLiveFetchState.sourceId}
                    offlineFallback={$pdgLiveFetchState.offlineFallback}
                />
            </main>
        {/if}
    </div>
</div>

<style>
    .pdg-shell {
        display: flex;
        flex-direction: column;
        gap: 0.28rem;
        height: 100%;
        min-height: 0;
        overflow: hidden;
        font-family: var(--font-mono);
    }

    .pdg-control-band {
        border: 1px solid var(--border);
        background: var(--bg-surface);
        padding: 0.36rem 0.42rem;
        display: flex;
        flex-direction: column;
        gap: 0.26rem;
    }

    .pdg-controls-copy h4 {
        margin: 0 0 0.1rem;
        font-size: 0.74rem;
        color: var(--fg-primary);
        letter-spacing: 0.05em;
        text-transform: uppercase;
    }

    .pdg-controls-copy p {
        margin: 0;
        font-size: 0.58rem;
        color: var(--fg-secondary);
        line-height: 1.3;
    }

    .pdg-control-grid {
        display: grid;
        grid-template-columns: minmax(11rem, 1fr) minmax(14rem, 1.45fr) repeat(3, minmax(8rem, 0.82fr)) minmax(10rem, 1fr) minmax(6rem, 0.56fr);
        gap: 0.22rem;
        align-items: stretch;
    }

    .pdg-field {
        display: flex;
        flex-direction: column;
        gap: 0.12rem;
        min-width: 0;
        border: 1px solid var(--border);
        background: var(--bg-inset);
        padding: 0.2rem 0.24rem;
    }

    .pdg-field > span {
        font-size: 0.49rem;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        color: var(--fg-secondary);
        line-height: 1.08;
    }

    .pdg-field select {
        background: var(--bg-inset);
        border: 1px solid var(--border);
        color: var(--fg-primary);
        font-size: 0.58rem;
        padding: 0.14rem 0.22rem;
        min-height: 1.56rem;
        font-family: var(--font-mono);
    }

    .pdg-field--search :global(.pdg-search) {
        gap: 0.14rem;
    }

    .pdg-field--search :global(.search-input),
    .pdg-field--search :global(.clear-button) {
        min-height: 1.56rem;
        font-size: 0.58rem;
    }

    .pdg-field--search :global(.search-input) {
        padding: 0.14rem 0.22rem;
    }

    .pdg-field--search :global(.clear-button) {
        padding: 0.14rem 0.24rem;
    }

    .mode-buttons {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.12rem;
    }

    .mode-buttons button,
    .reset-button {
        border: 1px solid var(--border);
        background: var(--bg-surface);
        color: var(--fg-secondary);
        font-family: var(--font-mono);
        font-size: 0.58rem;
        padding: 0.14rem 0.22rem;
        min-height: 1.56rem;
        cursor: pointer;
    }

    .mode-buttons button.active {
        border-color: var(--hl-symbol);
        color: var(--fg-primary);
        background: rgba(var(--color-accent-rgb), 0.12);
    }

    .pdg-field--check .spire-check {
        width: fit-content;
    }

    .mass-inline {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.14rem;
    }

    .mass-field {
        display: inline-flex;
        align-items: center;
        gap: 0.08rem;
        min-width: 0;
    }

    .mass-field > span {
        font-size: 0.48rem;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        color: var(--fg-secondary);
        white-space: nowrap;
    }

    .mass-inline :global(.spire-number-input) {
        min-width: 0;
    }

    .pdg-field--reset {
        justify-content: flex-end;
    }

    .pdg-facet-row {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 0.22rem;
    }

    .facet-panel {
        border: 1px solid var(--border);
        background: var(--bg-inset);
        padding: 0.18rem 0.22rem;
        display: flex;
        flex-direction: column;
        gap: 0.14rem;
        min-width: 0;
    }

    .facet-panel h5 {
        margin: 0;
        font-size: 0.52rem;
        letter-spacing: 0.05em;
        text-transform: uppercase;
        color: var(--fg-secondary);
        font-family: var(--font-mono);
    }

    .facet-chip-row {
        display: flex;
        flex-wrap: wrap;
        gap: 0.1rem;
    }

    .pdg-stats {
        display: grid;
        grid-template-columns: repeat(5, minmax(0, 1fr));
        gap: 0.2rem;
    }

    .pdg-stats article {
        border: 1px solid var(--border);
        background: var(--bg-inset);
        padding: 0.16rem 0.2rem;
        display: flex;
        flex-direction: column;
        gap: 0.05rem;
        min-width: 0;
    }

    .pdg-stats span,
    .pdg-stats strong,
    .pdg-controls-copy p,
    .pdg-controls-copy h4,
    .pdg-field > span,
    .pdg-field select,
    .facet-panel h5,
    .chip-check {
        font-family: var(--font-mono);
    }

    .pdg-stats span {
        font-size: 0.49rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--fg-secondary);
    }

    .pdg-stats strong {
        font-size: 0.62rem;
        color: var(--fg-primary);
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .spire-check {
        display: inline-flex;
        align-items: center;
        gap: 0.12rem;
        border: 1px solid var(--border);
        background: var(--bg-surface);
        padding: 0.08rem 0.16rem;
        font-size: 0.56rem;
        color: var(--fg-secondary);
        cursor: pointer;
    }

    .spire-check--compact {
        white-space: nowrap;
    }

    .spire-check:hover {
        border-color: var(--hl-symbol);
        color: var(--fg-primary);
    }

    .chip-check {
        background: var(--bg-surface);
        white-space: nowrap;
        padding: 0.06rem 0.14rem;
    }

    .chip-check span {
        line-height: 1.1;
    }

    .spire-check input[type='checkbox'] {
        appearance: none;
        width: 0.68rem;
        height: 0.68rem;
        border: 1px solid var(--border);
        background: var(--bg-inset);
        margin: 0;
        display: grid;
        place-items: center;
    }

    .spire-check input[type='checkbox']::after {
        content: '';
        width: 0.34rem;
        height: 0.34rem;
        background: var(--hl-symbol);
        opacity: 0;
    }

    .spire-check input[type='checkbox']:checked {
        border-color: var(--hl-symbol);
    }

    .spire-check input[type='checkbox']:checked::after {
        opacity: 1;
    }

    .reset-button {
        text-transform: uppercase;
        letter-spacing: 0.05em;
        min-width: 4.4rem;
    }

    .reset-button:hover,
    .mode-buttons button:hover {
        border-color: var(--hl-symbol);
        color: var(--fg-primary);
    }

    .pdg-main-grid {
        display: grid;
        grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
        gap: 0.34rem;
        flex: 1 1 0;
        min-height: 0;
        overflow: hidden;
    }

    .pdg-main-grid.pdg-main-grid--single {
        grid-template-columns: minmax(0, 1fr);
    }

    .pdg-pane {
        border: 1px solid var(--border);
        background: var(--bg-surface);
        padding: 0.32rem;
        min-height: 0;
        overflow: hidden;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .pdg-pane--list {
        --pdg-catalog-columns: minmax(48px, 0.56fr) minmax(54px, 0.66fr) minmax(140px, 2.2fr) minmax(56px, 0.6fr) minmax(56px, 0.6fr) minmax(88px, 0.95fr);
    }

    .pdg-pane h4 {
        margin: 0;
        font-size: 0.68rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--fg-primary);
        border-bottom: 1px solid var(--border);
        padding-bottom: 0.2rem;
    }

    .pdg-pane h4 span {
        color: var(--fg-secondary);
        font-weight: 400;
    }

    .catalog-header {
        display: grid;
        grid-template-columns: var(--pdg-catalog-columns);
        gap: 0.3rem;
        border: 1px solid var(--border);
        background: var(--bg-inset);
        color: var(--fg-secondary);
        text-transform: uppercase;
        letter-spacing: 0.05em;
        font-size: 0.5rem;
        font-weight: 700;
        padding: 0.2rem 0.5rem;
        font-variant-numeric: tabular-nums;
    }

    .catalog-header span:nth-child(1),
    .catalog-header span:nth-child(2),
    .catalog-header span:nth-child(4),
    .catalog-header span:nth-child(5),
    .catalog-header span:nth-child(6) {
        text-align: center;
    }

    .catalog-header span:nth-child(3) {
        text-align: left;
    }

    .pdg-pane--detail {
        overflow-y: auto;
    }

    .edition-banner {
        border: 1px solid var(--hl-value);
        color: var(--hl-value);
        background: color-mix(in srgb, var(--hl-value) 10%, var(--bg-inset));
        padding: 0.28rem 0.38rem;
        font-size: 0.56rem;
    }

    .edition-banner.error {
        border-color: var(--hl-error);
        color: var(--hl-error);
        background: color-mix(in srgb, var(--hl-error) 10%, var(--bg-inset));
    }

    @media (max-width: 1440px) {
        .pdg-control-grid {
            grid-template-columns: minmax(11rem, 1fr) minmax(12rem, 1.35fr) repeat(2, minmax(8rem, 0.84fr)) minmax(10rem, 1fr) minmax(6rem, 0.6fr);
        }

        .pdg-stats {
            grid-template-columns: repeat(3, minmax(0, 1fr));
        }
    }

    @media (max-width: 1180px) {
        .pdg-main-grid,
        .pdg-facet-row,
        .mass-inline,
        .pdg-stats {
            grid-template-columns: 1fr;
        }

        .pdg-control-grid {
            grid-template-columns: repeat(2, minmax(0, 1fr));
        }

        .pdg-facet-row {
            grid-template-columns: 1fr;
        }

        .pdg-pane--list {
            --pdg-catalog-columns: minmax(44px, 0.56fr) minmax(50px, 0.64fr) minmax(120px, 2fr) minmax(52px, 0.58fr) minmax(52px, 0.58fr) minmax(76px, 0.9fr);
        }
    }

    @media (max-width: 840px) {
        .pdg-control-grid,
        .mode-buttons,
        .pdg-stats {
            grid-template-columns: 1fr;
        }

        .pdg-pane--list {
            --pdg-catalog-columns: minmax(40px, 0.54fr) minmax(44px, 0.62fr) minmax(110px, 1.95fr) minmax(48px, 0.56fr) minmax(48px, 0.56fr) minmax(68px, 0.84fr);
        }
    }
</style>
