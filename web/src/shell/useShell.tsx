import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'

import { who, type Who } from './account'
import type { Project, Theme as StoredTheme } from '../gen/bindings'
import { ask, commands } from './live'
import type { PaneName } from './paneList'
import { closed, moved, opened, type Strip } from './strip'
import { useProjects } from './useProjects'

export type Theme = StoredTheme
export type PrefsPane =
  | 'account'
  | 'projects'
  | 'general'
  | 'appearance'
  | 'providers'
  | 'skills'
  | 'storage'
  | 'usage'

/*
 * Three facts about the panes, because they came apart the moment tabs
 * arrived:
 *
 *   open      the strip's order — first opened is leftmost, and a new one
 *             lands on the right. Clicking an existing tab must not move it,
 *             or the strip reshuffles under the pointer.
 *   active    the pane you are looking at.
 *   previous  what a split would pair with — recency, which is a different
 *             order from the strip and the reason these are two lists.
 */
interface Shell {
  readonly open: readonly PaneName[]
  readonly active: PaneName | null
  show: (name: PaneName) => void
  close: (name: PaneName) => void
  move: (name: PaneName, to: number) => void

  readonly side: boolean
  readonly files: boolean
  toggleSide: () => void
  toggleFiles: () => void

  readonly theme: Theme
  setTheme: (theme: Theme) => void

  readonly project: Project | null
  readonly projects: readonly Project[]
  readonly projectsError: string | null
  setProject: (id: string) => void
  forgetProject: (id: string) => void
  reloadProjects: () => void

  readonly signedIn: boolean
  readonly account: Who
  signIn: () => void
  signOut: () => void

  readonly prefs: PrefsPane | null
  openPrefs: (pane?: PrefsPane) => void
  closePrefs: () => void
}

const ShellContext = createContext<Shell | null>(null)

export function useShell(): Shell {
  const shell = useContext(ShellContext)
  if (!shell) throw new Error('useShell outside the shell')
  return shell
}

export function ShellProvider({ children }: { children: React.ReactNode }): React.JSX.Element {
  const [{ open, active }, setStrip] = useState<Strip>({ open: ['board'], active: 'board' })
  const [side, setSide] = useState(true)
  const [files, setFiles] = useState(true)
  const [theme, setThemeState] = useState<Theme>('system')
  const [signedIn, setSignedIn] = useState(false)
  const [account, setAccount] = useState<Who>(who(null))
  const [prefs, setPrefs] = useState<PrefsPane | null>(null)
  const projects = useProjects()

  /* The three strip rules live in strip.ts so the tests can call them. */
  const show = useCallback((name: PaneName) => setStrip((was) => opened(was, name)), [])
  const close = useCallback((name: PaneName) => setStrip((was) => closed(was, name)), [])
  const move = useCallback((name: PaneName, to: number) => setStrip((was) => moved(was, name, to)), [])

  /* The choice is written where the next launch will find it; the window
     paints from local state so the click does not wait on disk. */
  const setTheme = useCallback((next: Theme) => {
    setThemeState(next)
    document.documentElement.dataset.theme = next
    void ask(() => commands.settingsWrite(null, next))
  }, [])

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    void ask(() => commands.settingsRead()).then((asked) => {
      if (!asked.data) return
      setThemeState(asked.data.theme)
      setAccount(who(asked.data.account))
      setSignedIn(asked.data.account !== null)
    })
  }, [])

  const value = useMemo<Shell>(
    () => ({
      open,
      active,
      show,
      close,
      move,
      side,
      files,
      toggleSide: () => setSide((was) => !was),
      toggleFiles: () => setFiles((was) => !was),
      theme,
      setTheme,
      ...projects,
      signedIn,
      account,
      signIn: () => setSignedIn(true),
      signOut: () => {
        setSignedIn(false)
        setPrefs(null)
      },
      prefs,
      openPrefs: (pane: PrefsPane = 'account') => setPrefs(pane),
      closePrefs: () => setPrefs(null),
    }),
    [open, active, show, close, move, side, files, theme, setTheme, projects, signedIn, account, prefs],
  )

  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
}
