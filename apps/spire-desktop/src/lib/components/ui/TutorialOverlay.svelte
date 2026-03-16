<!--
  SPIRE - Tutorial Overlay

  Non-intrusive guided onboarding overlay.  Draws a dark semi-transparent
  mask over the entire viewport and cuts a transparent "spotlight" window
  over the bounding rectangle of the element targeted by the current
  tutorial step (found via `[data-tour-id="<id>"]`).

  The overlay never alters underlying component logic - it only reads
  DOM bounding rectangles and renders a positioned tooltip with
  navigation controls.
-->
<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import {
    tutorialActive,
    currentStep,
    currentStepIndex,
    totalSteps,
    isFirstStep,
    isLastStep,
    nextStep,
    prevStep,
    skipTutorial,
  } from "$lib/core/services/TutorialService";

  // Spotlight bounding rectangle
  let spotRect = { top: 0, left: 0, width: 0, height: 0 };
  // Tooltip position
  let tipTop = 0;
  let tipLeft = 0;

  let tooltipEl: HTMLDivElement | undefined;
  let highlightedCanvasWidgetId: string | null = null;

  // Recompute spotlight when step changes
  $: if ($tutorialActive && $currentStep) {
    computeSpotlight($currentStep.targetId);
  }

  async function computeSpotlight(targetId: string): Promise<void> {
    await tick();
    const el = document.querySelector(`[data-tour-id="${targetId}"]`);
    if (!el) {
      highlightedCanvasWidgetId = null;
      // Element not found - use a centered fallback
      spotRect = {
        top: window.innerHeight / 2 - 50,
        left: window.innerWidth / 2 - 100,
        width: 200,
        height: 100,
      };
    } else {
      (el as HTMLElement).scrollIntoView({ block: "nearest", inline: "nearest" });
      focusCanvasWidget(el as Element);
      await tick();
      const rect = el.getBoundingClientRect();
      const pad = 6; // Padding around the element
      spotRect = {
        top: rect.top - pad,
        left: rect.left - pad,
        width: rect.width + pad * 2,
        height: rect.height + pad * 2,
      };
    }

    // Position tooltip after spotlight is computed
    await tick();
    positionTooltip();
  }

  function focusCanvasWidget(target: Element): void {
    const canvasWidget = target.closest(".canvas-widget[data-canvas-item-id]") as HTMLElement | null;
    const widgetId = canvasWidget?.dataset.canvasItemId ?? null;
    highlightedCanvasWidgetId = widgetId;
    if (widgetId) {
      window.dispatchEvent(
        new CustomEvent("spire:canvas:focus-widget", {
          detail: { widgetId },
        }),
      );
      requestAnimationFrame(() => {
        window.dispatchEvent(
          new CustomEvent("spire:canvas:focus-widget", {
            detail: { widgetId },
          }),
        );
      });
    }
  }

  function positionTooltip(): void {
    if (!tooltipEl || !$currentStep) return;

    const tipRect = tooltipEl.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const placement = $currentStep.placement ?? "bottom";
    const gap = 12;

    let top = 0;
    let left = 0;

    switch (placement) {
      case "bottom":
        top = spotRect.top + spotRect.height + gap;
        left = spotRect.left + spotRect.width / 2 - tipRect.width / 2;
        break;
      case "top":
        top = spotRect.top - tipRect.height - gap;
        left = spotRect.left + spotRect.width / 2 - tipRect.width / 2;
        break;
      case "right":
        top = spotRect.top + spotRect.height / 2 - tipRect.height / 2;
        left = spotRect.left + spotRect.width + gap;
        break;
      case "left":
        top = spotRect.top + spotRect.height / 2 - tipRect.height / 2;
        left = spotRect.left - tipRect.width - gap;
        break;
    }

    // Clamp to viewport
    if (left < 8) left = 8;
    if (left + tipRect.width > vw - 8) left = vw - tipRect.width - 8;
    if (top < 8) top = 8;
    if (top + tipRect.height > vh - 8) top = vh - tipRect.height - 8;

    tipTop = top;
    tipLeft = left;
  }

  // Recalculate on window resize
  function handleResize(): void {
    if ($tutorialActive && $currentStep) {
      computeSpotlight($currentStep.targetId);
    }
  }

  // Close on Escape
  function handleKeydown(event: KeyboardEvent): void {
    if (!$tutorialActive) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      skipTutorial();
    } else if (event.key === "ArrowRight" || event.key === "Enter") {
      event.preventDefault();
      nextStep();
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      prevStep();
    }
  }

  onMount(() => {
    window.addEventListener("resize", handleResize);
    window.addEventListener("keydown", handleKeydown, { capture: true });
  });

  onDestroy(() => {
    window.removeEventListener("resize", handleResize);
    window.removeEventListener("keydown", handleKeydown, { capture: true });
  });
</script>

