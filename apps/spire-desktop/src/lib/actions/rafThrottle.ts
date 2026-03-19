import { rafThrottle } from "$lib/utils/throttle";

export interface RafThrottleActionOptions<T extends Event = Event> {
  event: string;
  handler: (event: T) => void;
  options?: AddEventListenerOptions;
}

export function rafThrottleAction<T extends Event = Event>(
  node: HTMLElement,
  initialOptions: RafThrottleActionOptions<T>,
): { update: (next: RafThrottleActionOptions<T>) => void; destroy: () => void } {
  let current = initialOptions;
  let detach: (() => void) | null = null;

  const bind = (): void => {
    detach?.();
    const throttled = rafThrottle((event: Event) => {
      current.handler(event as T);
    });
    const listener = (event: Event): void => {
      throttled(event);
    };
    node.addEventListener(current.event, listener, current.options);
    detach = () => {
      node.removeEventListener(current.event, listener, current.options);
      throttled.cancel();
    };
  };

  bind();

  return {
    update(next: RafThrottleActionOptions<T>): void {
      current = next;
      bind();
    },
    destroy(): void {
      detach?.();
      detach = null;
    },
  };
}
