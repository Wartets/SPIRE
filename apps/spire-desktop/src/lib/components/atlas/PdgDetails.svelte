<script lang="ts">
  import type { PdgDecayChannel, PdgDecayTable, PdgParticleRecord, PdgValue } from '$lib/types/spire';
  import { getDecayTable } from '$lib/api';
  import MeasuredValue from '$lib/components/shared/MeasuredValue.svelte';
  import ProvenanceBadge from '$lib/components/shared/ProvenanceBadge.svelte';
  import { draggableParticle } from '$lib/utils/dnd';
  import { formatQuantumFraction } from '$lib/core/physics/fractionFormat';

  export let particle: PdgParticleRecord | null = null;
  export let history: PdgParticleRecord[] = [];

  let activeTab: 'overview' | 'decays' = 'overview';
  let decayPolicy: 'Catalog' | 'StrictPhysical' = 'Catalog';
  let decayTable: PdgDecayTable | null = null;
  let loadingDecays = false;
  let decayError = '';
  let selectedChannelId: number | null = null;
  let showGenericWarning = true;
  let requestId = 0;

  $: normalizedHistory = history.length > 0 ? history : particle ? [particle] : [];
  $: latestRecord = normalizedHistory[0] ?? null;

  $: if (particle?.pdg_id) {
    void loadDecays(particle.pdg_id, decayPolicy);
  } else {
    decayTable = null;
    decayError = '';
    selectedChannelId = null;
  }

  $: genericCount = decayTable?.channels.filter((channel) => channel.is_generic).length ?? 0;

  async function loadDecays(parentPdgId: number, policy: 'Catalog' | 'StrictPhysical'): Promise<void> {
    const currentRequest = ++requestId;
    loadingDecays = true;
    decayError = '';

    try {
      const table = await getDecayTable(parentPdgId, policy);
      if (currentRequest !== requestId) return;
      decayTable = table;
      if (table.channels.length === 0) {
        selectedChannelId = null;
      } else if (!table.channels.some((ch) => ch.mode_id === selectedChannelId)) {
        selectedChannelId = table.channels[0].mode_id;
      }
      showGenericWarning = true;
    } catch (error) {
      if (currentRequest !== requestId) return;
      decayTable = null;
      selectedChannelId = null;
      decayError = error instanceof Error ? error.message : String(error);
    } finally {
      if (currentRequest === requestId) {
        loadingDecays = false;
      }
    }
  }

  function formatQuantumNumber(value: string | number | null | undefined): string {
    if (value === null || value === undefined) return '—';
    if (typeof value === 'number') {
      return formatQuantumFraction(value, { signed: true, maxDenominator: 24, tolerance: 1e-5 });
    }
    return String(value);
  }

  function valueKindToConfidence(kind: 'exact' | 'symmetric' | 'asymmetric'): 'high' | 'medium' | 'low' {
    if (kind === 'exact') return 'high';
    if (kind === 'symmetric') return 'medium';
    return 'low';
  }

  function formatNumericValue(value: number): string {
    if (!Number.isFinite(value)) return '—';
    const fraction = formatQuantumFraction(value, { signed: false, maxDenominator: 24, tolerance: 1e-5 });
    if (fraction.includes('/')) return fraction;
    if (Math.abs(value) >= 1e3 || Math.abs(value) < 1e-4) {
      return value.toExponential(3);
    }
    return value.toPrecision(5);
  }

  function formatPdgValue(value: PdgValue | undefined): string {
    if (!value) return '—';
    if (value.kind === 'exact') return formatNumericValue(value.value);
    if (value.kind === 'symmetric') return `${formatNumericValue(value.value)} ± ${formatNumericValue(value.error)}`;
    return `${formatNumericValue(value.value)} -${formatNumericValue(value.error.minus)} +${formatNumericValue(value.error.plus)}`;
  }

  function scalarValue(value: PdgValue | undefined): number | null {
    if (!value) return null;
    return value.value;
  }

  function compareClass(reference: number | null, probe: number | null): 'same' | 'higher' | 'lower' | 'missing' {
    if (reference === null || probe === null) return 'missing';
    const diff = probe - reference;
    if (Math.abs(diff) < 1e-10) return 'same';
    return diff > 0 ? 'higher' : 'lower';
  }

  function compareText(reference: number | null, probe: number | null): string {
    const kind = compareClass(reference, probe);
    if (kind === 'same') return 'matches latest';
    if (kind === 'higher') return 'higher than latest';
    if (kind === 'lower') return 'lower than latest';
    return 'not available';
  }

  function deltaText(reference: number | null, probe: number | null): string {
    if (reference === null || probe === null) return 'n/a';
    const delta = probe - reference;
    if (Math.abs(delta) < 1e-12) return '0';
    const sign = delta > 0 ? '+' : '−';
    const absDelta = Math.abs(delta);
    const compact = absDelta >= 1e3 || absDelta < 1e-4
      ? absDelta.toExponential(2)
      : absDelta.toPrecision(3);
    return `${sign}${compact}`;
  }

  function formatChannel(channel: PdgDecayChannel): string {
    const terms = channel.products.map(([product, multiplicity]) => {
      const head = multiplicity > 1 ? `${multiplicity}×` : '';
      if (product.kind === 'concrete') {
        return `${head}${product.mcid}`;
      }
      return `${head}${product.description}`;
    });

    if (channel.description?.trim()) {
      return `${terms.join(' + ')} • ${channel.description}`;
    }
    return terms.join(' + ');
  }
