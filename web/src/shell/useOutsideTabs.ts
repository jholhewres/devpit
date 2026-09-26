import { useEffect } from 'react'

import type { PaneName } from './paneList'
import { closed, opened, type Tab } from './strip'
import { remember, remembered } from './tabs'
import { onOutsideTabs } from './window'

/*
 * Terminal tabs opened or closed from outside the window — an orchestrator
 * starting a session in a project, or stopping one — kept in that project's
 * strip: the one on screen through the strip itself, any other through what
 * it remembers, so the tab is there when the project is opened.
 */
export function useOutsideTabs(
  projectId: string | null,
  show: (kind: PaneName, tab?: Partial<Tab>) => void,
  close: (id: string) => void,
): void {
  useEffect(
    () =>
      onOutsideTabs(
        (tab) => {
          const made: Tab = { id: tab.tabId, kind: 'term', panes: tab.paneId ? [tab.paneId] : [] }
          if (tab.projectId === projectId) {
            show('term', made)
            return
          }
          const was = remembered(tab.projectId)
          remember(tab.projectId, { ...opened(was, made), active: was.active ?? tab.tabId })
        },
        (tab) => {
          if (tab.projectId === projectId) close(tab.tabId)
          else remember(tab.projectId, closed(remembered(tab.projectId), tab.tabId))
        },
      ),
    [projectId, show, close],
  )
}
