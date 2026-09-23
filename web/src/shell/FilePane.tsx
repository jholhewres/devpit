import { useEffect, useState } from 'react'

import { bytes } from './disk'
import { Code } from './Code'
import { ofPath } from './languages'
import { ask, commands } from './live'
import { Markdown } from './MarkdownView'
import type { Tab } from './strip'
import { useFile } from './useFile'
import { useShell } from './useShell'
import { useWantedLine } from './revealLine'

/*
 * One file, one tab.
 *
 * What is drawn comes from the backend's reading of the file, not from its
 * extension here: the first bytes decide, so a PNG named `notes.md` is a
 * picture rather than a pane full of mojibake.
 */

export function FilePane({ tab }: { tab: Tab }): React.JSX.Element {
  const { close, active, markUnsaved } = useShell()
  const path = tab.path ?? null
  const edit = useFile(path)
  const line = useWantedLine(path, edit.file !== null)
  const [preview, setPreview] = useState(true)

  const mine = active?.id === tab.id
  const kind = edit.file?.kind

  /* The strip draws the cross and this pane holds the edit, so the answer to
     "would closing lose anything" has to travel between them. Cleared on the
     way out: a tab that closed is not a tab with unsaved work. */
  useEffect(() => {
    markUnsaved(tab.id, edit.dirty)
    return () => markUnsaved(tab.id, false)
  }, [tab.id, edit.dirty, markUnsaved])

  /* ⌘S saves the tab in front, and only that one. */
  useEffect(() => {
    if (!mine) return
    const onKey = (event: KeyboardEvent): void => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
        event.preventDefault()
        if (edit.dirty) edit.save()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [mine, edit])

  const name = path?.split('/').pop() ?? 'No file open'
  const editing = kind === 'text' || (kind === 'markdown' && !preview)

  return (
    <>
      <div className="pane__bar">
        <span className="pane__t">
          <b>{name}</b>
          {path ? ` · ${path}` : ''}
          {edit.dirty ? ' ·' : ''}
        </span>
        <span className="drag" />
        {kind === 'markdown' && (
          <button className="chip" onClick={() => setPreview((was) => !was)}>
            {preview ? 'Source' : 'Preview'}
          </button>
        )}
        {edit.dirty && (
          <button className="chip" onClick={edit.save} disabled={edit.saving}>
            {edit.saving ? 'Saving…' : 'Save'}
          </button>
        )}
        <button className="sq26" onClick={() => close(tab.id)} aria-label="Close file">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      {edit.clash && (
        <div className="clash">
          <span className="clash__t">{edit.clash}</span>
          <button className="btn" onClick={edit.reload}>
            Reload from disk
          </button>
          <button className="btn btn--danger" onClick={edit.overwrite}>
            Overwrite
          </button>
        </div>
      )}

      <div className={editing ? 'code code--edit' : 'code'}>
        {!path && (
          <div className="exempty">
            <span className="exempty__t">No file open</span>
            <span className="exempty__d">Pick one in the Explorer.</span>
          </div>
        )}
        {edit.error && <div className="exempty__t">{edit.error}</div>}

        {!edit.error && edit.file?.notShown && (
          <div className="exempty">
            <span className="exempty__t">{edit.file.notShown}</span>
            <span className="exempty__d">{bytes(edit.file.bytes) ?? 'empty'}</span>
            <button
              className="btn"
              onClick={() => void ask(() => commands.pathReveal(edit.file!.fullPath))}
            >
              Show in the finder
            </button>
          </div>
        )}

        {kind === 'image' && edit.file?.dataUrl && (
          <div className="media">
            <img className="media__img" src={edit.file.dataUrl} alt={name} />
            <div className="media__what">{bytes(edit.file.bytes)}</div>
          </div>
        )}

        {kind === 'pdf' && edit.file?.dataUrl && (
          <object className="media__pdf" data={edit.file.dataUrl} type="application/pdf">
            <div className="exempty__t">This window cannot draw the PDF.</div>
          </object>
        )}

        {kind === 'markdown' && preview && <Markdown source={edit.text} />}

        {editing && (
          <Code
            text={edit.text}
            language={path ? ofPath(path) : null}
            onChange={edit.change}
            line={line}
          />
        )}
      </div>
    </>
  )
}
