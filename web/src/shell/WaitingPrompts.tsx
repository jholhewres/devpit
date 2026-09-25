import { useState } from 'react'

import type { LiveSession, PendingPrompt } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The questions this account's sessions are stopped on, answered from here.
 *
 * A pick is pressed in the session's own terminal — arrows to it, then Enter —
 * so it is the person answering, which a message from the orchestrator can
 * never be. The question shown goes back with the pick, and nothing is pressed
 * unless the screen still shows exactly it: a question that moved on is not
 * answered by a click meant for the one before it.
 */

type Waiting = LiveSession & { waiting: PendingPrompt }

export function WaitingPrompts({
  profileId,
  sessions,
  onAnswered,
}: {
  profileId: string
  sessions: readonly LiveSession[]
  onAnswered: () => void
}): React.JSX.Element | null {
  // Per session: the pick in flight, so a second click cannot land on the
  // question after this one, and what the last pick was told.
  const [sending, setSending] = useState<ReadonlySet<string>>(new Set())
  const [said, setSaid] = useState<Readonly<Record<string, string>>>({})
  const waiting = sessions.filter((one): one is Waiting => one.waiting !== null)
  if (waiting.length === 0) return null

  const answer = (one: Waiting, choice: number | null): void => {
    setSending((was) => new Set(was).add(one.name))
    void ask(() => commands.orchestratorAnswer(profileId, one.name, one.waiting, choice)).then((sent) => {
      setSaid((was) => ({ ...was, [one.name]: sent.error ?? '' }))
      setSending((was) => {
        const now = new Set(was)
        now.delete(one.name)
        return now
      })
      onAnswered()
    })
  }

  return (
    <section className="wprompt" aria-label="Waiting on you">
      <p className="wprompt__t">Waiting on you</p>
      {waiting.map((one) => {
        const busy = sending.has(one.name)
        return (
          <div className="wprompt__i" key={one.name}>
            <p className="wprompt__who">
              <span className="osess__name">{one.name}</span>
              <span className="osess__where">{one.projectName ?? 'outside devpit'}</span>
            </p>
            <p className="wprompt__q">{one.waiting.question}</p>
            <div className="wprompt__opts">
              {one.waiting.options.map((option, at) => (
                <button key={at} className="wprompt__o" disabled={busy} title={option.hint ?? undefined} onClick={() => answer(one, at)}>
                  <span className="wprompt__n">{at + 1}</span>
                  {option.label}
                </button>
              ))}
              <button className="wprompt__o wprompt__esc" disabled={busy} onClick={() => answer(one, null)} title="Dismiss the question (Esc)">
                Esc
              </button>
            </div>
            {said[one.name] && <p className="osess__said">{said[one.name]}</p>}
          </div>
        )
      })}
    </section>
  )
}
