import { useState } from 'react'

import { Diagram } from '../../shell/Diagram'
import type { Tab } from '../../shell/strip'
import { TextEditor } from '../TextEditor'
import { usePluginFile } from '../usePluginFile'
import { MERMAID, stemOf } from './mermaidFiles'

/*
 * One diagram: its text on the left, what it draws on the right.
 *
 * The preview goes through `Diagram`, which is what a fenced mermaid block in
 * a note or a card already uses — one renderer, one version, one set of
 * settings. A diagram that does not parse shows the reason above its own
 * source rather than an empty frame, which is `Diagram`'s own behaviour and
 * exactly what somebody halfway through typing one needs.
 */

export function MermaidPane({ tab, name }: { tab: Tab; name: string }): React.JSX.Element {
  const file = usePluginFile(MERMAID, name)
  /* What the preview draws: the text as typed, ahead of the save. */
  const [source, setSource] = useState<string | null>(null)
  const drawing = source ?? file.text

  if (file.problem) {
    return (
      <div className="plg-excalidraw__problem" role="alert">
        {file.problem}
      </div>
    )
  }

  return (
    <div className="mmd" data-tab={tab.id}>
      <div className="mmd__edit">
        {file.text !== null && (
          <TextEditor
            key={file.generation}
            value={file.text}
            label={`${stemOf(name)} source`}
            onChange={(text) => {
              setSource(text)
              file.change(text)
            }}
          />
        )}
      </div>
      <div className="mmd__see">{drawing && <Diagram source={drawing} />}</div>
      {file.conflict && (
        <div className="plg-excalidraw__problem" role="alert">
          This diagram changed on disk while it was open.
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
