import { useCallback, useEffect, useRef, useState } from 'react'

import type { Tab } from './strip'
import type { AgentState, Doing } from './useAgents'

/*
 * Finishes nobody has looked at.
 *
 * An agent that finished while you were in another tab is news until you go
 * and look; one that finished in the tab you were watching is not. Keyed by
 * pane because that is what the hooks report on, and a split tab has two; a
 * chat's turn has no pane, and is keyed by its tab.
 */

export type Unread = ReadonlySet<string>

/* The panes with an unread finish, after one pane's agent reported. */
export function afterState(
  unread: Unread,
  paneId: string,
  state: AgentState,
  inFront: readonly string[],
): Unread {
  // Working again, or waiting on you, is not a finish to catch up on; a
  // finish in front of you leaves the set as it was.
  const wanted = state !== 'done' ? false : inFront.includes(paneId) ? unread.has(paneId) : true
  // The same set when nothing changed: a new one is a new context value, and
  // every agent report would re-render the whole shell for nothing.
  if (wanted === unread.has(paneId)) return unread
  const next = new Set(unread)
  if (wanted) next.add(paneId)
  else next.delete(paneId)
  return next
}

/* The panes with an unread finish, once some panes came to the front. */
export function afterLooking(unread: Unread, inFront: readonly string[]): Unread {
  if (!inFront.some((paneId) => unread.has(paneId))) return unread
  return new Set([...unread].filter((paneId) => !inFront.includes(paneId)))
}

/* The keys unread, once a chat's turn ended in tab `tabId`. */
export function afterFinish(unread: Unread, tabId: string, inFront: readonly string[]): Unread {
  if (inFront.includes(tabId) || unread.has(tabId)) return unread
  return new Set(unread).add(tabId)
}

export function unreadIn(unread: Unread, tab: Tab | null | undefined): boolean {
  if (!tab) return false
  return unread.has(tab.id) || (tab.panes?.some((paneId) => unread.has(paneId)) ?? false)
}

/* What is in front: the active tab's panes, and the tab itself for a chat. */
const frontOf = (active: Tab | null): string => (active ? [...(active.panes ?? []), active.id].join(' ') : '')

export function useUnread(doing: Doing, active: Tab | null): { unread: Unread; finished: (tabId: string) => void } {
  const [unread, setUnread] = useState<Unread>(() => new Set())
  const was = useRef<Doing>(doing)
  // Compared as a string so a new array holding the same panes is no change.
  const front = frontOf(active)
  const shown = useRef(front)
  shown.current = front
  const finished = useCallback((tabId: string) => setUnread((held) => afterFinish(held, tabId, shown.current.split(' '))), [])

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

  return { unread, finished }
}
