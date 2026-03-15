<script lang="ts">
  import { get } from "svelte/store";
  import {
    WORKSPACE_COLORS,
    activeWorkspaceId,
    addWorkspace,
    duplicateWorkspace,
    removeWorkspace,
    renameWorkspace,
    reorderWorkspaces,
    setWorkspaceColor,
    switchWorkspace,
    workspaces,
    setWorkspaceDescription,
  } from "$lib/stores/layoutStore";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import { tooltip } from "$lib/actions/tooltip";

  let dragTabId: string | null = null;
  let dropTargetTabId: string | null = null;

  let renamingTabId: string | null = null;
  let renameValue = "";

  function formatTooltip(
    description: string,
    createdAt: string,
    updatedAt: string,
  ): string {
    const created = new Date(createdAt).toLocaleString();
    const updated = new Date(updatedAt).toLocaleString();
    return `${description}\nCreated: ${created}\nUpdated: ${updated}`;
  }

  function handleTabDragStart(event: DragEvent, wsId: string): void {
    dragTabId = wsId;
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "move";
      event.dataTransfer.setData("text/plain", wsId);
    }
  }

  function handleTabDragOver(event: DragEvent, wsId: string): void {
    if (!dragTabId || dragTabId === wsId) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    dropTargetTabId = wsId;
  }

  function handleTabDrop(event: DragEvent, wsId: string): void {
    event.preventDefault();
    if (dragTabId && dragTabId !== wsId) {
      reorderWorkspaces(dragTabId, wsId);
    }
    dragTabId = null;
    dropTargetTabId = null;
  }

  function handleTabDragEnd(): void {
    dragTabId = null;
    dropTargetTabId = null;
  }

  function startRenameTab(wsId: string): void {
    const ws = get(workspaces).find((w) => w.id === wsId);
    if (!ws) return;
    renamingTabId = wsId;
    renameValue = ws.name;
  }

  function commitRename(): void {
    if (renamingTabId && renameValue.trim()) {
      renameWorkspace(renamingTabId, renameValue.trim());
    }
    renamingTabId = null;
  }

  function cancelRename(): void {
    renamingTabId = null;
  }

  function handleRenameKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter") {
      event.preventDefault();
      commitRename();
    } else if (event.key === "Escape") {
      event.preventDefault();
      cancelRename();
    }
  }

  function handleTabDblClick(wsId: string): void {
    startRenameTab(wsId);
  }

  function handleTabContextMenu(event: MouseEvent, wsId: string): void {
    if (event.shiftKey) return;
    event.preventDefault();
    event.stopPropagation();

    const colorSubmenuItems: import("$lib/types/menu").ContextMenuItem[] = WORKSPACE_COLORS.map((c, i) => ({
      type: "action" as const,
      id: `ctx-ws-color-${i}`,
      label: c,
      icon: "●",
      iconColor: c,
      action: () => setWorkspaceColor(wsId, c),
    }));

    const items: import("$lib/types/menu").ContextMenuItem[] = [
      { type: "action", id: "ctx-ws-rename", label: "Rename", icon: "✎", action: () => startRenameTab(wsId) },
      {
        type: "action",
        id: "ctx-ws-description",
        label: "Edit Description",
        icon: "📝",
        action: () => {
          const target = get(workspaces).find((w) => w.id === wsId);
          if (!target) return;
          const value = window.prompt("Workspace description:", target.description);
          if (value === null) return;
          setWorkspaceDescription(wsId, value.trim() || "Untitled workspace", false);
        },
      },
      { type: "submenu", id: "ctx-ws-color", label: "Accent Color", icon: "●", children: colorSubmenuItems },
      { type: "separator", id: "sep-ws-1" },
      { type: "action", id: "ctx-ws-duplicate", label: "Duplicate Workspace", action: () => duplicateWorkspace(wsId) },
      { type: "action", id: "ctx-ws-new", label: "New Workspace", shortcut: "+", action: () => addWorkspace() },
      { type: "separator", id: "sep-ws-2" },
      ...(($workspaces.length > 1)
        ? [{ type: "action" as const, id: "ctx-ws-close", label: "Close Workspace", icon: "✕", action: () => removeWorkspace(wsId) }]
        : []),
    ];

    showContextMenu(event.clientX, event.clientY, items);
  }
