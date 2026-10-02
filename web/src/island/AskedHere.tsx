import { useEffect, useState } from 'react'

import type { IslandAsked, IslandSession, PendingPrompt } from '../gen/bindings'
import { ask, commands } from '../shell/live'

/*
 * A question a session put to the person, drawn where its step would be.
 *
 * What the hook carried shows at once: the question, its label, its choices.
 * While the session waits, the choices its terminal draws become buttons, and
 * a pick is pressed there — as the person, in their own terminal, and only on
 * the question they see, which the app checks against the screen again.
 */

export function AskedHere({
  session,
  questions,
  waiting,
}: {
  session: IslandSession
  questions: readonly IslandAsked[]
  waiting: boolean
}): React.JSX.Element {
  const [prompt, setPrompt] = useState<PendingPrompt | null>(null)
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)

  /* The screen is read while it waits, so a question answered in the
     terminal stops offering buttons here. */
  useEffect(() => {
    if (!waiting) {
      setPrompt(null)
      return
    }
    let gone = false
    const read = (): void =>
      void ask(() => commands.islandPrompt(session.sessionId)).then((answer) => {
        if (!gone) setPrompt(answer.data ?? null)
      })
    read()
    const every = window.setInterval(read, 2_000)
    return () => {
      gone = true
      window.clearInterval(every)
    }
  }, [waiting, session.sessionId])

  const pick = (seen: PendingPrompt, choice: number | null): void => {
    setBusy(true)
    void ask(() => commands.islandAnswer(session.sessionId, seen, choice)).then((answer) => {
      setBusy(false)
      setSaid(answer.error)
      if (!answer.error) setPrompt(null)
    })
  }

  return (
    <div className="isl-ask">
      {questions.map((one, at) => (
        <div className="isl-ask__q" key={at}>
          {one.header && <span className="isl-ask__h">{one.header}</span>}
          <p className="isl-ask__t">{one.question}</p>
          {/* The choices as asked, until the terminal's own can be pressed. */}
          {!prompt && (
            <ol className="isl-ask__opts">
              {one.options.map((option, n) => (
                <li key={n}>{option}</li>
              ))}
            </ol>
          )}
        </div>
      ))}
      {prompt && (
        <div className="isl-ask__picks" aria-label="Answer in its terminal">
          {prompt.options.map((option, n) => (
            <button key={n} className="isl-ask__pick" disabled={busy} title={option.hint ?? undefined} onClick={() => pick(prompt, n)}>
              <span className="isl-ask__n">{n + 1}</span>
              {option.label}
            </button>
          ))}
          <button className="isl-ask__pick isl-ask__esc" disabled={busy} onClick={() => pick(prompt, null)}>
            Esc
          </button>
        </div>
      )}
      {said && <p className="isl-ask__said">{said}</p>}
    </div>
  )
}
