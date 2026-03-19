export {};

type WorkerRequest<TPayload> = {
  id: number;
  payload: TPayload;
};

type WorkerResponse<TResult> = {
  id: number;
  ok: boolean;
  data?: TResult;
  error?: string;
};

type HistogramPreparationPayload = {
  task: "prepareHistogram";
  binContents: number[];
  binEdges: number[];
};

type RawHistogramPayload = {
  task: "binRawValues";
  values: number[];
  min: number;
  max: number;
  nBins: number;
};

type WorkerPayload = HistogramPreparationPayload | RawHistogramPayload;

type PreparedHistogram = {
  labels: string[];
  backgroundColor: string[];
  borderColor: string[];
  maxVal: number;
};

type BinnedHistogram = {
  binContents: number[];
  binEdges: number[];
};

function prepareHistogram(binContents: number[], binEdges: number[]): PreparedHistogram {
  const labels: string[] = [];
  const maxVal = Math.max(...binContents, 1e-30);

  for (let i = 0; i < binContents.length; i += 1) {
    const lo = binEdges[i] ?? 0;
    const hi = binEdges[i + 1] ?? lo;
    labels.push(((lo + hi) * 0.5).toFixed(2));
  }

  const backgroundColor = binContents.map((value) => {
    const ratio = maxVal > 0 ? value / maxVal : 0;
    const r = Math.round(30 + 200 * ratio);
    const g = Math.round(100 + 60 * (1 - ratio));
    const b = Math.round(220 - 150 * ratio);
    return `rgba(${r}, ${g}, ${b}, 0.85)`;
  });

  const borderColor = backgroundColor.map((color) => color.replace("0.85", "1.0"));
  return { labels, backgroundColor, borderColor, maxVal };
}

function binRawValues(values: number[], min: number, max: number, nBins: number): BinnedHistogram {
  const safeBins = Math.max(1, Math.floor(nBins));
  const width = Math.max(1e-12, max - min);
  const invWidth = safeBins / width;
  const binContents = Array(safeBins).fill(0);
  const binEdges = Array.from({ length: safeBins + 1 }, (_, i) => min + (width * i) / safeBins);

  for (const value of values) {
    if (!Number.isFinite(value) || value < min || value >= max) continue;
    const idx = Math.min(safeBins - 1, Math.max(0, Math.floor((value - min) * invWidth)));
    binContents[idx] += 1;
  }

  return { binContents, binEdges };
}

self.onmessage = (event: MessageEvent<WorkerRequest<WorkerPayload>>) => {
  const { id, payload } = event.data;
  try {
    const data = payload.task === "prepareHistogram"
      ? prepareHistogram(payload.binContents, payload.binEdges)
      : binRawValues(payload.values, payload.min, payload.max, payload.nBins);

    const response: WorkerResponse<PreparedHistogram | BinnedHistogram> = {
      id,
      ok: true,
      data,
    };
    self.postMessage(response);
  } catch (error: unknown) {
    const response: WorkerResponse<never> = {
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
    self.postMessage(response);
  }
};
