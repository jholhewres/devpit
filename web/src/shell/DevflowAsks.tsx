import { useCallback, useEffect, useState } from 'react'

import type { DevflowAsk, DevflowAsks as Asks } from '../gen/bindings'
import { ask, commands } from './live'
import { onCarried } from './window'

/*
 * Devflow changes only the person makes: switching it on in a project, or
 * moving its Decisions to enforce. An agent asks, or a devflow.yaml says so
 * on its own; the card waits under the top bar, the way a reminder does, and
 * nothing changes until the click.
 */

export function devflowQuestion(one: DevflowAsk): { question: string; why: string; yes: string } {
  switch (one.change) {
    case 'enable':
      return {
        question: `Switch Devflow on in ${one.project}?`,
        why: 'Its agents get the Devflow tools; the click writes enabled: true in its devflow.yaml.',
        yes: 'Switch on',
      }
    case 'enforce':
      return {
        question: `Enforce Decisions in ${one.project}'s Devflow?`,
        why: 'Its gates then block instead of only logging; the click writes mode: enforce.',
        yes: 'Enforce',
      }
    case 'command':
      return {
        question: `Run a team layer's gate command in ${one.project}?`,
        why: `${one.detail ?? ''} — code from outside the project, asked again whenever it or its commit changes.`,
        yes: 'Run it',
      }
    case 'autopilot':
      return {
        question: `Start the autopilot in ${one.project}?`,
        why: `${one.detail ?? ''} — one session per card, each through the gate; it stops for you at a card others wait on.`,
        yes: 'Start',
      }
  }
}

export function DevflowAsks(): React.JSX.Element | null {
  const [asks, setAsks] = useState<readonly DevflowAsk[]>([])
  const [problem, setProblem] = useState<string | null>(null)

  const told = useCallback((answer: { data: Asks | null; error: string | null }): void => {
    if (answer.data) setAsks(answer.data.asks)
    setProblem(answer.error)
  }, [])
  const reload = useCallback(() => void ask(() => commands.devflowAsks()).then(told), [told])

  useEffect(() => {
    reload()
    return onCarried<null>('devflow:asked', reload)
  }, [reload])

  if (asks.length === 0) return null
  const answer = (id: string, yes: boolean): void => void ask(() => commands.devflowAnswer(id, yes)).then(told)

  return (
    <section className="remind" role="alert" aria-label="Devflow">
      <div className="remind__list">
        {asks.map((one) => {
          const said = devflowQuestion(one)
          return (
            <div key={one.id} className="remind__one">
              <span className="remind__what" title={said.why}>
                <b>{said.question}</b>
                <span className="remind__when">{said.why}</span>
              </span>
              <span className="remind__acts">
                <button className="remind__btn" onClick={() => answer(one.id, false)}>
                  Not now
                </button>
                <button className="remind__btn remind__btn--done" onClick={() => answer(one.id, true)}>
                  {said.yes}
                </button>
              </span>
            </div>
          )
        })}
        {problem && <span className="remind__when">{problem}</span>}
      </div>
    </section>
  )
}
