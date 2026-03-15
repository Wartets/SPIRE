import SpireTooltip from "$lib/components/ui/SpireTooltip.svelte";

export interface TooltipOptions {
  text: string;
  delay?: number;
  placement?: "top" | "bottom";
  maxWidth?: number;
}

const VIEWPORT_MARGIN = 8;
const TARGET_GAP = 8;

interface TooltipRuntime {
  host: HTMLDivElement;
  component: SpireTooltip;
}

function canUseDom(): boolean {
  return typeof window !== "undefined" && typeof document !== "undefined";
}

function computePosition(
  targetRect: DOMRect,
  tooltipRect: DOMRect,
  placement: "top" | "bottom",
): { top: number; left: number } {
  const preferredTop =
    placement === "bottom"
      ? targetRect.bottom + TARGET_GAP
      : targetRect.top - tooltipRect.height - TARGET_GAP;

  let top = preferredTop;
  if (placement === "top" && top < VIEWPORT_MARGIN) {
    top = targetRect.bottom + TARGET_GAP;
  } else if (placement === "bottom" && top + tooltipRect.height > window.innerHeight - VIEWPORT_MARGIN) {
    top = targetRect.top - tooltipRect.height - TARGET_GAP;
  }

  if (top < VIEWPORT_MARGIN) {
    top = VIEWPORT_MARGIN;
  }

  if (top + tooltipRect.height > window.innerHeight - VIEWPORT_MARGIN) {
    top = window.innerHeight - tooltipRect.height - VIEWPORT_MARGIN;
  }

  let left = targetRect.left + targetRect.width / 2 - tooltipRect.width / 2;
  if (left < VIEWPORT_MARGIN) {
    left = VIEWPORT_MARGIN;
  }
  if (left + tooltipRect.width > window.innerWidth - VIEWPORT_MARGIN) {
    left = window.innerWidth - tooltipRect.width - VIEWPORT_MARGIN;
  }

  return { top, left };
}

export function tooltip(node: HTMLElement, options: TooltipOptions) {
  if (!canUseDom()) {
    return {
      update() {
        // SSR no-op
      },
      destroy() {
        // SSR no-op
      },
    };
  }

  let currentOptions: TooltipOptions = {
    delay: 400,
    placement: "top",
    maxWidth: 360,
    ...options,
  };

  let runtime: TooltipRuntime | null = null;
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;
  let visible = false;

  const clearTimer = (): void => {
    if (hoverTimer !== null) {
      clearTimeout(hoverTimer);
      hoverTimer = null;
    }
  };

  const teardownRuntime = (): void => {
    if (!runtime) return;
    runtime.component.$destroy();
    runtime.host.remove();
    runtime = null;
  };

  const syncPosition = (): void => {
    if (!runtime || !visible) return;
    const targetRect = node.getBoundingClientRect();
    const tooltipRect = runtime.host.getBoundingClientRect();
    const position = computePosition(targetRect, tooltipRect, currentOptions.placement ?? "top");
    runtime.component.$set({
      top: position.top,
      left: position.left,
      visible: true,
      text: currentOptions.text,
      maxWidth: currentOptions.maxWidth ?? 360,
    });
  };

  const show = (): void => {
    if (visible || !currentOptions.text?.trim()) return;
    visible = true;

    if (!runtime) {
      const host = document.createElement("div");
      document.body.appendChild(host);
      const component = new SpireTooltip({
        target: host,
        props: {
          text: currentOptions.text,
          visible: true,
          top: -9999,
          left: -9999,
          maxWidth: currentOptions.maxWidth ?? 360,
        },
      });
      runtime = { host, component };
    } else {
      runtime.component.$set({ text: currentOptions.text, visible: true });
    }

    requestAnimationFrame(() => syncPosition());
    window.addEventListener("scroll", syncPosition, true);
    window.addEventListener("resize", syncPosition);
  };

  const hide = (): void => {
    clearTimer();
    visible = false;
    if (runtime) {
      runtime.component.$set({ visible: false });
    }
    window.removeEventListener("scroll", syncPosition, true);
    window.removeEventListener("resize", syncPosition);
  };

  const onEnter = (): void => {
    clearTimer();
    hoverTimer = setTimeout(() => {
      hoverTimer = null;
      show();
    }, currentOptions.delay ?? 400);
  };

  const onLeave = (): void => {
    hide();
  };

  node.addEventListener("mouseenter", onEnter);
  node.addEventListener("mouseleave", onLeave);
  node.addEventListener("focusin", onEnter);
  node.addEventListener("focusout", onLeave);

  return {
    update(next: TooltipOptions) {
      currentOptions = {
        delay: 400,
        placement: "top",
        maxWidth: 360,
        ...next,
      };
      if (runtime && visible) {
        runtime.component.$set({
          text: currentOptions.text,
          maxWidth: currentOptions.maxWidth ?? 360,
        });
        syncPosition();
      }
    },
    destroy() {
      clearTimer();
      hide();
      node.removeEventListener("mouseenter", onEnter);
      node.removeEventListener("mouseleave", onLeave);
      node.removeEventListener("focusin", onEnter);
      node.removeEventListener("focusout", onLeave);
      teardownRuntime();
    },
  };
}
