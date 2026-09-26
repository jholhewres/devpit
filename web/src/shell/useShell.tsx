import { createContext, useCallback, useContext, useLayoutEffect, useMemo, useState } from 'react'

import { who } from './account'
import { useOverlays } from './useOverlays'
import type { Shell } from './shape'
import { useAccount } from './useAccount'
import { useAgents } from './useAgents'
import { useClosing } from './useClosing'
import { usePaneSessions } from './paneSessions'
import { useSubagents } from './subagents'
import { useUnread } from './unread'
import { useSweep } from './useSweep'
import { useOutsideTabs } from './useOutsideTabs'
import { useWidths } from './useWidths'
import { useTabs } from './useTabs'
import { useProjects } from './useProjects'
import { useRunning } from './useRunning'
import { useTheme } from './useTheme'
import { createShellStore, ShellStoreContext } from './shellStore'

export type { Closing } from './useClosing'
export type { PrefsPane, Theme } from './shape'

const ShellContext = createContext<Shell | null>(null)

export function useShell(): Shell {
  const shell = useContext(ShellContext)
  if (!shell) throw new Error('useShell outside the shell')
  return shell
}

export function ShellProvider({ children }: { children: React.ReactNode }): React.JSX.Element {
  const [side, setSide] = useState(true)
  const [files, setFiles] = useState(true)
  const overlays = useOverlays()
  const [palette, setPalette] = useState(false)
  const [wantedCard, setWantedCard] = useState<string | null>(null)

  const membership = useAccount()
  const projects = useProjects()
  const tabs = useTabs(projects.project?.id ?? null)
  useOutsideTabs(projects.project?.id ?? null, tabs.show, tabs.close)
  const running = useRunning(projects.project?.id ?? null)
  const doing = useAgents()
  const unread = useUnread(doing, tabs.active)
  const subagents = useSubagents()
  const agentSessions = usePaneSessions()
  const sizing = useWidths()

  /* Which tabs hold an edit that is not on disk. Held here because the cross
     is drawn by the strip and the edit lives in the pane, and neither can see
     the other — the pane reports, the strip's close reads. */
  const [unsaved, setUnsaved] = useState<ReadonlySet<string>>(() => new Set())
  const markUnsaved = useCallback((id: string, dirty: boolean) => {
    setUnsaved((was) => {
      if (was.has(id) === dirty) return was
      const next = new Set(was)
      if (dirty) next.add(id)
      else next.delete(id)
      return next
    })
  }, [])
  const guard = useClosing({ open: tabs.open, closeNow: tabs.close, running, unsaved })
  const { setConfirmStop } = guard

  const sweep = useSweep(tabs.open, tabs.active?.id ?? null, guard.close)

  const { theme, setTheme } = useTheme(setConfirmStop)

  const value = useMemo<Shell>(
    () => ({
      ...tabs,
      ...guard,
      running,
      doing,
      unread,
      subagents,
      agentSessions,
      closeNow: tabs.close,
      sweep,
      markUnsaved,
      side,
      files,
      ...sizing,
      toggleSide: () => setSide((was) => !was),
      toggleFiles: () => setFiles((was) => !was),
      theme,
      setTheme,
      ...projects,
      setProject: (id: string) => {
        projects.setProject(id)
        overlays.closePrefs()
      },
      membership,
      signedIn: membership.account !== null,
      account: who(membership.account),
      signIn: () => void membership.signIn(),
      signOut: () => {
        void membership.signOut()
        /* Signing out from inside Settings leaves a screen about an account
           that is gone. */
        overlays.closePrefs()
      },
      ...overlays,
      wantedCard,
      openCard: setWantedCard,
      palette,
      openPalette: () => setPalette(true),
      closePalette: () => setPalette(false),
    }),
    [
      tabs,
      guard,
      running,
      doing,
      unread,
      subagents,
      agentSessions,
      sweep,
      markUnsaved,
      side,
      files,
      sizing,
      theme,
      setTheme,
      projects,
      membership,
      overlays,
      palette,
      wantedCard,
    ],
  )

  /* The same shell, for the components that read one slice of it
     (`useShellPick`): put while rendering so a child mounting now reads it,
     announced once the render is committed. */
  const [store] = useState(() => createShellStore(value))
  store.put(value)
  useLayoutEffect(() => {
    store.put(value)
    store.tell()
  }, [store, value])

  return (
    <ShellStoreContext.Provider value={store}>
      <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
    </ShellStoreContext.Provider>
  )
}
