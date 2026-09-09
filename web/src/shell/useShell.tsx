import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'

import type { Project } from '../gen/bindings'
import { ask, commands } from './live'
import type { PaneName } from './paneList'
import { forgotten, found, type Open } from './projects'
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

  readonly project: Project | null
  readonly projects: readonly Project[]
  readonly projectsError: string | null
  setProject: (id: string) => void
  forgetProject: (id: string) => void
  reloadProjects: () => void

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

export function ShellProvider({ children }: { children: React.ReactNode }): React.JSX.Element {
  const [{ open, active }, setStrip] = useState<Strip>({ open: ['board'], active: 'board' })
  const [side, setSide] = useState(true)
  const [files, setFiles] = useState(true)
  const [theme, setThemeState] = useState<Theme>('dark')
  const [open_, setOpenProjects] = useState<Open>({ projects: [], current: null })
  const [projectsError, setProjectsError] = useState<string | null>(null)
  const { projects } = open_
  const project = found(open_)

  /* The list comes from disk. A project whose git cannot be read still comes
     back, marked — missing from the list would read as never added. */
  const reloadProjects = useCallback(() => {
    void ask(() => commands.projectList()).then((asked) => {
      setProjectsError(asked.error)
      if (!asked.data) return
      setOpenProjects((was) => ({
        projects: asked.data!.projects,
        current: was.current ?? asked.data!.projects[0]?.id ?? null,
      }))
    })
  }, [])

  useEffect(reloadProjects, [reloadProjects])

  const setProject = useCallback((id: string) => {
    setOpenProjects((was) => ({ ...was, current: id }))
    void ask(() => commands.projectOpen(id))
  }, [])
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

  const forgetProject = useCallback((id: string) => {
    setOpenProjects((was) => forgotten(was, id))
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
      project,
      projects,
      projectsError,
      setProject,
      forgetProject,
      reloadProjects,
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
    [open, active, show, close, move, side, files, theme, setTheme, project, projects, projectsError, setProject, forgetProject, reloadProjects, signedIn, prefs],
  )

  return <ShellContext.Provider value={value}>{children}</ShellContext.Provider>
}
