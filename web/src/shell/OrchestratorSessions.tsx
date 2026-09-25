import { useEffect, useState } from 'react'

import type { LiveSession } from '../gen/bindings'
import { ask, commands } from './live'
import { useShell } from './useShell'
import { WaitingPrompts } from './WaitingPrompts'

/*
 * The sessions this account has running, beside the orchestrator's chat: the
 * same list it reaches with ListAgents, so what it says it is doing can be
 * checked at a glance. A row goes to where the session works.
 */

/* Read again this often while the chat is on screen. The CLI rewrites a
   listing when a session's status changes, and nothing pushes it here. */
const EVERY_MS = 3000

/* Stopped on the person first, then busy — what is worth looking at — then by name. */
const weight = (one: LiveSession): number => (one.waiting ? 2 : one.status === 'busy' ? 1 : 0)
export const inOrder = (sessions: readonly LiveSession[]): readonly LiveSession[] =>
  [...sessions].sort((a, b) => weight(b) - weight(a) || a.name.localeCompare(b.name))

export function OrchestratorSessions({ profileId }: { profileId: string }): React.JSX.Element {
  const { setProject, show, openCard } = useShell()
  const [sessions, setSessions] = useState<readonly LiveSession[]>([])
  const [open, setOpen] = useState(true)
  /* The one session being answered, and what is typed for it. */
  const [replying, setReplying] = useState<string | null>(null)
  const [reply, setReply] = useState('')
  const [said, setSaid] = useState<string | null>(null)
  /* Bumped after an answer, so the next question shows without waiting. */
  const [asked, setAsked] = useState(0)

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
  }, [profileId, asked])

  const go = (one: LiveSession): void => {
    if (!one.projectId) return
    setProject(one.projectId)
    if (one.cardId) {
      show('board')
      openCard(one.cardId)
    }
  }

  /* Typed into that session's own terminal, as the person: a message from
     the orchestrator would approve nothing there, and must not. */
  const answer = (one: LiveSession): void => {
    const text = reply.trim()
    if (!text) return
    void ask(() => commands.orchestratorReply(profileId, one.name, text)).then((sent) => {
      setSaid(sent.error ?? `Sent to ${one.name}`)
      if (sent.error) return
      setReply('')
      setReplying(null)
    })
  }

  const busy = sessions.filter((one) => one.status === 'busy').length
  return (
    <aside className="osess" data-open={open ? 'true' : undefined} aria-label="Sessions">
      <button className="osess__head" onClick={() => setOpen((was) => !was)} aria-expanded={open}>
        <span>Sessions</span>
        <span className="osess__count">{busy > 0 ? `${busy} busy · ${sessions.length}` : sessions.length}</span>
      </button>
      {/* Above the fold, and shown folded too: a question waiting is the one
          thing here that needs the person. */}
      <WaitingPrompts profileId={profileId} sessions={sessions} onAnswered={() => setAsked((was) => was + 1)} />
      {open && (
        <ul className="osess__list">
          {sessions.length === 0 && <li className="osess__none">Nothing running on this account.</li>}
          {sessions.map((one) => (
            <li key={one.name} className="osess__item">
              <button className="osess__row" onClick={() => go(one)} disabled={!one.projectId} title={one.cwd}>
                <span className="osess__dot" data-status={one.status} />
                <span className="osess__name">{one.name}</span>
                <span className="osess__where">{one.projectName ?? 'outside devpit'}{one.cardId ? ' · card' : ''}</span>
              </button>
              {one.inDevpit && (
                <button className="osess__answer" onClick={() => setReplying((was) => (was === one.name ? null : one.name))} title="Reply in its terminal, as you" aria-label={`Reply to ${one.name}`}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M9 14 4 9l5-5" /><path d="M4 9h10a6 6 0 0 1 6 6v5" /></svg>
                </button>
              )}
              {replying === one.name && (
                <form className="osess__reply" onSubmit={(event) => (event.preventDefault(), answer(one))}>
                  <input autoFocus value={reply} maxLength={4000} placeholder="As you, in its terminal" onChange={(event) => setReply(event.target.value)} />
                </form>
              )}
            </li>
          ))}
        </ul>
      )}
      {open && said && <p className="osess__said">{said}</p>}
    </aside>
  )
}
