import { useCallback, useEffect, useMemo, useState } from 'react'

import type { AgentThread, LiveSession, Project } from '../gen/bindings'
import { ask, commands } from './live'
import { inOrder, refreshSessions, STATE_WORDS, stateOf, useLiveSessions } from './liveStatus'
import { ReplyDrafts } from './ReplyDrafts'
import { SessionTerminal } from './SessionTerminal'
import { opened } from './strip'
import { remember, remembered } from './tabs'
import { PanelAct, PanelEmpty, PanelHead } from './PanelHead'
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

/** A row's own key: a name started twice is two sessions, and keyed by name
 *  they opened and closed as one. */
export const rowOf = (one: LiveSession): string => `${one.name}:${one.pid}`

/* "3 min" has to move while the panel is on screen, and only then. */
function useNow(shown: boolean): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    if (!shown) return
    setNow(Date.now())
    const every = setInterval(() => setNow(Date.now()), 30_000)
    return () => clearInterval(every)
  }, [shown])
  return now
}

export function SessionsView({ shown }: { shown: boolean }): React.JSX.Element {
  const { project, projects, setProject, show, openCard, openPane } = useShell()
  const [linked, setLinked] = useState<readonly string[]>([])
  const profileId = project?.orchestrator ?? null
  const sessions = useLiveSessions(profileId)
  const [threads, setThreads] = useState<readonly AgentThread[]>([])
  const [open, setOpen] = useState<string | null>(null)
  const [terminal, setTerminal] = useState<LiveSession | null>(null)
  const now = useNow(shown)

  const readHistory = useCallback(() => {
    if (!project?.orchestrator) return
    void ask(() => commands.orchestratorAgents(project.id)).then((answer) => setThreads(answer.data ?? []))
  }, [project])
  useEffect(() => {
    if (!shown) return
    readHistory()
    if (project) void ask(() => commands.orchestratorLinks(project.id)).then((answer) => setLinked(answer.data ?? []))
  }, [shown, readHistory, project])

  /* A new terminal in a project: a tab of its own there, kept in its strip,
     opened here over the chat. */
  const newTerminal = (one: Project): void => {
    const tabId = crypto.randomUUID()
    void ask(() => commands.sessionEnsure(one.id, tabId, null)).then((made) => {
      if (!made.data) return
      const was = remembered(one.id)
      remember(one.id, { ...opened(was, { id: tabId, kind: 'term', panes: [made.data.focusedId] }), active: was.active ?? tabId })
      setTerminal({
        name: 'Terminal',
        pid: 0,
        job: null,
        status: 'idle',
        kind: 'interactive',
        cwd: one.rootPath,
        projectId: one.id,
        projectName: one.name,
        cardId: null,
        since: null,
        inDevpit: true,
        waiting: null,
        pane: { projectId: one.id, paneId: made.data.focusedId },
        sessionId: null,
        step: null,
        draft: null,
      })
    })
  }
  /* A new chat there: its strip gets one, and the project opens on it. */
  const newChat = (one: Project): void => {
    remember(one.id, opened(remembered(one.id), { id: crypto.randomUUID(), kind: 'chat' }))
    setProject(one.id)
  }

  /* A background session attached in a new tab of its project, and that
     terminal opened here over the chat. */
  const watch = (one: LiveSession): void => {
    if (!profileId || !one.job || !one.projectId) return
    const projectId = one.projectId
    void ask(() => commands.sessionWatch(profileId, projectId, one.job!)).then((answer) => {
      if (answer.data) setTerminal({ ...one, inDevpit: true, pane: { projectId, paneId: answer.data } })
    })
  }

  const go = (one: LiveSession): void => {
    if (!one.projectId) return
    setProject(one.projectId)
    if (one.pane) openPane(one.pane.paneId)
    else if (one.cardId) {
      show('board')
      openCard(one.cardId)
    }
  }

  /* Every project with a session, and every linked one even without: where
     a new terminal or chat can be started. */
  const groups = useMemo(() => {
    const by = new Map<string, { project: Project | null; members: LiveSession[] }>()
    for (const one of inOrder(sessions)) {
      const key = one.projectId ?? 'outside'
      const found = projects.find((candidate) => candidate.id === one.projectId) ?? null
      by.set(key, { project: found, members: [...(by.get(key)?.members ?? []), one] })
    }
    for (const id of linked) {
      const found = projects.find((candidate) => candidate.id === id)
      if (found && !by.has(id)) by.set(id, { project: found, members: [] })
    }
    return [...by.values()]
  }, [sessions, linked, projects])
  const earlier = threads.filter((thread) => !sessions.some((one) => one.name === thread.name))
  const waiting = sessions.filter((one) => one.waiting).length
  const busy = sessions.filter((one) => one.status === 'busy' && !one.waiting).length
  const historyOf = (name: string): AgentThread | undefined => threads.find((thread) => thread.name === name)

  return (
    <div className="sess">
      <PanelHead
        title="Sessions"
        meta={
          sessions.length > 0 && (
            <span className="sess__sum">
              {waiting > 0 && <b className="sess__wait">{waiting} waiting</b>}
              {busy > 0 && <span>{busy} working</span>}
              <span>{sessions.length} running</span>
            </span>
          )
        }
      >
        <PanelAct label="Read again" onClick={() => (profileId && refreshSessions(profileId), readHistory())}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5" /></svg>
        </PanelAct>
      </PanelHead>

      {profileId && <ReplyDrafts profileId={profileId} sessions={sessions} onDone={() => refreshSessions(profileId)} />}

      {sessions.length === 0 && (
        <PanelEmpty
          title="No session running"
          hint={
            groups.length > 0
              ? 'Start one from a project below — a terminal or a chat — or ask the orchestrator to start one on a card.'
              : 'Ask the orchestrator to start one on a card, or open Claude Code in a project’s terminal.'
          }
        />
      )}

      {groups.map(({ project: where, members }) => (
        <section className="sess__group" key={where?.id ?? 'outside'}>
          <div className="sess__gh">
            <h3 className="sess__g">{where?.name ?? members[0]?.projectName ?? 'Outside devpit'}</h3>
            {where && (
              <>
                <button className="sess__icon" onClick={() => newTerminal(where)} title={`New terminal in ${where.name}, opened here`} aria-label={`New terminal in ${where.name}`}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m5 8 4 4-4 4M12 16h7" /></svg>
                </button>
                <button className="sess__icon" onClick={() => newChat(where)} title={`New chat in ${where.name}`} aria-label={`New chat in ${where.name}`}>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 11.5a8.4 8.4 0 0 1-9 8.4 9.9 9.9 0 0 1-4.2-.9L3 20.5l1.6-4.4A8.4 8.4 0 0 1 3.6 11.5 8.4 8.4 0 0 1 12 3.1a8.4 8.4 0 0 1 9 8.4Z" /></svg>
                </button>
              </>
            )}
          </div>
          {members.length === 0 && sessions.length > 0 && <p className="sess__none">No session running here.</p>}
          {members.map((one) => {
            const state = stateOf(sessions, one.name)
            const history = historyOf(one.name)
            const last = history?.events[history.events.length - 1]
            const expanded = open === rowOf(one)
            return (
              <div className="sess__one" key={rowOf(one)} data-state={state} data-open={expanded ? 'true' : undefined}>
                <div className="sess__row">
                  <button className="sess__main" aria-expanded={expanded} onClick={() => setOpen(expanded ? null : rowOf(one))} title={one.cwd}>
                    <i className="deleg__dot" data-state={state} />
                    <span className="sess__name">{one.name}</span>
                    <span className="sess__state">{STATE_WORDS[state]}</span>
                    <span className="sess__meta">
                      {[one.cardId ? 'card' : null, since(one.since, now)].filter(Boolean).join(' · ')}
                      {/* What it is on right now beats what it last said: a
                          working session's last reply is from the turn before. */}
                      {one.step && state === 'busy'
                        ? ` · ${one.step}`
                        : last && ` · ${ARROW[last.kind as keyof typeof ARROW] ?? ''} ${last.summary ?? last.text}`}
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
                {expanded && (
                  <Details session={one} history={history} profileId={profileId} onWatch={one.job && one.projectId && !one.pane ? () => watch(one) : undefined} />
                )}
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

function Details({
  session,
  history,
  profileId,
  onWatch,
}: {
  session: LiveSession
  history: AgentThread | undefined
  profileId: string | null
  /** A background session, opened in a terminal to be watched and typed into. */
  onWatch?: () => void
}): React.JSX.Element {
  const [reply, setReply] = useState('')
  const [said, setSaid] = useState<string | null>(null)
  /* Stopping asks once, in place: work in flight is lost. */
  const [stopping, setStopping] = useState(false)

  const stop = (): void => {
    if (!profileId) return
    void ask(() => commands.orchestratorStop(profileId, session.name)).then((done) => {
      setSaid(done.error ?? done.data ?? null)
      setStopping(false)
      refreshSessions(profileId)
    })
  }

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
      ) : onWatch ? (
        <p className="sess__note">
          Runs in the background, where nothing shows it.{' '}
          <button className="sess__more-btn" onClick={onWatch}>
            Watch it in a terminal
          </button>
        </p>
      ) : (
        <p className="sess__note">Opened outside devpit: it can be messaged, not typed into from here.</p>
      )}
      {said && <p className="sess__note">{said}</p>}
      <div className="sess__acts">
        {stopping ? (
          <>
            <span className="sess__warn">Stop it and close its terminal? Work in flight is lost.</span>
            <button className="sess__btn" onClick={() => setStopping(false)}>Keep it</button>
            <button className="sess__btn sess__btn--bad" onClick={stop}>Stop</button>
          </>
        ) : (
          <button className="sess__btn" onClick={() => setStopping(true)} aria-label={`Stop ${session.name}`}>
            Stop session…
          </button>
        )}
      </div>
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
