import { useCallback, useMemo, useState } from 'react'

import type { PrefsPane } from './shape'

/*
 * The two screens that take the window: Settings and the Manager.
 *
 * Apart from the shell because they are the same kind of thing and neither is
 * a pane. Settings is about the app rather than a project; the Manager shows
 * **every** project's board, which puts it above any one of them. A pane lives
 * inside a project, so neither belongs in the pane area — and the Manager did,
 * until somebody asked why the view of all projects opened inside one.
 *
 * Both leave on Escape, wired once in `AppShell` beside the other overlays.
 */

export interface Overlays {
  readonly prefs: PrefsPane | null
  openPrefs: (pane?: PrefsPane) => void
  closePrefs: () => void
  readonly managing: boolean
  openManager: () => void
  closeManager: () => void
}

export function useOverlays(): Overlays {
  const [prefs, setPrefs] = useState<PrefsPane | null>(null)
  const [managing, setManaging] = useState(false)

  // Stable, so a component picking one of them re-renders for nothing else.
  const openPrefs = useCallback((pane: PrefsPane = 'account') => setPrefs(pane), [])
  const closePrefs = useCallback(() => setPrefs(null), [])
  const openManager = useCallback(() => setManaging(true), [])
  const closeManager = useCallback(() => setManaging(false), [])
  return useMemo(
    () => ({ prefs, openPrefs, closePrefs, managing, openManager, closeManager }),
    [prefs, managing, openPrefs, closePrefs, openManager, closeManager],
  )
}
