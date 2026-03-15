<script lang="ts">
  import { tooltip } from "$lib/actions/tooltip";
  import elements from "$lib/data/periodic_table.json";

  interface ElementEntry {
    atomicNumber: number;
    symbol: string;
    name: string;
    period: number;
    group: number;
    category: string;
  }

  const table = elements as ElementEntry[];

  const compositionHints: Record<string, string> = {
    H: "Hydrogen nucleus = proton; neutral atom = proton + electron.",
    He: "Common isotope 4He = 2 protons + 2 neutrons + 2 electrons.",
    C: "Carbon-12 nucleus = 6 protons + 6 neutrons.",
    O: "Oxygen-16 nucleus = 8 protons + 8 neutrons.",
    Fe: "Iron-56 nucleus = 26 protons + 30 neutrons.",
    U: "Uranium nuclei are heavy and neutron-rich; useful for decay chains.",
  };

  function categoryColor(category: string): string {
    switch (category) {
      case "alkali-metal":
        return "var(--hl-error)";
      case "alkaline-earth-metal":
        return "var(--hl-value)";
      case "transition-metal":
        return "var(--hl-symbol)";
      case "metalloid":
        return "#9b59b6";
      case "nonmetal":
        return "var(--hl-success)";
      case "halogen":
        return "#e67e22";
      case "noble-gas":
        return "#1abc9c";
      default:
        return "var(--fg-secondary)";
    }
  }

  function elementTooltip(el: ElementEntry): string {
    const hint = compositionHints[el.symbol] ?? "Composition hint: nucleus contains protons + neutrons; neutral atom also includes electrons.";
    return [
      `${el.name} (${el.symbol})`,
      `Z = ${el.atomicNumber}, period ${el.period}, group ${el.group}`,
      `category: ${el.category}`,
      hint,
    ].join("\n");
  }
</script>

<div class="periodic-wrap">
  {#each table as el (el.atomicNumber)}
    <div
      class="cell"
      style={`grid-column:${el.group};grid-row:${el.period};--accent:${categoryColor(el.category)}`}
      use:tooltip={{ text: elementTooltip(el), maxWidth: 420 }}
      aria-label={`Element ${el.name}`}
    >
      <span class="z">{el.atomicNumber}</span>
      <span class="symbol">{el.symbol}</span>
      <span class="name">{el.name}</span>
    </div>
  {/each}
</div>

<style>
  .periodic-wrap {
    display: grid;
    grid-template-columns: repeat(18, minmax(2.5rem, 1fr));
    gap: 0.2rem;
    overflow: auto;
    padding: 0.2rem;
  }

  .cell {
    border: 1px solid var(--border);
    background: var(--bg-inset);
    min-height: 3.2rem;
    padding: 0.2rem;
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
    border-left: 3px solid var(--accent);
  }

  .z {
    font-size: 0.6rem;
    color: var(--fg-secondary);
  }

  .symbol {
    font-size: 0.9rem;
    font-weight: 700;
    color: var(--fg-primary);
  }

  .name {
    font-size: 0.56rem;
    color: var(--fg-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
