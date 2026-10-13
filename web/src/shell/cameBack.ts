/*
 * The window coming back to the person: focus regained, or shown again.
 *
 * The tree, the file index, the history, a session's folder, the reminders'
 * clock and Devflow all read again then, each on listeners of its own — and a
 * window restored fires both events, so one return was two rounds of reads.
 * One pair of listeners tells each of them once.
 */

/** Returns closer together than this are one return. */
export const QUIET_MS = 1000

const heard = new Set<() => void>()
let last = -Infinity

const came = (): void => {
  const now = Date.now()
  if (now - last < QUIET_MS) return
  last = now
  for (const tell of [...heard]) tell()
}

const shown = (): void => {
  if (document.visibilityState === 'visible') came()
}

/** Calls `then` each time the window comes back, until the answer is called. */
export function onCameBack(then: () => void): () => void {
  /* Its own entry, so one function given twice is two listeners and dropping one leaves the other. */
  const mine = (): void => then()
  if (heard.size === 0) {
    window.addEventListener('focus', came)
    document.addEventListener('visibilitychange', shown)
  }
  heard.add(mine)
  return () => {
    heard.delete(mine)
    if (heard.size > 0) return
    window.removeEventListener('focus', came)
    document.removeEventListener('visibilitychange', shown)
  }
}
