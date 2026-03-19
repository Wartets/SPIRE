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

export type HistogramWorkerPayload = {
  rawData: Float64Array;
  bins: number;
  min: number;
  max: number;
};

export type HistogramWorkerResult = {
  frequencies: number[];
  errors: number[];
  overflow: number;
  underflow: number;
};

function binHistogram(payload: HistogramWorkerPayload): HistogramWorkerResult {
  const bins = Math.max(1, Math.floor(payload.bins));
  const min = Number.isFinite(payload.min) ? payload.min : 0;
  const max = Number.isFinite(payload.max) ? payload.max : min + 1;
  const width = Math.max(1e-12, max - min);
  const inv = bins / width;

  const frequencies = new Float64Array(bins);
  let overflow = 0;
  let underflow = 0;

  for (let i = 0; i < payload.rawData.length; i += 1) {
    const value = payload.rawData[i];
    if (!Number.isFinite(value)) continue;
    if (value < min) {
      underflow += 1;
      continue;
    }
    if (value >= max) {
      overflow += 1;
      continue;
    }

    const idx = Math.min(bins - 1, Math.max(0, Math.floor((value - min) * inv)));
    frequencies[idx] += 1;
  }

  const errors = Array.from(frequencies, (count) => Math.sqrt(Math.max(0, count)));

  return {
    frequencies: Array.from(frequencies),
    errors,
    overflow,
    underflow,
  };
}

self.onmessage = (event: MessageEvent<WorkerRequest<HistogramWorkerPayload>>) => {
  const { id, payload } = event.data;
  try {
    const result = binHistogram(payload);
    const response: WorkerResponse<HistogramWorkerResult> = {
      id,
      ok: true,
      data: result,
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
