import { useCallback, useEffect, useRef, useState } from 'react'

import type { ChatQueue, QueuedMessage } from '../gen/bindings'
import { ask, commands } from './live'
import { EMPTY, edited, ended, firstAgain, handedBack, merged, queued, raised, resumed, without, type Extras } from './queueing'

export interface Queue {
  readonly waiting: readonly QueuedMessage[]
  /** Held by a stop or a failed turn: nothing leaves until resumed. */
  readonly paused: boolean
  /** Kept to go when the turn in flight is over, with what was attached to it. */
  readonly add: (prompt: string, extras: Extras) => void
  readonly drop: (id: string) => void
  readonly edit: (id: string, prompt: string) => void
  /** One place earlier. */
  readonly raise: (id: string) => void
  /** Sent into the turn in flight now, instead of after it. */
  readonly now: (id: string) => void
  /** The first one: into the turn in flight, or as the next turn when none runs. */
  readonly first: () => void
  readonly pause: () => void
  readonly resume: () => void
  /** Everything waiting, taken off the queue as one draft. */
  readonly takeAll: () => ({ prompt: string } & Extras) | null
}

/** What the queue needs of the chat it waits on. */
export interface Talk {
  readonly sending: boolean
  /** Whether the last turn this chat sent ended in failure. */
  readonly failed?: boolean
  readonly say: (prompt: string, back?: (prompt: string) => void, given?: Extras) => void
  readonly steer?: (prompt: string, given?: Extras) => Promise<boolean>
}

const fresh = (): string => `q_${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`

/*
 * What was typed while a turn ran, sent in order once it is over.
 *
 * Kept on disk per conversation with its attachments, so a closed tab or a
 * reload finds it where it was. A stop or a failed turn holds it rather than
 * sending into the silence that followed; resuming lets it go again.
 */
export function useQueue(projectId: string | null, conversationId: string, talk: Talk): Queue {
  const { sending, failed = false, say, steer } = talk
  const [queue, setQueue] = useState<ChatQueue>(EMPTY)
  const current = useRef(queue)
  useEffect(() => {
    current.current = queue
  }, [queue])
  /* What disk last answered; null until it has, and nothing is written over a queue never read. */
  const read = useRef<ChatQueue | null>(null)

  useEffect(() => {
    read.current = null
    if (!projectId) return
    let open = true
    void ask(() => commands.chatQueue(projectId, conversationId)).then((kept) => {
      if (!open || !kept.data) return
      read.current = kept.data
      setQueue(merged(kept.data, current.current))
    })
    return () => {
      open = false
      setQueue(EMPTY)
    }
  }, [projectId, conversationId])

  /* Written whole after each change, one write after another so the last change is the one kept. */
  const writes = useRef<Promise<unknown>>(Promise.resolve())
  useEffect(() => {
    if (!projectId || !read.current || queue === read.current) return
    writes.current = writes.current.then(() => ask(() => commands.chatQueueKeep(projectId, conversationId, queue)))
  }, [queue, projectId, conversationId])

  /* Refused before it opened, a message goes back to the head of the queue. */
  const go = useCallback(
    (next: QueuedMessage) => say(next.prompt, () => setQueue((was) => firstAgain(was, next)), next),
    [say],
  )

  /* One message per turn that ends — on the edge, not the level: between
     sending one and its turn being seen to start, the queue must not empty
     itself into turns that all start at once. */
  const was = useRef(sending)
  useEffect(() => {
    const over = was.current && !sending
    was.current = sending
    if (!over) return
    const { next, queue: after } = ended(current.current, failed)
    if (after !== current.current) setQueue(after)
    if (next) go(next)
  }, [sending, failed, go])

  const add = useCallback(
    (prompt: string, extras: Extras) => setQueue((was) => queued(was, { id: fresh(), prompt, files: [...extras.files], skills: [...extras.skills] })),
    [],
  )
  const drop = useCallback((id: string) => setQueue((was) => without(was, id)), [])
  const edit = useCallback((id: string, prompt: string) => setQueue((was) => edited(was, id, prompt)), [])
  const raise = useCallback((id: string) => setQueue((was) => raised(was, id)), [])
  const pause = useCallback(() => setQueue((was) => (was.items.length > 0 && !was.paused ? { ...was, paused: true } : was)), [])
  const resume = useCallback(() => {
    const { next, queue: after } = resumed(current.current, sending)
    setQueue(after)
    if (next) go(next)
  }, [sending, go])

  /* Off the queue only once the turn took it: refused, it still goes next. */
  const steering = useRef(new Set<string>())
  const now = useCallback(
    (id: string) => {
      const item = current.current.items.find((one) => one.id === id)
      if (!item || !steer || steering.current.has(id)) return
      steering.current.add(id)
      void steer(item.prompt, item)
        .then((taken) => taken && setQueue((was) => without(was, id)))
        .finally(() => steering.current.delete(id))
    },
    [steer],
  )
  const first = useCallback(() => {
    const item = current.current.items[0]
    if (!item) return
    if (sending) now(item.id)
    else resume()
  }, [sending, now, resume])

  const takeAll = useCallback(() => {
    const items = current.current.items
    if (items.length === 0) return null
    setQueue(EMPTY)
    return handedBack(items)
  }, [])

  return { waiting: queue.items, paused: queue.paused, add, drop, edit, raise, now, first, pause, resume, takeAll }
}
