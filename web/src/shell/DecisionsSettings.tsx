import { useEffect, useState } from 'react'

import type { Deciding } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefMore } from './PrefRow'
import { PrefSwitch } from './PrefSwitch'
import { useShell } from './useShell'

/*
 * Decisions: typed questions answered by Jev, a model that writes no text.
 * Off until a key is kept here; without one every gate does what it did
 * before, so nothing here is required.
 */

const PROVIDERS: readonly { id: string; label: string; url: string; model: string }[] = [
  { id: 'openrouter', label: 'OpenRouter', url: 'https://openrouter.ai/api/v1', model: 'typesafe/jev-1.13' },
  { id: 'vercel', label: 'Vercel AI Gateway', url: 'https://ai-gateway.vercel.sh/v1', model: 'typesafe-ai/jev' },
]

const dollars = (value: number): string => `$${value.toFixed(value < 0.01 && value > 0 ? 4 : 2)}`

export function DecisionsSettings(): React.JSX.Element {
  const { project } = useShell()
  const [now, setNow] = useState<Deciding | null>(null)
  const [key, setKey] = useState('')
  const [said, setSaid] = useState<string | null>(null)
  const [tried, setTried] = useState<string | null>(null)
  const [testing, setTesting] = useState(false)

  useEffect(() => {
    void ask(() => commands.decisionsRead()).then((answer) => setNow(answer.data))
  }, [])

  const save = (next: Deciding): void => {
    setNow(next)
    void ask(() => commands.decisionsSet(next.provider, next.model, next.url, next.dailyCapUsd)).then((answer) => {
      setSaid(answer.error)
      if (answer.data) setNow(answer.data)
    })
  }

  const keep = (value: string | null): void => {
    void ask(() => commands.decisionsKeySet(value)).then((answer) => {
      setSaid(answer.error)
      if (answer.error || !now) return
      setKey('')
      setNow({ ...now, keySet: value !== null })
    })
  }

  const send = (projectId: string, yes: boolean): void => {
    void ask(() => commands.decisionsProjectSet(projectId, yes)).then((answer) => {
      setSaid(answer.error)
      if (answer.data) setNow(answer.data)
    })
  }

  /* Through the same path a gate takes, so it also counts toward today. */
  const test = (): void => {
    setTesting(true)
    setTried(null)
    void ask(() => commands.decisionsTest()).then((answer) => {
      setTesting(false)
      const got = answer.data
      if (!got) return setTried(answer.error ?? 'no answer')
      if (got.note) return setTried(got.note)
      setTried(`Answered in ${got.latencyMs ?? 0} ms for ${dollars(got.costUsd ?? 0)}${got.probability == null ? '' : ` — p ${got.probability.toFixed(2)}`}.`)
      void ask(() => commands.decisionsRead()).then((again) => again.data && setNow(again.data))
    })
  }

  if (!now) return <></>
  const provider = PROVIDERS.find((one) => one.id === now.provider) ?? PROVIDERS[0]
  const sent = project ? !now.optedOut.includes(project.id) : true
  /* A float crosses the contract as number | null: NaN has no JSON. */
  const cap = now.dailyCapUsd ?? 1
  return (
    <>
      <div className="pref pref--stack">
        <div className="pref__body">
          <span className="pref__t">Decisions</span>
          <span className="pref__d">Yes/no, a choice or a score, each with a calibrated confidence — asked by devpit and its agents, answered by Jev. {now.keySet ? 'On.' : 'Off until a key is kept.'}</span>
          <PrefMore>
            Every state passes through devpit&rsquo;s redaction first, every request asks for zero retention and no training, and the day&rsquo;s cap stops it. Without a key, the network or budget, everything works as it does without it. Agents reach it as <code>devpit_decide</code> and <code>devpit_rubric_run</code>.
          </PrefMore>
          <span className="voice__row" role="radiogroup" aria-label="Provider">
            {PROVIDERS.map((one) => (
              <button key={one.id} className="btn" data-on={now.provider === one.id ? 'true' : undefined} role="radio" aria-checked={now.provider === one.id} onClick={() => save({ ...now, provider: one.id })}>
                {one.label}
              </button>
            ))}
          </span>
          <label className="voice__row">
            <span className="voice__l">Address</span>
            <input key={`url-${provider.id}`} className="voice__in" placeholder={provider.url} defaultValue={now.url} onBlur={(event) => event.target.value !== now.url && save({ ...now, url: event.target.value })} />
          </label>
          <label className="voice__row">
            <span className="voice__l">Model</span>
            <input key={`model-${provider.id}`} className="voice__in" placeholder={provider.model} defaultValue={now.model} onBlur={(event) => event.target.value !== now.model && save({ ...now, model: event.target.value })} />
          </label>
          <label className="voice__row">
            <span className="voice__l">Daily cap</span>
            <input
              className="voice__in voice__in--short"
              type="number"
              min="0"
              step="0.25"
              aria-label="Daily cap in dollars"
              defaultValue={cap}
              onBlur={(event) => {
                const next = Number(event.target.value)
                if (Number.isFinite(next) && next !== cap) save({ ...now, dailyCapUsd: next })
              }}
            />
          </label>
          <span className="voice__row">
            <span className="voice__l">Key</span>
            <input className="voice__in" type="password" autoComplete="off" placeholder={now.keySet ? 'Kept — type to replace' : `Paste your ${provider.label} key`} value={key} onChange={(event) => setKey(event.target.value)} />
            <button className="btn" disabled={!key.trim()} onClick={() => keep(key)}>
              Keep
            </button>
            {now.keySet && (
              <button className="btn" onClick={() => keep(null)}>
                Forget
              </button>
            )}
          </span>
          <span className="voice__row">
            <button className="btn" disabled={testing || !now.keySet} onClick={test}>
              {testing ? 'Asking…' : 'Test'}
            </button>
            {tried && <span className="pref__d">{tried}</span>}
          </span>
          <span className="pref__d">
            Today: {dollars(now.spentTodayUsd ?? 0)} of {dollars(cap)}, {now.decidedToday} {now.decidedToday === 1 ? 'decision' : 'decisions'}.
          </span>
          {said && <span className="pref__d voice__bad">{said}</span>}
        </div>
      </div>
      {project && (
        <PrefSwitch
          on={sent}
          onFlip={() => send(project.id, !sent)}
          title={`Send ${project.name}'s state`}
          said="Off keeps this project's cards, diffs and output from ever reaching Decisions; its gates then do what they did without it."
        />
      )}
    </>
  )
}
