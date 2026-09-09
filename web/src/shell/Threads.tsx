import { useEffect, useState } from 'react'

import type { Thread } from '../gen/bindings'
import { money } from './chat'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * The conversations this project has had.
 *
 * Drawn in an empty chat, which is where you look when you want the one you
 * had yesterday. The transcripts were always on disk; until this, closing a
 * tab left a file nobody could reach again.
 */

export function Threads({ hide }: { hide: string }): React.JSX.Element {
  const { project, show } = useShell()
  const [threads, setThreads] = useState<readonly Thread[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!project) return
    void ask(() => commands.chatList(project.id)).then((answer) => {
      setThreads(answer.data?.conversations ?? [])
      setError(answer.error)
    })
  }, [project])

  /* The conversation you are already in is not somewhere to go. */
  const shown = threads.filter((thread) => thread.id !== hide)
  if (shown.length === 0) {
    return <div className="exempty__t">{error ?? 'Nothing said yet.'}</div>
  }

  return (
    <>
      <div className="exempty__t">Nothing said yet.</div>
      <div className="threads">
        <div className="threads__h">Earlier</div>
        {shown.map((thread) => (
          <button
            className="threads__r"
            key={thread.id}
            onClick={() =>
              show('chat', { id: thread.id, title: thread.title.slice(0, 24) })
            }
          >
            <span className="threads__t">{thread.title}</span>
            <span className="threads__m">
              {[thread.profile, thread.model, money(thread.costUsd ?? 0), when(thread.lastAt)]
                .filter(Boolean)
                .join(' · ')}
            </span>
          </button>
        ))}
      </div>
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
