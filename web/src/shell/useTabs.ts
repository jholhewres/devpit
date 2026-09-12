import { useCallback, useEffect, useState } from 'react'

import { ask, commands } from './live'
import type { PaneName } from './paneList'
import {
  attached,
  closed,
  focused,
  launched as sent,
  moved,
  opened,
  renamed,
  type Strip,
  type Tab,
} from './strip'
import { empty, remember, remembered } from './tabs'

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
  focus: (id: string) => void
  move: (id: string, to: number) => void
  rename: (id: string, title: string) => void
  attach: (id: string, panes: readonly string[]) => void
  /** Forgets the agent a terminal was opened to run, once it has been sent. */
  launched: (id: string) => void
  /** Which tab is being renamed, and on which surface.

      The surface is not decoration: the strip and the sidebar draw the same
      tab, so an id alone puts a field in both. They then race for the focus,
      and the one that loses it commits and closes the other. */
  readonly renaming: Renaming | null
  setRenaming: (renaming: Renaming | null) => void
}

/* The strip belongs to the project: switching restores what that one had
   open, and the window itself opens on the empty state. */
export function useTabs(projectId: string | null): Tabs {
  const [strip, setStrip] = useState<Strip>(empty)
  const [renaming, setRenaming] = useState<Renaming | null>(null)

  useEffect(() => setStrip(remembered(projectId)), [projectId])
  useEffect(() => remember(projectId, strip), [projectId, strip])

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

  return {
    open: strip.open,
    active: focused(strip),
    show,
    close,
    focus,
    move,
    rename,
    attach,
    launched,
    renaming,
    setRenaming,
  }
}
