import { useState } from 'react'

import { Markdown } from '../../shell/Markdown'
import type { Tab } from '../../shell/strip'
import { TextEditor } from '../TextEditor'
import { usePluginFile } from '../usePluginFile'
import { NOTES_KIND, stemOf } from './noteFiles'

/*
 * One note: what was typed on the left, what it reads as on the right.
 *
 * The preview is the window's own `Markdown`, which already draws fenced
 * mermaid blocks — so a note with a diagram in it works without a line of
 * work here, and a heading looks the way a card's description does.
 *
 * Wikilinks, backlinks and a graph are not here. They are the second half of
 * this Capability and a plan of their own; what this is, is a note.
 */

export function NotePane({ tab, name }: { tab: Tab; name: string }): React.JSX.Element {
  const file = usePluginFile(NOTES_KIND, name)
  /* What the preview reads: the text as typed, ahead of the save. */
  const [typed, setTyped] = useState<string | null>(null)
  const showing = typed ?? file.text

  if (file.problem) {
    return (
      <div className="plg-excalidraw__problem" role="alert">
        {file.problem}
      </div>
    )
  }

  return (
    <div className="note" data-tab={tab.id}>
      <div className="note__edit">
        {file.text !== null && (
          <TextEditor
            key={file.generation}
            value={file.text}
            language="markdown"
            label={`${stemOf(name)} source`}
            onChange={(text) => {
              setTyped(text)
              file.change(text)
            }}
          />
        )}
      </div>
      <div className="note__see md">
        {showing !== null && showing !== '' ? (
          <Markdown source={showing} />
        ) : (
          <p className="bell__none">An empty note.</p>
        )}
      </div>
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
