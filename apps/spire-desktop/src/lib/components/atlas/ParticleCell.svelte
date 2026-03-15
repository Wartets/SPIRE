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
  <span class="meta">m={particle.mass.toPrecision(3)} GeV • Q={particle.quantum_numbers.electric_charge}</span>
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
</style>
