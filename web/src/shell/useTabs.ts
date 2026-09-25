import { useCallback, useEffect, useMemo, useState } from 'react'

import { ask, commands } from './live'
import { useWantedPane } from './useWantedPane'
import type { PaneName } from './paneList'
import { attached, closed, drafted as taken, focused, joined, launched as sent, moved, opened, renamed, replaced, type Strip, type Tab } from './strip'
import { remember, remembered } from './tabs'

export type Where = 'strip' | 'sidebar'

export interface Renaming {
  readonly id: string
  readonly where: Where
}

export interface Tabs {
  readonly open: readonly Tab[]
  readonly active: Tab | null
  show: (kind: PaneName, tab?: Partial<Tab>) => void
  close: (id: string) => void
  /** Moves every pane of `from` beside `into`'s, and `from` leaves the strip. */
  join: (from: string, into: string) => Promise<string | null>
  focus: (id: string) => void
  move: (id: string, to: number) => void
  rename: (id: string, title: string) => void
  attach: (id: string, panes: readonly string[]) => void
  /** Forgets the agent a terminal was opened to run, once it has been sent. */
  launched: (id: string) => void
  /** Forgets the words a chat was opened with, once they are in its composer. */
  drafted: (id: string) => void
  /** Puts another tab in this one's place. */
  replace: (id: string, tab: Tab) => void
  /** Brings the tab holding this pane to the front once it is open. */
  openPane: (paneId: string | null) => void
  /** Which tab is being renamed, and on which surface.

      The surface is not decoration: the strip and the sidebar draw the same
      tab, so an id alone puts a field in both. They then race for the focus,
      and the one that loses it commits and closes the other. */
  readonly renaming: Renaming | null
  setRenaming: (renaming: Renaming | null) => void
}

/* The strip belongs to the project: switching restores what that one had
   open, and the window itself opens on the empty state.

   The strip is held with the project it belongs to, and swapped in the same
   render the project changes. Swapped in an effect, it lagged one commit
   behind: the old project's terminal tabs rendered once under the new
   project, asked it for their layout, and each left a shell in a tmux window
   of the wrong project that nothing could reach again. */
interface Held {
  readonly projectId: string | null
  readonly strip: Strip
}

export function useTabs(projectId: string | null): Tabs {
  const [held, setHeld] = useState<Held>(() => ({ projectId, strip: remembered(projectId) }))
  const [renaming, setRenaming] = useState<Renaming | null>(null)

  let current = held
  if (held.projectId !== projectId) {
    current = { projectId, strip: remembered(projectId) }
    setHeld(current)
  }
  const strip = current.strip

  const setStrip = useCallback(
    (next: (was: Strip) => Strip) => setHeld((was) => ({ ...was, strip: next(was.strip) })),
    [],
  )
  useEffect(() => remember(held.projectId, held.strip), [held])

  const show = useCallback(
    (kind: PaneName, tab: Partial<Tab> = {}) =>
      setStrip((was) => opened(was, { id: crypto.randomUUID(), kind, ...tab })),
    [],
  )

  /* Closing a terminal tab closes its whole tree, or the session keeps
     windows nothing is looking at. */
  const close = useCallback(
    (id: string) =>
      setStrip((was) => {
        /* A terminal tab owns a tree, so closing it takes every window in
           that tree — not one leaf. Leaving the rest running would leave
           shells nothing can reach again. */
        const going = was.open.find((tab) => tab.id === id)
        if (going?.kind === 'term' && projectId) {
          void ask(() => commands.sessionCloseTab(projectId, id))
        }
        return closed(was, id)
      }),
    [projectId],
  )

  /* Answers the refusal, or null. The strip changes only once the backend
     has moved the panes: dropping the tab first and failing would be a
     close that forgot to end its shells. */
  const join = useCallback(
    async (from: string, into: string): Promise<string | null> => {
      if (!projectId) return 'no project is open'
      const answer = await ask(() => commands.sessionJoinTabs(projectId, from, into, 'horizontal'))
      if (!answer.data) return answer.error ?? 'could not join those tabs'
      setStrip((was) => joined(was, from, into))
      return null
    },
    [projectId],
  )

  const focus = useCallback((id: string) => setStrip((was) => ({ ...was, active: id })), [])
  const move = useCallback((id: string, to: number) => setStrip((was) => moved(was, id, to)), [])
  const rename = useCallback(
    (id: string, title: string) => setStrip((was) => renamed(was, id, title)),
    [],
  )
  const attach = useCallback(
    (id: string, panes: readonly string[]) => setStrip((was) => attached(was, id, panes)),
    [],
  )
  const launched = useCallback((id: string) => setStrip((was) => sent(was, id)), [])
  const drafted = useCallback((id: string) => setStrip((was) => taken(was, id)), [])
  const replace = useCallback((id: string, tab: Tab) => setStrip((was) => replaced(was, id, tab)), [])

  const openPane = useWantedPane(strip.open, focus)

  /* One object per change, not per render: this is spread into the shell's
     context, and a new object here re-rendered every screen that reads it. */
  return useMemo(
    () => ({
      open: strip.open,
      active: focused(strip),
      show,
      close,
      join,
      focus,
      move,
      rename,
      attach,
      launched,
      drafted,
      replace,
      renaming,
      setRenaming,
      openPane,
    }),
    [strip, show, close, join, focus, move, rename, attach, launched, drafted, replace, renaming, openPane],
  )
}
