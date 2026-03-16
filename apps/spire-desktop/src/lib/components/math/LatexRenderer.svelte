<script lang="ts">
  import { afterUpdate, onMount } from "svelte";

  export let latex = "";
  export let mode: "rendered" | "raw" = "rendered";
  export let block = true;

  let host: HTMLElement | null = null;

  async function typesetIfAvailable(): Promise<void> {
    if (!host || mode !== "rendered") return;
    const mathJax = (window as Window & {
      MathJax?: {
        typesetPromise?: (elements?: Element[]) => Promise<void>;
      };
    }).MathJax;

    if (mathJax?.typesetPromise) {
      await mathJax.typesetPromise([host]);
    }
  }

  onMount(() => {
    void typesetIfAvailable();
  });

  afterUpdate(() => {
    void typesetIfAvailable();
  });
</script>

{#if mode === "raw"}
  <pre class="latex-raw"><code>{latex}</code></pre>
{:else}
  <div class:math-block={block} class="latex-render" bind:this={host}>
    {`\\(${latex}\\)`}
  </div>
{/if}

<style>
  .latex-render {
    color: var(--fg-primary);
    font-size: 0.8rem;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .math-block {
    display: block;
  }

  .latex-raw {
    margin: 0;
    color: var(--hl-value);
    font-size: 0.78rem;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    padding: 0.35rem 0.45rem;
  }

  .latex-raw code {
    font-family: var(--font-mono);
  }
</style>
