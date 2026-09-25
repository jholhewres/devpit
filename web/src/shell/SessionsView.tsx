import { useCallback, useEffect, useMemo, useState } from 'react'

import type { AgentThread, LiveSession } from '../gen/bindings'
import { ask, commands } from './live'
import { inOrder, refreshSessions, STATE_WORDS, stateOf, useLiveSessions } from './liveStatus'
import { SessionTerminal } from './SessionTerminal'
import { useShell } from './useShell'

/*
 * The sessions an orchestrator works with, in one place: how each stands,
 * what can be done with it, and what passed between them.
 *
 * Grouped by project, the ones that need the person first. A row says who and
 * how it is; its two buttons open the session's terminal right here or go to
 * it; opening the row shows the question it waits on, a reply typed into its
 * terminal as the person, and the history of what it was asked and answered.
 * Sessions no longer running stay below, with their history.
 */

const ARROW = { sent: '→', heard: '←', idle: '✓' } as const
const HISTORY = 4

const when = (at: string): string => {
  const date = new Date(at)
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })
}

/** How long it has been in its state, in words. */
export function since(ms: number | null, now: number): string | null {
  if (ms === null) return null
  const minutes = Math.max(0, Math.round((now - ms) / 60000))
  if (minutes < 1) return 'just now'
  if (minutes < 60) return `${minutes} min`
  const hours = Math.round(minutes / 60)
  return hours < 24 ? `${hours} h` : `${Math.round(hours / 24)} d`
}

export function SessionsView({ shown }: { shown: boolean }): React.JSX.Element {
  const { project, setProject, show, openCard, openPane } = useShell()
  const profileId = project?.orchestrator ?? null
  const sessions = useLiveSessions(profileId)
  const [threads, setThreads] = useState<readonly AgentThread[]>([])
  const [open, setOpen] = useState<string | null>(null)
  const [terminal, setTerminal] = useState<LiveSession | null>(null)
  const now = Date.now()

  const readHistory = useCallback(() => {
    if (!project?.orchestrator) return
    void ask(() => commands.orchestratorAgents(project.id)).then((answer) => setThreads(answer.data ?? []))
  }, [project])
  useEffect(() => {
    if (shown) readHistory()
  }, [shown, readHistory])

  const go = (one: LiveSession): void => {
    if (!one.projectId) return
    setProject(one.projectId)
    if (one.pane) openPane(one.pane.paneId)
    else if (one.cardId) {
      show('board')
      openCard(one.cardId)
    }
  }

  const groups = useMemo(() => {
    const by = new Map<string, LiveSession[]>()
    for (const one of inOrder(sessions)) {
      const key = one.projectName ?? 'Outside devpit'
      by.set(key, [...(by.get(key) ?? []), one])
    }
    return [...by.entries()]
  }, [sessions])
  const earlier = threads.filter((thread) => !sessions.some((one) => one.name === thread.name))
  const waiting = sessions.filter((one) => one.waiting).length
  const busy = sessions.filter((one) => one.status === 'busy' && !one.waiting).length
  const historyOf = (name: string): AgentThread | undefined => threads.find((thread) => thread.name === name)

  return (
    <div className="sess">
      <div className="sess__top">
        <span className="sess__t">Sessions</span>
        <span className="sess__sum">
          {waiting > 0 && <b className="sess__wait">{waiting} waiting</b>}
          {busy > 0 && <span>{busy} working</span>}
          {sessions.length === 0 ? 'none running' : <span>{sessions.length} running</span>}
        </span>
        <button className="sess__icon" onClick={() => (profileId && refreshSessions(profileId), readHistory())} title="Read again" aria-label="Refresh sessions">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5" /></svg>
        </button>
      </div>

      {sessions.length === 0 && (
        <p className="sess__note">No session of this account is running. Ask the orchestrator to start one on a card, or open Claude Code in a project&rsquo;s terminal.</p>
      )}

      {groups.map(([name, members]) => (
        <section className="sess__group" key={name}>
          <h3 className="sess__g">{name}</h3>
          {members.map((one) => {
            const state = stateOf(sessions, one.name)
            const history = historyOf(one.name)
            const last = history?.events[history.events.length - 1]
            const expanded = open === one.name
            return (
              <div className="sess__one" key={one.name} data-state={state} data-open={expanded ? 'true' : undefined}>
                <div className="sess__row">
                  <button className="sess__main" aria-expanded={expanded} onClick={() => setOpen(expanded ? null : one.name)} title={one.cwd}>
                    <i className="deleg__dot" data-state={state} />
                    <span className="sess__name">{one.name}</span>
                    <span className="sess__state">{STATE_WORDS[state]}</span>
                    <span className="sess__meta">
                      {[one.cardId ? 'card' : null, since(one.since, now)].filter(Boolean).join(' · ')}
                      {last && ` · ${ARROW[last.kind as keyof typeof ARROW] ?? ''} ${last.summary ?? last.text}`}
                    </span>
                  </button>
                  {one.pane && (
                    <button className="sess__icon" onClick={() => setTerminal(one)} title="Open its terminal here" aria-label={`Open ${one.name}'s terminal here`}>
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="m7 10 3 2-3 2M13 14h4" /></svg>
                    </button>
                  )}
                  {one.projectId && (
                    <button className="sess__icon" onClick={() => go(one)} title="Go to it: its project and tab" aria-label={`Go to ${one.name}`}>
                      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M7 17 17 7M8 7h9v9" /></svg>
                    </button>
                  )}
                </div>
                {expanded && <Details session={one} history={history} profileId={profileId} />}
              </div>
            )
          })}
        </section>
      ))}

      {earlier.length > 0 && (
        <section className="sess__group">
          <h3 className="sess__g">Earlier · not running</h3>
          {earlier.map((thread) => (
            <div className="sess__one" key={thread.name} data-state="gone" data-open={open === thread.name ? 'true' : undefined}>
              <div className="sess__row">
                <button className="sess__main" aria-expanded={open === thread.name} onClick={() => setOpen(open === thread.name ? null : thread.name)}>
                  <i className="deleg__dot" data-state="gone" />
                  <span className="sess__name">{thread.name}</span>
                  <span className="sess__state">{when(thread.lastAt)}</span>
                </button>
              </div>
              {open === thread.name && <History thread={thread} />}
            </div>
          ))}
        </section>
      )}

      {terminal && (
        <SessionTerminal
          session={terminal}
          onClose={() => setTerminal(null)}
          onGo={() => {
            setTerminal(null)
            go(terminal)
          }}
        />
      )}
    </div>
  )
}

