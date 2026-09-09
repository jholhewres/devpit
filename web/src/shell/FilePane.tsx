import { useEffect, useState } from 'react'

import type { FileContents } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'

/* The skeleton is held for a beat even when the read is instant: shape then
   text reads as loading, where text appearing with no warning reads as a jump. */
export function FilePane({ path, show }: { path: string | null; show: boolean }): React.JSX.Element {
  const { project } = useShell()
  const [file, setFile] = useState<FileContents | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [reading, setReading] = useState(false)

  useEffect(() => {
    if (!project || !path) return
    setReading(true)
    const settle = window.setTimeout(() => setReading(false), 340)
    void ask(() => commands.fileRead(project.id, null, path)).then((asked) => {
      setError(asked.error)
      setFile(asked.data ?? null)
    })
    return () => window.clearTimeout(settle)
  }, [project, path])

  const name = path?.split('/').pop() ?? null

  return (
    <div className="pane pane--file" data-pane="file" data-show={String(show)}>
      <div className="pane__bar">
        <span className="pane__t">{name ? <><b>{name}</b> · {path}</> : 'No file open'}</span>
        <span className="drag" />
      </div>
      <div className={reading ? 'code loading' : 'code'}>
        {!path && (
          <div className="exempty">
            <span className="exempty__t">No file open</span>
            <span className="exempty__d">Pick one in the Explorer.</span>
          </div>
        )}
        {error && <div className="exempty__t">{error}</div>}
        {!error && file?.notShown && (
          <div className="exempty">
            <span className="exempty__t">{file.notShown}</span>
            {file.bytes !== null && <span className="exempty__d">{file.bytes} bytes</span>}
          </div>
        )}
        {!error && file?.text !== null && file?.text !== undefined && (
          <pre className="code__body">{file.text}</pre>
        )}
      </div>
    </div>
  )
}
