<!--
  SPIRE - Global Context Menu  (root overlay)

  Mounted once in +layout.svelte.  Reads from `contextMenuStore`,
  positions the root panel with viewport clamping, delegates rendering
  and keyboard nav to ContextMenuList.svelte.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import {
    contextMenu,
    hideContextMenu,
  } from "$lib/stores/contextMenuStore";
  import ContextMenuList from "./ContextMenuList.svelte";

  let menuEl: HTMLDivElement | undefined;
  let listRef: ContextMenuList | undefined;
  let clampedX = 0;
  let clampedY = 0;

  async function clampToViewport(rawX: number, rawY: number): Promise<void> {
    await tick();
    if (!menuEl) {
      clampedX = rawX;
      clampedY = rawY;
      return;
    }
    const rect = menuEl.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const pad = 4;

    const overflowH = rawX + rect.width > vw - pad;
    const overflowV = rawY + rect.height > vh - pad;

    clampedX = overflowH ? Math.max(pad, rawX - rect.width) : rawX;
    clampedY = overflowV ? Math.max(pad, rawY - rect.height) : rawY;
  }

  $: if ($contextMenu.visible) {
    clampToViewport($contextMenu.x, $contextMenu.y);
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (!$contextMenu.visible) return;
    if (listRef) listRef.handleKeydown(event);
  }

  function handleClickOutside(): void {
    if ($contextMenu.visible) hideContextMenu();
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown, { capture: true });
  });
  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown, { capture: true });
  });
</script>

{#if $contextMenu.visible}
  <div
    class="ctx-backdrop"
    on:mousedown={handleClickOutside}
    role="presentation"
  ></div>

  <div
    class="ctx-root"
    bind:this={menuEl}
    style="left: {clampedX}px; top: {clampedY}px;"
  >
    <ContextMenuList
      bind:this={listRef}
      items={$contextMenu.items}
      isRoot={true}
      hasFocus={true}
    />
  </div>
{/if}

<style>
  .ctx-backdrop {
    position: fixed;
    inset: 0;
    z-index: 10000;
  }
  .ctx-root {
    position: fixed;
    z-index: 10001;
  }
</style>