function Details({ session, history, profileId }: { session: LiveSession; history: AgentThread | undefined; profileId: string | null }): React.JSX.Element {
  const [reply, setReply] = useState('')
  const [said, setSaid] = useState<string | null>(null)

  /* Typed into that session's own terminal, as the person: a message from
     the orchestrator would approve nothing there, and must not. */
  const send = (): void => {
    const text = reply.trim()
    if (!text || !profileId) return
    void ask(() => commands.orchestratorReply(profileId, session.name, text)).then((sent) => {
      setSaid(sent.error ?? 'Sent, as you.')
      if (sent.error) return
      setReply('')
      refreshSessions(profileId)
    })
  }

  return (
    <div className="sess__more">
      {session.waiting && (
        <p className="sess__q">
          <b>Waiting on you:</b> {session.waiting.question}
        </p>
      )}
      {session.inDevpit ? (
        <form className="sess__reply" onSubmit={(event) => (event.preventDefault(), send())}>
          <input value={reply} maxLength={4000} placeholder={`Reply in ${session.name}'s terminal, as you`} aria-label={`Reply to ${session.name}`} onChange={(event) => setReply(event.target.value)} />
        </form>
      ) : (
        <p className="sess__note">Opened outside devpit: it can be messaged, not typed into from here.</p>
      )}
      {said && <p className="sess__note">{said}</p>}
      {history ? <History thread={history} /> : <p className="sess__note">Nothing has passed between this chat and it yet.</p>}
    </div>
  )
}

function History({ thread }: { thread: AgentThread }): React.JSX.Element {
  const [all, setAll] = useState(false)
  const events = all ? thread.events : thread.events.slice(-HISTORY)
  return (
    <div className="sess__hist">
      {thread.events.length > HISTORY && !all && (
        <button className="sess__more-btn" onClick={() => setAll(true)}>
          Show all {thread.events.length}
        </button>
      )}
      <ol>
        {events.map((event, at) => (
          <li key={at} data-kind={event.kind}>
            <span className="sess__when">
              {ARROW[event.kind as keyof typeof ARROW] ?? '·'} {when(event.at)}
            </span>
            {event.summary && <strong>{event.summary}</strong>}
            <span className="sess__text">{event.kind === 'idle' ? `Finished a turn. ${event.text}` : event.text}</span>
          </li>
        ))}
      </ol>
    </div>
  )
}
