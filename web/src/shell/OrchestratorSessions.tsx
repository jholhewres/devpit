import { useEffect, useState } from 'react'

import type { LiveSession } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * The sessions this account has running, beside the orchestrator's chat: the
 * same list it reaches with ListAgents, so what it says it is doing can be
 * checked at a glance. A row goes to where the session works.
 */

/* Read again this often while the chat is on screen. The CLI rewrites a
   listing when a session's status changes, and nothing pushes it here. */
const EVERY_MS = 3000

/** Busy first — that is what is worth looking at — then by name. */
export const inOrder = (sessions: readonly LiveSession[]): readonly LiveSession[] =>
  [...sessions].sort((a, b) => Number(b.status === 'busy') - Number(a.status === 'busy') || a.name.localeCompare(b.name))

export function OrchestratorSessions({ profileId }: { profileId: string }): React.JSX.Element {
  const { setProject, show, openCard } = useShell()
  const [sessions, setSessions] = useState<readonly LiveSession[]>([])
  const [open, setOpen] = useState(true)

  useEffect(() => {
    let gone = false
    const read = (): void => {
      if (document.hidden) return
      void ask(() => commands.orchestratorSessions(profileId)).then((found) => {
        if (!gone && found.data) setSessions(inOrder(found.data.sessions))
      })
    }
    read()
    const timer = setInterval(read, EVERY_MS)
    return () => {
      gone = true
      clearInterval(timer)
    }
  }, [profileId])

  const go = (one: LiveSession): void => {
    if (!one.projectId) return
    setProject(one.projectId)
    if (one.cardId) {
      show('board')
      openCard(one.cardId)
    }
  }

  const busy = sessions.filter((one) => one.status === 'busy').length
  return (
    <aside className="osess" data-open={open ? 'true' : undefined} aria-label="Sessions">
      <button className="osess__head" onClick={() => setOpen((was) => !was)} aria-expanded={open}>
        <span>Sessions</span>
        <span className="osess__count">{busy > 0 ? `${busy} busy · ${sessions.length}` : sessions.length}</span>
      </button>
      {open && (
        <ul className="osess__list">
          {sessions.length === 0 && <li className="osess__none">Nothing running on this account.</li>}
          {sessions.map((one) => (
            <li key={one.name}>
              <button className="osess__row" onClick={() => go(one)} disabled={!one.projectId} title={one.cwd}>
                <span className="osess__dot" data-status={one.status} />
                <span className="osess__name">{one.name}</span>
                <span className="osess__where">{one.projectName ?? 'outside devpit'}{one.cardId ? ' · card' : ''}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </aside>
  )
}
