<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import SpireSlider from "$lib/components/ui/SpireSlider.svelte";
  import type { PipelineNode } from "$lib/stores/pipelineGraphStore";
  import {
    NODE_DEFINITIONS,
    type PipelineParameterSchema,
    type PipelineParameterValue,
  } from "$lib/core/pipeline/graph";

  export let node: PipelineNode | null = null;
  export let modelOptions: string[] = [];
  export let selectedEdgeId: string | null = null;
  export let selectedEdgePayload: unknown = null;
  export let selectedEdgeState: { status?: string; error?: string } | null = null;

  const dispatch = createEventDispatcher<{
    paramchange: { nodeId: string; key: string; value: PipelineParameterValue };
  }>();

  function schemaForNode(selected: PipelineNode | null): PipelineParameterSchema[] {
    if (!selected) return [];
    return NODE_DEFINITIONS[selected.type].parameterSchema;
  }

  function currentValue(
    selected: PipelineNode,
    key: string,
    fallback: PipelineParameterValue,
  ): PipelineParameterValue {
    return selected.parameters[key] ?? fallback;
  }

  function update(nodeId: string, key: string, value: PipelineParameterValue): void {
    dispatch("paramchange", { nodeId, key, value });
  }

  $: schemas = schemaForNode(node);
  $: payloadJson = selectedEdgePayload === undefined
    ? "undefined"
    : JSON.stringify(selectedEdgePayload, null, 2);
</script>

