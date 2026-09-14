import { useEffect, useState } from 'react'

import type { Thread } from '../gen/bindings'
import { money } from './chat'
import { OutsideThreads } from './OutsideThreads'
import { SessionSearch } from './SessionSearch'
import { ask, commands } from './live'
import { short } from './strip'
import { useShell } from './useShell'

/*
 * The conversations this project has had.
 *
 * Folded away rather than listed: the sidebar is for what is running, and a
 * project with two hundred transcripts would bury that under a wall of
 * yesterday. Ten at a time, because the one you want is nearly always recent
 * and the rest is what search is for.
 *
 * The transcripts were always on disk; until this, closing a tab left a file
 * nobody could reach again.
 */

const SHOWN = 10

export function Threads(): React.JSX.Element {
  const { project, show, open } = useShell()
  const [threads, setThreads] = useState<readonly Thread[]>([])
  const [error, setError] = useState<string | null>(null)
  const [showing, setShowing] = useState(false)

  useEffect(() => {
    if (!project || !showing) return
    void ask(() => commands.chatList(project.id)).then((answer) => {
      setThreads(answer.data?.conversations ?? [])
      setError(answer.error)
    })
  }, [project, showing])

  /* A conversation already open is not somewhere to go — it is a tab. */
  const here = new Set(open.map((tab) => tab.id))
  const shown = threads.filter((thread) => !here.has(thread.id)).slice(0, SHOWN)

  return (
    <>
      <button
        className="heading heading--act"
        aria-expanded={showing}
        onClick={() => setShowing((was) => !was)}
      >
        Earlier
        <svg
          className="heading__v"
          width="10"
          height="10"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.4"
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{ transform: showing ? 'none' : 'rotate(-90deg)' }}
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>

      {showing && error && <div className="sessions__none">{error}</div>}
      {showing && !error && shown.length === 0 && (
        <div className="sessions__none">Nothing earlier.</div>
      )}

      {showing &&
        shown.map((thread) => (
          <button
            className="card"
            key={thread.id}
            title={thread.title}
            onClick={() => show('chat', { id: thread.id, title: thread.title })}
          >
            <span className="card__l1">
              <span className="card__t">{short(thread.title, 30)}</span>
            </span>
            <span className="card__l2">
              <span className="card__loose">
                {[thread.profile, money(thread.costUsd ?? 0), when(thread.lastAt)]
                  .filter(Boolean)
                  .join(' · ')}
              </span>
            </span>
          </button>
        ))}

      {showing && <SessionSearch />}
      {showing && <OutsideThreads />}
    </>
  )
}

/* Against the reader's own clock, in the reader's own locale. */
function when(seconds: number | null): string {
  if (!seconds) return ''
  return new Date(seconds * 1000).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
  })
}
