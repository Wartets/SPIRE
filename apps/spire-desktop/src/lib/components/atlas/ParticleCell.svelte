<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Field } from "$lib/types/spire";
  import { tooltip } from "$lib/actions/tooltip";

  export let particle: Field;
  export let selectable = false;
  export let flashing = false;

  const dispatch = createEventDispatcher<{ select: Field }>();

  function qnSummary(field: Field): string {
    const qn = field.quantum_numbers;
    return [
      `${field.name} (${field.id})`,
      `symbol: ${field.symbol}`,
      `mass: ${field.mass.toExponential(3)} GeV`,
      `width: ${field.width.toExponential(3)} GeV`,
      `Q: ${qn.electric_charge}, 2s: ${qn.spin}, color: ${qn.color}`,
      `T3: ${qn.weak_isospin}, Y: ${qn.hypercharge}, B: ${qn.baryon_number}`,
      `Le/Lmu/Ltau: ${qn.lepton_numbers.electron}/${qn.lepton_numbers.muon}/${qn.lepton_numbers.tau}`,
    ].join("\n");
  }

  function chargeLabel(q: number): string {
    if (Math.abs(q) < 1e-12) return "0";
    return q > 0 ? `+${q}` : `${q}`;
  }

  function familyLabel(field: Field): string {
    const qn = field.quantum_numbers;
    if (qn.color === "Triplet" || qn.color === "AntiTriplet" || qn.color === "Octet") return "colored";
    if (Math.abs(qn.baryon_number) > 0) return "baryonic";
    if (qn.lepton_numbers.electron || qn.lepton_numbers.muon || qn.lepton_numbers.tau) return "leptonic";
    if (qn.spin === 0) return "scalar";
    if (qn.spin === 2) return "vector";
    return "field";
  }

  function handleClick(): void {
    dispatch("select", particle);
  }
</script>

<button
  class="particle-cell"
  class:selectable
  class:flashing
  on:click={handleClick}
  use:tooltip={{ text: qnSummary(particle), maxWidth: 460 }}
  aria-label={`Particle ${particle.name}`}
>
  <span class="symbol">{particle.symbol}</span>
  <span class="id">{particle.id}</span>
  <span class="meta">m={particle.mass.toPrecision(3)} GeV • Γ={particle.width.toPrecision(2)} GeV</span>
  <div class="badges">
    <span class="badge badge-charge">Q={chargeLabel(particle.quantum_numbers.electric_charge)}</span>
    <span class="badge badge-spin">s={particle.quantum_numbers.spin / 2}</span>
    <span class="badge badge-family">{familyLabel(particle)}</span>
  </div>
</button>

<style>
  .particle-cell {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.15rem;
    width: 100%;
    text-align: left;
    border: 1px solid var(--border);
    background: var(--bg-inset);
    color: var(--fg-primary);
    padding: 0.4rem;
    font-family: var(--font-mono);
  }

  .particle-cell {
    cursor: pointer;
  }

  .particle-cell:hover {
    border-color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.08);
  }

  .particle-cell.flashing {
    animation: flash-pulse 0.8s ease-out;
  }

  @keyframes flash-pulse {
    0%   { border-color: var(--hl-success, #4ade80); box-shadow: 0 0 6px var(--hl-success, #4ade80); }
    60%  { border-color: var(--hl-success, #4ade80); box-shadow: 0 0 2px var(--hl-success, #4ade80); }
    100% { border-color: var(--border); box-shadow: none; }
  }

  .symbol {
    font-size: 0.95rem;
    color: var(--hl-symbol);
    font-weight: 700;
  }

  .id {
    font-size: 0.66rem;
    color: var(--fg-secondary);
  }

  .meta {
    font-size: 0.64rem;
    color: var(--fg-secondary);
  }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 0.18rem;
    margin-top: 0.05rem;
  }

  .badge {
    font-size: 0.57rem;
    color: var(--fg-secondary);
    border: 1px solid var(--border);
    padding: 0.03rem 0.2rem;
  }

  .badge-charge {
    color: var(--hl-value);
    border-color: color-mix(in srgb, var(--hl-value) 55%, var(--border));
  }

  .badge-spin {
    color: var(--hl-symbol);
    border-color: color-mix(in srgb, var(--hl-symbol) 55%, var(--border));
  }

  .badge-family {
    color: var(--hl-success);
    border-color: color-mix(in srgb, var(--hl-success) 55%, var(--border));
  }
</style>
