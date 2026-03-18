<!--
  SPIRE - Widget Container

  Replacement for the old WidgetCell + BaseWidget combination in the
  docking system.  Renders the widget header bar with docking controls
  (split horizontal, split vertical, tear off, close) and delegates
  the inner content to the appropriate physics widget component.

  Props:
    node - The WidgetLeaf from the layout tree.
-->
<script lang="ts">
  import type { WidgetLeaf } from "$lib/stores/layoutStore";
  import { closeNode, splitNode, moveNode, insertWidgetRelative } from "$lib/stores/layoutStore";
  import type { DropPosition } from "$lib/stores/layoutStore";
  import { WIDGET_LABELS } from "$lib/components/workbench/widgetRegistry";
  import { getWidgetComponent } from "$lib/core/registry/WidgetRegistry";
  import UnknownWidget from "$lib/components/shared/UnknownWidget.svelte";
  import { tearOffWidget } from "$lib/core/services/WindowManager";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { pipelineLinks } from "$lib/core/services/PipelineService";
  import { getWidgetContextItems } from "$lib/core/services/widgetContextActions";
  import { tooltip } from "$lib/actions/tooltip";
  import { longpress } from "$lib/actions/longpress";
  import Icon from "$lib/components/ui/Icon.svelte";

  export let node: WidgetLeaf;

  $: label = WIDGET_LABELS[node.widgetType] ?? node.widgetType;
  $: widgetComponent = getWidgetComponent(node.widgetType);
  $: linked = $pipelineLinks.some(
    (l) => l.source.widgetId === node.id || l.sink.widgetId === node.id,
  );

  function handleSplitH(): void {
    splitNode(node.id, "row", node.widgetType);
  }

  function handleSplitV(): void {
    splitNode(node.id, "col", node.widgetType);
  }

  function handleTearOff(): void {
    tearOffWidget(node.widgetType, node.id, label);
  }

  function handleClose(): void {
    closeNode(node.id);
  }

  function openHeaderContextAt(x: number, y: number): void {
    const widgetItems = getWidgetContextItems(node.widgetType);
    showContextMenu(x, y, [
      ...widgetItems,
      ...(widgetItems.length > 0
        ? [{ type: "separator" as const, id: "sep-header-widget" }]
        : []),
      { type: "action", id: "split-h", label: "Split Horizontal", icon: "column", action: handleSplitH },
      { type: "action", id: "split-v", label: "Split Vertical", icon: "row", action: handleSplitV },
      { type: "action", id: "tear-off", label: "Tear Off to Window", icon: "⧉", action: handleTearOff },
      { type: "separator", id: "sep-1" },
      { type: "action", id: "close", label: "Close Widget", icon: "✕", action: handleClose },
    ]);
  }

  function handleHeaderContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    openHeaderContextAt(e.clientX, e.clientY);
  }

  /** Right-click on widget body → show widget-specific items + layout items. */
  function openBodyContextAt(x: number, y: number, target?: EventTarget | null): void {
    const widgetItems = getWidgetContextItems(node.widgetType);
    const selectedText = (window.getSelection?.()?.toString() ?? "").trim();
    const hit = target instanceof HTMLElement ? target : null;
    const anchor = hit?.closest("a") as HTMLAnchorElement | null;
    const href = anchor?.href?.trim() || "";
    const elementLabel = hit?.getAttribute("aria-label") || hit?.getAttribute("title") || hit?.tagName?.toLowerCase();

    const elementItems: import("$lib/types/menu").ContextMenuItem[] = [
      ...(selectedText
        ? [{
            type: "action" as const,
            id: "copy-selected-text",
            label: "Copy Selected Text",
            icon: "TXT",
            action: () => navigator.clipboard.writeText(selectedText),
          }, {
            type: "action" as const,
            id: "search-selected-text",
            label: `Search Web for “${selectedText.slice(0, 44)}${selectedText.length > 44 ? "…" : ""}”`,
            icon: "WEB",
            action: () => {
              const query = encodeURIComponent(selectedText);
              window.open(`https://duckduckgo.com/?q=${query}`, "_blank", "noopener,noreferrer");
            },
          }, {
            type: "action" as const,
            id: "search-selected-arxiv",
            label: "Search arXiv for Selection",
            icon: "arX",
            action: () => {
              const query = encodeURIComponent(selectedText);
              window.open(`https://arxiv.org/search/?query=${query}&searchtype=all`, "_blank", "noopener,noreferrer");
            },
          }]
        : []),
      ...(href
        ? [{
            type: "action" as const,
            id: "open-clicked-link",
            label: "Open Clicked Link",
            icon: "URL",
            action: () => window.open(href, "_blank", "noopener,noreferrer"),
          }, {
            type: "action" as const,
            id: "copy-clicked-link",
            label: "Copy Clicked Link",
            icon: "CPY",
            action: () => navigator.clipboard.writeText(href),
          }]
        : []),
      ...(elementLabel
        ? [{
            type: "action" as const,
            id: "inspect-element",
            label: `Element: ${elementLabel}`,
            icon: "EL",
            action: () => {},
            disabled: true,
          }]
        : []),
    ];

    const layoutItems: import("$lib/types/menu").ContextMenuItem[] = [
      { type: "action", id: "split-h", label: "Split Horizontal", icon: "column", action: handleSplitH },
      { type: "action", id: "split-v", label: "Split Vertical", icon: "row", action: handleSplitV },
      { type: "action", id: "tear-off", label: "Tear Off to Window", icon: "⧉", action: handleTearOff },
      { type: "action", id: "close", label: "Close Widget", icon: "✕", action: handleClose },
    ];
    const items: import("$lib/types/menu").ContextMenuItem[] = [
      ...widgetItems,
      ...(widgetItems.length > 0
        ? [{ type: "separator" as const, id: "sep-body" }]
        : []),
      ...elementItems,
      ...(elementItems.length > 0
        ? [{ type: "separator" as const, id: "sep-body-element" }]
        : []),
      ...layoutItems,
    ];
    showContextMenu(x, y, items);
  }

  function handleBodyContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    openBodyContextAt(e.clientX, e.clientY, e.target);
  }

  // ── Docking Drag-and-Drop ──

  let dropZone: DropPosition | null = null;

  function computeDropZone(rect: DOMRect, clientX: number, clientY: number): Exclude<DropPosition, "center"> {
    const relX = (clientX - rect.left) / Math.max(1, rect.width);
    const relY = (clientY - rect.top) / Math.max(1, rect.height);
    const dx = relX - 0.5;
    const dy = relY - 0.5;

    if (Math.abs(dx) >= Math.abs(dy)) {
      return dx < 0 ? "left" : "right";
    }
    return dy < 0 ? "top" : "bottom";
  }

  function handleDragStart(e: DragEvent): void {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData("text/plain", node.id);
    e.dataTransfer.effectAllowed = "move";
  }

  function handleDragOver(e: DragEvent): void {
    e.preventDefault();
    if (!e.dataTransfer) return;
    e.dataTransfer.dropEffect = "move";

    // Compute drop zone from cursor position within the container.
    // We intentionally avoid an ambiguous "center" zone because the
    // current docking reducer does not support tab-merge semantics.
    // This keeps visual target and final drop position consistent.
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropZone = computeDropZone(rect, e.clientX, e.clientY);
  }

  function handleDragLeave(e: DragEvent): void {
    const container = e.currentTarget as HTMLElement | null;
    const next = e.relatedTarget as Node | null;
    if (container && next && container.contains(next)) {
      return;
    }
    dropZone = null;
  }

  function handleDrop(e: DragEvent): void {
    e.preventDefault();
    const widgetType = e.dataTransfer?.getData("application/x-spire-widget-type");
    if (widgetType && dropZone) {
      insertWidgetRelative(node.id, dropZone, widgetType as import("$lib/stores/notebookStore").WidgetType);
      dropZone = null;
      return;
    }

    const sourceId = e.dataTransfer?.getData("text/plain");
    if (sourceId && sourceId !== node.id && dropZone) {
      moveNode(sourceId, node.id, dropZone);
    }
    dropZone = null;
  }
