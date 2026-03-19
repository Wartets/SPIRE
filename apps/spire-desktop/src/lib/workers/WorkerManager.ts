type WorkerRequest = {
  id: number;
  payload: unknown;
};

type WorkerResponse = {
  id: number;
  ok: boolean;
  data?: unknown;
  error?: string;
};

interface WorkerClient {
  worker: Worker;
  pending: Map<number, { resolve: (value: unknown) => void; reject: (reason: Error) => void }>;
  sequence: number;
}

const clients = new Map<string, WorkerClient>();

function makeKey(url: URL): string {
  return String(url);
}

function createClient(workerUrl: URL): WorkerClient {
  const worker = new Worker(workerUrl, { type: "module" });
  const client: WorkerClient = {
    worker,
    pending: new Map(),
    sequence: 0,
  };

  worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
    const message = event.data;
    const request = client.pending.get(message.id);
    if (!request) return;
    client.pending.delete(message.id);

    if (message.ok) {
      request.resolve(message.data);
      return;
    }

    request.reject(new Error(message.error || "Worker request failed"));
  };

  worker.onerror = (event: ErrorEvent) => {
    const error = new Error(event.message || "Worker runtime error");
    for (const request of client.pending.values()) {
      request.reject(error);
    }
    client.pending.clear();
  };

  return client;
}

function getClient(workerUrl: URL): WorkerClient {
  const key = makeKey(workerUrl);
  const existing = clients.get(key);
  if (existing) return existing;
  const created = createClient(workerUrl);
  clients.set(key, created);
  return created;
}

export async function runWorkerTask<TPayload, TResult>(workerUrl: URL, payload: TPayload): Promise<TResult> {
  const client = getClient(workerUrl);
  const id = ++client.sequence;

  return new Promise<TResult>((resolve, reject) => {
    client.pending.set(id, {
      resolve: (value: unknown) => resolve(value as TResult),
      reject,
    });

    const request: WorkerRequest = { id, payload };
    client.worker.postMessage(request);
  });
}

export function terminateAllWorkerClients(): void {
  for (const client of clients.values()) {
    for (const request of client.pending.values()) {
      request.reject(new Error("Worker client terminated"));
    }
    client.pending.clear();
    client.worker.terminate();
  }
  clients.clear();
}