{#if $tutorialActive && $currentStep}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="tutorial-backdrop" on:click={skipTutorial}>
    <!-- Spotlight cutout using CSS clip-path -->
    <svg class="tutorial-mask" viewBox="0 0 {window.innerWidth} {window.innerHeight}">
      <defs>
        <mask id="spotlight-mask">
          <rect x="0" y="0" width="100%" height="100%" fill="white" />
          <rect
            x={spotRect.left}
            y={spotRect.top}
            width={spotRect.width}
            height={spotRect.height}
            fill="black"
            rx="0"
          />
        </mask>
      </defs>
      <rect
        x="0" y="0"
        width="100%" height="100%"
        fill="rgba(0, 0, 0, 0.65)"
        mask="url(#spotlight-mask)"
      />
    </svg>

    <!-- Spotlight border highlight -->
    <div
      class="spotlight-border"
      style="
        top: {spotRect.top}px;
        left: {spotRect.left}px;
        width: {spotRect.width}px;
        height: {spotRect.height}px;
      "
    ></div>
  </div>

  <!-- Tooltip (outside backdrop so it's clickable) -->
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="tutorial-tooltip"
    bind:this={tooltipEl}
    style="top: {tipTop}px; left: {tipLeft}px;"
    on:click|stopPropagation
  >
    <div class="tutorial-step-header">
      <span class="tutorial-step-title">{$currentStep.title}</span>
      <span class="tutorial-step-counter">
        {$currentStepIndex + 1} / {$totalSteps}
      </span>
    </div>

    <p class="tutorial-step-content">{$currentStep.content}</p>

    <div class="tutorial-controls">
      <div class="tutorial-step-meta">
        <span class="tutorial-chip">{highlightedCanvasWidgetId ? "Canvas focus synced" : "Guided walkthrough"}</span>
      </div>
      <button
        class="tutorial-btn tutorial-btn-skip"
        on:click={skipTutorial}
      >
        Skip Tutorial
      </button>
      <div class="tutorial-nav">
        <button
          class="tutorial-btn"
          on:click={prevStep}
          disabled={$isFirstStep}
        >
          ← Previous
        </button>
        <button
          class="tutorial-btn tutorial-btn-next"
          on:click={nextStep}
        >
          {$isLastStep ? "Finish" : "Next →"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tutorial-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    z-index: 12990;
    cursor: pointer;
  }

  .tutorial-mask {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }

  .spotlight-border {
    position: fixed;
    border: 2px solid var(--color-accent);
    pointer-events: none;
    z-index: 12991;
    box-shadow: 0 0 0 2px rgba(94, 184, 255, 0.3);
  }

  .tutorial-tooltip {
    position: fixed;
    z-index: 12995;
    max-width: 400px;
    min-width: 280px;
    padding: 0.8rem 1rem;
    background: var(--color-bg-base);
    border: 1px solid var(--color-accent);
    color: var(--color-text-primary);
    font-family: var(--font-mono, monospace);
    font-size: 0.75rem;
    line-height: 1.5;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }

  .tutorial-step-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .tutorial-step-title {
    font-weight: 700;
    font-size: 0.82rem;
    color: var(--fg-accent, var(--color-text-primary));
  }
  .tutorial-step-counter {
    font-size: 0.62rem;
    color: var(--fg-secondary, var(--color-text-muted));
    padding: 0.1rem 0.3rem;
    border: 1px solid var(--border, #333);
  }

  .tutorial-step-content {
    margin: 0 0 0.6rem;
    color: var(--color-text-primary);
  }

  .tutorial-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .tutorial-step-meta {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    flex: 1;
    min-width: 0;
  }
  .tutorial-chip {
    display: inline-flex;
    align-items: center;
    padding: 0.15rem 0.45rem;
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .tutorial-nav {
    display: flex;
    gap: 0.35rem;
  }

  .tutorial-btn {
    background: var(--color-bg-surface);
    border: 1px solid var(--border, #333);
    color: var(--fg-secondary, var(--color-text-muted));
    padding: 0.25rem 0.5rem;
    font-size: 0.68rem;
    cursor: pointer;
    font-family: var(--font-mono, monospace);
  }
  .tutorial-btn:hover:not(:disabled) {
    border-color: var(--border-focus, var(--color-text-muted));
    color: var(--color-text-primary);
  }
  .tutorial-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .tutorial-btn-next {
    background: var(--color-accent);
    color: var(--color-bg-base);
    border-color: var(--color-accent);
    font-weight: 700;
  }
  .tutorial-btn-next:hover {
    filter: brightness(1.15);
  }
  .tutorial-btn-skip {
    color: var(--fg-secondary, var(--color-text-muted));
    border: none;
    background: none;
    text-decoration: underline;
    padding: 0.25rem 0;
  }
  .tutorial-btn-skip:hover {
    color: var(--color-error);
  }
</style>
