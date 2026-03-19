import { runWorkerTask } from "$lib/workers/WorkerManager";

export function executeWorkerTask<TPayload, TResult>(workerUrl: URL, payload: TPayload): Promise<TResult> {
  return runWorkerTask<TPayload, TResult>(workerUrl, payload);
}
