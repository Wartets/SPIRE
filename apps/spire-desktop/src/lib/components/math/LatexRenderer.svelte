<script lang="ts">
  import { onMount } from "svelte";
  import "katex/dist/katex.min.css";
  import { executeWorkerTask } from "$lib/workers/executeWorkerTask";

  type KatexRenderResponse = {
    ok: boolean;
    html?: string;
    error?: string;
  };

  const RENDER_CACHE_LIMIT = 600;
  const renderCache = new Map<string, string>();
  let sharedKatexLoadError = "";

  function cacheGet(key: string): string | undefined {
    const existing = renderCache.get(key);
    if (existing === undefined) return undefined;
    renderCache.delete(key);
    renderCache.set(key, existing);
    return existing;
  }

  function cacheSet(key: string, value: string): void {
    if (renderCache.has(key)) {
      renderCache.delete(key);
    }
    renderCache.set(key, value);
    if (renderCache.size > RENDER_CACHE_LIMIT) {
      const oldest = renderCache.keys().next().value;
      if (oldest) {
        renderCache.delete(oldest);
      }
    }
  }

  export let latex = "";
  export let mode: "rendered" | "raw" = "rendered";
  export let block = true;

  let host: HTMLElement | null = null;
  let hasRenderedMath = false;
  let fallbackActive = false;
  let renderError = "";
  let lastRenderKey = "";
  let renderVersion = 0;

  function setFallbackText(): void {
    if (!host) return;
    host.textContent = latex;
    fallbackActive = true;
    hasRenderedMath = false;
  }

  async function typesetIfAvailable(force = false): Promise<void> {
    if (!host) return;
    const target = host;

    const renderKey = `${mode}::${block ? "1" : "0"}::${latex}`;
    if (!force && renderKey === lastRenderKey) return;
    lastRenderKey = renderKey;

    if (mode !== "rendered") {
      fallbackActive = false;
      hasRenderedMath = false;
      renderError = "";
      return;
    }

    const trimmed = latex.trim();
    if (!trimmed) {
      target.textContent = "";
      fallbackActive = false;
      hasRenderedMath = false;
      renderError = "";
      return;
    }

    const cacheKey = `${block ? "block" : "inline"}::${trimmed}`;
    const cachedHtml = cacheGet(cacheKey);
    if (cachedHtml) {
      target.innerHTML = cachedHtml;
      fallbackActive = false;
      hasRenderedMath = true;
      renderError = "";
      return;
    }

    const requestVersion = ++renderVersion;

    try {
      const response = await executeWorkerTask<
        { expression: string; displayMode: boolean },
        KatexRenderResponse
      >(
        new URL("$lib/workers/katexRender.worker.ts", import.meta.url),
        { expression: trimmed, displayMode: block },
      );

      if (requestVersion !== renderVersion || target !== host) return;

      if (!response.ok || !response.html) {
        sharedKatexLoadError = response.error || "KaTeX is not available";
        renderError = sharedKatexLoadError;
        setFallbackText();
        return;
      }

      cacheSet(cacheKey, response.html);
      target.innerHTML = response.html;
      fallbackActive = false;
      hasRenderedMath = true;
      renderError = "";
    } catch (error: unknown) {
      renderError = error instanceof Error ? error.message : String(error);
      setFallbackText();
    }
  }

  onMount(() => {
    void typesetIfAvailable(true);
  });

  $: if (host) {
    void typesetIfAvailable();
  }
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
    overflow-x: auto;
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

  .latex-render :global(.katex-display) {
    margin: 0.3rem 0;
  }

  .latex-render :global(.katex) {
    font-size: 1em;
  }

  .latex-note {
    margin-top: 0.25rem;
    font-size: 0.64rem;
    color: var(--fg-secondary);
    font-style: italic;
  }
</style>
