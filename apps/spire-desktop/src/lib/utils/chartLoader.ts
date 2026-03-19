import type { Chart as ChartType } from "chart.js";

type ChartCtor = new (item: HTMLCanvasElement | CanvasRenderingContext2D, config: unknown) => ChartType;

let chartCtor: ChartCtor | null = null;
let loading: Promise<ChartCtor | null> | null = null;

export async function ensureChartCtor(): Promise<ChartCtor | null> {
  if (chartCtor) return chartCtor;
  if (loading) return loading;

  loading = (async () => {
    try {
      const mod = await import("chart.js/auto");
      chartCtor = mod.default as unknown as ChartCtor;
      return chartCtor;
    } catch {
      return null;
    } finally {
      loading = null;
    }
  })();

  return loading;
}
