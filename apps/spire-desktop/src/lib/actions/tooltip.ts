export interface TooltipOptions {
  text: string;
  delay?: number;
  placement?: "top" | "bottom";
  maxWidth?: number;
}

const VIEWPORT_MARGIN = 8;
const TARGET_GAP = 8;

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

  let host: HTMLDivElement | null = null;
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;
  let visible = false;

  const clearTimer = (): void => {
    if (hoverTimer !== null) {
      clearTimeout(hoverTimer);
      hoverTimer = null;
    }
  };

  const teardownRuntime = (): void => {
    if (!host) return;
    host.remove();
    host = null;
  };

  const syncPosition = (): void => {
    if (!host || !visible) return;
    const targetRect = node.getBoundingClientRect();
    const tooltipRect = host.getBoundingClientRect();
    const position = computePosition(targetRect, tooltipRect, currentOptions.placement ?? "top");
    host.style.top = `${position.top}px`;
    host.style.left = `${position.left}px`;
  };

  const ensureHost = (): HTMLDivElement => {
    if (host) return host;
    const element = document.createElement("div");
    element.style.position = "fixed";
    element.style.zIndex = "2000";
    element.style.pointerEvents = "none";
    element.style.padding = "0.2rem 0.45rem";
    element.style.border = "1px solid var(--border)";
    element.style.background = "var(--bg-primary)";
    element.style.color = "var(--fg-primary)";
    element.style.fontFamily = "var(--font-mono)";
    element.style.fontSize = "0.66rem";
    element.style.lineHeight = "1.2";
    element.style.whiteSpace = "normal";
    element.style.wordBreak = "break-word";
    element.style.boxShadow = "0 10px 24px rgba(0, 0, 0, 0.34)";
    element.style.opacity = "0";
    element.style.transform = "translateY(-2px)";
    element.style.transition = "opacity 120ms ease, transform 120ms ease";
    document.body.appendChild(element);
    host = element;
    return element;
  };

  const show = (): void => {
    if (visible || !currentOptions.text?.trim()) return;
    visible = true;

    const element = ensureHost();
    element.textContent = currentOptions.text;
    element.style.maxWidth = `${currentOptions.maxWidth ?? 360}px`;
    element.style.display = "block";
    element.style.opacity = "1";
    element.style.transform = "translateY(0)";

    requestAnimationFrame(() => syncPosition());
    window.addEventListener("scroll", syncPosition, true);
    window.addEventListener("resize", syncPosition);
  };

  const hide = (): void => {
    clearTimer();
    visible = false;
    if (host) {
      host.style.opacity = "0";
      host.style.transform = "translateY(-2px)";
      host.style.display = "none";
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
      if (host && visible) {
        host.textContent = currentOptions.text;
        host.style.maxWidth = `${currentOptions.maxWidth ?? 360}px`;
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
