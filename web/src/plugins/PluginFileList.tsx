import { useCallback, useEffect, useState } from 'react'

import type { PluginFile } from '../gen/bindings'
import { ask, commands } from '../shell/live'
import { paneMeta, type PaneName } from '../shell/paneList'
import { since } from '../shell/projects'
import type { Tab } from '../shell/strip'
import { useShell } from '../shell/useShell'
import { fileName, fileTab, stemOf, type FileKind } from './pluginFile'

/*
 * A Capability's files, and a field for the next one.
 *
 * Read from the plugin's data folder on every look: a file copied into it
 * from outside is one of these like any other.
 *
 * Generic because the third Capability would have been the third copy of it.
 * What a Capability brings is its kind, the pane its files open in, and the
 * sentence saying where they are kept.
 */

export function PluginFileList({
  kind,
  tab,
  pane,
  title,
  note,
  icon,
}: {
  kind: FileKind
  tab: Tab
  /** The pane one of its files opens in. */
  pane: PaneName
  /** What the Capability is called on screen. */
  title: string
  /** Where the files live, said once at the top of the list. */
  note: React.ReactNode
  icon: React.JSX.Element
}): React.JSX.Element {
  const { project, show, close } = useShell()
  const [files, setFiles] = useState<readonly PluginFile[]>([])
  const [problem, setProblem] = useState<string | null>(null)
  const [draft, setDraft] = useState('')

  const load = useCallback(() => {
    if (!project) return
    void ask(() => commands.pluginDataList(project.id, kind.pluginId)).then((answer) => {
      setFiles(answer.data?.files ?? [])
      setProblem(answer.error)
    })
  }, [project, kind.pluginId])

  useEffect(load, [load])

  const create = (): void => {
    const wanted = fileName(kind, draft)
    if ('problem' in wanted) return setProblem(wanted.problem)
    if (!project) return
    void ask(() =>
      commands.pluginDataWrite(project.id, kind.pluginId, wanted.name, kind.empty, null),
    ).then((answer) => {
      if (!answer.data) {
        return setProblem(
          answer.code === 'conflict'
            ? `${stemOf(kind, wanted.name)} already exists.`
            : answer.error,
        )
      }
      setDraft('')
      setProblem(null)
      load()
      show(pane, fileTab(kind, answer.data.name))
    })
  }

  return (
    <>
      <div className="pane__bar">
        <span className="pane__ico">{icon}</span>
        <span className="pane__t">
          <b>{title}</b>
          {files.length > 0 ? ` · ${files.length}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={load} aria-label={`Refresh ${title}`}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2M3 20v-5h5M21 4v5h-5" /></svg>
        </button>
        <button className="sq26" onClick={() => close(tab.id)} aria-label={`Close ${title}`}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="list">
        <div className="list__in">
          <p className="list__note">{note}</p>

          <form
            className="plg-excalidraw__new"
            onSubmit={(event) => {
              event.preventDefault()
              create()
            }}
          >
            <input
              className="plg-excalidraw__field"
              placeholder={`New ${kind.noun}`}
              aria-label={`New ${kind.noun} name`}
              spellCheck={false}
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
            />
            <button className="btn" type="submit">
              Create
            </button>
          </form>
          {problem && (
            <p className="plg-excalidraw__problem" role="alert">
              {problem}
            </p>
          )}

          {files.length > 0 && (
            <div className="list__h">
              In this project <span>{files.length}</span>
            </div>
          )}
          {files.map((file) => (
            <button
              className="cap"
              key={file.name}
              onClick={() => show(pane, fileTab(kind, file.name))}
            >
              <span className="cap__ico">{paneMeta(pane).icon}</span>
              <span className="cap__body">
                <span className="cap__top">
                  <span className="cap__name">{stemOf(kind, file.name)}</span>
                </span>
                <span className="cap__what">
                  edited {since(file.modified === null ? null : file.modified / 1000)}
                </span>
              </span>
            </button>
          ))}
        </div>
      </div>
    </>
  )
}
