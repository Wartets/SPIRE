<!--
  SPIRE - Recursive Context Menu List

  Renders a list of ContextMenuItems.  Handles:
  - Action, separator, toggle, submenu item types
  - Nested submenu spawning with hover delay
  - Keyboard-navigable activeIndex
  - Viewport clamping for child submenus

  Used by ContextMenu.svelte (root) and by itself (svelte:self for nesting).
-->
<script lang="ts">
  import { tick } from "svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { hideContextMenu } from "$lib/stores/contextMenuStore";
  import type { ContextMenuItem } from "$lib/types/menu";
  import Icon from "$lib/components/ui/Icon.svelte";

  /** The items to render. */
  export let items: ContextMenuItem[] = [];
  /** Whether this list is the root level (shows native-menu hint). */
  export let isRoot: boolean = false;
  /** Direction of the parent menu so submenus open the correct way. */
  export let parentDirection: "right" | "left" = "right";
  /** When true, indicates this list panel is keyboard-active. */
  export let hasFocus: boolean = false;

  // ── Submenu hover delay ──
  let openSubmenuId: string | null = null;
  let submenuTimer: ReturnType<typeof setTimeout> | null = null;
  const SUBMENU_DELAY = 150;

  // ── Keyboard focus ──
  let activeIndex = -1;

  // ── Submenu position ──
  let submenuPosMap: Record<string, { x: number; y: number; dir: "right" | "left" }> = {};
  let listEl: HTMLDivElement | undefined;

  $: navigableIndices = items
    .map((item, i) => ({ item, i }))
    .filter(({ item }) => item.type !== "separator")
    .map(({ i }) => i);

  function moveActive(delta: number): void {
    if (navigableIndices.length === 0) return;
    const cur = navigableIndices.indexOf(activeIndex);
    let next: number;
    if (cur === -1) {
      next = delta > 0 ? 0 : navigableIndices.length - 1;
    } else {
      next = (cur + delta + navigableIndices.length) % navigableIndices.length;
    }
    activeIndex = navigableIndices[next];
  }

  function activateItem(item: ContextMenuItem): void {
    if (item.type === "separator") return;
    if (item.disabled) return;
    if (item.type === "action") {
      hideContextMenu();
      item.action();
    } else if (item.type === "toggle") {
      item.action(!item.checked);
    } else if (item.type === "submenu") {
      openSubmenuId = item.id;
    }
  }

  async function computeSubmenuPos(itemId: string, el: HTMLElement): Promise<void> {
    await tick();
    if (!listEl) return;
    const listRect = listEl.getBoundingClientRect();
    const itemRect = el.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const subWidth = 220; // estimated submenu width
    const pad = 4;

    let dir: "right" | "left" = parentDirection;
    let x: number;
    let y: number = itemRect.top;

    if (dir === "right") {
      x = listRect.right;
      if (x + subWidth > vw - pad) {
        dir = "left";
        x = listRect.left - subWidth;
      }
    } else {
      x = listRect.left - subWidth;
      if (x < pad) {
        dir = "right";
        x = listRect.right;
      }
    }

    // Vertical clamp
    const estimatedHeight = 300;
    if (y + estimatedHeight > vh - pad) {
      y = Math.max(pad, vh - estimatedHeight - pad);
    }

    submenuPosMap = { ...submenuPosMap, [itemId]: { x, y, dir } };
  }

  function handleItemMouseEnter(item: ContextMenuItem, idx: number, el: HTMLElement): void {
    activeIndex = idx;
    if (submenuTimer) clearTimeout(submenuTimer);
    if (item.type === "submenu" && !item.disabled) {
      computeSubmenuPos(item.id, el);
      submenuTimer = setTimeout(() => {
        openSubmenuId = item.id;
      }, SUBMENU_DELAY);
    } else {
      submenuTimer = setTimeout(() => {
        openSubmenuId = null;
      }, SUBMENU_DELAY);
    }
  }

  function handleItemMouseLeave(): void {
    if (submenuTimer) clearTimeout(submenuTimer);
  }

  /** Called from parent (ContextMenu.svelte) for root keyboard events. */
  export function handleKeydown(event: KeyboardEvent): void {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        event.stopPropagation();
        if (openSubmenuId) {
          openSubmenuId = null;
        } else {
          hideContextMenu();
        }
        break;

      case "ArrowDown":
        event.preventDefault();
        moveActive(1);
        break;

      case "ArrowUp":
        event.preventDefault();
        moveActive(-1);
        break;

      case "ArrowRight": {
        event.preventDefault();
        if (activeIndex >= 0) {
          const item = items[activeIndex];
          if (item?.type === "submenu" && !item.disabled) {
            openSubmenuId = item.id;
          }
        }
        break;
      }

      case "ArrowLeft":
        event.preventDefault();
        if (openSubmenuId) {
          openSubmenuId = null;
        }
        break;

      case "Enter":
      case " ": {
        event.preventDefault();
        if (activeIndex >= 0) {
          const item = items[activeIndex];
          if (item) activateItem(item);
        }
        break;
      }
    }
  }
