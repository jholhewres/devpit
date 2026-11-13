import type { Message } from '../gen/bindings'
import { usePicked, type Store } from './picked'

/*
 * A conversation's messages, held outside React.
 *
 * A streamed batch replaces one message. In component state it drew the whole
 * pane again — the composer, the corner, the thread — for one turn's text.
 * Here each reader picks what it needs, and only a reader whose pick changed
 * is drawn: a batch reaches the live turn alone.
 */
export interface ChatMessages extends Store<readonly Message[]> {
  set(next: readonly Message[] | ((was: readonly Message[]) => readonly Message[])): void
  /** One message by id, from an index built once per change. */
  byId(id: string): Message | undefined
}

export function chatMessages(first: readonly Message[] = []): ChatMessages {
  let current = first
  let index: Map<string, Message> | null = null
  const listeners = new Set<() => void>()
  return {
    get: () => current,
    set: (next) => {
      const value = typeof next === 'function' ? next(current) : next
      if (value === current) return
      current = value
      index = null
      listeners.forEach((listener) => listener())
    },
    subscribe: (listener) => {
      listeners.add(listener)
      return () => void listeners.delete(listener)
    },
    byId: (id) => (index ??= new Map(current.map((one) => [one.id, one]))).get(id),
  }
}

/** What `pick` reads of the messages, drawn again only when that changes. */
export const useMessagesPick = <T>(messages: ChatMessages, pick: (all: readonly Message[]) => T): T => usePicked(messages, pick)

/** One message, drawn again only when it is replaced. */
export const useMessage = (messages: ChatMessages, id: string): Message | undefined => usePicked(messages, () => messages.byId(id))
