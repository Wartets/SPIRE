/**
 * SPIRE - Universal Export Utilities
 *
 * Helper functions for downloading generated data and graphics in
 * various formats.  Every widget that produces output (plots, tables,
 * diagrams) can call these to offer one-click export.
 *
 * Supported formats:
 *   - JSON (structured data)
 *   - CSV  (tabular data)
 *   - PNG  (rasterised canvas graphics)
 *   - SVG  (vector graphics)
 */

// ---------------------------------------------------------------------------
// Internal: trigger a file download via a hidden anchor element
// ---------------------------------------------------------------------------

function triggerDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.style.display = "none";
  document.body.appendChild(anchor);
  anchor.click();
  // Cleanup after a brief delay to ensure the download starts
  setTimeout(() => {
    URL.revokeObjectURL(url);
    document.body.removeChild(anchor);
  }, 100);
}

// ---------------------------------------------------------------------------
// JSON Export
// ---------------------------------------------------------------------------

/**
 * Download arbitrary data as a formatted JSON file.
 *
 * @param data     - Any JSON-serialisable value.
 * @param filename - Target filename (e.g., "analysis_result.json").
 */
export function downloadAsJson(data: unknown, filename: string): void {
  const json = JSON.stringify(data, null, 2);
  const blob = new Blob([json], { type: "application/json;charset=utf-8" });
  triggerDownload(blob, filename);
}

// ---------------------------------------------------------------------------
// CSV Export
// ---------------------------------------------------------------------------

/**
 * Download tabular data as a CSV file.
 *
 * Values are automatically quoted if they contain commas, quotes,
 * or newlines.
 *
 * @param headers - Column header labels.
 * @param rows    - Array of rows, each row being an array of cell values.
 * @param filename - Target filename (e.g., "histogram.csv").
 */
export function downloadAsCsv(
  headers: string[],
  rows: (string | number)[][],
  filename: string,
): void {
  function escapeCell(value: string | number): string {
    const str = String(value);
    if (str.includes(",") || str.includes('"') || str.includes("\n")) {
      return `"${str.replace(/"/g, '""')}"`;
    }
    return str;
  }

  const lines: string[] = [];
  lines.push(headers.map(escapeCell).join(","));
  for (const row of rows) {
    lines.push(row.map(escapeCell).join(","));
  }

  const csv = lines.join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  triggerDownload(blob, filename);
}

// ---------------------------------------------------------------------------
// PNG Export (from Canvas element)
// ---------------------------------------------------------------------------

/**
 * Download a `<canvas>` element's current content as a PNG image.
 *
 * @param canvas   - The HTMLCanvasElement to export.
 * @param filename - Target filename (e.g., "dalitz_plot.png").
 */
export function downloadAsPng(
  canvas: HTMLCanvasElement,
  filename: string,
): void {
  canvas.toBlob((blob) => {
    if (blob) {
      triggerDownload(blob, filename);
    }
  }, "image/png");
}

// ---------------------------------------------------------------------------
// SVG Export (from SVG element)
// ---------------------------------------------------------------------------

/**
 * Download an `<svg>` element as an SVG file.
 *
 * The element is serialised to XML string with proper namespace
 * declarations so it can be opened in any vector graphics editor.
 *
 * @param svgElement - The SVGElement to export.
 * @param filename   - Target filename (e.g., "feynman_diagram.svg").
 */
export function downloadAsSvg(
  svgElement: SVGElement,
  filename: string,
): void {
  const serializer = new XMLSerializer();
  let svgString = serializer.serializeToString(svgElement);

  // Ensure the XML namespace is present
  if (!svgString.includes("xmlns=")) {
    svgString = svgString.replace(
      "<svg",
      '<svg xmlns="http://www.w3.org/2000/svg"',
    );
  }

  const blob = new Blob([svgString], { type: "image/svg+xml;charset=utf-8" });
  triggerDownload(blob, filename);
}

// ---------------------------------------------------------------------------
// Context Menu Helpers
// ---------------------------------------------------------------------------

/**
 * Build standard export context menu items for a canvas-based widget.
 * Returns items suitable for the context menu store.
 */
export function canvasExportMenuItems(
  canvas: HTMLCanvasElement | null,
  data: unknown,
  baseName: string,
): { id: string; label: string; icon: string; action: () => void; separator?: boolean }[] {
  const items: { id: string; label: string; icon: string; action: () => void; separator?: boolean }[] = [];

  if (canvas) {
    items.push({
      id: "export-png",
      label: "Export as PNG",
      icon: "📷",
      action: () => downloadAsPng(canvas, `${baseName}.png`),
      separator: true,
    });
  }

  if (data) {
    items.push({
      id: "export-json",
      label: "Export as JSON",
      icon: "📄",
      action: () => downloadAsJson(data, `${baseName}.json`),
    });
  }

  return items;
}

/**
 * Build standard export context menu items for an SVG-based widget.
 */
export function svgExportMenuItems(
  svgElement: SVGElement | null,
  data: unknown,
  baseName: string,
): { id: string; label: string; icon: string; action: () => void; separator?: boolean }[] {
  const items: { id: string; label: string; icon: string; action: () => void; separator?: boolean }[] = [];

  if (svgElement) {
    items.push({
      id: "export-svg",
      label: "Export as SVG",
      icon: "🖼",
      action: () => downloadAsSvg(svgElement, `${baseName}.svg`),
      separator: true,
    });
  }

  if (data) {
    items.push({
      id: "export-json",
      label: "Export as JSON",
      icon: "📄",
      action: () => downloadAsJson(data, `${baseName}.json`),
    });
  }

  return items;
}

/**
 * Build CSV export context menu item for tabular data.
 */
export function csvExportMenuItem(
  headers: string[],
  rows: (string | number)[][],
  baseName: string,
): { id: string; label: string; icon: string; action: () => void; separator?: boolean } {
  return {
    id: "export-csv",
    label: "Export as CSV",
    icon: "📊",
    separator: true,
    action: () => downloadAsCsv(headers, rows, `${baseName}.csv`),
  };
}
