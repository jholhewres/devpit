import { useEffect, useRef, useState } from 'react'

import type { Tab } from './strip'
import type { AgentState, Doing } from './useAgents'

/*
 * Finishes nobody has looked at.
 *
 * An agent that finished while you were in another tab is news until you go
 * and look; one that finished in the tab you were watching is not. Keyed by
 * pane because that is what the hooks report on, and a split tab has two.
 */

export type Unread = ReadonlySet<string>

/* The panes with an unread finish, after one pane's agent reported. */
export function afterState(
  unread: Unread,
  paneId: string,
  state: AgentState,
  inFront: readonly string[],
): Unread {
  const next = new Set(unread)
  if (state === 'done' && !inFront.includes(paneId)) next.add(paneId)
  // Working again, or waiting on you, is not a finish to catch up on.
  if (state !== 'done') next.delete(paneId)
  return next
}

/* The panes with an unread finish, once some panes came to the front. */
export function afterLooking(unread: Unread, inFront: readonly string[]): Unread {
  if (!inFront.some((paneId) => unread.has(paneId))) return unread
  return new Set([...unread].filter((paneId) => !inFront.includes(paneId)))
}

export function unreadIn(unread: Unread, tab: Tab | null | undefined): boolean {
  return tab?.panes?.some((paneId) => unread.has(paneId)) ?? false
}

export function useUnread(doing: Doing, active: Tab | null): Unread {
  const [unread, setUnread] = useState<Unread>(() => new Set())
  const was = useRef<Doing>(doing)
  const inFront = active?.panes ?? []
  // Compared as a string so a new array holding the same panes is no change.
  const front = inFront.join(' ')

  useEffect(() => {
    const before = was.current
    was.current = doing
    setUnread((held) =>
      Object.entries(doing).reduce(
        (next, [paneId, state]) =>
          before[paneId] === state ? next : afterState(next, paneId, state, front.split(' ')),
        held,
      ),
    )
  }, [doing, front])

  useEffect(() => {
    setUnread((held) => afterLooking(held, front ? front.split(' ') : []))
  }, [front])

  return unread
}
