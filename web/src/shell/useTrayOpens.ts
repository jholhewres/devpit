import { useEffect, useRef } from 'react'

import type { PaneName } from './paneList'
import type { PrefsPane } from './shape'
import type { Tab } from './strip'
import { onCarried } from './window'

/*
 * The tray asking for something: devpit is brought forward by Rust, and this
 * opens it — a card whose reminder went off, a recent project, a new
 * terminal, Settings. A session waiting on you goes the island's way, through
 * `useIslandOpens`.
 */
export function useTrayOpens(
  current: string | null,
  setProject: (projectId: string) => void,
  show: (kind: PaneName, tab?: Partial<Tab>) => void,
  openCard: (cardId: string | null) => void,
  openPrefs: (pane?: PrefsPane) => void,
): void {
  const held = useRef({ current, setProject, show, openCard, openPrefs })
  held.current = { current, setProject, show, openCard, openPrefs }

  useEffect(() => {
    const drops = [
      onCarried<[string, string]>('tray:open-card', ([projectId, cardId]) => {
        const now = held.current
        if (projectId !== now.current) now.setProject(projectId)
        now.show('board')
        now.openCard(cardId)
      }),
      onCarried<string>('tray:open-project', (projectId) => {
        const now = held.current
        if (projectId !== now.current) now.setProject(projectId)
      }),
      onCarried<null>('tray:new-terminal', () => held.current.show('term')),
      onCarried<null>('tray:settings', () => held.current.openPrefs('general')),
    ]
    return () => drops.forEach((drop) => drop())
  }, [])
}
