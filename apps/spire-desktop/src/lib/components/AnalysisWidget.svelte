<!--
  SPIRE - Analysis Widget

  Interactive kinematic analysis and histogramming. Allows users to define
  Rhai observable scripts (e.g., pT, invariant mass), configure histogram
  binning, run Monte Carlo event generation, and visualise the resulting
  distributions as bar charts using Chart.js.

  Supports linear/logarithmic Y-axis scaling.
-->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { appendLog } from "$lib/stores/physicsStore";
  import { runAnalysis, validateScript } from "$lib/api";
  import { configureNlo, configureShower } from "$lib/api";
  import { registerCommand, unregisterCommand } from "$lib/core/services/CommandRegistry";
  import { addCitations } from "$lib/core/services/CitationRegistry";
  import HoverDef from "$lib/components/ui/HoverDef.svelte";
  import SpireNumberInput from "$lib/components/ui/SpireNumberInput.svelte";
  import { tooltip } from "$lib/actions/tooltip";
  import { isolateEvents } from "$lib/actions/widgetEvents";
  import { showContextMenu } from "$lib/stores/contextMenuStore";
  import type { AnalysisResult, HistogramData, Histogram2DData, DetectorPreset, ParticleKind, PlotDefinition2D } from "$lib/types/spire";
  import { extractAndPushProfile } from "$lib/core/services/TelemetryService";
  import { publishWidgetInterop, widgetInteropState } from "$lib/stores/widgetInteropStore";
  import WebglHeatmap from "./WebglHeatmap.svelte";
  import {
    Chart,
    BarController,
    BarElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
  } from "chart.js";

  const logAnalysis = (message: string): void => {
    appendLog(message, { category: "Analysis" });
  };

  // Register only the Chart.js components we need (tree-shakeable).
  Chart.register(
    BarController,
    BarElement,
    CategoryScale,
    LinearScale,
    LogarithmicScale,
    Tooltip,
    Title,
    Legend,
  );

  // ---------------------------------------------------------------------------
  // Observable Presets
  // ---------------------------------------------------------------------------
  interface ObservablePreset {
    label: string;
    script: string;
    min: number;
    max: number;
    nBins: number;
    description: string;
  }

  const PRESETS: ObservablePreset[] = [
    {
      label: "Leading pT",
      script: "event.momenta[0].pt()",
      min: 0, max: 60, nBins: 30,
      description: "Transverse momentum of the first final-state particle",
    },
    {
      label: "Subleading pT",
      script: "event.momenta[1].pt()",
      min: 0, max: 60, nBins: 30,
      description: "Transverse momentum of the second final-state particle",
    },
    {
      label: "Invariant Mass (pair)",
      script: "let p1 = event.momenta[0]; let p2 = event.momenta[1]; let s = p1 + p2; s.m()",
      min: 50, max: 150, nBins: 40,
      description: "Invariant mass of the first two final-state particles",
    },
    {
      label: "Pseudorapidity η",
      script: "event.momenta[0].eta()",
      min: -5, max: 5, nBins: 50,
      description: "Pseudorapidity of the first final-state particle",
    },
    {
      label: "Azimuthal Angle φ",
      script: "event.momenta[0].phi()",
      min: -3.15, max: 3.15, nBins: 32,
      description: "Azimuthal angle of the first final-state particle",
    },
    {
      label: "Energy",
      script: "event.momenta[0].e()",
      min: 0, max: 100, nBins: 25,
      description: "Energy of the first final-state particle",
    },
  ];

  // ---------------------------------------------------------------------------
  // Reco-Level Observable Presets (require detector simulation)
  // ---------------------------------------------------------------------------
  const RECO_PRESETS: ObservablePreset[] = [
    {
      label: "Leading Jet pT",
      script: "let jets = reco.jets; if jets.len() > 0 { jets[0].pt() } else { -1.0 }",
      min: 0, max: 300, nBins: 30,
      description: "Transverse momentum of the hardest reconstructed jet",
    },
    {
      label: "Jet Multiplicity",
      script: "reco.n_jets().to_float()",
      min: 0, max: 10, nBins: 10,
      description: "Number of reconstructed jets",
    },
    {
      label: "Missing ET",
      script: "reco.met_pt()",
      min: 0, max: 200, nBins: 30,
      description: "Missing transverse energy (MET) magnitude",
    },
    {
      label: "Leading Jet η",
      script: "let jets = reco.jets; if jets.len() > 0 { jets[0].eta() } else { 0.0 }",
      min: -5, max: 5, nBins: 50,
      description: "Pseudorapidity of the leading reconstructed jet",
    },
    {
      label: "Leading Jet Mass",
      script: "let jets = reco.jets; if jets.len() > 0 { jets[0].m() } else { -1.0 }",
      min: 0, max: 50, nBins: 25,
      description: "Invariant mass of the leading reconstructed jet",
    },
    {
      label: "N Leptons",
      script: "reco.n_leptons().to_float()",
      min: 0, max: 6, nBins: 6,
      description: "Number of reconstructed leptons (electrons + muons)",
    },
  ];

  // ---------------------------------------------------------------------------
  // 2D Observable Presets
  // ---------------------------------------------------------------------------
  interface Observable2DPreset {
    label: string;
    xScript: string;
    yScript: string;
    xMin: number;
    xMax: number;
    yMin: number;
    yMax: number;
    nx: number;
    ny: number;
    description: string;
  }

  const PRESETS_2D: Observable2DPreset[] = [
    {
      label: "pT vs η",
      xScript: "event.momenta[0].pt()",
      yScript: "event.momenta[0].eta()",
      xMin: 0, xMax: 60, yMin: -5, yMax: 5,
      nx: 25, ny: 25,
      description: "Transverse momentum vs pseudorapidity correlation",
    },
    {
      label: "pT vs φ",
      xScript: "event.momenta[0].pt()",
      yScript: "event.momenta[0].phi()",
      xMin: 0, xMax: 60, yMin: -3.15, yMax: 3.15,
      nx: 25, ny: 25,
      description: "Transverse momentum vs azimuthal angle",
    },
    {
      label: "η₁ vs η₂",
      xScript: "event.momenta[0].eta()",
      yScript: "event.momenta[1].eta()",
      xMin: -5, xMax: 5, yMin: -5, yMax: 5,
      nx: 20, ny: 20,
      description: "Pseudorapidity correlation between first two particles",
    },
    {
      label: "E vs pT",
      xScript: "event.momenta[0].e()",
      yScript: "event.momenta[0].pt()",
      xMin: 0, xMax: 100, yMin: 0, yMax: 60,
      nx: 25, ny: 25,
      description: "Energy vs transverse momentum",
    },
  ];

  // ---------------------------------------------------------------------------
  // Detector Presets
  // ---------------------------------------------------------------------------
  interface DetectorPresetInfo {
    value: DetectorPreset | '';
    label: string;
    description: string;
  }

  const DETECTOR_PRESETS: DetectorPresetInfo[] = [
    { value: '', label: 'None (truth-level)', description: 'No detector simulation - observables use truth-level kinematics' },
    { value: 'perfect', label: 'Perfect Detector', description: 'No smearing, 100% efficiency - useful for validation' },
    { value: 'lhc_like', label: 'LHC-like (ATLAS/CMS)', description: 'R = 0.4, pT > 25 GeV, hadronic σ/E ∝ 0.50/√E' },
    { value: 'ilc_like', label: 'ILC-like (e⁺e⁻)', description: 'R = 0.7, pT > 10 GeV, hadronic σ/E ∝ 0.30/√E' },
  ];

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------
  /** Analysis mode: 1D bar charts or 2D heatmaps. */
  let analysisMode: "1d" | "2d" = "1d";

  let plotName: string = "Leading pT";
  let observableScript: string = "event.momenta[0].pt()";
  let histMin: number = 0;
  let histMax: number = 60;
  let nBins: number = 30;
  let cmsEnergy: number = 100;
  let numEvents: number = 5000;
  let nFinalState: number = 2;
  let cutScript: string = "";

  // 2D-specific state
  let plot2DName: string = "pT vs η";
  let xObservableScript: string = "event.momenta[0].pt()";
  let yObservableScript: string = "event.momenta[0].eta()";
  let xMin2D: number = 0;
  let xMax2D: number = 60;
  let yMin2D: number = -5;
  let yMax2D: number = 5;
  let nx2D: number = 25;
  let ny2D: number = 25;

  // Detector simulation state
  let detectorPreset: DetectorPreset | '' = '';
  let particleKinds: ParticleKind[] = [];

  // NLO corrections state (Phase 46)
  let nloEnabled: boolean = false;
  let nloScheme: "CataniSeymour" | "FKS" | "Antenna" = "CataniSeymour";
  let nloAlpha: number = 1.0;

  // Parton shower state (Phase 46)
  let showerEnabled: boolean = false;
  let showerProvider: "pythia8" | "herwig7" | "sherpa" | "custom" = "pythia8";
  let showerHadronisation: boolean = true;
  let showerMPI: boolean = false;

  // Keep particle_kinds array in sync with final-state count.
  $: {
    if (particleKinds.length !== nFinalState) {
      particleKinds = Array(nFinalState).fill('hadron') as ParticleKind[];
    }
  }

  /** Whether a detector simulation is active. */
  $: detectorActive = detectorPreset !== '';

  let loading: boolean = false;
  let errorMsg: string = "";
  let scriptValid: boolean = true;
  let validationMsg: string = "";

  let logScale: boolean = false;
  let result: AnalysisResult | null = null;
  let activeHistogram: HistogramData | null = null;

  let activeHistogram2D: Histogram2DData | null = null;

  let canvasEl: HTMLCanvasElement;
  let chart: Chart | null = null;

  // WebGL heatmap reactive props
  let heatmapData: Float32Array = new Float32Array(0);
  let heatmapNx: number = 0;
  let heatmapNy: number = 0;
  let heatmapMax: number = 1;
  let heatmapXEdges: number[] = [];
  let heatmapYEdges: number[] = [];
  let heatmapTitle: string = "";
  let heatmapColorScale: "viridis" | "magma" = "viridis";

  function cssVar(name: string, fallback: string): string {
    if (typeof window === "undefined") return fallback;
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }

  // ---------------------------------------------------------------------------
  // Preset Selection
  // ---------------------------------------------------------------------------
  function applyPreset(preset: ObservablePreset): void {
    plotName = preset.label;
    observableScript = preset.script;
    histMin = preset.min;
    histMax = preset.max;
    nBins = preset.nBins;
    logAnalysis(`Analysis preset: ${preset.label}`);
  }

  interface Observable2DPreset {
    label: string;
    xScript: string;
    yScript: string;
    nx: number;
    ny: number;
    xMin: number;
    xMax: number;
    yMin: number;
    yMax: number;
    description: string;
  }

  function apply2DPreset(preset: Observable2DPreset): void {
    plot2DName = preset.label;
    xObservableScript = preset.xScript;
    yObservableScript = preset.yScript;
    nx2D = preset.nx;
    ny2D = preset.ny;
    xMin2D = preset.xMin;
    xMax2D = preset.xMax;
    yMin2D = preset.yMin;
    yMax2D = preset.yMax;
    logAnalysis(`2D preset: ${preset.label}`);
  }

  // ---------------------------------------------------------------------------
  // Script Validation
  // ---------------------------------------------------------------------------
  let validationTimer: ReturnType<typeof setTimeout> | null = null;
  let interopUnsub: (() => void) | null = null;

  function handleScriptInput(): void {
    if (validationTimer) clearTimeout(validationTimer);
    validationTimer = setTimeout(async () => {
      if (!observableScript.trim()) {
        scriptValid = false;
        validationMsg = "Script cannot be empty";
        return;
      }
      try {
        await validateScript(observableScript);
        scriptValid = true;
        validationMsg = "✓ Valid";
      } catch (err) {
        scriptValid = false;
        validationMsg = String(err);
      }
    }, 400);
  }

  // ---------------------------------------------------------------------------
  // Run Analysis
  // ---------------------------------------------------------------------------
  async function handleRun(): Promise<void> {
    loading = true;
    errorMsg = "";
    result = null;
    activeHistogram = null;
    activeHistogram2D = null;

    // Client-side validation.
    if (analysisMode === '1d') {
      if (nBins < 1 || nBins > 1000) {
        errorMsg = "Bin count must be between 1 and 1000.";
        loading = false;
        return;
      }
      if (histMin >= histMax) {
        errorMsg = "Histogram min must be less than max.";
        loading = false;
        return;
      }
    } else {
      if (nx2D < 1 || nx2D > 500 || ny2D < 1 || ny2D > 500) {
        errorMsg = "2D bin counts must be between 1 and 500.";
        loading = false;
        return;
      }
      if (xMin2D >= xMax2D || yMin2D >= yMax2D) {
        errorMsg = "2D axis min must be less than max.";
        loading = false;
        return;
      }
    }
    if (cmsEnergy <= 0) {
      errorMsg = "CMS energy must be positive.";
      loading = false;
      return;
    }
    if (numEvents < 1 || numEvents > 1_000_000) {
      errorMsg = "Number of events must be between 1 and 1,000,000.";
      loading = false;
      return;
    }

    const finalMasses = Array(nFinalState).fill(0.0);
    const cutScripts = cutScript.trim() ? [cutScript.trim()] : [];

    // Build 1D plots array (always include if mode is 1d).
    const plots1D = analysisMode === '1d'
      ? [{
          name: plotName,
          observable_script: observableScript,
          n_bins: nBins,
          min: histMin,
          max: histMax,
        }]
      : [];

    // Build 2D plots array (include if mode is 2d).
    const plots2D = analysisMode === '2d'
      ? [{
          name: plot2DName,
          x_observable_script: xObservableScript,
          y_observable_script: yObservableScript,
          nx: nx2D,
          ny: ny2D,
          x_min: xMin2D,
          x_max: xMax2D,
          y_min: yMin2D,
          y_max: yMax2D,
        }]
      : null;

    try {
      const modeLabel = analysisMode === '1d' ? plotName : plot2DName;
      logAnalysis(
        `Running ${analysisMode.toUpperCase()} analysis: ${modeLabel}, ${numEvents} events at √s = ${cmsEnergy} GeV`,
      );

      // Send NLO configuration if enabled (Phase 46).
      if (nloEnabled) {
        await configureNlo({
          enabled: true,
          subtraction_scheme: nloScheme,
          y_min: 0.0,
          y_max: 1.0,
          alpha: nloAlpha,
        });
        logAnalysis(`NLO corrections enabled - scheme: ${nloScheme}, α = ${nloAlpha}`);
      }

      // Send shower configuration if enabled (Phase 46).
      if (showerEnabled) {
        await configureShower({
          enabled: true,
          provider: showerProvider,
          executable_path: "",
          hadronisation: showerHadronisation,
          qed_radiation: false,
          mpi: showerMPI,
          seed: 42,
        });
        logAnalysis(`Parton shower enabled - provider: ${showerProvider}`);
      }

      const analysisResult = await runAnalysis({
        plots: plots1D,
        plots_2d: plots2D,
        cut_scripts: cutScripts,
        num_events: numEvents,
        cms_energy: cmsEnergy,
        final_masses: finalMasses,
        detector_preset: detectorPreset || null,
        particle_kinds: detectorPreset ? particleKinds : null,
      });

      result = analysisResult;
      extractAndPushProfile(analysisResult, `Analysis: ${analysisResult.events_generated} events`);

      if (analysisMode === '1d' && analysisResult.histograms.length > 0) {
        activeHistogram = analysisResult.histograms[0];
        renderChart(activeHistogram);
      } else if (analysisMode === '2d' && analysisResult.histograms_2d.length > 0) {
        activeHistogram2D = analysisResult.histograms_2d[0];
        // Feed data to the reactive WebGL heatmap component.
        const h = activeHistogram2D;
        heatmapData = new Float32Array(h.bin_contents);
        heatmapNx = h.nx;
        heatmapNy = h.ny;
        heatmapMax = Math.max(...h.bin_contents, 1e-30);
        heatmapXEdges = h.x_bin_edges;
        heatmapYEdges = h.y_bin_edges;
        heatmapTitle = h.name;
      }

      const xsecPb = analysisResult.cross_section * 0.3894e9;
      const errPb = analysisResult.cross_section_error * 0.3894e9;
      addCitations(["kleiss1986", "james1980", "peskin1995"]);
      logAnalysis(
        `Analysis complete: ${analysisResult.events_passed}/${analysisResult.events_generated} events passed` +
          ` | σ ≈ ${xsecPb.toExponential(3)} ± ${errPb.toExponential(2)} pb`,
      );
    } catch (err) {
      errorMsg = String(err);
      logAnalysis(`Analysis error: ${errorMsg}`);
    } finally {
      loading = false;
    }
  }

  // ---------------------------------------------------------------------------
  // Chart Rendering
  // ---------------------------------------------------------------------------
  function renderChart(data: HistogramData): void {
    if (!canvasEl) return;

    // Destroy previous chart.
    if (chart) {
      chart.destroy();
      chart = null;
    }

    // Build bin-centre labels.
    const labels: string[] = [];
    for (let i = 0; i < data.bin_contents.length; i++) {
      const centre =
        (data.bin_edges[i] + data.bin_edges[i + 1]) / 2;
      labels.push(centre.toFixed(2));
    }

    // Colour gradient from cool (low) to warm (high).
    const maxVal = Math.max(...data.bin_contents, 1e-30);
    const bgColors = data.bin_contents.map((v) => {
      const ratio = v / maxVal;
      const r = Math.round(30 + 200 * ratio);
      const g = Math.round(100 + 60 * (1 - ratio));
      const b = Math.round(220 - 150 * ratio);
      return `rgba(${r}, ${g}, ${b}, 0.85)`;
    });

    const borderColors = bgColors.map((c) => c.replace("0.85", "1.0"));
    const titleColor = cssVar("--color-text-primary", "#e8e8e8");
    const mutedColor = cssVar("--color-text-muted", "#8a8a8a");
    const textPrimaryRgb = cssVar("--color-text-primary-rgb", "232, 232, 232");
    const gridMinor = `rgba(${textPrimaryRgb}, 0.05)`;
    const gridMajor = `rgba(${textPrimaryRgb}, 0.08)`;

    chart = new Chart(canvasEl, {
      type: "bar",
      data: {
        labels,
        datasets: [
          {
            label: data.name,
            data: data.bin_contents,
            backgroundColor: bgColors,
            borderColor: borderColors,
            borderWidth: 1,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          title: {
            display: true,
            text: data.name,
            color: titleColor,
            font: { size: 14 },
          },
          legend: {
            display: false,
          },
          tooltip: {
            callbacks: {
              title: (items) => {
                const idx = items[0].dataIndex;
                const lo = data.bin_edges[idx].toFixed(2);
                const hi = data.bin_edges[idx + 1].toFixed(2);
                return `[${lo}, ${hi})`;
              },
              label: (item) => {
                const val = (item.raw as number).toExponential(3);
                const err = data.bin_errors[item.dataIndex].toExponential(2);
                return `${val} ± ${err}`;
              },
            },
          },
        },
        scales: {
          x: {
            title: {
              display: true,
              text: data.name,
              color: mutedColor,
            },
            ticks: { color: mutedColor, maxTicksLimit: 10 },
            grid: { color: gridMinor },
          },
          y: {
            type: logScale ? "logarithmic" : "linear",
            title: {
              display: true,
              text: "Weighted Counts",
              color: mutedColor,
            },
            ticks: { color: mutedColor },
            grid: { color: gridMajor },
          },
        },
      },
    });
  }

  // ---------------------------------------------------------------------------
  // Scale Toggle
  // ---------------------------------------------------------------------------
  function toggleScale(): void {
    logScale = !logScale;
    if (activeHistogram) {
      renderChart(activeHistogram);
    }
  }

  // ---------------------------------------------------------------------------
  // Export & Context Menus
  // ---------------------------------------------------------------------------
  function exportHistogramCsv(): void {
    if (!activeHistogram) return;
    const h = activeHistogram;
    const nBinsCount = h.bin_edges.length - 1;
    const rows = ["bin_low,bin_high,contents,error,density"];
    for (let i = 0; i < nBinsCount; i++) {
      const lo = h.bin_edges[i];
      const hi = h.bin_edges[i + 1];
      const n = h.bin_contents[i] ?? 0;
      const err = h.bin_errors[i] ?? 0;
      const width = hi - lo;
      const dens = width > 0 ? n / width : n;
      rows.push(`${lo.toFixed(4)},${hi.toFixed(4)},${n.toFixed(6)},${err.toFixed(6)},${dens.toFixed(6)}`);
    }
    const blob = new Blob([rows.join("\n")], { type: "text/csv" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${h.name || "histogram"}.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function openChartContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    showContextMenu(e.clientX, e.clientY, [
      {
        type: "action",
        id: "ana-export-csv",
        label: "Export Histogram CSV",
        icon: "⬇",
        disabled: !activeHistogram,
        action: exportHistogramCsv,
      },
      { type: "separator", id: "ana-sep1" },
      {
        type: "action",
        id: "ana-copy-xsec",
        label: "Copy Cross Section",
        icon: "σ",
        disabled: !result,
        action: () => {
          if (!result) return;
          const pb = (result.cross_section * 0.3894e9).toExponential(4);
          navigator.clipboard.writeText(`σ = ${pb} pb`);
        },
      },
      {
        type: "action",
        id: "ana-copy-mean",
        label: "Copy Mean Value",
        icon: "μ",
        disabled: !activeHistogram,
        action: () => {
          if (!activeHistogram) return;
          navigator.clipboard.writeText(activeHistogram.mean.toFixed(6));
        },
      },
      {
        type: "action",
        id: "ana-toggle-log",
        label: activeHistogram ? (logScale ? "Switch to Linear Y" : "Switch to Log Y") : "Toggle Log Scale",
        icon: "↕",
        disabled: !activeHistogram,
        action: toggleScale,
      },
    ]);
  }

  function openStatsContext(e: MouseEvent): void {
    e.preventDefault();
    e.stopPropagation();
    if (!result) return;
    const pb = (result.cross_section * 0.3894e9).toExponential(4);
    const pbErr = (result.cross_section_error * 0.3894e9).toExponential(2);
    showContextMenu(e.clientX, e.clientY, [
      {
        type: "action",
        id: "ana-copy-sigma",
        label: "Copy σ ± δσ",
        icon: "σ",
        action: () => navigator.clipboard.writeText(`σ = ${pb} ± ${pbErr} pb`),
      },
      {
        type: "action",
        id: "ana-copy-events",
        label: "Copy Event Count",
        icon: "#",
        action: () => navigator.clipboard.writeText(
          `${result!.events_passed} / ${result!.events_generated} events`
        ),
      },
      { type: "separator", id: "ana-sep2" },
      {
        type: "action",
        id: "ana-export-csv2",
        label: "Export Histogram CSV",
        icon: "⬇",
        disabled: !activeHistogram,
        action: exportHistogramCsv,
      },
    ]);
  }

  // ---------------------------------------------------------------------------
  // Lifecycle
  // ---------------------------------------------------------------------------
  const ANALYSIS_CMD_IDS = [
    "spire.analysis.run_mc",
  ];

  onMount(() => {
    handleScriptInput();
    interopUnsub = widgetInteropState.subscribe((state) => {
      const reactionPayload = state.reaction?.payload as
        | { cmsEnergy?: number; finalState?: string[] }
        | undefined;
      if (!reactionPayload || loading || result) return;

      if (typeof reactionPayload.cmsEnergy === "number" && Number.isFinite(reactionPayload.cmsEnergy)) {
        cmsEnergy = reactionPayload.cmsEnergy;
      }

      if (Array.isArray(reactionPayload.finalState) && reactionPayload.finalState.length > 0) {
        nFinalState = reactionPayload.finalState.length;
      }
    });

    registerCommand({
      id: "spire.analysis.run_mc",
      title: "Run Monte Carlo Integration",
      category: "Analysis",
      shortcut: "Mod+Shift+M",
      execute: () => handleRun(),
      pinned: true,
      icon: "A",
    });
  });

  onDestroy(() => {
    for (const id of ANALYSIS_CMD_IDS) unregisterCommand(id);
    interopUnsub?.();
    if (chart) {
      chart.destroy();
      chart = null;
    }
    if (validationTimer) clearTimeout(validationTimer);
  });

  $: publishWidgetInterop("analysis", {
    mode: analysisMode,
    cmsEnergy,
    numEvents,
    nFinalState,
    eventsGenerated: result?.events_generated ?? null,
    eventsPassed: result?.events_passed ?? null,
    crossSection: result?.cross_section ?? null,
    histogramCount: result?.histograms.length ?? 0,
    histogram2DCount: result?.histograms_2d.length ?? 0,
  });
</script>

<!-- ======================================================================= -->
<!-- Template                                                                 -->
<!-- ======================================================================= -->

<div class="analysis-widget" data-tour-id="analysis-widget" use:isolateEvents>
  <!-- Configuration Panel -->
  <div class="config-panel">
    <!-- Mode Toggle -->
    <div class="mode-toggle">
      <button
        class="mode-btn"
        class:active={analysisMode === '1d'}
        on:click={() => analysisMode = '1d'}
      >1D Histogram</button>
      <button
        class="mode-btn"
        class:active={analysisMode === '2d'}
        on:click={() => analysisMode = '2d'}
      >2D Heatmap</button>
    </div>

    {#if analysisMode === '1d'}
    <!-- ── 1D Mode ─────────────────────────────────────────────────── -->
    <!-- Preset Selector -->
    <div class="preset-row">
      <span class="preset-label">Presets:</span>
      <div class="preset-buttons">
        {#each PRESETS as preset}
          <button
            class="preset-btn"
            use:tooltip={{ text: preset.description }}
            on:click={() => applyPreset(preset)}
          >
            {preset.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Observable Script -->
    <div class="field">
      <label for="obs-script">Observable Script (Rhai):</label>
      <textarea
        id="obs-script"
        bind:value={observableScript}
        on:input={handleScriptInput}
        rows="3"
        spellcheck="false"
        class:invalid={!scriptValid}
      ></textarea>
      <span class="validation-msg" class:valid={scriptValid} class:error={!scriptValid}>
        {validationMsg}
      </span>
    </div>

    <!-- Plot Name & Binning -->
    <div class="field-row">
      <div class="field compact">
        <label for="plot-name">Name:</label>
        <input id="plot-name" type="text" bind:value={plotName} />
      </div>
      <div class="field compact">
        <label for="n-bins">Bins:</label>
        <SpireNumberInput inputId="n-bins" bind:value={nBins} min={1} max={1000} step={1} ariaLabel="Histogram bins" />
      </div>
      <div class="field compact">
        <label for="hist-min">Min:</label>
        <SpireNumberInput inputId="hist-min" bind:value={histMin} step={0.1} ariaLabel="Histogram minimum" />
      </div>
      <div class="field compact">
        <label for="hist-max">Max:</label>
        <SpireNumberInput inputId="hist-max" bind:value={histMax} step={0.1} ariaLabel="Histogram maximum" />
      </div>
    </div>

    {:else}
    <!-- ── 2D Mode ─────────────────────────────────────────────────── -->
    <div class="preset-row">
      <span class="preset-label">2D Presets:</span>
      <div class="preset-buttons">
        {#each PRESETS_2D as p2d}
          <button
            class="preset-btn"
            use:tooltip={{ text: p2d.description }}
            on:click={() => apply2DPreset(p2d)}
          >{p2d.label}</button>
        {/each}
      </div>
    </div>

    <div class="field">
      <label for="x-obs-script">X Observable Script (Rhai):</label>
      <textarea
        id="x-obs-script"
        bind:value={xObservableScript}
        rows="2"
        spellcheck="false"
      ></textarea>
    </div>

    <div class="field">
      <label for="y-obs-script">Y Observable Script (Rhai):</label>
      <textarea
        id="y-obs-script"
        bind:value={yObservableScript}
        rows="2"
        spellcheck="false"
      ></textarea>
    </div>

    <div class="field-row">
      <div class="field compact">
        <label for="plot-name-2d">Name:</label>
        <input id="plot-name-2d" type="text" bind:value={plot2DName} />
      </div>
      <div class="field compact">
        <label for="nx-bins">X Bins:</label>
        <SpireNumberInput inputId="nx-bins" bind:value={nx2D} min={1} max={500} step={1} ariaLabel="2D X bins" />
      </div>
      <div class="field compact">
        <label for="ny-bins">Y Bins:</label>
        <SpireNumberInput inputId="ny-bins" bind:value={ny2D} min={1} max={500} step={1} ariaLabel="2D Y bins" />
      </div>
    </div>

    <div class="field-row">
      <div class="field compact">
        <label for="x-min-2d">X Min:</label>
        <SpireNumberInput inputId="x-min-2d" bind:value={xMin2D} step={0.1} ariaLabel="2D X minimum" />
      </div>
      <div class="field compact">
        <label for="x-max-2d">X Max:</label>
        <SpireNumberInput inputId="x-max-2d" bind:value={xMax2D} step={0.1} ariaLabel="2D X maximum" />
      </div>
      <div class="field compact">
        <label for="y-min-2d">Y Min:</label>
        <SpireNumberInput inputId="y-min-2d" bind:value={yMin2D} step={0.1} ariaLabel="2D Y minimum" />
      </div>
      <div class="field compact">
        <label for="y-max-2d">Y Max:</label>
        <SpireNumberInput inputId="y-max-2d" bind:value={yMax2D} step={0.1} ariaLabel="2D Y maximum" />
      </div>
    </div>
    {/if}

    <!-- Physics Parameters (shared) -->
    <div class="field-row">
      <div class="field compact">
        <label for="cms-energy"><HoverDef term="cross_section">√s</HoverDef> (GeV):</label>
        <SpireNumberInput inputId="cms-energy" bind:value={cmsEnergy} min={0.1} step={0.1} ariaLabel="CMS energy" />
      </div>
      <div class="field compact">
        <label for="num-events"><HoverDef term="mc_events">Events</HoverDef>:</label>
        <SpireNumberInput inputId="num-events" bind:value={numEvents} min={1} max={1000000} step={100} ariaLabel="Event count" />
      </div>
      <div class="field compact">
        <label for="n-final">Final-state N:</label>
        <SpireNumberInput inputId="n-final" bind:value={nFinalState} min={2} max={8} step={1} ariaLabel="Final-state multiplicity" />
      </div>
    </div>

    <!-- Optional Cut Script -->
    <div class="field">
      <label for="cut-script">Cut Script (optional):</label>
      <input
        id="cut-script"
        type="text"
        bind:value={cutScript}
        placeholder="e.g., event.momenta[0].pt() > 20.0"
      />
    </div>

    <!-- Detector Simulation -->
    <div class="field">
      <label for="detector-preset">Detector Simulation:</label>
      <select id="detector-preset" bind:value={detectorPreset}>
        {#each DETECTOR_PRESETS as dp}
          <option value={dp.value}>{dp.label}</option>
        {/each}
      </select>
      {#if detectorActive}
        <span class="detector-hint">
          {DETECTOR_PRESETS.find(d => d.value === detectorPreset)?.description ?? ''}
        </span>
      {/if}
    </div>

    <!-- Particle Kinds (when detector is active) -->
    {#if detectorActive}
      <div class="field">
        <span class="field-label">Particle Kinds:</span>
        <div class="particle-kinds-row">
          {#each particleKinds as kind, i}
            <div class="particle-kind-item">
              <span class="pk-label">p{i + 1}:</span>
              <select bind:value={particleKinds[i]}>
                <option value="hadron">hadron</option>
                <option value="electron">electron</option>
                <option value="muon">muon</option>
                <option value="photon">photon</option>
                <option value="invisible">invisible</option>
              </select>
            </div>
          {/each}
        </div>
      </div>

      <!-- Reco-Level Observable Presets -->
      <div class="preset-section">
        <span class="field-label">Reco Presets:</span>
        <div class="preset-row">
          {#each RECO_PRESETS as rp}
            <button
              class="preset-btn"
              use:tooltip={{ text: rp.description }}
              on:click={() => applyPreset(rp)}
            >{rp.label}</button>
          {/each}
        </div>
      </div>
    {/if}

    <!-- NLO Corrections (Phase 46) -->
    <div class="field nlo-toggle">
      <label class="toggle-row">
        <input type="checkbox" bind:checked={nloEnabled} />
        <span>NLO Corrections</span>
      </label>
      {#if nloEnabled}
        <div class="sub-options">
          <div class="sub-field">
            <label for="nlo-scheme">Subtraction Scheme:</label>
            <select id="nlo-scheme" bind:value={nloScheme}>
              <option value="CataniSeymour">Catani–Seymour</option>
              <option value="FKS">FKS</option>
              <option value="Antenna">Antenna</option>
            </select>
          </div>
          <div class="sub-field">
            <label for="nlo-alpha">α parameter:</label>
            <SpireNumberInput inputId="nlo-alpha" step={0.1} min={0} max={2} bind:value={nloAlpha} ariaLabel="NLO alpha parameter" />
          </div>
        </div>
      {/if}
    </div>

    <!-- Parton Shower (Phase 46) -->
    <div class="field shower-toggle">
      <label class="toggle-row">
        <input type="checkbox" bind:checked={showerEnabled} />
        <span>Parton Shower</span>
      </label>
      {#if showerEnabled}
        <div class="sub-options">
          <div class="sub-field">
            <label for="shower-provider">Provider:</label>
            <select id="shower-provider" bind:value={showerProvider}>
              <option value="pythia8">Pythia 8</option>
              <option value="herwig7">Herwig 7</option>
              <option value="sherpa">Sherpa</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <label class="toggle-row sub-toggle">
            <input type="checkbox" bind:checked={showerHadronisation} />
            <span>Hadronisation</span>
          </label>
          <label class="toggle-row sub-toggle">
            <input type="checkbox" bind:checked={showerMPI} />
            <span>Multi-Parton Interactions</span>
          </label>
        </div>
      {/if}
    </div>

    <!-- Run Button -->
    <div class="run-row">
      <button
        class="run-btn"
        on:click={handleRun}
        disabled={loading || (analysisMode === '1d' && !scriptValid)}
      >
        {#if loading}
          ... Running
        {:else}
          > Run Analysis
        {/if}
      </button>
      {#if activeHistogram && analysisMode === '1d'}
        <button class="scale-btn" on:click={toggleScale}>
          {logScale ? "Linear" : "Log"} Y
        </button>
      {/if}
    </div>

    <!-- Error Display -->
    {#if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {/if}

    <!-- Summary Statistics -->
    {#if result}
      <div class="stats-panel" role="region" aria-label="Analysis summary statistics" on:contextmenu={openStatsContext}>
        <div class="stat">
          <span class="stat-label">Events:</span>
          <span class="stat-value">
            {result.events_passed.toLocaleString()} / {result.events_generated.toLocaleString()} passed
          </span>
        </div>
        <div class="stat">
          <span class="stat-label">σ (pb):</span>
          <span class="stat-value">
            {(result.cross_section * 0.3894e9).toExponential(4)}
            ± {(result.cross_section_error * 0.3894e9).toExponential(2)}
          </span>
        </div>
        {#if activeHistogram}
          <div class="stat">
            <span class="stat-label">Mean:</span>
            <span class="stat-value">{activeHistogram.mean.toFixed(3)}</span>
          </div>
          <div class="stat">
            <span class="stat-label">UF / OF:</span>
            <span class="stat-value">
              {activeHistogram.underflow.toExponential(2)} / {activeHistogram.overflow.toExponential(2)}
            </span>
          </div>
        {/if}
        {#if activeHistogram2D}
          <div class="stat">
            <span class="stat-label">Entries:</span>
            <span class="stat-value">{activeHistogram2D.entries.toLocaleString()}</span>
          </div>
          <div class="stat">
            <span class="stat-label">Grid:</span>
            <span class="stat-value">{activeHistogram2D.nx}×{activeHistogram2D.ny}</span>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Chart / Heatmap Canvas -->
  <div
    class="chart-container"
    role="img"
    aria-label="Analysis chart area"
    use:isolateEvents={{ wheel: true, pointer: false, mouse: false, touch: true }}
    on:contextmenu={openChartContext}
  >
    {#if analysisMode === '1d'}
      <canvas bind:this={canvasEl}></canvas>
      {#if !activeHistogram && !loading}
        <div class="placeholder">
          Configure an observable and click <strong>Run Analysis</strong> to generate a histogram.
        </div>
      {/if}
    {:else}
      {#if activeHistogram2D}
        <WebglHeatmap
          data={heatmapData}
          nx={heatmapNx}
          ny={heatmapNy}
          maxVal={heatmapMax}
          xEdges={heatmapXEdges}
          yEdges={heatmapYEdges}
          title={heatmapTitle}
          colorScale={heatmapColorScale}
        />
      {:else if !loading}
        <div class="placeholder">
          Configure X/Y observables and click <strong>Run Analysis</strong> to generate a 2D heatmap.
        </div>
      {/if}
    {/if}
  </div>
</div>

<!-- ======================================================================= -->
<!-- Styles                                                                   -->
<!-- ======================================================================= -->

<style>
  .analysis-widget {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    height: 100%;
    overflow-y: auto;
    font-size: 0.85rem;
  }

  .config-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.5rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 55%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 80%, transparent);
    border-radius: var(--radius-md);
  }

  .preset-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .preset-row .preset-label {
    color: var(--color-accent);
    font-weight: 600;
    white-space: nowrap;
  }

  .preset-buttons {
    display: flex;
    gap: 0.3rem;
    flex-wrap: wrap;
  }

  .preset-btn {
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid rgba(var(--color-accent-rgb), 0.3);
    border-radius: var(--radius-sm);
    background: rgba(var(--color-accent-rgb), 0.08);
    color: var(--color-text-primary);
    cursor: pointer;
    transition: background var(--transition-fast);
  }

  .preset-btn:hover {
    background: rgba(var(--color-accent-rgb), 0.2);
    color: var(--color-text-primary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .field label,
  .field-label {
    color: var(--color-text-muted);
    font-size: 0.78rem;
  }

  .field-row {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .field.compact {
    flex: 1;
    min-width: 80px;
  }

  input,
  textarea {
    background: color-mix(in srgb, var(--color-bg-base) 70%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 65%, transparent);
    border-radius: var(--radius-sm);
    color: var(--color-text-primary);
    padding: 0.3rem 0.5rem;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.8rem;
  }

  textarea {
    resize: vertical;
    min-height: 2.5rem;
  }

  textarea.invalid {
    border-color: rgba(255, 80, 80, 0.5);
  }

  .validation-msg {
    font-size: 0.72rem;
  }

  .validation-msg.valid {
    color: var(--color-success);
  }

  .validation-msg.error {
    color: var(--color-error);
  }

  .run-row {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .run-btn {
    padding: 0.4rem 1.2rem;
    font-size: 0.85rem;
    font-weight: 600;
    border: none;
    border-radius: var(--radius-md);
    background: color-mix(in srgb, var(--color-success) 25%, var(--color-bg-surface));
    border: 1px solid color-mix(in srgb, var(--color-success) 70%, var(--color-border));
    color: var(--color-text-primary);
    cursor: pointer;
    transition: opacity var(--transition-fast), background-color var(--transition-fast);
  }

  .run-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .run-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .scale-btn {
    padding: 0.3rem 0.8rem;
    font-size: 0.78rem;
    border: 1px solid rgba(var(--color-accent-rgb), 0.3);
    border-radius: var(--radius-sm);
    background: rgba(var(--color-accent-rgb), 0.1);
    color: var(--color-accent);
    cursor: pointer;
  }

  .scale-btn:hover {
    background: rgba(var(--color-accent-rgb), 0.25);
  }

  .error-box {
    padding: 0.4rem 0.6rem;
    background: rgba(var(--color-error-rgb), 0.12);
    border: 1px solid rgba(var(--color-error-rgb), 0.3);
    border-radius: var(--radius-sm);
    color: var(--color-error);
    font-size: 0.8rem;
    word-break: break-word;
  }

  .stats-panel {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
    padding: 0.4rem 0;
    border-top: 1px solid color-mix(in srgb, var(--color-border) 40%, transparent);
  }

  .stat {
    display: flex;
    gap: 0.3rem;
  }

  .stat-label {
    color: var(--color-text-muted);
    font-size: 0.78rem;
  }

  .stat-value {
    color: var(--color-text-primary);
    font-family: "JetBrains Mono", "Fira Code", monospace;
    font-size: 0.78rem;
  }

  .chart-container {
    position: relative;
    flex: 1;
    min-height: 250px;
  }

  .chart-container canvas {
    width: 100% !important;
    height: 100% !important;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--color-text-muted);
    font-size: 0.85rem;
    text-align: center;
    padding: 1rem;
  }

  /* --- Detector Simulation UI --- */

  select {
    background: color-mix(in srgb, var(--color-bg-base) 70%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-border) 65%, transparent);
    border-radius: var(--radius-sm);
    color: var(--color-text-primary);
    padding: 0.3rem 0.5rem;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .detector-hint {
    font-size: 0.72rem;
    color: var(--color-text-muted);
    font-style: italic;
    margin-top: 0.1rem;
  }

  .particle-kinds-row {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .particle-kind-item {
    display: flex;
    align-items: center;
    gap: 0.2rem;
  }

  .pk-label {
    font-size: 0.72rem;
    color: var(--color-text-muted);
    font-family: "JetBrains Mono", "Fira Code", monospace;
  }

  .particle-kind-item select {
    font-size: 0.72rem;
    padding: 0.15rem 0.3rem;
  }

  .preset-section {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .preset-section .field-label {
    color: var(--color-text-muted);
    font-size: 0.78rem;
  }

  /* --- Mode Toggle --- */

  .mode-toggle {
    display: flex;
    gap: 0.3rem;
    margin-bottom: 0.25rem;
  }

  .mode-btn {
    flex: 1;
    padding: 0.3rem 0.8rem;
    font-size: 0.78rem;
    font-weight: 600;
    border: 1px solid rgba(var(--color-accent-rgb), 0.2);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--color-bg-elevated) 35%, transparent);
    color: var(--color-text-muted);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .mode-btn.active {
    background: rgba(var(--color-accent-rgb), 0.15);
    color: var(--color-accent);
    border-color: rgba(var(--color-accent-rgb), 0.5);
  }

  .mode-btn:hover:not(.active) {
    background: color-mix(in srgb, var(--color-bg-elevated) 60%, transparent);
    color: var(--color-text-primary);
  }

  /* --- NLO & Shower Toggles (Phase 46) --- */

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    cursor: pointer;
    font-size: 0.82rem;
    color: var(--color-text-primary);
  }

  .toggle-row input[type="checkbox"] {
    accent-color: var(--color-accent);
    width: 14px;
    height: 14px;
    cursor: pointer;
  }

  .sub-options {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    padding: 0.4rem 0 0 1.4rem;
    border-left: 2px solid rgba(var(--color-accent-rgb), 0.15);
    margin-top: 0.3rem;
  }

  .sub-field {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
  }

  .sub-field label {
    color: var(--color-text-muted);
    white-space: nowrap;
    min-width: 0;
  }

  .sub-field select,
  .sub-field :global(.spire-number-input) {
    font-size: 0.75rem;
    background: color-mix(in srgb, var(--color-bg-elevated) 65%, transparent);
  }

  .sub-toggle {
    font-size: 0.78rem;
    color: var(--color-text-muted);
  }

  .nlo-toggle,
  .shower-toggle {
    border-top: 1px solid color-mix(in srgb, var(--color-border) 40%, transparent);
    padding-top: 0.5rem;
    margin-top: 0.25rem;
  }
</style>
