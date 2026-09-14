import type { Part } from '../gen/bindings'
import { byAgent, totals, type TurnFile } from './turnChanges'
import { useShell } from './useShell'

/*
 * The files a finished turn changed, under the turn that changed them.
 *
 * Measured from the checkout before and after, so it is the whole list and
 * not just the edits the agent reported. A file no edit call names is marked:
 * something the agent ran changed it.
 */

export function TurnChanges({ files, parts }: { files: readonly TurnFile[]; parts: readonly Part[] }): React.JSX.Element | null {
  const { show } = useShell()
  if (files.length === 0) return null
  const { added, removed } = totals(files)

  return (
    <div className="tchg">
      <div className="tchg__h">
        <span>
          {files.length} file{files.length === 1 ? '' : 's'} changed
        </span>
        <span className="ecard__add">+{added}</span>
        <span className="ecard__del">−{removed}</span>
      </div>
      {files.map((file) => (
        <button
          key={file.path}
          className="tchg__f"
          onClick={() => show('diff', { id: `diff:${file.path}`, path: file.path, title: `${file.path.split('/').pop()} diff` })}
        >
          <span className="tchg__p">{file.path}</span>
          {!byAgent(file, parts) && (
            <span className="tchg__by" title="No edit call names this file; something the agent ran changed it">
              by a command
            </span>
          )}
          <span className="ecard__add">+{file.added}</span>
          <span className="ecard__del">−{file.removed}</span>
        </button>
      ))}
    </div>
  )
}
