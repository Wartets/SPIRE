<script lang="ts">
  import type { PdgParticleRecord } from '$lib/types/spire';
  import MeasuredValue from '$lib/components/shared/MeasuredValue.svelte';
  import ProvenanceBadge from '$lib/components/shared/ProvenanceBadge.svelte';

  export let particle: PdgParticleRecord | null = null;

  function addToActiveModel() {
    console.log('Add to active model:', particle?.pdg_id);
  }

  function formatQuantumNumber(value: string | number | null | undefined): string {
    if (value === null || value === undefined) return '—';
    return String(value);
  }

  function valueKindToConfidence(kind: 'exact' | 'symmetric' | 'asymmetric'): 'high' | 'medium' | 'low' {
    if (kind === 'exact') return 'high';
    if (kind === 'symmetric') return 'medium';
    return 'low';
  }
</script>

{#if particle}
  <div class="pdg-details">
    <div class="detail-header">
      <div class="header-main">
        <h2 class="particle-name">{particle.label ?? 'Unknown particle'}</h2>
        <span class="particle-id">PDG ID: {particle.pdg_id}</span>
      </div>
      <button class="add-button" on:click={addToActiveModel}>
        + Add to Active Model
      </button>
    </div>

    <div class="tab-content">
      <div class="overview-grid">
        <div class="section">
          <h3>Quantum Numbers</h3>
          <table class="properties-table">
            <tbody>
              <tr><td>Charge</td><td>{formatQuantumNumber(particle.quantum_numbers.charge)}</td></tr>
              <tr><td>Spin J</td><td>{formatQuantumNumber(particle.quantum_numbers.spin_j)}</td></tr>
              <tr><td>Parity</td><td>{formatQuantumNumber(particle.quantum_numbers.parity)}</td></tr>
              <tr><td>C-Parity</td><td>{formatQuantumNumber(particle.quantum_numbers.c_parity)}</td></tr>
            </tbody>
          </table>
        </div>

        <div class="section">
          <h3>Provenance</h3>
          <table class="properties-table">
            <tbody>
              <tr><td>Edition</td><td>{particle.provenance.edition}</td></tr>
              <tr><td>Source ID</td><td>{particle.provenance.source_id}</td></tr>
              <tr><td>Fingerprint</td><td>{particle.provenance.fingerprint}</td></tr>
            </tbody>
          </table>
        </div>
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
    </div>
  </div>
{/if}

<style>
  .pdg-details {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
    overflow: hidden;
  }

  .detail-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    padding-bottom: 0.5rem;
    border-bottom: 2px solid var(--primary, #1976d2);
  }

  .header-main {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .particle-name {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--on-surface, #000);
  }

  .particle-id {
    font-size: 0.85rem;
    color: var(--on-surface-variant, #666);
    font-family: monospace;
  }

  .add-button {
    padding: 0.75rem 1.5rem;
    background: var(--primary, #1976d2);
    color: white;
    border: none;
    border-radius: 4px;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.2s ease;
    white-space: nowrap;
  }

  .add-button:hover {
    background: var(--primary-dark, #1565c0);
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem 0;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2rem;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--on-surface, #000);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .properties-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
  }

  .properties-table tr {
    border-bottom: 1px solid var(--outline, #ddd);
  }

  .properties-table td {
    padding: 0.5rem;
  }

  .properties-table tr td:first-child {
    font-weight: 500;
    color: var(--on-surface-variant, #666);
    width: 40%;
  }

  .properties-table tr td:last-child {
    color: var(--on-surface, #000);
    font-family: monospace;
  }

  .measurements-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .measurement-row {
    display: grid;
    grid-template-columns: 120px 1fr 200px;
    gap: 1rem;
    align-items: center;
    padding: 0.75rem;
    background: var(--surface-variant, #f5f5f5);
    border-radius: 4px;
  }

  .measurement-label {
    font-weight: 600;
    color: var(--on-surface-variant, #666);
  }

  .measurement-value {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .empty-state {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--on-surface-variant, #999);
  }
</style>
