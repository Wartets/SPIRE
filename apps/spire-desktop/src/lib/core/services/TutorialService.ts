/**
 * SPIRE - Tutorial Service
 *
 * Guided onboarding state machine for new users.  Manages a sequence
 * of TutorialStep objects that target specific DOM elements via
 * `data-tour-id` attributes.  The TutorialOverlay component reads
 * this service's stores to render a spotlight effect and step content.
 *
 * The tutorial is entirely non-intrusive: it never modifies widget
 * logic or component state.  It simply highlights existing DOM
 * elements by reading their bounding rectangles.
 *
 * Architecture:
 *   - `tutorialActive`: boolean store - whether the tutorial is running.
 *   - `currentStepIndex`: number store - which step is being shown.
 *   - `currentStep`: derived - the active TutorialStep object.
 *   - Navigation: `nextStep()`, `prevStep()`, `skipTutorial()`.
 *   - The default tutorial walks through a Bhabha scattering calculation.
 */

import { writable, derived, get } from "svelte/store";

// ---------------------------------------------------------------------------
// Tutorial Step Model
// ---------------------------------------------------------------------------

export interface TutorialStep {
  /**
   * The `data-tour-id` attribute value of the DOM element to spotlight.
   * The overlay will find the element with `[data-tour-id="<targetId>"]`
   * and cut a transparent window over its bounding client rectangle.
   */
  targetId: string;
  /** Step title shown in the tooltip (bold header). */
  title: string;
  /** Explanatory content (1–3 sentences describing the physics and UI). */
  content: string;
  /**
   * Preferred tooltip placement relative to the spotlit element.
   * Falls back gracefully if there is insufficient viewport space.
   */
  placement?: "top" | "bottom" | "left" | "right";
}

// ---------------------------------------------------------------------------
// Default Tutorial: First Cross-Section Calculation (Bhabha Scattering)
// ---------------------------------------------------------------------------

export const DEFAULT_TUTORIAL: TutorialStep[] = [
  {
    targetId: "model-loader",
    title: "Step 1 - Load a Theoretical Model",
    content:
      "Start by loading the Standard Model. This populates the particle spectrum (fields, masses, quantum numbers) and the Lagrangian interaction vertices that define the Feynman rules.",
    placement: "right",
  },
  {
    targetId: "reaction-input",
    title: "Step 2 - Define the Reaction",
    content:
      "Specify the initial and final states of your scattering process. For Bhabha scattering, set the initial state to e⁻ e⁺ and the final state to e⁻ e⁺. Set the centre-of-mass energy (e.g. 91.2 GeV for the Z pole).",
    placement: "right",
  },
  {
    targetId: "reaction-run",
    title: "Step 3 - Run the Pipeline",
    content:
      "Click 'Run Full Pipeline' to construct the reaction, generate all Feynman diagrams at tree level, derive the symbolic amplitudes, and compute the kinematics. Each step validates conservation laws automatically.",
    placement: "bottom",
  },
  {
    targetId: "diagram-visualizer",
    title: "Step 4 - Inspect the Diagrams",
    content:
      "The Diagram Visualizer shows all topologically distinct Feynman diagrams for this process. For e⁻e⁺ → e⁻e⁺ you will see s-channel (Z/γ) and t-channel diagrams. Each diagram's symmetry factor and channel type are displayed.",
    placement: "left",
  },
  {
    targetId: "analysis-widget",
    title: "Step 5 - Run a Monte Carlo Analysis",
    content:
      "Open the Analysis widget to generate Monte Carlo events. Choose an observable (e.g. leading pT), set the number of events and √s, then click 'Run'. The histogram shows the differential cross-section distribution.",
    placement: "top",
  },
];

// ---------------------------------------------------------------------------
// Stores
// ---------------------------------------------------------------------------

/** The tutorial step sequence currently in use. */
const _steps = writable<TutorialStep[]>(DEFAULT_TUTORIAL);

/** Whether the tutorial overlay is currently active. */
export const tutorialActive = writable<boolean>(false);

/** Zero-based index of the current step. */
export const currentStepIndex = writable<number>(0);

/** The total number of steps in the current tutorial. */
export const totalSteps = derived(_steps, ($s) => $s.length);

/** The current TutorialStep object, or undefined if out of bounds. */
export const currentStep = derived(
  [_steps, currentStepIndex],
  ([$s, $idx]) => $s[$idx] as TutorialStep | undefined,
);

/** Whether the user is on the last step. */
export const isLastStep = derived(
  [_steps, currentStepIndex],
  ([$s, $idx]) => $idx >= $s.length - 1,
);

/** Whether the user is on the first step. */
export const isFirstStep = derived(currentStepIndex, ($idx) => $idx === 0);

// ---------------------------------------------------------------------------
// Navigation API
// ---------------------------------------------------------------------------

/**
 * Start the tutorial from the first step.
 * Optionally accepts a custom step sequence.
 */
export function startTutorial(steps?: TutorialStep[]): void {
  if (steps) {
    _steps.set(steps);
  }
  currentStepIndex.set(0);
  tutorialActive.set(true);
}

/**
 * Advance to the next step.  If already on the last step, ends the tutorial.
 */
export function nextStep(): void {
  currentStepIndex.update((idx) => {
    const total = get(_steps).length;
    if (idx >= total - 1) {
      tutorialActive.set(false);
      return 0;
    }
    return idx + 1;
  });
}

/**
 * Go back to the previous step.  No-op on the first step.
 */
export function prevStep(): void {
  currentStepIndex.update((idx) => Math.max(0, idx - 1));
}

/**
 * End the tutorial immediately.
 */
export function skipTutorial(): void {
  tutorialActive.set(false);
  currentStepIndex.set(0);
}

/**
 * Jump to a specific step by index.
 */
export function goToStep(index: number): void {
  const total = get(_steps).length;
  if (index >= 0 && index < total) {
    currentStepIndex.set(index);
  }
}
