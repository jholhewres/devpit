/*
 * Writes that must land in the order they were made.
 *
 * Every command now runs on a pool in the app, so two writes sent a moment
 * apart — a theme switched twice, a panel dragged — can finish in either
 * order, and the one that finishes last is what is stored. Writes to the same
 * thing go through one queue here: each is sent once the one before it has
 * answered.
 */
const queues = new Map<string, Promise<unknown>>()

export function inOrder<T>(key: string, write: () => Promise<T>): Promise<T> {
  const next = (queues.get(key) ?? Promise.resolve()).then(write, write)
  queues.set(
    key,
    next.catch(() => undefined),
  )
  return next
}
