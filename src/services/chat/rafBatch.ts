type Flush<T> = (batch: T[]) => void;

interface BatchHandle<T> {
  push: (item: T) => void;
  drain: () => void;
  size: () => number;
}

export function createRafBatch<T>(flush: Flush<T>, intervalMs = 0): BatchHandle<T> {
  let buffer: T[] = [];
  let scheduled: number | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const run = () => {
    scheduled = null;
    const out = buffer;
    buffer = [];
    if (out.length > 0) {
      flush(out);
    }
  };

  const scheduleFrame = () => {
    if (typeof requestAnimationFrame !== "undefined") {
      scheduled = requestAnimationFrame(run);
    } else {
      scheduled = 1;
      Promise.resolve().then(run);
    }
  };

  const handle: BatchHandle<T> = {
    push(item: T) {
      buffer.push(item);
      if (scheduled === null && timer === null) {
        if (intervalMs > 0) {
          timer = setTimeout(() => {
            timer = null;
            scheduleFrame();
          }, intervalMs);
        } else scheduleFrame();
      }
    },
    drain() {
      if (timer !== null) {
        clearTimeout(timer);
        timer = null;
      }
      if (scheduled !== null) {
        if (typeof cancelAnimationFrame !== "undefined" && scheduled !== 1) {
          cancelAnimationFrame(scheduled);
        }
        scheduled = null;
      }
      run();
    },
    size() {
      return buffer.length;
    },
  };

  return handle;
}
