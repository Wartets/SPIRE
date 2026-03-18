<script lang="ts">
  import * as LucideIcons from "lucide-svelte";

  export let name = "";
  export let size = 14;
  export let strokeWidth = 1.8;
  export let title = "";
  export let fallbackText = true;

  const ALIASES: Record<string, string> = {
    "▶": "play",
    RUN: "play",
    "⏸": "pause",
    "⏮": "skip-back",
    "⏭": "skip-forward",
    "⏹": "stop",
    "✕": "close",
    "×": "close",
    CP: "copy",
    CPY: "copy",
    TXT: "text",
    TEX: "text",
    WEB: "search",
    arX: "file-search",
    URL: "link",
    DOI: "external",
    DOC: "docs",
    EL: "search",
    "⬌": "row",
    "⬍": "column",
    "↔": "workflow",
    "<->": "workflow",
    "⧉": "panels",
    "+": "plus",
    "✎": "edit",
    "←": "left",
    "→": "right",
    "↑": "up",
    "↓": "down",
    "⇡": "up",
    "⇧": "upload",
    "⇩": "download",
    "⇢": "rocket",
    "↺": "reset",
    "↻": "refresh",
    "✓": "check",
    "●": "circle",
    "◉": "circle-dot",
    PUB: "share",
    CFG: "settings",
    UI: "layout",
    PNG: "image",
    SVG: "image",
    JSON: "text",
    CSV: "text",
    PDF: "docs",
    RNG: "search",
    BR: "sigma",
    "Γ": "sigma",
    "σ": "sigma",
    "Σ": "sigma",
    "μ": "sigma",
    "λ": "sigma",
    "∂": "sigma",
    N: "hash",
    "#": "hash",
    "?": "help",
    SOL: "sun",
    "Λ": "atom",
    Z: "atom",
    h: "atom",
    B: "atom",
    K: "atom",
    M: "atom",
    P: "play",
    R: "reset",
    S: "save",
    A: "atom",
    "⊕": "copy",
    "↕": "move-vertical",
    "⤢": "expand",
    "⬇": "download",
    "◴": "clock",
    NB: "notebook",
  };

  const ICON_COMPONENT_BY_NAME: Record<string, string> = {
    play: "Play",
    pause: "Pause",
    "skip-back": "SkipBack",
    "skip-forward": "SkipForward",
    stop: "Square",
    square: "Square",
    close: "X",
    x: "X",
    copy: "Copy",
    search: "Search",
    "file-search": "FileSearch",
    link: "Link2",
    external: "ExternalLink",
    docs: "BookOpen",
    grip: "GripVertical",
    row: "Rows3",
    column: "Columns3",
    panels: "PanelsTopLeft",
    plus: "Plus",
    edit: "Pencil",
    left: "ArrowLeft",
    right: "ArrowRight",
    up: "ArrowUp",
    down: "ArrowDown",
    check: "Check",
    circle: "Circle",
    "circle-dot": "CircleDot",
    text: "Type",
    image: "Image",
    download: "Download",
    upload: "Upload",
    workflow: "GitCompareArrows",
    share: "Share2",
    refresh: "RefreshCw",
    reset: "RotateCcw",
    rocket: "Rocket",
    atom: "Atom",
    sigma: "Sigma",
    sun: "Sun",
    settings: "Settings",
    notebook: "NotebookPen",
    layout: "LayoutGrid",
    hash: "Hash",
    help: "CircleHelp",
    "move-vertical": "MoveVertical",
    expand: "Expand",
    save: "Save",
    clock: "Clock3",
  };

  function normalize(raw: string): string {
    const trimmed = raw.trim();
    return ALIASES[trimmed] ?? trimmed.toLowerCase();
  }

  $: resolvedName = normalize(name);
  $: resolvedComponentName = ICON_COMPONENT_BY_NAME[resolvedName] ?? "Circle";
  $: resolvedIcon =
    (LucideIcons as unknown as Record<string, any>)[resolvedComponentName] ?? null;
</script>

{#if fallbackText && name}
  <span class="spire-icon" title={title || undefined} aria-hidden={title ? undefined : true}>
    <svelte:component this={resolvedIcon as any} size={Math.max(10, size)} strokeWidth={strokeWidth} />
  </span>
{/if}

<style>
  .spire-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    line-height: 1;
    flex-shrink: 0;
  }

  .spire-icon :global(svg) {
    display: block;
  }

</style>