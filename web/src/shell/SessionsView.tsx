import { useCallback, useEffect, useMemo, useState } from 'react'

import type { AgentThread, LiveSession, Project } from '../gen/bindings'
import { ask, commands } from './live'
import { refreshSessions, STATE_WORDS, useLiveSessions } from './liveStatus'
import { LINKS_CHANGED } from './projectProposal'
import { panelGroups, stateOfOne, useCardTitles } from './sessionsPanel'
import { ReplyDrafts } from './ReplyDrafts'
import { SessionTerminal } from './SessionTerminal'
import { opened } from './strip'
import { remember, remembered } from './tabs'
import { PanelAct, PanelEmpty, PanelHead } from './PanelHead'
import { useShell } from './useShell'
import { SessionCost } from './SessionCost'

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

/* Whether linked projects with nothing running get a group: a choice kept
   per window, and off until asked for. */
const EMPTY_KEY = 'devpit.sessions.showEmpty'
const keptEmpty = (): boolean => {
  try {
    return localStorage.getItem(EMPTY_KEY) === '1'
  } catch {
    return false
  }
}
const keepEmpty = (on: boolean): void => {
  try {
    localStorage.setItem(EMPTY_KEY, on ? '1' : '0')
  } catch {
    /* Not kept: the panel still works, it forgets on reload. */
  }
}

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
  const [query, setQuery] = useState('')
  const [showEmpty, setShowEmpty] = useState(keptEmpty)
  /* The row whose Stop is being confirmed, in place. */
  const [stopping, setStopping] = useState<string | null>(null)
  const [said, setSaid] = useState<string | null>(null)
  const now = useNow(shown)
  const cardTitles = useCardTitles(sessions, shown)

  const readHistory = useCallback(() => {
    if (!project?.orchestrator) return
    void ask(() => commands.orchestratorAgents(project.id)).then((answer) => setThreads(answer.data ?? []))
  }, [project])
  const readLinks = useCallback(() => {
    if (project) void ask(() => commands.orchestratorLinks(project.id)).then((answer) => setLinked(answer.data ?? []))
  }, [project])
  useEffect(() => {
    if (!shown) return
    readHistory()
    readLinks()
  }, [shown, readHistory, readLinks])
  /* Linked from the Boards panel or a proposal: this panel's groups move too. */
  useEffect(() => {
    window.addEventListener(LINKS_CHANGED, readLinks)
    return () => window.removeEventListener(LINKS_CHANGED, readLinks)
  }, [readLinks])

  /* A project with a session the orchestrator cannot reach yet, linked in
     one click: its tools act only on linked projects. */
  const link = (id: string): void => {
    if (!project) return
    void ask(() => commands.orchestratorLink(project.id, [...new Set([...linked, id])])).then((answer) => {
      setSaid(answer.error)
      if (answer.error) return
      window.dispatchEvent(new CustomEvent(LINKS_CHANGED))
    })
  }

  const stop = (one: LiveSession): void => {
    if (!profileId) return
    void ask(() => commands.orchestratorStop(profileId, one.name)).then((done) => {
      setSaid(done.error ?? done.data ?? null)
      setStopping(null)
      refreshSessions(profileId)
    })
  }

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

  const groups = useMemo(
    () => panelGroups(sessions, linked, projects, query, showEmpty, cardTitles),
    [sessions, linked, projects, query, showEmpty, cardTitles],
  )
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
        <PanelAct
          label={showEmpty ? 'Hide projects with nothing running' : 'Show linked projects with nothing running'}
          active={showEmpty}
          onClick={() => setShowEmpty((was) => (keepEmpty(!was), !was))}
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M3 7h18M3 12h18M3 17h10" /></svg>
        </PanelAct>
        <PanelAct label="Read again" onClick={() => (profileId && refreshSessions(profileId), readHistory())}>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-3-6.7L21 8M21 3v5h-5" /></svg>
        </PanelAct>
      </PanelHead>

      {profileId && <ReplyDrafts profileId={profileId} sessions={sessions} onDone={() => refreshSessions(profileId)} />}

      {sessions.length > 3 && (
        <input className="sess__find" type="search" value={query} placeholder="Find a session, project or card" aria-label="Find a session" onChange={(event) => setQuery(event.target.value)} />
      )}
      {said && <p className="sess__note sess__said">{said}</p>}

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

      {sessions.length > 0 && groups.length === 0 && <p className="sess__note">No session matches “{query}”.</p>}

      {groups.map(({ project: where, members, linked: reached }) => (
        <section className="sess__group" key={where?.id ?? 'outside'}>
          <div className="sess__gh">
            <h3 className="sess__g">{where?.name ?? members[0]?.projectName ?? 'Outside devpit'}</h3>
            {where && !reached && (
              <button className="sess__link" onClick={() => link(where.id)} title={`Link ${where.name} to this orchestrator: its tools reach only linked projects`}>
                Link
              </button>
            )}
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
          {members.length === 0 && <p className="sess__none">Nothing running here.</p>}
          {members.map((one) => {
            const state = stateOfOne(one)
            const card = one.cardId ? cardTitles.get(one.cardId) ?? 'a card' : null
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
                      {[card, since(one.since, now)].filter(Boolean).join(' · ')}
                      {/* What it is on right now beats what it last said: a
                          working session's last reply is from the turn before. */}
                      {one.step && state === 'busy'
                        ? ` · ${one.step}`
                        : last && ` · ${ARROW[last.kind as keyof typeof ARROW] ?? ''} ${last.summary ?? last.text}`}
                    </span>
                  </button>
                  <SessionCost sessionId={one.sessionId} />
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
                  <button className="sess__icon sess__icon--stop" onClick={() => setStopping(rowOf(one))} title="Stop it" aria-label={`Stop ${one.name}`}>
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><rect x="6" y="6" width="12" height="12" rx="2" /></svg>
                  </button>
                </div>
                {stopping === rowOf(one) && (
                  <div className="sess__acts sess__stopask">
                    <span className="sess__warn">Stop {one.name} and close its terminal? Work in flight is lost.</span>
                    <button className="sess__btn" onClick={() => setStopping(null)}>Keep it</button>
                    <button className="sess__btn sess__btn--bad" onClick={() => stop(one)}>Stop</button>
                  </div>
                )}
                {expanded && (
                  <Details
                    session={one}
                    history={history}
                    profileId={profileId}
                    onWatch={one.job && one.projectId && !one.pane ? () => watch(one) : undefined}
                    onTerminal={one.pane ? () => setTerminal(one) : undefined}
                  />
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
          profileId={profileId}
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

/** Claude Code's command that makes a session reachable from claude.ai, or
 *  stops it when it already is (2.1.287). */
export const REMOTE_CONTROL = '/remote-control'

function Details({
  session,
  history,
  profileId,
  onWatch,
  onTerminal,
}: {
  session: LiveSession
  history: AgentThread | undefined
  profileId: string | null
  /** A background session, opened in a terminal to be watched and typed into. */
  onWatch?: () => void
  /** Its own terminal, opened over the panel. */
  onTerminal?: () => void
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

  /* Remote Control is the session's own command, so it is typed there as the
     person; its page and code appear in that terminal, which opens to show
     them. The orchestrator cannot do this: it types into nobody's terminal. */
  const remote = (): void => {
    if (!profileId) return
    void ask(() => commands.orchestratorReply(profileId, session.name, REMOTE_CONTROL)).then((sent) => {
      setSaid(sent.error ?? `Typed ${REMOTE_CONTROL} — its terminal shows the link, or that it disconnected.`)
      if (!sent.error) onTerminal?.()
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
          <>
            {session.pane && !session.waiting && (
              <button className="sess__btn" onClick={remote} title={`Types ${REMOTE_CONTROL} in its terminal, as you: on, or off when it already is`}>
                Remote Control
              </button>
            )}
            <button className="sess__btn" onClick={() => setStopping(true)} aria-label={`Stop ${session.name}`}>
              Stop session…
            </button>
          </>
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
