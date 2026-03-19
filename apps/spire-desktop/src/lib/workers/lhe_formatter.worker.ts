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

export type LheFormatterPayload = {
  header?: string;
  init?: string;
  events: string[];
  footer?: string;
};

function formatLhe(payload: LheFormatterPayload): string {
  const chunks: string[] = [];
  chunks.push(payload.header ?? '<LesHouchesEvents version="3.0">');
  if (payload.init) chunks.push(payload.init);
  chunks.push(...payload.events);
  chunks.push(payload.footer ?? '</LesHouchesEvents>');
  return chunks.join("\n");
}

self.onmessage = (event: MessageEvent<WorkerRequest<LheFormatterPayload>>) => {
  const { id, payload } = event.data;
  try {
    const response: WorkerResponse<string> = {
      id,
      ok: true,
      data: formatLhe(payload),
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
