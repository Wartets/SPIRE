<script lang="ts">
  import type { ComponentType } from "svelte";
  import {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Atom,
    BookOpen,
    Check,
    Circle,
    CircleDot,
    Columns2,
    Copy,
    Download,
    ExternalLink,
    FileSearch,
    FileText,
    GripVertical,
    Image,
    Link2,
    NotebookPen,
    PanelsTopLeft,
    Pause,
    Play,
    Plus,
    RefreshCw,
    Rocket,
    RotateCcw,
    Rows3,
    Search,
    Settings2,
    Share2,
    Sigma,
    SkipBack,
    SkipForward,
    Square,
    SquarePen,
    Sun,
    Upload,
    Workflow,
    X,
  } from "lucide-svelte";

  export let name = "";
  export let size = 14;
  export let strokeWidth = 1.8;
  export let title = "";
  export let fallbackText = true;

  const ICONS: Record<string, ComponentType> = {
    play: Play,
    pause: Pause,
    "skip-back": SkipBack,
    "skip-forward": SkipForward,
    stop: Square,
    square: Square,
    close: X,
    x: X,
    copy: Copy,
    search: Search,
    "file-search": FileSearch,
    link: Link2,
    external: ExternalLink,
    docs: BookOpen,
    grip: GripVertical,
    row: Rows3,
    column: Columns2,
    panels: PanelsTopLeft,
    plus: Plus,
    edit: SquarePen,
    left: ArrowLeft,
    right: ArrowRight,
    up: ArrowUp,
    down: ArrowDown,
    check: Check,
    circle: Circle,
    "circle-dot": CircleDot,
    text: FileText,
    image: Image,
    download: Download,
    upload: Upload,
    workflow: Workflow,
    share: Share2,
    refresh: RefreshCw,
    reset: RotateCcw,
    rocket: Rocket,
    atom: Atom,
    sigma: Sigma,
    sun: Sun,
    settings: Settings2,
    notebook: NotebookPen,
  };

  const ALIASES: Record<string, string> = {
    "▶": "play",
    RUN: "play",
    "⏸": "pause",
    "⏮": "skip-back",
    "⏭": "skip-forward",
    "⏹": "stop",
    "✕": "close",
    CPY: "copy",
    TXT: "text",
    WEB: "search",
    arX: "file-search",
    URL: "link",
    DOI: "external",
    DOC: "docs",
    EL: "search",
    "⬌": "row",
    "⬍": "column",
    "⧉": "panels",
    "+": "plus",
    "✎": "edit",
    "←": "left",
    "→": "right",
    "⇧": "upload",
    "⇩": "download",
    "⇢": "rocket",
    "↺": "reset",
    "↻": "refresh",
    "✓": "check",
    "●": "circle",
    "◉": "circle-dot",
    PUB: "share",
    "<->": "workflow",
    CFG: "settings",
    PNG: "image",
    SVG: "image",
    JSON: "text",
    CSV: "text",
  };

  function normalize(raw: string): string {
    const trimmed = raw.trim();
    return ALIASES[trimmed] ?? trimmed.toLowerCase();
  }

  $: resolvedName = normalize(name);
  $: iconComponent = ICONS[resolvedName] ?? null;
</script>

{#if iconComponent}
  <span class="spire-icon" title={title || undefined} aria-hidden={title ? undefined : true}>
    <svelte:component this={iconComponent} {size} {strokeWidth} />
  </span>
{:else if fallbackText && name}
  <span class="spire-icon spire-icon-fallback" title={title || undefined}>{name}</span>
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

  .spire-icon-fallback {
    font-size: 0.8em;
  }
</style>