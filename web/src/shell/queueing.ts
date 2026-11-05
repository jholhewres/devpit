import type { Attachment, ChatQueue, QueuedMessage } from '../gen/bindings'

/*
 * The rules of what waits in a conversation, apart from the hook that keeps
 * it: what a turn's end sends, what a stop holds, what goes back to the
 * composer. Tested by calling them, without mounting a chat.
 */

/** What goes with a message besides its words. */
export interface Extras {
  readonly files: readonly Attachment[]
  readonly skills: readonly string[]
}

export const EMPTY: ChatQueue = { items: [], paused: false }

/* A queue with nothing in it holds nothing back either: the next one queued
   after a stop goes when its turn ends, like any other. */
const holding = (items: QueuedMessage[], paused: boolean): ChatQueue => ({ items, paused: paused && items.length > 0 })

/** What the end of a turn does: the next message goes, unless the queue is held or the turn failed. */
export function ended(queue: ChatQueue, failed: boolean): { next: QueuedMessage | null; queue: ChatQueue } {
  if (queue.items.length === 0) return { next: null, queue }
  if (queue.paused || failed) return { next: null, queue: holding(queue.items, true) }
  const [next, ...rest] = queue.items
  return { next, queue: holding(rest, false) }
}

/** Resumed: unheld, and the first one taken off to go now when no turn runs. */
export function resumed(queue: ChatQueue, sending: boolean): { next: QueuedMessage | null; queue: ChatQueue } {
  if (sending || queue.items.length === 0) return { next: null, queue: holding(queue.items, false) }
  const [next, ...rest] = queue.items
  return { next, queue: holding(rest, false) }
}

export const queued = (queue: ChatQueue, item: QueuedMessage): ChatQueue => holding([...queue.items, item], queue.paused)
export const without = (queue: ChatQueue, id: string): ChatQueue => holding(queue.items.filter((one) => one.id !== id), queue.paused)
export const firstAgain = (queue: ChatQueue, item: QueuedMessage): ChatQueue =>
  holding([item, ...queue.items.filter((one) => one.id !== item.id)], queue.paused)
export const edited = (queue: ChatQueue, id: string, prompt: string): ChatQueue =>
  holding(queue.items.map((one) => (one.id === id ? { ...one, prompt } : one)), queue.paused)

/** One place earlier, swapping with the one before it. */
export function raised(queue: ChatQueue, id: string): ChatQueue {
  const at = queue.items.findIndex((one) => one.id === id)
  if (at <= 0) return queue
  const items = [...queue.items]
  ;[items[at - 1], items[at]] = [items[at]!, items[at - 1]!]
  return holding(items, queue.paused)
}

/** Everything waiting as one draft: the words in order, each file and skill once. */
export function handedBack(items: readonly QueuedMessage[]): { prompt: string } & Extras {
  const files = items.flatMap((one) => one.files).filter((file, at, all) => all.findIndex((had) => had.path === file.path) === at)
  return { prompt: items.map((one) => one.prompt).join('\n\n'), files, skills: [...new Set(items.flatMap((one) => one.skills))] }
}

/** Two queues read and written apart, as one: what was kept first, then what was queued before it was read. */
export function merged(kept: ChatQueue, early: ChatQueue): ChatQueue {
  const fresh = early.items.filter((one) => !kept.items.some((had) => had.id === one.id))
  if (fresh.length === 0 && (kept.paused || !early.paused)) return kept
  return holding([...kept.items, ...fresh], kept.paused || early.paused)
}
