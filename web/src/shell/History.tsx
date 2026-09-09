import { useEffect, useState } from 'react'

import type { Commit } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * What was committed here, most recent first.
 *
 * Local history only. Nothing fetches, so what is listed is what this
 * checkout has — and the branch chip beside it says how far behind that is.
 */

export function History(): React.JSX.Element {
  const { project, show } = useShell()
  const [commits, setCommits] = useState<readonly Commit[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!project) return
    void ask(() => commands.projectHistory(project.id, null)).then((answer) => {
      setCommits(answer.data?.commits ?? [])
      setError(answer.error)
    })
  }, [project])

  return (
    <>
      {error && (
        <div className="exempty">
          <span className="exempty__t">{error}</span>
        </div>
      )}
      {!error && commits.length === 0 && (
        <div className="exempty">
          <span className="exempty__t">No commits yet.</span>
        </div>
      )}
      {commits.map((commit) => (
        <button
          className="gitrow"
          key={commit.sha}
          onClick={() =>
            show('diff', {
              id: `commit:${commit.sha}`,
              path: commit.sha,
              title: commit.sha,
            })
          }
        >
          <span className="gitrow__n">{commit.subject}</span>
          <span className="gitrow__end">
            <span className="branchmenu__s">{commit.author}</span>
            <code>{commit.sha}</code>
          </span>
        </button>
      ))}
    </>
  )
}