</script>

{#if particle}
  <div class="pdg-details">
    <header class="detail-header">
      <div class="header-main">
        <h2 class="particle-name">{particle.label ?? 'Unknown particle'}</h2>
        <span class="particle-id">PDG:{particle.pdg_id}</span>
      </div>
      <button
        class="drag-chip"
        use:draggableParticle={{
          type: 'pdg-particle',
          pdgId: particle.pdg_id,
          label: particle.label,
          edition: particle.provenance.edition,
          sourceId: particle.provenance.source_id,
        }}
        title="Drag into Reaction Workspace or Decay Calculator"
      >
        Drag
      </button>
    </header>

    <div class="tab-bar" role="tablist" aria-label="Particle detail tabs">
      <button class:active={activeTab === 'overview'} on:click={() => (activeTab = 'overview')}>Overview</button>
      <button class:active={activeTab === 'decays'} on:click={() => (activeTab = 'decays')}>Decays</button>
    </div>

    <div class="tab-content">
      {#if activeTab === 'overview'}
        <div class="overview-grid">
          <section class="section">
            <h3>Quantum Numbers</h3>
            <table class="properties-table">
              <tbody>
                <tr>
                  <td>Charge</td>
                  <td>{formatQuantumNumber(particle.quantum_numbers.charge)}</td>
                </tr>
                <tr><td>Spin J</td><td>{formatQuantumNumber(particle.quantum_numbers.spin_j)}</td></tr>
                <tr><td>Parity</td><td>{formatQuantumNumber(particle.quantum_numbers.parity)}</td></tr>
                <tr><td>C-Parity</td><td>{formatQuantumNumber(particle.quantum_numbers.c_parity)}</td></tr>
              </tbody>
            </table>
          </section>

          <section class="section">
            <h3>Provenance</h3>
            <table class="properties-table">
              <tbody>
                <tr><td>Edition</td><td>{particle.provenance.edition}</td></tr>
                <tr><td>Source ID</td><td>{particle.provenance.source_id}</td></tr>
                <tr><td>Fingerprint</td><td>{particle.provenance.fingerprint}</td></tr>
              </tbody>
            </table>
          </section>
        </div>

        <div class="measurements-section">
          {#if particle.mass}
            <div class="measurement-row">
              <div class="measurement-label">Mass</div>
              <div class="measurement-value">
                <MeasuredValue value={particle.mass} showSource={false} />
              </div>
              <ProvenanceBadge source="PDG" confidence={valueKindToConfidence(particle.mass.kind)} />
            </div>
          {/if}

          {#if particle.width}
            <div class="measurement-row">
              <div class="measurement-label">Width</div>
              <div class="measurement-value">
                <MeasuredValue value={particle.width} showSource={false} />
              </div>
              <ProvenanceBadge source="PDG" confidence={valueKindToConfidence(particle.width.kind)} />
            </div>
          {/if}

          {#if particle.lifetime}
            <div class="measurement-row">
              <div class="measurement-label">Lifetime</div>
              <div class="measurement-value">
                <MeasuredValue value={particle.lifetime} showSource={false} />
              </div>
              <ProvenanceBadge source="PDG" confidence={valueKindToConfidence(particle.lifetime.kind)} />
            </div>
          {/if}

          {#if !particle.mass && !particle.width && !particle.lifetime}
            <div class="empty-state"><p>No measurements available</p></div>
          {/if}
        </div>

        {#if normalizedHistory.length > 1 && latestRecord}
          <section class="section compare-section">
            <h3>Edition Compare (latest vs archived)</h3>
            <div class="compare-cards">
              <article>
                <span>Mass</span>
                <strong>{formatPdgValue(latestRecord.mass)} → {formatPdgValue(particle.mass)}</strong>
                <em class={compareClass(scalarValue(latestRecord.mass), scalarValue(particle.mass))}>Δ {deltaText(scalarValue(latestRecord.mass), scalarValue(particle.mass))}</em>
              </article>
              <article>
                <span>Width</span>
                <strong>{formatPdgValue(latestRecord.width)} → {formatPdgValue(particle.width)}</strong>
                <em class={compareClass(scalarValue(latestRecord.width), scalarValue(particle.width))}>Δ {deltaText(scalarValue(latestRecord.width), scalarValue(particle.width))}</em>
              </article>
              <article>
                <span>Lifetime</span>
                <strong>{formatPdgValue(latestRecord.lifetime)} → {formatPdgValue(particle.lifetime)}</strong>
                <em class={compareClass(scalarValue(latestRecord.lifetime), scalarValue(particle.lifetime))}>Δ {deltaText(scalarValue(latestRecord.lifetime), scalarValue(particle.lifetime))}</em>
              </article>
            </div>
            <div class="compare-legend">
              <span class="same">same = matches latest</span>
              <span class="higher">higher = above latest</span>
              <span class="lower">lower = below latest</span>
              <span class="missing">missing = unavailable</span>
            </div>
            <div class="compare-grid compare-grid--header">
              <span>Edition</span>
              <span>Mass</span>
              <span>Width</span>
              <span>Lifetime</span>
            </div>
            <div class="compare-grid compare-grid--latest">
              <strong>{latestRecord.provenance.edition} (latest)</strong>
              <span>{formatPdgValue(latestRecord.mass)}</span>
              <span>{formatPdgValue(latestRecord.width)}</span>
              <span>{formatPdgValue(latestRecord.lifetime)}</span>
            </div>
            {#each normalizedHistory.slice(1) as revision (`${revision.provenance.edition}-${revision.provenance.source_id}`)}
              <div class="compare-grid">
                <strong>{revision.provenance.edition} · {revision.provenance.source_id}</strong>
                <span class={compareClass(scalarValue(latestRecord.mass), scalarValue(revision.mass))} title={compareText(scalarValue(latestRecord.mass), scalarValue(revision.mass))}>{formatPdgValue(revision.mass)}</span>
                <span class={compareClass(scalarValue(latestRecord.width), scalarValue(revision.width))} title={compareText(scalarValue(latestRecord.width), scalarValue(revision.width))}>{formatPdgValue(revision.width)}</span>
                <span class={compareClass(scalarValue(latestRecord.lifetime), scalarValue(revision.lifetime))} title={compareText(scalarValue(latestRecord.lifetime), scalarValue(revision.lifetime))}>{formatPdgValue(revision.lifetime)}</span>
              </div>
            {/each}
          </section>
        {/if}
      {:else}
        <div class="decay-tools">
          <label>
            Extraction
            <select bind:value={decayPolicy}>
              <option value="Catalog">Catalog</option>
              <option value="StrictPhysical">Strict physical</option>
            </select>
          </label>
          {#if decayTable}
            <span class="decay-stats">channels:{decayTable.channels.length}</span>
          {/if}
        </div>

        {#if showGenericWarning && genericCount > 0 && decayPolicy === 'Catalog'}
          <div class="warning-banner" role="status">
            <span>{genericCount} generic channels are shown and may be non-samplable.</span>
            <button on:click={() => (showGenericWarning = false)}>dismiss</button>
          </div>
        {/if}

        {#if loadingDecays}
          <p class="empty-state">Loading decay channels…</p>
        {:else if decayError}
          <p class="error-state">{decayError}</p>
        {:else if decayTable && decayTable.channels.length > 0}
          <div class="decay-list" role="listbox" aria-label="Decay channels">
            {#each decayTable.channels as channel}
              <button
                class="decay-row"
                class:selected={selectedChannelId === channel.mode_id}
                on:click={() => (selectedChannelId = channel.mode_id)}
              >
                <span class="mode-id">#{channel.mode_id}</span>
                <span class="mode-desc">{formatChannel(channel)}</span>
                {#if channel.is_generic}
                  <span class="generic-tag">generic</span>
                {/if}
                <span class="branching">{formatPdgValue(channel.branching_ratio)}</span>
              </button>
            {/each}
          </div>
        {:else}
          <p class="empty-state">No decay channels available for this policy.</p>
        {/if}
      {/if}
    </div>
  </div>
{/if}

<style>
  .pdg-details {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    height: 100%;
    min-height: 0;
    overflow: hidden;
    font-family: var(--font-mono);
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--spacing-sm);
    padding-bottom: var(--spacing-sm);
    border-bottom: 1px solid var(--border);
  }

  .header-main {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .particle-name {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--fg-primary);
  }

  .particle-id {
    font-size: var(--text-xs);
    color: var(--hl-symbol);
    font-variant-numeric: tabular-nums;
  }

  .drag-chip {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-secondary);
    padding: 0.25rem 0.55rem;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    text-transform: uppercase;
  }

  :global(.drag-chip[data-dragging='true']) {
    opacity: 0.55;
  }

  .tab-bar {
    display: flex;
    gap: 0;
    border: 1px solid var(--border);
  }

  .tab-bar button {
    flex: 1;
    border: none;
    border-right: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    padding: 0.35rem 0.5rem;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    text-transform: uppercase;
  }

  .tab-bar button:last-child {
    border-right: none;
  }

  .tab-bar button.active {
    color: var(--fg-primary);
    background: var(--bg-surface);
  }

  .tab-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);
    padding-right: 0.1rem;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
    gap: var(--spacing-sm);
  }

  .section {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: var(--spacing-sm);
  }

  .section h3 {
    margin: 0 0 var(--spacing-sm) 0;
    font-size: var(--text-xs);
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }

  .properties-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-xs);
  }

  .properties-table td {
    border-bottom: 1px solid var(--border);
    padding: 0.22rem 0;
    font-variant-numeric: tabular-nums;
  }

  .properties-table tr td:first-child {
    color: var(--fg-secondary);
    width: 42%;
  }

  .properties-table tr td:last-child {
    color: var(--fg-primary);
    text-align: right;
  }

  .measurements-section {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .compare-section {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
  }

  .compare-cards {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 0.2rem;
  }

  .compare-cards article {
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 0.18rem 0.22rem;
    display: flex;
    flex-direction: column;
    gap: 0.08rem;
    min-width: 0;
  }

  .compare-cards span {
    font-size: 0.52rem;
    color: var(--fg-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .compare-cards strong {
    font-size: 0.62rem;
    color: var(--fg-primary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .compare-cards em {
    font-style: normal;
    font-size: 0.56rem;
    font-variant-numeric: tabular-nums;
  }

  .compare-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem;
    font-size: 0.54rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .compare-legend span {
    border: 1px solid var(--border);
    padding: 0.08rem 0.2rem;
    background: var(--bg-surface);
  }

  .compare-grid {
    display: grid;
    grid-template-columns: minmax(9rem, 1.3fr) repeat(3, minmax(0, 1fr));
    gap: 0.22rem;
    align-items: center;
    font-size: var(--text-xs);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 0.2rem 0.25rem;
  }

  .compare-grid--header {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--fg-secondary);
    background: transparent;
  }

  .compare-grid--latest {
    border-color: var(--hl-symbol);
    background: color-mix(in srgb, var(--hl-symbol) 8%, var(--bg-inset));
  }

  .compare-grid .same {
    color: var(--fg-primary);
  }

  .compare-grid .higher {
    color: var(--hl-symbol);
  }

  .compare-grid .lower {
    color: var(--hl-value);
  }

  .compare-grid .missing {
    color: var(--fg-secondary);
    opacity: 0.8;
  }

  @media (max-width: 980px) {
    .compare-cards {
      grid-template-columns: 1fr;
    }
  }

  .measurement-row {
    display: grid;
    grid-template-columns: 88px 1fr auto;
    gap: var(--spacing-sm);
    align-items: center;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: var(--spacing-sm);
  }

  .measurement-label {
    color: var(--fg-secondary);
    font-size: var(--text-xs);
    text-transform: uppercase;
  }

  .measurement-value {
    color: var(--fg-primary);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .decay-tools {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-sm);
    border: 1px solid var(--border);
    background: var(--bg-inset);
    padding: var(--spacing-sm);
    font-size: var(--text-xs);
    color: var(--fg-secondary);
  }

  .decay-tools select {
    margin-left: 0.4rem;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 0.18rem 0.28rem;
  }

  .decay-stats {
    color: var(--fg-secondary);
    font-variant-numeric: tabular-nums;
  }

  .warning-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid var(--hl-value);
    color: var(--hl-value);
    background: color-mix(in srgb, var(--bg-inset) 88%, var(--hl-value));
    padding: var(--spacing-sm);
    font-size: var(--text-xs);
  }

  .warning-banner button {
    border: 1px solid var(--hl-value);
    background: transparent;
    color: inherit;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    text-transform: uppercase;
    padding: 0.12rem 0.3rem;
  }

  .decay-list {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    background: var(--bg-inset);
  }

  .decay-row {
    display: grid;
    grid-template-columns: 54px 1fr auto 150px;
    align-items: center;
    gap: 0.4rem;
    text-align: left;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    padding: 0.35rem 0.45rem;
  }

  .decay-row:last-child {
    border-bottom: none;
  }

  .decay-row.selected {
    background: color-mix(in srgb, var(--hl-symbol) 12%, var(--bg-inset));
  }

  .mode-id {
    color: var(--hl-symbol);
    font-variant-numeric: tabular-nums;
  }

  .mode-desc {
    color: var(--fg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .generic-tag {
    border: 1px solid var(--hl-value);
    color: var(--hl-value);
    padding: 0.04rem 0.25rem;
    text-transform: uppercase;
    font-size: 0.62rem;
  }

  .branching {
    text-align: right;
    color: var(--hl-value);
    font-variant-numeric: tabular-nums;
  }

  .empty-state,
  .error-state {
    margin: 0;
    padding: var(--spacing-sm);
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-secondary);
    font-size: var(--text-xs);
  }

  .error-state {
    border-color: var(--hl-error);
    color: var(--hl-error);
  }
</style>
