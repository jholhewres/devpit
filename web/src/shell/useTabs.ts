import { useCallback, useEffect, useState } from 'react'

import { ask, commands } from './live'
import type { PaneName } from './paneList'
import {
  attached,
  closed,
  focused,
  moved,
  opened,
  renamed,
  type Strip,
  type Tab,
} from './strip'
import { empty, remember, remembered } from './tabs'

export interface Tabs {
  readonly open: readonly Tab[]
  readonly active: Tab | null
  show: (kind: PaneName, tab?: Partial<Tab>) => void
  close: (id: string) => void
  focus: (id: string) => void
  move: (id: string, to: number) => void
  rename: (id: string, title: string) => void
  attach: (id: string, paneId: string) => void
}

/* The strip belongs to the project: switching restores what that one had
   open, and the window itself opens on the empty state. */
export function useTabs(projectId: string | null): Tabs {
  const [strip, setStrip] = useState<Strip>(empty)

  useEffect(() => setStrip(remembered(projectId)), [projectId])
  useEffect(() => remember(projectId, strip), [projectId, strip])

  const show = useCallback(
    (kind: PaneName, tab: Partial<Tab> = {}) =>
      setStrip((was) => opened(was, { id: crypto.randomUUID(), kind, ...tab })),
    [],
  )

  /* Closing a terminal tab closes its pane too, or the session keeps a leaf
     nothing is looking at. */
  const close = useCallback(
    (id: string) =>
      setStrip((was) => {
        const going = was.open.find((tab) => tab.id === id)
        if (going?.paneId && projectId) {
          void ask(() => commands.sessionCloseLeaf(projectId, going.paneId!))
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
    (id: string, paneId: string) => setStrip((was) => attached(was, id, paneId)),
    [],
  )

  return { open: strip.open, active: focused(strip), show, close, focus, move, rename, attach }
}
