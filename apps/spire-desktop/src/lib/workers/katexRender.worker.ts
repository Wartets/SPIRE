type RenderRequest = {
  expression: string;
  displayMode: boolean;
};

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

type RenderResponse = {
  ok: boolean;
  html?: string;
  error?: string;
};

self.onmessage = async (event: MessageEvent<WorkerRequest<RenderRequest>>) => {
  const { id, payload } = event.data;
  const { expression, displayMode } = payload;
  try {
    const katex = await import("katex");
    const html = katex.renderToString(expression, {
      displayMode,
      throwOnError: false,
      strict: "warn",
      trust: false,
      output: "html",
    });
    const response: WorkerResponse<RenderResponse> = {
      id,
      ok: true,
      data: { ok: true, html },
    };
    self.postMessage(response);
  } catch (error: unknown) {
    const response: WorkerResponse<RenderResponse> = {
      id,
      ok: true,
      data: {
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      },
    };
    self.postMessage(response);
  }
};
