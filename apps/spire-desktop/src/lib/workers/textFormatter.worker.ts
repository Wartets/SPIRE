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

type CsvPayload = {
  task: "csv";
  header: string;
  rows: string[];
};

type LhePayload = {
  task: "lhe";
  header?: string;
  init?: string;
  events: string[];
  footer?: string;
};

type LatexJoinPayload = {
  task: "latex";
  fragments: string[];
  separator?: string;
};

type WorkerPayload = CsvPayload | LhePayload | LatexJoinPayload;

function formatCsv(payload: CsvPayload): string {
  return [payload.header, ...payload.rows].join("\n");
}

function formatLhe(payload: LhePayload): string {
  const chunks: string[] = [];
  chunks.push(payload.header ?? '<LesHouchesEvents version="3.0">');
  if (payload.init) chunks.push(payload.init);
  chunks.push(...payload.events);
  chunks.push(payload.footer ?? "</LesHouchesEvents>");
  return chunks.join("\n");
}

function formatLatex(payload: LatexJoinPayload): string {
  return payload.fragments.join(payload.separator ?? "\n");
}

self.onmessage = (event: MessageEvent<WorkerRequest<WorkerPayload>>) => {
  const { id, payload } = event.data;
  try {
    const data = payload.task === "csv"
      ? formatCsv(payload)
      : payload.task === "lhe"
        ? formatLhe(payload)
        : formatLatex(payload);

    const response: WorkerResponse<string> = {
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
