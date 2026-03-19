export type RafThrottled<TArgs extends unknown[] = unknown[]> = ((...args: TArgs) => void) & {
  cancel: () => void;
  flush: () => void;
};

export function rafThrottle<TArgs extends unknown[]>(fn: (...args: TArgs) => void): RafThrottled<TArgs> {
  let scheduled = false;
  let frameId = 0;
  let lastArgs: TArgs | null = null;

  const invoke = (): void => {
    scheduled = false;
    frameId = 0;
    if (!lastArgs) return;
    const args = lastArgs;
    lastArgs = null;
    fn(...args);
  };

  const throttled = ((...args: TArgs) => {
    lastArgs = args;
    if (scheduled) return;
    scheduled = true;
    frameId = requestAnimationFrame(invoke);
  }) as RafThrottled<TArgs>;

  throttled.cancel = (): void => {
    if (frameId) {
      cancelAnimationFrame(frameId);
    }
    frameId = 0;
    scheduled = false;
    lastArgs = null;
  };

  throttled.flush = (): void => {
    if (!scheduled) return;
    if (frameId) {
      cancelAnimationFrame(frameId);
    }
    invoke();
  };

  return throttled;
}
