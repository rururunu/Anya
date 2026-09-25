/**
 * Boot-phase timing.
 *
 * The window feels slow to start because several phases run serially before the
 * first app frame lands: bundle eval → settings IPC → mount → router ready →
 * first paint → splash fade. These marks record where the wall time actually
 * goes, so the next change targets the phase that is really slow instead of a
 * guess.
 *
 * Scope caveat: the clock starts when this module is first evaluated, i.e. while
 * the bundle is being evaluated. The span from process spawn to webview
 * navigation start (native window creation) happens before any JS runs and is
 * NOT covered here.
 */
const marks: Array<{ label: string; at: number }> = [];
const origin = performance.now();

/** Record a phase boundary. Cheap; safe to leave in place. */
export function markBootPhase(label: string): void {
  marks.push({ label, at: performance.now() });
}

/** Milliseconds elapsed since this module was evaluated. */
export function bootElapsedMs(): number {
  return Math.round(performance.now() - origin);
}

/**
 * Emit the phases recorded so far, as per-phase deltas plus a total. Reported
 * once per window at the end of its boot chain.
 */
export function reportBootPhases(scope: string): void {
  if (marks.length === 0) {
    return;
  }

  const phases: Record<string, string> = {};
  let previous = origin;
  for (const mark of marks) {
    phases[mark.label] = `${Math.round(mark.at - previous)}ms`;
    previous = mark.at;
  }

  // Written with `console` rather than the app logger on purpose: the logger
  // drops info/debug outside dev builds (services/logger.ts minLevel), and the
  // console is the only sink it has — there is no frontend → host log bridge.
  // eslint-disable-next-line no-console -- the whole point is to be readable.
  console.info(`[boot-timing] ${scope}`, {
    total: `${Math.round(previous - origin)}ms`,
    ...phases,
  });
}