</script>

<div
  class="widget-container"
  class:drop-left={dropZone === "left"}
  class:drop-right={dropZone === "right"}
  class:drop-top={dropZone === "top"}
  class:drop-bottom={dropZone === "bottom"}
  class:drop-center={dropZone === "center"}
  data-docking-drop-target={node.id}
  on:dragover={handleDragOver}
  on:dragleave={handleDragLeave}
  on:drop={handleDrop}
  role="group"
>
  <header
    class="wc-header"
    on:contextmenu={handleHeaderContext}
    use:longpress={{
      duration: 460,
      moveTolerance: 12,
      onLongPress: (detail) => openHeaderContextAt(detail.x, detail.y),
    }}
    role="toolbar"
    tabindex="-1"
    aria-label="Widget controls"
  >
    <span
      class="wc-drag-handle"
      draggable="true"
      on:dragstart={handleDragStart}
      use:tooltip={{ text: "Drag to reorganise" }}
      role="button"
      tabindex="-1"
      aria-label="Drag handle"
    ><Icon name="grip" size={13} /></span>
    <span class="wc-title">{label}</span>
    {#if linked}
      <span class="wc-link-badge" use:tooltip={{ text: "Connected via pipeline" }}><Icon name="workflow" size={12} /></span>
    {/if}
    <div class="wc-controls">
      <button
        class="wc-btn"
        on:click={handleSplitH}
        use:tooltip={{ text: "Split Horizontal" }}
        aria-label="Split Horizontal"
      ><Icon name="column" size={13} /></button>
      <button
        class="wc-btn"
        on:click={handleSplitV}
        use:tooltip={{ text: "Split Vertical" }}
        aria-label="Split Vertical"
      ><Icon name="row" size={13} /></button>
      <button
        class="wc-btn"
        on:click={handleTearOff}
        use:tooltip={{ text: "Tear Off to New Window" }}
        aria-label="Tear Off"
      ><Icon name="panels" size={13} /></button>
      <button
        class="wc-btn wc-close"
        on:click={handleClose}
        use:tooltip={{ text: "Close Widget" }}
        aria-label="Close"
      ><Icon name="close" size={13} /></button>
    </div>
  </header>

  <div
    class="wc-body"
    on:contextmenu={handleBodyContext}
    use:longpress={{
      duration: 480,
      moveTolerance: 14,
      onLongPress: (detail) => openBodyContextAt(detail.x, detail.y),
    }}
    role="region"
    aria-label="Widget content"
  >
    {#if widgetComponent}
      <svelte:component this={widgetComponent} />
    {:else}
      <UnknownWidget widgetType={node.widgetType} />
    {/if}
  </div>
</div>

<style>
  .widget-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    min-width: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    overflow: hidden;
  }

  .wc-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.15rem 0.4rem;
    background: var(--bg-inset);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 1.5rem;
    user-select: none;
  }

  .wc-title {
    font-size: 0.68rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--fg-accent);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    min-width: 0;
  }

  .wc-controls {
    display: flex;
    align-items: center;
    gap: 2px;
    flex-shrink: 0;
  }

  .wc-btn {
    background: none;
    border: 1px solid transparent;
    color: var(--fg-secondary);
    font-size: 0.72rem;
    cursor: pointer;
    padding: 0 0.18rem;
    line-height: 1;
    font-family: var(--font-mono);
  }

  .wc-btn:hover {
    color: var(--fg-accent);
    border-color: var(--border-focus);
  }

  .wc-close:hover {
    color: var(--hl-error);
    border-color: var(--hl-error);
  }

  .wc-link-badge {
    font-size: 0.6rem;
    color: var(--hl-symbol);
    opacity: 0.8;
    flex-shrink: 0;
  }

  /* ── Drag Handle ──────────────────────────────────────────── */
  .wc-drag-handle {
    cursor: grab;
    color: var(--fg-secondary);
    font-size: 0.7rem;
    line-height: 1;
    padding: 0 0.1rem;
    flex-shrink: 0;
    user-select: none;
    opacity: 0.5;
    transition: opacity 0.1s;
  }

  .wc-drag-handle:hover {
    opacity: 1;
    color: var(--fg-accent);
  }

  .wc-drag-handle:active {
    cursor: grabbing;
  }

  /* ── Drop Zone Indicators ─────────────────────────────────── */
  .widget-container.drop-left   { border-left:   3px solid var(--hl-symbol); }
  .widget-container.drop-right  { border-right:  3px solid var(--hl-symbol); }
  .widget-container.drop-top    { border-top:    3px solid var(--hl-symbol); }
  .widget-container.drop-bottom { border-bottom: 3px solid var(--hl-symbol); }
  .widget-container.drop-center { outline: 2px dashed var(--hl-symbol); outline-offset: -2px; }

  .wc-body {
    flex: 1;
    overflow: auto;
    padding: 0.5rem;
    min-height: 0;
    min-width: 0;
  }

  /* ── Responsive ──────────────────────────────────────────── */
  @media (max-width: 768px) {
    .wc-header { padding: 0.1rem 0.25rem; gap: 0.15rem; }
    .wc-title { font-size: 0.6rem; }
    .wc-btn { font-size: 0.62rem; padding: 0 0.1rem; }
    .wc-body { padding: 0.25rem; }
  }
</style>
