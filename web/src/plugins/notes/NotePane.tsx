import { lazy, Suspense } from 'react'

import { darkNow } from '../../shell/terminal'
import type { Tab } from '../../shell/strip'
import { usePluginFile } from '../usePluginFile'
import { NOTES_KIND } from './noteFiles'

/* The editor is the heaviest thing a note loads, so it arrives when a note is
   opened and not before. */
const NoteEditor = lazy(() =>
  import('./NoteEditor').then((module) => ({ default: module.NoteEditor })),
)

/*
 * One note, edited the way it reads.
 *
 * No split and no preview: what is on screen is the note. A pane of source
 * beside a pane of output is two things to look at for one document, and the
 * whole point of a block editor is that there is only one.
 *
 * What is written to disk is Markdown, because that is what the editor works
 * in and what the file is.
 */

export function NotePane({ tab, name }: { tab: Tab; name: string }): React.JSX.Element {
  const file = usePluginFile(NOTES_KIND, name)

  if (file.problem) {
    return (
      <div className="plg-excalidraw__problem" role="alert">
        {file.problem}
      </div>
    )
  }

  return (
    <div className="note" data-tab={tab.id}>
      {file.text !== null && (
        <Suspense fallback={<p className="bell__none">Opening…</p>}>
          <NoteEditor
            key={file.generation}
            value={file.text}
            dark={darkNow()}
            onChange={file.change}
          />
        </Suspense>
      )}
      {file.conflict && (
        <div className="plg-excalidraw__problem" role="alert">
          This note changed on disk while it was open.
          <button className="btn" onClick={file.reload}>
            Take what is on disk
          </button>
          <button className="btn" onClick={file.keepMine}>
            Keep mine
          </button>
        </div>
      )}
    </div>
  )
}
