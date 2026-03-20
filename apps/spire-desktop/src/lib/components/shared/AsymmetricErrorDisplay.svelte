<script lang="ts">
  export let upper: number = 0;
  export let lower: number = 0;
  export let central: number = 0;

  // Compute visual bar: upper/lower as fraction of central (max 100%)
  $: upperPercent = central > 0 ? Math.min((upper / central) * 100, 100) : 0;
  $: lowerPercent = central > 0 ? Math.min((lower / central) * 100, 100) : 0;

  function formatNumber(n: number): string {
    if (n < 0.001) {
      return n.toExponential(2);
    }
    return n.toFixed(3);
  }
</script>

<div class="error-display">
  <span class="error-label lower">-{formatNumber(lower)}</span>
  <div class="bar-container">
    <div class="bar-track">
      <div class="bar-lower" style="width: {lowerPercent}%"></div>
      <div class="bar-center"></div>
      <div class="bar-upper" style="width: {upperPercent}%"></div>
    </div>
  </div>
  <span class="error-label upper">+{formatNumber(upper)}</span>
</div>

<style>
  .error-display {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    font-family: 'Courier New', monospace;
  }

  .error-label {
    white-space: nowrap;
    min-width: 60px;
    text-align: right;
  }

  .error-label.lower {
    text-align: right;
    color: var(--warning, #ff9800);
  }

  .error-label.upper {
    text-align: left;
    color: var(--success, #4caf50);
  }

  .bar-container {
    flex: 1;
    min-width: 100px;
  }

  .bar-track {
    display: flex;
    align-items: center;
    height: 12px;
    background: var(--surface-variant, #e0e0e0);
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-lower {
    height: 100%;
    background: linear-gradient(90deg, var(--warning, #ff9800), transparent);
    transition: width 0.2s ease;
  }

  .bar-center {
    width: 2px;
    height: 100%;
    background: var(--on-surface, #000);
    opacity: 0.5;
  }

  .bar-upper {
    height: 100%;
    background: linear-gradient(90deg, transparent, var(--success, #4caf50));
    transition: width 0.2s ease;
  }
</style>
