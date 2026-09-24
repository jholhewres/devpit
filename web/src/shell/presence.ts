import { commands } from '../gen/bindings'
import { inTauri } from './window'

/* Tells the app somebody is using the window, at most once a minute: error
   reports only go out after five minutes of nobody, and a minute's precision
   is all that needs. */
export const EVERY_MS = 60_000

export function reportPresence(
  target: Window = window,
  seen = commands.presenceSeen,
  now: () => number = Date.now,
): void {
  if (!inTauri()) return
  let last = -Infinity
  const touched = (): void => {
    const at = now()
    if (at - last < EVERY_MS) return
    last = at
    void seen().catch(() => undefined)
  }
  for (const kind of ['keydown', 'pointerdown', 'wheel'] as const) {
    target.addEventListener(kind, touched, { passive: true })
  }
}
