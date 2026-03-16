<script lang="ts">
  import { afterUpdate, onMount } from "svelte";

  export let latex = "";
  export let mode: "rendered" | "raw" = "rendered";
  export let block = true;

  let host: HTMLElement | null = null;
  let hasRenderedMath = false;
  let fallbackActive = false;
  let renderError = "";

  function setFallbackText(): void {
    if (!host) return;
    host.textContent = latex;
    fallbackActive = true;
    hasRenderedMath = false;
  }

  async function typesetIfAvailable(): Promise<void> {
    if (!host || mode !== "rendered") return;
    if (!latex.trim()) {
      host.textContent = "";
      fallbackActive = false;
      hasRenderedMath = false;
      renderError = "";
      return;
    }

    const mathJax = (window as Window & {
      MathJax?: {
        typesetPromise?: (elements?: Element[]) => Promise<void>;
      };
    }).MathJax;

    if (!mathJax?.typesetPromise) {
      setFallbackText();
      return;
    }

    fallbackActive = false;
    host.textContent = block ? `\\[${latex}\\]` : `\\(${latex}\\)`;

    try {
      await mathJax.typesetPromise([host]);
      hasRenderedMath = true;
      renderError = "";
    } catch (error: unknown) {
      renderError = error instanceof Error ? error.message : String(error);
      setFallbackText();
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
  <div class:math-block={block} class:latex-fallback={fallbackActive} class="latex-render" bind:this={host} aria-label="Mathematical expression">
    {latex}
  </div>
  {#if renderError && !hasRenderedMath}
    <div class="latex-note">Rendering fallback active</div>
  {/if}
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

  .latex-render.latex-fallback {
    font-family: var(--font-mono);
    color: var(--hl-value);
  }

  .latex-note {
    margin-top: 0.25rem;
    font-size: 0.64rem;
    color: var(--fg-secondary);
    font-style: italic;
  }
</style>
