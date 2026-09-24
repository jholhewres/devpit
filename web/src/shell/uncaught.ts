import { commands } from '../gen/bindings'
import { inTauri } from './window'

/* Errors nothing else caught, handed to devpit's own report. The backend
   drops them unless the person switched reports on, so the window never has
   to know the switch. */
export function reportUncaught(
  target: Window = window,
  send = commands.errorsReport,
  now: () => number = Date.now,
): void {
  if (!inTauri()) return
  const allowed = sparing(now)
  const hand = (error: unknown): void => {
    const { message, stack } = describe(error)
    if (!allowed(message)) return
    // A report that fails is not reported: that way lies a loop.
    void send(message, stack).catch(() => undefined)
  }
  target.addEventListener('error', (event) => hand(event.error ?? event.message))
  target.addEventListener('unhandledrejection', (event) => hand(event.reason))
}

/* An error in something that redraws throws on every frame. The same message
   goes once per QUIET_MS, and no more than MOST_PER_MINUTE go at all. */
export const QUIET_MS = 5_000
export const MOST_PER_MINUTE = 20

export function sparing(now: () => number): (message: string) => boolean {
  const last = new Map<string, number>()
  let sent: number[] = []
  return (message) => {
    const at = now()
    sent = sent.filter((then) => at - then < 60_000)
    const before = last.get(message)
    if (before !== undefined && at - before < QUIET_MS) return false
    if (sent.length >= MOST_PER_MINUTE) return false
    if (last.size > 200) last.clear()
    last.set(message, at)
    sent.push(at)
    return true
  }
}

export function describe(error: unknown): { message: string; stack: string | null } {
  if (error instanceof Error) return { message: `${error.name}: ${error.message}`, stack: error.stack ?? null }
  return { message: String(error), stack: null }
}
