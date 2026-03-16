interface ZIndexState {
  order: string[];
  selectedId: string | null;
  currentMaxZ: number;
}

export interface ZIndexSnapshot {
  order: string[];
  selectedId: string | null;
  currentMaxZ: number;
}

/**
 * Central z-order registry for infinite-canvas widget stacking.
 */
export function createZIndexManager(initialIds: string[] = []) {
  const state: ZIndexState = {
    order: [...initialIds],
    selectedId: initialIds.at(-1) ?? null,
    currentMaxZ: initialIds.length,
  };

  function sync(ids: string[]): void {
    const nextSet = new Set(ids);
    const retained = state.order.filter((id) => nextSet.has(id));
    const additions = ids.filter((id) => !retained.includes(id));
    state.order = [...retained, ...additions];
    state.currentMaxZ = state.order.length;

    if (state.selectedId && !nextSet.has(state.selectedId)) {
      state.selectedId = state.order.at(-1) ?? null;
    }
  }

  function bringToFront(id: string): void {
    const without = state.order.filter((entry) => entry !== id);
    state.order = [...without, id];
    state.currentMaxZ = state.order.length;
    state.selectedId = id;
  }

  function zIndexOf(id: string): number {
    const index = state.order.indexOf(id);
    return index >= 0 ? index + 1 : 1;
  }

  function remove(id: string): void {
    state.order = state.order.filter((entry) => entry !== id);
    state.currentMaxZ = state.order.length;
    if (state.selectedId === id) {
      state.selectedId = state.order.at(-1) ?? null;
    }
  }

  function snapshot(): ZIndexSnapshot {
    return {
      order: [...state.order],
      selectedId: state.selectedId,
      currentMaxZ: state.currentMaxZ,
    };
  }

  return {
    sync,
    remove,
    bringToFront,
    zIndexOf,
    snapshot,
  };
}
