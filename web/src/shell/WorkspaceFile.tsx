import { useState } from 'react'

import { bytes } from './disk'
import { Code } from './Code'
import { ofPath } from './languages'
import { ask, commands } from './live'
import { Markdown } from './MarkdownView'
import { useWorkspaceFile } from './useWorkspace'

/*
 * One workspace file, shown beside the list.
 *
 * Read-only, and deliberately. A transcript, a lock file and the state
 * database are records of what the product did; a panel that let you type
 * into them would be offering to rewrite history that other code is still
 * reading. Opening the file in a real editor is one click away and says so.
 */

export function WorkspaceFile({
  path,
  full,
  onClose,
}: {
  /** Relative to the workspace root, which is what the command takes. */
  path: string
  /** Absolute, which is what Open and Reveal take. */
  full: string
  onClose: () => void
}): React.JSX.Element {
  const { file, error, loading } = useWorkspaceFile(path)
  const [source, setSource] = useState(false)
  const name = path.split('/').pop() ?? path
  const kind = file?.kind

  return (
    <aside className="fb__side">
      <div className="fb__sideh">
        <span className="fb__sidet" title={full}>
          {name}
        </span>
        {kind === 'markdown' && (
          <button className="chip" onClick={() => setSource((was) => !was)}>
            {source ? 'Preview' : 'Source'}
          </button>
        )}
        <button className="chip" onClick={() => void ask(() => commands.pathOpen(full))}>
          Open
        </button>
        <button className="chip" onClick={() => void ask(() => commands.pathReveal(full))}>
          Reveal
        </button>
        <button className="sq26" onClick={onClose} aria-label="Close preview">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
        </button>
      </div>

      <div className="fb__sideb">
        {loading && <div className="exempty__t">Reading…</div>}
        {error && <div className="exempty__t">{error}</div>}

        {file?.notShown && (
          <div className="exempty">
            <span className="exempty__t">{file.notShown}</span>
            <span className="exempty__d">{bytes(file.bytes) ?? 'empty'}</span>
            <button className="btn" onClick={() => void ask(() => commands.pathReveal(full))}>
              Show in the finder
            </button>
          </div>
        )}

        {kind === 'image' && file?.dataUrl && (
          <div className="media">
            <img className="media__img" src={file.dataUrl} alt={name} />
            <div className="media__what">{bytes(file.bytes)}</div>
          </div>
        )}

        {kind === 'pdf' && file?.dataUrl && (
          <object className="media__pdf" data={file.dataUrl} type="application/pdf">
            <div className="exempty__t">This window cannot draw the PDF.</div>
          </object>
        )}

        {kind === 'markdown' && !source && <Markdown source={file?.text ?? ''} />}

        {(kind === 'text' || (kind === 'markdown' && source)) && (
          <div className="code code--edit">
            <Code text={file?.text ?? ''} language={ofPath(path)} readOnly />
          </div>
        )}
      </div>
    </aside>
  )
}
