<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { Field } from "$lib/types/spire";
  import { tooltip } from "$lib/actions/tooltip";

  export let particle: Field;
  export let selectable = false;

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
    if (!selectable) return;
    dispatch("select", particle);
  }
</script>

<button
  class="particle-cell"
  class:selectable
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

  .particle-cell.selectable {
    cursor: pointer;
  }

  .particle-cell.selectable:hover {
    border-color: var(--hl-symbol);
    background: rgba(var(--color-accent-rgb), 0.08);
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