</script>

<div class="ctx-list" class:ctx-has-focus={hasFocus} bind:this={listEl} role="menu">
  {#each items as item, idx (item.id)}
    {#if item.type === "separator"}
      <div class="ctx-separator" role="separator"></div>
    {:else}
      <div
        class="ctx-item-wrapper"
        on:mouseenter={(e) => handleItemMouseEnter(item, idx, e.currentTarget)}
        on:mouseleave={handleItemMouseLeave}
        role="presentation"
      >
        <button
          class="ctx-item"
          class:disabled={item.disabled}
          class:active={activeIndex === idx}
          class:checked={item.type === "toggle" && item.checked}
          on:click={() => activateItem(item)}
          role="menuitem"
          aria-disabled={item.disabled ?? false}
          use:tooltip={{ text: item.disabled && item.tooltip ? item.tooltip : "" }}
        >
          {#if item.type === "toggle"}
            <span class="ctx-check">{item.checked ? "☑" : "☐"}</span>
          {/if}

          {#if item.icon}
            <span class="ctx-icon" style={item.iconColor ? `color: ${item.iconColor}` : ''}>
              <Icon name={item.icon} size={14} />
            </span>
          {/if}

          <span class="ctx-label">{item.label}</span>

          {#if item.type !== "submenu" && item.shortcut}
            <span class="ctx-shortcut">{item.shortcut}</span>
          {/if}

          {#if item.type === "submenu"}
            <span class="ctx-arrow">▸</span>
          {/if}
        </button>

        <!-- Nested submenu -->
        {#if item.type === "submenu" && openSubmenuId === item.id && item.children?.length}
          {@const pos = submenuPosMap[item.id]}
          {#if pos}
            <div
              class="ctx-submenu-portal"
              style="left: {pos.x}px; top: {pos.y}px;"
            >
              <svelte:self
                items={item.children}
                parentDirection={pos.dir}
                isRoot={false}
              />
            </div>
          {/if}
        {/if}
      </div>
    {/if}
  {/each}

  {#if isRoot}
    <div class="ctx-separator" role="separator"></div>
    <div class="ctx-hint" role="note">
      Shift + Right-click for native menu
    </div>
  {/if}
</div>

<style>
  .ctx-list {
    width: max-content;
    min-width: 100px;
    max-width: 320px;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--color-bg-base);
    border: 1px solid var(--border, #333);
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.7), 0 2px 8px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    padding: 0.25rem 0;
    font-family: var(--font-mono);
    border-radius: 4px;
  }

  .ctx-separator {
    height: 1px;
    background: var(--border, #333);
    margin: 0.2rem 0.4rem;
  }

  .ctx-item-wrapper {
    position: relative;
  }

  .ctx-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.35rem 0.6rem;
    background: none;
    border: none;
    color: var(--color-text-primary);
    font-size: 0.75rem;
    font-family: var(--font-mono);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    transition: background 0.06s;
  }

  .ctx-item:hover:not(.disabled),
  .ctx-item.active:not(.disabled) {
    background: var(--color-bg-surface);
    color: var(--fg-accent, var(--color-text-primary));
  }

  .ctx-item.disabled {
    color: var(--fg-secondary, var(--color-text-muted));
    cursor: default;
    opacity: 0.5;
  }

  .ctx-check {
    width: 1em;
    text-align: center;
    font-size: 0.8rem;
    flex-shrink: 0;
  }

  .ctx-icon {
    min-width: 1.35em;
    text-align: center;
    font-size: 0.82rem;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-right: 0.05rem;
  }

  .ctx-icon :global(.spire-icon-fallback) {
    font-size: 0.66rem;
    letter-spacing: 0.03em;
    font-weight: 700;
  }

  .ctx-label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ctx-shortcut {
    font-size: 0.6rem;
    color: var(--fg-secondary, var(--color-text-muted));
    margin-left: auto;
    flex-shrink: 0;
    padding-left: 1rem;
  }

  .ctx-arrow {
    font-size: 0.7rem;
    color: var(--fg-secondary, var(--color-text-muted));
    margin-left: auto;
    flex-shrink: 0;
  }

  .ctx-hint {
    padding: 0.3rem 0.6rem;
    font-size: 0.58rem;
    color: var(--fg-secondary, var(--color-text-muted));
    font-style: italic;
    opacity: 0.6;
    user-select: none;
    cursor: default;
  }

  .ctx-submenu-portal {
    position: fixed;
    z-index: 10002;
  }
</style>
