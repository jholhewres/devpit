import { useCallback, useEffect, useState } from 'react'

import type { PluginFile } from '../../gen/bindings'
import { ask, commands } from '../../shell/live'
import { paneMeta } from '../../shell/paneList'
import { since } from '../../shell/projects'
import type { Tab } from '../../shell/strip'
import { useShell } from '../../shell/useShell'
import { drawingName, drawingTab, EMPTY_DRAWING, PLUGIN_ID, stemOf } from './drawings'

/*
 * The project's drawings, and a field for the next one.
 *
 * Read from the plugin's data folder on every look: a `.excalidraw` file
 * copied into it from outside is a drawing like any other.
 */

export function DrawingList({ tab }: { tab: Tab }): React.JSX.Element {
  const { project, show, close } = useShell()
  const [files, setFiles] = useState<readonly PluginFile[]>([])
  const [problem, setProblem] = useState<string | null>(null)
  const [draft, setDraft] = useState('')

  const load = useCallback(() => {
    if (!project) return
    void ask(() => commands.pluginDataList(project.id, PLUGIN_ID)).then((answer) => {
      setFiles(answer.data?.files ?? [])
      setProblem(answer.error)
    })
  }, [project])

  useEffect(load, [load])

  const create = (): void => {
    const wanted = drawingName(draft)
    if ('problem' in wanted) return setProblem(wanted.problem)
    if (!project) return
    void ask(() => commands.pluginDataWrite(project.id, PLUGIN_ID, wanted.name, EMPTY_DRAWING, null)).then(
      (answer) => {
        if (!answer.data) {
          return setProblem(answer.code === 'conflict' ? `${stemOf(wanted.name)} already exists.` : answer.error)
        }
        setDraft('')
        setProblem(null)
        load()
        show('drawing', drawingTab(answer.data.name))
      },
    )
  }

  return (
    <>
      <div className="pane__bar">
        <span className="pane__ico">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M12 19l7-7 3 3-7 7-3-3ZM18 13l-1.5-7.5L2 2l3.5 14.5L13 18ZM2 2l7.6 7.6" /></svg>
        </span>
        <span className="pane__t">
          <b>Excalidraw</b>
          {files.length > 0 ? ` · ${files.length}` : ''}
        </span>
        <span className="drag"></span>
        <button className="sq26" onClick={load} aria-label="Refresh drawings">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 0 1-15.5 6.2M3 12a9 9 0 0 1 15.5-6.2M3 20v-5h5M21 4v5h-5" /></svg>
        </button>
        <button className="sq26" onClick={() => close(tab.id)} aria-label="Close Excalidraw">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="list">
        <div className="list__in">
          <p className="list__note">
            Excalidraw drawings, kept as <code>.excalidraw</code> files in this project&rsquo;s
            folder.
          </p>

          <form
            className="plg-excalidraw__new"
            onSubmit={(event) => {
              event.preventDefault()
              create()
            }}
          >
            <input
              className="plg-excalidraw__field"
              placeholder="New drawing"
              aria-label="New drawing name"
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
            <button className="cap" key={file.name} onClick={() => show('drawing', drawingTab(file.name))}>
              <span className="cap__ico">{paneMeta('drawing').icon}</span>
              <span className="cap__body">
                <span className="cap__top">
                  <span className="cap__name">{stemOf(file.name)}</span>
                </span>
                <span className="cap__what">edited {since(file.modified === null ? null : file.modified / 1000)}</span>
              </span>
            </button>
          ))}
        </div>
      </div>
    </>
  )
}
