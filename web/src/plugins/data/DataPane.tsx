import { lazy, Suspense, useEffect, useState } from 'react'

import type { Tab } from '../../shell/strip'
import { TextEditor } from '../TextEditor'
import { usePluginFile } from '../usePluginFile'
import { asJson, DATA, isYaml, MOST_NODES, stemOf } from './dataFiles'

/* The graph is the heaviest thing any Capability here loads, so it arrives
   when a data file is opened and not before. */
const Graph = lazy(() =>
  import('jsoncrack-react').then((module) => ({ default: module.JSONCrack })),
)

/*
 * One data file: its text on the left, the shape it has on the right.
 *
 * A file halfway through being typed is not an error worth a red screen. The
 * reason is said under the editor and the last graph that parsed stays on
 * screen, which is what somebody fixing a comma needs to see.
 */

export function DataPane({ tab, name }: { tab: Tab; name: string }): React.JSX.Element {
  const file = usePluginFile(DATA, name)
  const [typed, setTyped] = useState<string | null>(null)
  const [json, setJson] = useState<unknown>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const source = typed ?? file.text

  useEffect(() => {
    if (source === null) return
    let dropped = false
    void asJson(source, isYaml(name)).then((answer) => {
      if (dropped) return
      if ('problem' in answer) return setProblem(answer.problem)
      setProblem(null)
      setJson(answer.json)
    })
    return () => {
      dropped = true
    }
  }, [source, name])

  if (file.problem) {
    return (
      <div className="plg-excalidraw__problem" role="alert">
        {file.problem}
      </div>
    )
  }

  return (
    <div className="data" data-tab={tab.id}>
      <div className="data__edit">
        {file.text !== null && (
          <TextEditor
            key={file.generation}
            value={file.text}
            label={`${stemOf(name)} source`}
            onChange={(text) => {
              setTyped(text)
              file.change(text)
            }}
          />
        )}
        {problem && (
          <p className="data__problem" role="status">
            {problem}
          </p>
        )}
      </div>
      <div className="data__see">
        {json !== null && (
          <Suspense fallback={<p className="bell__none">Drawing…</p>}>
            <Graph json={json as never} maxRenderableNodes={MOST_NODES} showControls />
          </Suspense>
        )}
      </div>
      {file.conflict && (
        <div className="plg-excalidraw__problem" role="alert">
          This file changed on disk while it was open.
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