</script>

<div class="workspace-tabs">
  {#each $workspaces as ws (ws.id)}
    <div
      class="ws-tab"
      class:active={ws.id === $activeWorkspaceId}
      class:ws-tab-dragging={ws.id === dragTabId}
      class:ws-tab-drop-target={ws.id === dropTargetTabId && ws.id !== dragTabId}
      style="--ws-accent: {ws.color};"
      on:click={() => switchWorkspace(ws.id)}
      on:dblclick={() => handleTabDblClick(ws.id)}
      on:contextmenu={(e) => handleTabContextMenu(e, ws.id)}
      on:keydown={(e) => e.key === 'Enter' && switchWorkspace(ws.id)}
      draggable={renamingTabId !== ws.id}
      on:dragstart={(e) => handleTabDragStart(e, ws.id)}
      on:dragover={(e) => handleTabDragOver(e, ws.id)}
      on:drop={(e) => handleTabDrop(e, ws.id)}
      on:dragend={handleTabDragEnd}
      on:dragleave={() => { if (dropTargetTabId === ws.id) dropTargetTabId = null; }}
      use:tooltip={{ text: formatTooltip(ws.description, ws.createdAt, ws.updatedAt) }}
      role="tab"
      tabindex="0"
      aria-selected={ws.id === $activeWorkspaceId}
    >
      <span class="ws-tab-dot" style="background: {ws.color};"></span>
      {#if renamingTabId === ws.id}
        <!-- svelte-ignore a11y-autofocus -->
        <input
          class="ws-tab-rename-input"
          type="text"
          bind:value={renameValue}
          on:blur={commitRename}
          on:keydown={handleRenameKeydown}
          on:click|stopPropagation
          on:dblclick|stopPropagation
          autofocus
        />
      {:else}
        <span class="ws-tab-label">{ws.name}</span>
      {/if}
      {#if $workspaces.length > 1}
        <button
          class="ws-tab-close"
          on:click|stopPropagation={() => removeWorkspace(ws.id)}
          aria-label="Close workspace"
        >&times;</button>
      {/if}
    </div>
  {/each}
  <button
    class="ws-tab ws-tab-add"
    on:click={() => addWorkspace()}
    use:tooltip={{ text: "New Workspace" }}
  >+</button>
</div>

<style>
  .workspace-tabs {
    display: flex;
    align-items: stretch;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    -ms-overflow-style: none;
    min-height: 1.5rem;
  }
  .workspace-tabs::-webkit-scrollbar { display: none; }

  .ws-tab {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.6rem;
    background: transparent;
    border: none;
    border-right: 1px solid var(--border);
    color: var(--fg-secondary);
    font-size: 0.65rem;
    font-family: var(--font-mono);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.1s, color 0.1s;
    flex-shrink: 0;
  }

  .ws-tab:hover {
    background: var(--bg-surface);
    color: var(--fg-primary);
  }

  .ws-tab.active {
    background: var(--bg-surface);
    color: var(--fg-accent);
    border-bottom: 2px solid var(--ws-accent, var(--hl-symbol));
  }

  .ws-tab-dragging {
    opacity: 0.4;
  }

  .ws-tab-drop-target {
    border-left: 2px solid var(--ws-accent, var(--hl-symbol)) !important;
  }

  .ws-tab-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .ws-tab-label {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 120px;
  }

  .ws-tab-rename-input {
    background: var(--bg-inset);
    border: 1px solid var(--ws-accent, var(--hl-symbol));
    color: var(--fg-accent);
    font-family: var(--font-mono);
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 0.2rem;
    width: 7rem;
    outline: none;
  }

  .ws-tab-close {
    background: none;
    border: none;
    font-size: 0.72rem;
    color: var(--fg-secondary);
    cursor: pointer;
    line-height: 1;
    padding: 0;
    font-family: var(--font-mono);
  }

  .ws-tab-close:hover {
    color: var(--hl-error);
  }

  .ws-tab-add {
    color: var(--fg-secondary);
    font-size: 0.82rem;
    font-weight: 700;
    padding: 0.2rem 0.5rem;
    border-right: none;
  }

  .ws-tab-add:hover {
    color: var(--hl-symbol);
    background: var(--bg-surface);
  }
</style>
