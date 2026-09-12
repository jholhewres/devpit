/*
 * What went wrong, in words a person can act on.
 *
 * The backend rejects with an `RpcError` — a plain object, never an `Error` —
 * so `String(thrown)` gives `[object Object]`, which is what a terminal that
 * failed to attach actually printed at the top of its own pane. A message that
 * says nothing is worse than no message: it costs the same screen and hides
 * the one fact that would have explained the empty terminal.
 */

/** The message inside whatever a rejected call handed back. */
export function reason(thrown: unknown, fallback = 'the command failed'): string {
  if (typeof thrown === 'string') return thrown.trim() || fallback
  if (thrown instanceof Error) return thrown.message || fallback
  if (thrown && typeof thrown === 'object') {
    const shape = thrown as { message?: unknown; error?: unknown }
    if (typeof shape.message === 'string' && shape.message.trim()) return shape.message
    /* `{ status: 'error', error: RpcError }` — the wrapper the generated
       contract puts around a failure, one level further down. */
    if (typeof shape.error === 'string' && shape.error.trim()) return shape.error
    if (shape.error && typeof shape.error === 'object') return reason(shape.error, fallback)
    /* Anything else still beats `[object Object]`: the fields are the only
       description of the failure that exists. */
    try {
      const written = JSON.stringify(thrown)
      if (written && written !== '{}') return written
    } catch {
      /* Circular, so there is nothing to read out of it. */
    }
  }
  return fallback
}
