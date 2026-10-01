import { useEffect, useState } from 'react'

import type { IslandChecks } from '../gen/bindings'
import { ask, commands } from '../shell/live'

/*
 * The session's branch as its code host sees it — its pull request and how
 * the checks on it are doing — asked while the session is open on the
 * island, and again every minute it stays open.
 */

const AGAIN = 60_000

/** The words of the badge. */
export function checksWords(checks: IslandChecks): string {
  const pull = checks.pull ? `PR #${checks.pull.number} ${checks.pull.state.toLowerCase()}` : checks.branch
  return checks.checks ? `${pull} · checks ${checks.checks}` : pull
}

export function Checks({ sessionId }: { sessionId: string }): React.JSX.Element | null {
  const [checks, setChecks] = useState<IslandChecks | null>(null)

  useEffect(() => {
    let current = true
    const asked = (): void =>
      void ask(() => commands.islandChecks(sessionId)).then((answer) => {
        if (current) setChecks(answer.data)
      })
    asked()
    const every = window.setInterval(asked, AGAIN)
    return () => {
      current = false
      window.clearInterval(every)
    }
  }, [sessionId])

  if (!checks || (!checks.pull && !checks.checks)) return null
  const url = checks.pull?.url
  return (
    <button
      className="isl-checks"
      data-checks={checks.checks ?? undefined}
      title={checks.pull?.title ?? checks.branch}
      onClick={() => url && void ask(() => commands.urlOpen(url))}
    >
      <span className="isl-checks__dot" />
      <span className="isl-checks__words">{checksWords(checks)}</span>
    </button>
  )
}
