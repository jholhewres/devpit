import { useEffect, useRef, useState } from 'react'

import type { PaneName } from './paneList'
import type { Tab } from './strip'
import { onCarried } from './window'

/*
 * The island asking for a session: devpit is brought forward by Rust, and
 * this opens it — a terminal's pane, the way a notice does, or a chat's
 * conversation, once its project is the one on screen.
 */
export function useIslandOpens(
  current: string | null,
  setProject: (projectId: string) => void,
  openPane: (paneId: string | null) => void,
  show: (kind: PaneName, tab?: Partial<Tab>) => void,
): void {
  const held = useRef({ current, setProject, openPane })
  held.current = { current, setProject, openPane }
  const [wanted, setWanted] = useState<{ projectId: string; conversationId: string } | null>(null)

  useEffect(
    () =>
      onCarried<[string | null, string]>('island:open-pane', ([projectId, paneId]) => {
        const now = held.current
        if (projectId && projectId !== now.current) now.setProject(projectId)
        now.openPane(paneId)
      }),
    [],
  )

  useEffect(
    () =>
      onCarried<[string, string | null]>('island:open-chat', ([projectId, conversationId]) => {
        const now = held.current
        if (projectId !== now.current) now.setProject(projectId)
        // An orchestrator's project is its chat; nothing more to open.
        if (conversationId) setWanted({ projectId, conversationId })
      }),
    [],
  )

  /* The conversation opens in its own project's strip, which is there only
     after the switch has landed. */
  useEffect(() => {
    if (!wanted || wanted.projectId !== current) return
    show('chat', { id: wanted.conversationId })
    setWanted(null)
  }, [wanted, current, show])
}
