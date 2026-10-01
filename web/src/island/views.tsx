import type { IslandSession, IslandStep, IslandVerdict } from '../gen/bindings'
import { ask, commands } from '../shell/live'
import { Checks } from './Checks'
import { Mascot } from './Mascot'
import { Preview } from './Preview'
import { ROWS } from './shape'
import { askedWords, headline, labelled, nameOf, nowWords, stepWords, type Ask, type Mood } from './sessions'

/*
 * The island's views: the capsule, the list of sessions, one session, and a
 * question.
 *
 * Each is drawn to the size `shape.ts` gives it and no bigger; the outline is
 * animated around them, and a view that grew past it would be cut.
 */

/** Brings devpit forward on the terminal a session runs in. */
export const openPane = (session: IslandSession): void => {
  if (!session.paneId) return
  void ask(() => commands.islandOpenPane(session.projectId, session.paneId!))
}

/** How long ago, in the fewest characters that still say it. */
export function ago(now: number, at: number | null): string {
  const seconds = Math.max(0, Math.round((now - (at ?? now)) / 1000))
  if (seconds < 45) return 'now'
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`
  return `${Math.round(seconds / 3600)}h`
}

const STATE_WORD: Readonly<Record<IslandSession['state'], string>> = {
  open: 'idle',
  working: 'working',
  waiting: 'waiting',
  done: 'done',
  failed: 'failed',
  gone: 'gone',
}

/** What the head says for a session: where it runs, what it is doing, since when. */
export const whereOf = (session: IslandSession, now: number): string =>
  [session.card ? session.project : null, STATE_WORD[session.state], ago(now, session.at)].filter(Boolean).join(' · ')

export function Capsule({
  sessions,
  questions,
  mood,
  now,
}: {
  sessions: readonly IslandSession[]
  questions: readonly Ask[]
  mood: Mood
  now: number
}): React.JSX.Element {
  const top = sessions[0]
  const labels = labelled(sessions)
  const name = questions[0] ? 'Asks to' : top ? (labels.get(top.sessionId) ?? nameOf(top)) : 'devpit'
  const words = questions[0] ? askedWords(questions[0]) : top ? nowWords(top, now) : 'Nothing running'
  const busy = sessions.some((one) => one.state === 'working')
  return (
    <div className="isl-cap">
      <Mascot mood={mood} size={26} />
      <span className="isl-cap__text">
        <span className="isl-cap__name">{name}</span>
        <span className="isl-cap__now" data-moving={mood === 'working' || mood === 'thinking' ? 'true' : undefined}>
          {words}
        </span>
      </span>
      <span className="isl-cap__dots" title={headline(sessions, questions)}>
        {sessions.slice(0, 5).map((one) => (
          <i className="isl-cap__dot" data-state={one.state} key={one.sessionId} />
        ))}
        {sessions.length > 5 && <span className="isl-cap__more">+{sessions.length - 5}</span>}
      </span>
      {busy && <span className="isl-cap__run" />}
    </div>
  )
}

export function Rows({
  sessions,
  now,
  onChoose,
}: {
  sessions: readonly IslandSession[]
  now: number
  onChoose: (session: IslandSession) => void
}): React.JSX.Element {
  const labels = labelled(sessions)
  if (sessions.length === 0) return <div className="isl-rows__none">Sessions you start in devpit show up here.</div>
  return (
    <div className="isl-rows">
      {sessions.slice(0, ROWS).map((session) => (
        <button
          className="isl-row"
          key={session.sessionId}
          data-session={session.sessionId}
          style={{ '--tint': session.color ?? 'var(--accent)' } as React.CSSProperties}
          onClick={() => onChoose(session)}
        >
          <span className="isl-row__bar" />
          <span className="isl-row__main">
            <span className="isl-row__name">{labels.get(session.sessionId) ?? nameOf(session)}</span>
            <span className="isl-row__now">{nowWords(session, now)}</span>
          </span>
          <span className="isl-row__state" data-state={session.state}>
            {STATE_WORD[session.state]}
          </span>
          <span className="isl-row__ago">{ago(now, session.at)}</span>
        </button>
      ))}
    </div>
  )
}

export function Detail({ session, now }: { session: IslandSession; now: number }): React.JSX.Element {
  const steps = session.steps.slice(-6)
  const shown: IslandStep | null = [...session.steps].reverse().find((one) => one.touch) ?? session.steps.at(-1) ?? null
  return (
    <div className="isl-detail" data-session={session.sessionId}>
      <div className="isl-detail__side">
        <ol className="isl-line">
          {steps.length === 0 && (
            <li className="isl-line__step" data-state="idle">
              <span className="isl-line__node" />
              <span className="isl-line__words">{nowWords(session, now)}</span>
            </li>
          )}
          {steps.map((step, at) => (
            <li className="isl-line__step" data-state={step.failed ? 'failed' : step.done ? 'done' : 'running'} key={at} title={stepWords(step)}>
              <span className="isl-line__node" />
              <span className="isl-line__words">{stepWords(step)}</span>
            </li>
          ))}
        </ol>
        <Checks sessionId={session.sessionId} />
        {session.paneId && (
          <button className="isl-btn isl-btn--quiet" onClick={() => openPane(session)}>
            Open terminal
          </button>
        )}
      </div>
      <Preview session={session} step={shown} />
    </div>
  )
}

export function Asking({ question, onAnswer }: { question: Ask; onAnswer: (verdict: IslandVerdict) => void }): React.JSX.Element {
  return (
    <div className="isl-ask">
      <code className="isl-ask__what">{askedWords(question)}</code>
      <div className="isl-ask__row">
        {question.from === 'terminal' ? (
          <button className="isl-btn isl-btn--quiet" onClick={() => onAnswer('in_terminal')} title="Let the terminal ask, as it would without devpit">
            Answer in terminal
          </button>
        ) : (
          <span className="isl-ask__note">Answered here, it reaches the agent as yours.</span>
        )}
        <span className="isl-ask__gap" />
        <button className="isl-btn" onClick={() => onAnswer('deny')}>
          Deny
        </button>
        {question.keepable && (
          <button className="isl-btn" onClick={() => onAnswer('always')} title="Allow, and keep the rule the agent suggested">
            Always
          </button>
        )}
        <button className="isl-btn isl-btn--go" onClick={() => onAnswer('allow')}>
          Allow
        </button>
      </div>
    </div>
  )
}

/** The head of the open island: a face, what it is about, and the two controls. */
export function Head({
  mood,
  title,
  sub,
  color,
  onBack,
  pinned,
  onPin,
  onFold,
  sound,
  onSound,
}: {
  mood: Mood
  title: string
  sub: string
  color?: string | null
  onBack?: () => void
  pinned: boolean
  onPin: () => void
  onFold: () => void
  sound: boolean
  onSound: () => void
}): React.JSX.Element {
  return (
    <div className="isl-head">
      {onBack && (
        <button className="isl-icon" onClick={onBack} aria-label="Every session" title="Every session">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="m15 18-6-6 6-6" /></svg>
        </button>
      )}
      <Mascot mood={mood} size={40} color={color} looks />
      <span className="isl-head__text">
        <span className="isl-head__title">{title}</span>
        <span className="isl-head__sub">{sub}</span>
      </span>
      <button className="isl-icon" onClick={onSound} aria-label={sound ? 'Mute the island' : 'Let it make sounds'} title={sound ? 'Sounds on' : 'Sounds off'}>
        {sound ? (
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M11 5 6 9H3v6h3l5 4zM15.5 8.5a5 5 0 0 1 0 7M18.5 5.5a9 9 0 0 1 0 13" /></svg>
        ) : (
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M11 5 6 9H3v6h3l5 4zM22 9l-6 6M16 9l6 6" /></svg>
        )}
      </button>
      <button
        className="isl-icon"
        data-on={pinned ? 'true' : undefined}
        onClick={onPin}
        aria-label={pinned ? 'Let it fold on its own' : 'Keep it open'}
        title={pinned ? 'Let it fold on its own' : 'Keep it open'}
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M12 17v5M9 3h6l-1 6 3 3v2H7v-2l3-3z" /></svg>
      </button>
      <button className="isl-icon" onClick={onFold} aria-label="Fold" title="Fold">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round"><path d="m6 15 6-6 6 6" /></svg>
      </button>
    </div>
  )
}
