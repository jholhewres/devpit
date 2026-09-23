import { useCallback, useEffect, useRef, useState } from 'react'

export interface Queue {
  readonly waiting: readonly string[]
  /** Kept to go when the turn in flight is over. */
  readonly add: (prompt: string) => void
  readonly drop: (index: number) => void
  /** Everything waiting, taken off the queue — what a stop hands back. */
  readonly takeAll: () => string
}

/**
 * What was typed while a turn ran, sent in order once it is over.
 *
 * A thought that arrives mid-turn used to wait in your head until the button
 * came back. Held here instead, one message per turn, the way the terminal
 * queues it — and stopping the turn hands the lot back to the composer
 * rather than sending it into the silence the stop made.
 */
export function useQueue(sending: boolean, say: (prompt: string) => void): Queue {
  const [waiting, setWaiting] = useState<readonly string[]>([])

  /* One message per turn that ends — on the edge, not the level: between
     sending one and its turn being seen to start, the queue must not empty
     itself into turns that all start at once. */
  const was = useRef(sending)
  useEffect(() => {
    const ended = was.current && !sending
    was.current = sending
    if (!ended || waiting.length === 0) return
    const [next, ...rest] = waiting
    setWaiting(rest)
    say(next)
  }, [sending, waiting, say])

  const add = useCallback((prompt: string) => setWaiting((was) => [...was, prompt]), [])
  const drop = useCallback((index: number) => setWaiting((was) => was.filter((_, at) => at !== index)), [])
  const takeAll = useCallback((): string => {
    const all = waiting.join('\n\n')
    setWaiting([])
    return all
  }, [waiting])

  return { waiting, add, drop, takeAll }
}