<aside class="pipeline-inspector" aria-label="Pipeline node inspector">
  {#if node}
    <header class="inspector-header">
      <h3>{node.label}</h3>
      <p>{NODE_DEFINITIONS[node.type].description}</p>
    </header>

    <div class="inspector-fields">
      {#each schemas as schema (schema.key)}
        <label class="field" for={`${node.id}-${schema.key}`}>
          <span class="field-label">{schema.label}</span>

          {#if schema.control === "number"}
            <SpireNumberInput
              inputId={`${node.id}-${schema.key}`}
              value={Number(currentValue(node, schema.key, schema.defaultValue))}
              min={schema.min}
              max={schema.max}
              step={schema.step ?? 1}
              ariaLabel={schema.label}
              on:blur={(event) => {
                const target = event.currentTarget as HTMLInputElement | null;
                if (!target) return;
                const parsed = Number(target.value);
                if (!Number.isFinite(parsed)) return;
                update(node.id, schema.key, parsed);
              }}
            />
          {:else if schema.control === "slider"}
            <SpireSlider
              inputId={`${node.id}-${schema.key}`}
              value={Number(currentValue(node, schema.key, schema.defaultValue))}
              min={schema.min ?? 0}
              max={schema.max ?? 100}
              step={schema.step ?? 1}
              ariaLabel={schema.label}
              on:input={(event) => {
                const target = event.currentTarget as HTMLInputElement | null;
                if (!target) return;
                const parsed = Number(target.value);
                if (!Number.isFinite(parsed)) return;
                update(node.id, schema.key, parsed);
              }}
            />
            <span class="field-value">{Number(currentValue(node, schema.key, schema.defaultValue)).toFixed(2)}</span>
          {:else if schema.control === "toggle"}
            <button
              id={`${node.id}-${schema.key}`}
              class="toggle"
              class:active={Boolean(currentValue(node, schema.key, schema.defaultValue))}
              type="button"
              on:click={() =>
                update(
                  node.id,
                  schema.key,
                  !Boolean(currentValue(node, schema.key, schema.defaultValue)),
                )}
            >
              {Boolean(currentValue(node, schema.key, schema.defaultValue)) ? "Enabled" : "Disabled"}
            </button>
          {:else if schema.control === "select"}
            <select
              id={`${node.id}-${schema.key}`}
              class="select"
              value={String(currentValue(node, schema.key, schema.defaultValue))}
              on:change={(event) => {
                const target = event.currentTarget as HTMLSelectElement;
                update(node.id, schema.key, target.value);
              }}
            >
              {#if schema.key === "modelName" && modelOptions.length > 0}
                {#each modelOptions as option}
                  <option value={option}>{option}</option>
                {/each}
              {:else}
                {#each schema.options ?? [] as option}
                  <option value={option.value}>{option.label}</option>
                {/each}
              {/if}
            </select>
          {:else if schema.control === "textarea"}
            <textarea
              id={`${node.id}-${schema.key}`}
              class="textarea"
              value={String(currentValue(node, schema.key, schema.defaultValue))}
              spellcheck="false"
              on:change={(event) => {
                const target = event.currentTarget as HTMLTextAreaElement;
                update(node.id, schema.key, target.value);
              }}
            ></textarea>
          {:else}
            <input
              id={`${node.id}-${schema.key}`}
              class="text"
              type="text"
              value={String(currentValue(node, schema.key, schema.defaultValue))}
              on:change={(event) => {
                const target = event.currentTarget as HTMLInputElement;
                update(node.id, schema.key, target.value);
              }}
            />
          {/if}

          {#if schema.help}
            <span class="field-help">{schema.help}</span>
          {/if}
        </label>
      {/each}
    </div>
  {:else}
    <header class="inspector-header">
      <h3>Node Inspector</h3>
      <p>Select a pipeline node to configure its parameters.</p>
    </header>
  {/if}

  {#if selectedEdgeId}
    <section class="payload-section">
      <header class="payload-header">
        <h4>Wire Payload</h4>
        <div class="payload-meta">{selectedEdgeId}</div>
      </header>
      <div class="payload-status">
        Status: <strong>{selectedEdgeState?.status ?? "unknown"}</strong>
        {#if selectedEdgeState?.error}
          <span class="payload-error">Error: {selectedEdgeState.error}</span>
        {/if}
      </div>
      <pre class="payload-json">{payloadJson}</pre>
    </section>
  {/if}
</aside>

<style>
  .pipeline-inspector {
    position: absolute;
    top: 0.6rem;
    right: 0.6rem;
    width: min(24rem, 38vw);
    max-height: calc(100% - 1.2rem);
    overflow: auto;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    background: color-mix(in oklab, var(--color-bg-base) 82%, black 18%);
    box-shadow: var(--shadow-elevated);
    z-index: 8;
    pointer-events: auto;
  }

  .inspector-header {
    padding: 0.6rem 0.7rem;
    border-bottom: 1px solid var(--color-border);
  }

  .inspector-header h3 {
    margin: 0;
    font-size: var(--text-sm);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .inspector-header p {
    margin: 0.35rem 0 0;
    color: var(--color-text-muted);
    font-size: var(--text-xs);
    line-height: 1.4;
  }

  .inspector-fields {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    padding: 0.6rem 0.7rem 0.8rem;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.26rem;
  }

  .field-label {
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    letter-spacing: 0.03em;
  }

  .field-help {
    color: var(--color-text-muted);
    font-size: 0.63rem;
  }

  .field-value {
    color: var(--color-text-muted);
    font-size: 0.66rem;
    font-family: var(--font-mono);
  }

  .text,
  .select,
  .textarea {
    border: 1px solid var(--color-border);
    background: var(--color-bg-inset);
    color: var(--color-text-primary);
    padding: 0.28rem 0.42rem;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .text:focus,
  .select:focus,
  .textarea:focus {
    border-color: var(--color-accent);
    outline: none;
  }

  .textarea {
    min-height: 5.7rem;
    resize: vertical;
    line-height: 1.4;
  }

  .toggle {
    border: 1px solid var(--color-border);
    background: var(--color-bg-surface);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    text-align: left;
    padding: 0.32rem 0.45rem;
  }

  .toggle.active {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: rgba(var(--color-accent-rgb), 0.09);
  }

  .payload-section {
    border-top: 1px solid var(--color-border);
    padding: 0.55rem 0.7rem 0.72rem;
  }

  .payload-header {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    margin-bottom: 0.35rem;
  }

  .payload-header h4 {
    margin: 0;
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .payload-meta {
    color: var(--color-text-muted);
    font-size: 0.62rem;
    font-family: var(--font-mono);
    word-break: break-all;
  }

  .payload-status {
    margin-bottom: 0.35rem;
    color: var(--color-text-muted);
    font-size: 0.64rem;
  }

  .payload-error {
    display: block;
    color: var(--color-error);
    margin-top: 0.16rem;
  }

  .payload-json {
    margin: 0;
    padding: 0.45rem;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-bg-inset);
    color: var(--color-text-primary);
    font-size: 0.62rem;
    line-height: 1.35;
    overflow: auto;
    max-height: 14rem;
    font-family: var(--font-mono);
  }
</style>
