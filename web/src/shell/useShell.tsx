import { createContext, useCallback, useContext, useMemo, useState } from 'react'

import type { PaneName } from './paneList'
import { closed, moved, opened, type Strip } from './strip'

export type Theme = 'system' | 'light' | 'dark'
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

  readonly project: string
  setProject: (name: string) => void
  readonly projects: readonly string[]
  forgetProject: (name: string) => void

  readonly signedIn: boolean
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

const FIRST_PROJECTS = ['devpit', 'orca', 'anchored', 'waku', 'hg-portal']

export function ShellProvider({ children }: { children: React.ReactNode }): React.JSX.Element {
  const [{ open, active }, setStrip] = useState<Strip>({ open: ['board'], active: 'board' })
  const [side, setSide] = useState(true)
  const [files, setFiles] = useState(true)
  const [theme, setThemeState] = useState<Theme>('dark')
  const [project, setProject] = useState('devpit')
  const [projects, setProjects] = useState<string[]>(FIRST_PROJECTS)
  const [signedIn, setSignedIn] = useState(false)
  const [prefs, setPrefs] = useState<PrefsPane | null>(null)

  /* The three rules live in `strip.ts` so the tests can call them rather
     than restate them. */
  const show = useCallback((name: PaneName) => {
    setStrip((was) => opened(was, name))
  }, [])

  const close = useCallback((name: PaneName) => {
    setStrip((was) => closed(was, name))
  }, [])

  const move = useCallback((name: PaneName, to: number) => {
    setStrip((was) => moved(was, name, to))
  }, [])

  const setTheme = useCallback((next: Theme) => {
    setThemeState(next)
    document.documentElement.dataset.theme = next
  }, [])

  const forgetProject = useCallback(
    (name: string) =>
      setProjects((was) => {
        const left = was.filter((other) => other !== name)
        /* Removing the one you are standing in has to land somewhere. */
        setProject((current) => (current === name ? (left[0] ?? '') : current))
        return left
      }),
    [],
  )

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
      project,
      setProject,
      projects,
      forgetProject,
      signedIn,
      signIn: () => setSignedIn(true),
      signOut: () => {
        setSignedIn(false)
        setPrefs(null)
      },
      prefs,
      openPrefs: (pane: PrefsPane = 'account') => setPrefs(pane),
      closePrefs: () => setPrefs(null),
    }),
    [open, active, show, close, move, side, files, theme, setTheme, project, projects, forgetProject, signedIn, prefs],
  )

  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
}
