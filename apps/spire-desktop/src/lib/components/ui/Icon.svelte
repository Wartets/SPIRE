<script lang="ts">
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

  const GLYPHS: Record<string, string> = {
    play: "▶",
    pause: "⏸",
    "skip-back": "⏮",
    "skip-forward": "⏭",
    stop: "⏹",
    square: "□",
    close: "✕",
    x: "✕",
    copy: "⧉",
    search: "⌕",
    "file-search": "🔎",
    link: "🔗",
    external: "↗",
    docs: "📘",
    grip: "⋮",
    row: "⬌",
    column: "⬍",
    panels: "⧉",
    plus: "+",
    edit: "✎",
    left: "←",
    right: "→",
    up: "↑",
    down: "↓",
    check: "✓",
    circle: "●",
    "circle-dot": "◉",
    text: "TXT",
    image: "IMG",
    download: "⇩",
    upload: "⇧",
    workflow: "⇄",
    share: "⇪",
    refresh: "↻",
    reset: "↺",
    rocket: "⇢",
    atom: "⚛",
    sigma: "Σ",
    sun: "☼",
    settings: "⚙",
    notebook: "📓",
  };

  function normalize(raw: string): string {
    const trimmed = raw.trim();
    return ALIASES[trimmed] ?? trimmed.toLowerCase();
  }

  $: resolvedName = normalize(name);
  $: glyph = GLYPHS[resolvedName] ?? name;
</script>

{#if fallbackText && name}
  <span
    class="spire-icon spire-icon-fallback"
    title={title || undefined}
    aria-hidden={title ? undefined : true}
    style={`font-size: ${Math.max(10, size)}px; --icon-stroke-width: ${strokeWidth};`}
  >{glyph}</span>
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
    font-weight: 600;
  }
</style>