import { useEffect, useState } from 'react'

import type { Commit } from '../gen/bindings'
import { ask, commands } from './live'
import { Skeleton } from './Skeleton'
import { useShell } from './useShell'

/*
 * What was committed here, most recent first.
 *
 * Local history only. Nothing fetches, so what is listed is what this
 * checkout has — and the branch chip beside it says how far behind that is.
 *
 * Loaded one page at a time: `hasMore` is what the backend already knows, so
 * the button disappears the moment there is nothing older to load, without
 * this screen guessing from a short page.
 */

export function History(): React.JSX.Element {
  const { project, show } = useShell()
  const [commits, setCommits] = useState<readonly Commit[]>([])
  const [hasMore, setHasMore] = useState(false)
  // Starts true: an empty list before the first answer arrives is "not
  // fetched yet", not "no commits", and the two must not draw the same.
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    setCommits([])
    setHasMore(false)
    setError(null)
    if (!project) {
      setLoading(false)
      return
    }
    setLoading(true)
    void ask(() => commands.projectHistory(project.id, null, null)).then((answer) => {
      setCommits(answer.data?.commits ?? [])
      setHasMore(answer.data?.hasMore ?? false)
      setError(answer.error)
      setLoading(false)
    })
  }, [project])

  const older = (): void => {
    if (!project) return
    setLoading(true)
    void ask(() => commands.projectHistory(project.id, null, commits.length)).then((answer) => {
      setCommits((was) => [...was, ...(answer.data?.commits ?? [])])
      setHasMore(answer.data?.hasMore ?? false)
      setError(answer.error)
      setLoading(false)
    })
  }

  return (
    <>
      {error && (
        <div className="exempty">
          <span className="exempty__t">{error}</span>
        </div>
      )}
      {loading && commits.length === 0 && <Skeleton />}
      {!error && !loading && commits.length === 0 && (
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
      {hasMore && (
        <button className="btn" disabled={loading} onClick={older}>
          {loading ? 'Loading…' : 'Load older commits'}
        </button>
      )}
    </>
  )
}
