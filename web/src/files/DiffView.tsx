import { useEffect, useState } from 'react'
import { commands } from '../gen/bindings'

/**
 * One file's diff, coloured by what each line is.
 *
 * Parsed here rather than shipped as HTML from the backend: the contract hands
 * over git's own output, and a screen that wants it another way is the screen's
 * problem to solve. Sending markup across would make the backend own how a
 * diff looks.
 */
export function DiffView({
  projectId,
  worktreeId,
  path,
  onClose
}: {
  projectId: string
  worktreeId: string | null
  path: string
  onClose: () => void
}): React.JSX.Element {
  const [diff, setDiff] = useState<string | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    let live = true
    void commands.fileDiff(projectId, worktreeId, path).then((answer) => {
      if (!live) return
      if (answer.status === 'ok') setDiff(answer.data)
      else setProblem(answer.error.message)
    })
    return () => {
      live = false
    }
  }, [projectId, worktreeId, path])

  return (
    <div className="diff">
      <header className="diff__head">
        <span className="diff__path" title={path}>
          {path}
        </span>
        <span className="diff__against">against the last commit</span>
        <span className="file__spacer" />
        <button type="button" onClick={onClose}>
          close
        </button>
      </header>

      {problem !== null && <p className="file__problem">{problem}</p>}

      {diff !== null && diff.trim() === '' ? (
        // An unchanged file is a truthful empty answer, and saying so beats an
        // empty pane that reads as something failing to load.
        <p className="file__not-shown">Nothing uncommitted in this file.</p>
      ) : (
        <pre className="diff__body">
          {(diff ?? '').split('\n').map((line, at) => (
            <span key={at} className={`diff__line diff__line--${kindOf(line)}`}>
              {line}
              {'\n'}
            </span>
          ))}
        </pre>
      )}
    </div>
  )
}

/**
 * What a diff line is.
 *
 * `+++` and `---` are checked before `+` and `-`, or the two header lines of
 * every diff would be painted as one added and one removed line.
 */
function kindOf(line: string): 'meta' | 'hunk' | 'added' | 'removed' | 'context' {
  if (line.startsWith('+++') || line.startsWith('---')) return 'meta'
  if (line.startsWith('diff ') || line.startsWith('index ') || line.startsWith('new file'))
    return 'meta'
  if (line.startsWith('@@')) return 'hunk'
  if (line.startsWith('+')) return 'added'
  if (line.startsWith('-')) return 'removed'
  return 'context'
}
