import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'

import { who } from './account'
import { ask, commands } from './live'
import type { PrefsPane, Shell, Theme } from './shape'
import { useAccount } from './useAccount'
import { useAgents } from './useAgents'
import { useClosing } from './useClosing'
import { usePaneSessions } from './paneSessions'
import { useSubagents } from './subagents'
import { useUnread } from './unread'
import { useWidths } from './useWidths'
import { useTabs } from './useTabs'
import { useProjects } from './useProjects'
import { useRunning } from './useRunning'

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
  const [theme, setThemeState] = useState<Theme>('system')
  const [prefs, setPrefs] = useState<PrefsPane | null>(null)
  const [palette, setPalette] = useState(false)
  const [wantedCard, setWantedCard] = useState<string | null>(null)

  const membership = useAccount()
  const projects = useProjects()
  const tabs = useTabs(projects.project?.id ?? null)
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


  /* The choice is written where the next launch will find it; the window
     paints from local state so the click does not wait on disk. */
  const setTheme = useCallback((next: Theme) => {
    setThemeState(next)
    document.documentElement.dataset.theme = next
    void ask(() => commands.settingsWrite(next, null, null, null, null))
  }, [])

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    void ask(() => commands.settingsRead()).then((asked) => {
      if (!asked.data) return
      setThemeState(asked.data.theme)
      /* Null is "never asked", and never-asked asks. */
      setConfirmStop(asked.data.confirmStop)
    })
  }, [setConfirmStop])

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
        setPrefs(null)
      },
      membership,
      signedIn: membership.account !== null,
      account: who(membership.account),
      signIn: () => void membership.signIn(),
      signOut: () => {
        void membership.signOut()
        setPrefs(null)
      },
      prefs,
      openPrefs: (pane: PrefsPane = 'account') => setPrefs(pane),
      closePrefs: () => setPrefs(null),
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
      markUnsaved,
      side,
      files,
      sizing,
      theme,
      setTheme,
      projects,
      membership,
      prefs,
      palette,
      wantedCard,
    ],
  )

  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
}
