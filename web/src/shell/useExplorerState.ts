import { useEffect, useState } from 'react'

import { remember, remembered, type Mode, type View } from './explorer'

export interface ExplorerControls {
  readonly view: View
  readonly mode: Mode
  readonly query: string
  setView: (view: View) => void
  setMode: (mode: Mode) => void
  setQuery: (query: string) => void
}

/* View, mode and query are per project, the same way the tab strip is
   (`explorer.ts` mirrors `tabs.ts`): `RightPanel` never unmounts, so within
   one session this would already survive a close/reopen on its own, but a
   project switch has no reason to carry one project's search into another,
   and a real restart has nothing to restore from without this. */
export function useExplorerState(projectId: string | null): ExplorerControls {
  const [view, setView] = useState<View>('tree')
  const [mode, setMode] = useState<Mode>('names')
  const [query, setQuery] = useState('')

  useEffect(() => {
    const saved = remembered(projectId)
    setView(saved.view)
    setMode(saved.mode)
    setQuery(saved.query)
  }, [projectId])
  useEffect(() => remember(projectId, { view, mode, query }), [projectId, view, mode, query])

  return { view, mode, query, setView, setMode, setQuery }
}
