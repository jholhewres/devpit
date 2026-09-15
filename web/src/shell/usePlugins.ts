import { createContext, createElement, useCallback, useContext, useEffect, useMemo, useState } from 'react'

import type { PluginState } from '../gen/bindings'
import { isOn, ownerOf } from './capabilities'
import { ask, commands } from './live'
import type { PaneName } from './paneList'
import { useShell } from './useShell'

/*
 * Which plugins the project in front has installed, and which are on.
 *
 * One reading for the window, like the shell's: the sidebar, the palette,
 * the Capabilities pane and the tab strip all ask, and four readings would
 * disagree for as long as the slowest one takes.
 */

export interface Plugins {
  /** Null until the catalogue is read for this project. */
  readonly plugins: readonly PluginState[] | null
  readonly error: string | null
  /** Whether a pane kind may be opened now. */
  offers: (kind: PaneName) => boolean
  setEnabled: (pluginId: string, on: boolean) => void
  install: (pluginId: string) => void
  /** How many files were deleted, or null when it did not uninstall. */
  uninstall: (pluginId: string, deleteData: boolean) => Promise<number | null>
}

/** The app's own panes always; a plugin's only while it is read as installed and on. */
export function paneOffered(kind: PaneName, plugins: readonly PluginState[] | null): boolean {
  const owner = ownerOf(kind)
  return owner === undefined || (plugins?.some((one) => one.manifest.id === owner && isOn(one)) ?? false)
}

interface Read {
  readonly projectId: string | null
  readonly plugins: readonly PluginState[] | null
  readonly error: string | null
}

const PluginsContext = createContext<Plugins | null>(null)

export function usePlugins(): Plugins {
  const plugins = useContext(PluginsContext)
  if (!plugins) throw new Error('usePlugins outside the plugins provider')
  return plugins
}

export function PluginsProvider({ children }: { children: React.ReactNode }): React.JSX.Element {
  const { project, open, closeNow } = useShell()
  const projectId = project?.id ?? null
  const [read, setRead] = useState<Read>({ projectId: null, plugins: null, error: null })

  useEffect(() => {
    if (!projectId) return setRead({ projectId: null, plugins: null, error: null })
    let current = true
    void ask(() => commands.pluginList(projectId)).then((answer) => {
      if (current) setRead({ projectId, plugins: answer.data?.plugins ?? null, error: answer.error })
    })
    return () => {
      current = false
    }
  }, [projectId])

  /* Kept per project: an answer that lands after a switch describes the
     project that was left, and must not be drawn as this one's. */
  const plugins = read.projectId === projectId ? read.plugins : null

  /* Off is off in the strip too, including tabs a project restored from
     before it was switched off. Only a catalogue read for this project
     decides; one that could not be read closes nothing. */
  useEffect(() => {
    if (plugins === null) return
    for (const tab of open) if (!paneOffered(tab.kind, plugins)) closeNow(tab.id)
  }, [plugins, open, closeNow])

  /* Every change answers with the whole catalogue; it lands only on the project it was asked for. */
  const settle = useCallback(
    (asked: string, answered: readonly PluginState[] | undefined, error: string | null) =>
      setRead((was) => (was.projectId !== asked ? was : { projectId: asked, plugins: answered ?? was.plugins, error })),
    [],
  )

  const setEnabled = useCallback(
    (pluginId: string, on: boolean) => {
      if (!projectId) return
      void ask(() => commands.pluginSetEnabled(projectId, pluginId, on)).then((answer) =>
        settle(projectId, answer.data?.plugins, answer.error),
      )
    },
    [projectId, settle],
  )

  const install = useCallback(
    (pluginId: string) => {
      if (!projectId) return
      void ask(() => commands.pluginInstall(projectId, pluginId)).then((answer) =>
        settle(projectId, answer.data?.plugins, answer.error),
      )
    },
    [projectId, settle],
  )

  const uninstall = useCallback(
    (pluginId: string, deleteData: boolean): Promise<number | null> => {
      if (!projectId) return Promise.resolve(null)
      return ask(() => commands.pluginUninstall(projectId, pluginId, deleteData)).then((answer) => {
        settle(projectId, answer.data?.plugins, answer.error)
        return answer.data ? answer.data.removedFiles : null
      })
    },
    [projectId, settle],
  )

  const value = useMemo<Plugins>(
    () => ({
      plugins,
      error: read.projectId === projectId ? read.error : null,
      offers: (kind) => paneOffered(kind, plugins),
      setEnabled,
      install,
      uninstall,
    }),
    [plugins, read, projectId, setEnabled, install, uninstall],
  )

  return createElement(PluginsContext.Provider, { value }, children)
}
