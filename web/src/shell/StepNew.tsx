import { useEffect, useState } from 'react'

import type { Profile } from '../gen/bindings'
import { ask, commands } from './live'
import { stepConfig, type Fields } from './stepConfig'

/*
 * The form that makes a step.
 *
 * One field per kind, because the kinds share nothing: a command has a command
 * and a patience, a session has a model and an account to run it under. Until
 * this form asked for them, every step made here was saved as the one line the
 * person typed, which is not JSON, which is not what any runner reads.
 */

const KINDS = [
  { id: 'agent', label: 'Agent', hint: 'One headless turn. Returns JSON, takes no terminal.' },
  { id: 'session', label: 'Session', hint: 'A session you drive. Takes the terminal.' },
  { id: 'command', label: 'Command', hint: 'A command of yours: tests, a build, a deploy.' },
] as const

type Field = {
  key: string
  label: string
  placeholder: string
  /** A step without it would not run, so Create stays out of reach. */
  needed?: boolean
}

const FIELDS: Readonly<Record<string, readonly Field[]>> = {
  command: [
    { key: 'command', label: 'Command', placeholder: 'make test', needed: true },
    { key: 'timeoutSeconds', label: 'Give up after (seconds)', placeholder: 'as long as it takes' },
  ],
  session: [{ key: 'model', label: 'Model', placeholder: 'whatever the account defaults to' }],
  agent: [{ key: 'agent', label: 'Agent', placeholder: 'reviewer', needed: true }],
}

export function StepNew({
  onDone,
  onCancel,
}: {
  onDone: (kind: string, name: string, config: string, irreversible: boolean) => void
  onCancel: () => void
}): React.JSX.Element {
  const [kind, setKind] = useState<string>('command')
  const [name, setName] = useState('')
  const [fields, setFields] = useState<Fields>({})
  const [profiles, setProfiles] = useState<readonly Profile[]>([])
  const [irreversible, setIrreversible] = useState(false)
  const chosen = KINDS.find((one) => one.id === kind)
  const asked = FIELDS[kind] ?? []

  // Only the accounts, and only when a session is being made: the list is a
  // question about which CLI runs it, and the other kinds do not ask it yet.
  useEffect(() => {
    if (kind !== 'session') return
    void ask(() => commands.agentProfiles()).then((answer) => setProfiles(answer.data ?? []))
  }, [kind])

  const typed = (key: string, value: string): void =>
    setFields((was) => ({ ...was, [key]: value }))
  const missing = asked.some((one) => one.needed && !(fields[one.key] ?? '').trim())

  return (
    <div className="lstep__pop lstep__pop--wide">
      <p className="lstep__t">A new step</p>

      <div className="src__sw">
        {KINDS.map((one) => (
          <button
            className="src__o"
            key={one.id}
            aria-checked={kind === one.id}
            onClick={() => setKind(one.id)}
          >
            {one.label}
          </button>
        ))}
      </div>
      <p className="pref__d">{chosen?.hint}</p>

      <label className="fld">
        <span className="fld__l">Name</span>
        <input
          className="fld__b"
          autoFocus
          value={name}
          placeholder="tests"
          onChange={(event) => setName(event.target.value)}
        />
      </label>

      {asked.map((one) => (
        <label className="fld" key={one.key}>
          <span className="fld__l">{one.label}</span>
          <input
            className="fld__b"
            value={fields[one.key] ?? ''}
            spellCheck={false}
            placeholder={one.placeholder}
            onChange={(event) => typed(one.key, event.target.value)}
          />
        </label>
      ))}

      {kind === 'session' && (
        <label className="fld">
          <span className="fld__l">Account</span>
          <div className="src__sw">
            <button
              className="src__o"
              aria-checked={!fields.profile}
              onClick={() => typed('profile', '')}
            >
              Default
            </button>
            {profiles.map((one) => (
              <button
                className="src__o"
                key={one.id}
                aria-checked={fields.profile === one.id}
                onClick={() => typed('profile', one.id)}
              >
                {one.label}
              </button>
            ))}
          </div>
        </label>
      )}

      {/* Stated where it is chosen, not where it bites. A deploy has no undo,
          so it is confirmed rather than fired by dropping a card on a lane. */}
      <button
        className="ask__opt"
        role="checkbox"
        aria-checked={irreversible}
        onClick={() => setIrreversible((was) => !was)}
      >
        <span className="box">
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
            <path d="M20 6 9 17l-5-5" />
          </svg>
        </span>
        <span>
          <span className="ask__ot">This step has no undo</span>
          <span className="ask__od">It is never started by a drag alone &mdash; you confirm it.</span>
        </span>
      </button>

      <div className="ask__row">
        <button className="btn" onClick={onCancel}>Cancel</button>
        <button
          className="btn btn--go"
          disabled={!name.trim() || missing}
          onClick={() => onDone(kind, name.trim(), stepConfig(kind, fields), irreversible)}
        >
          Create
        </button>
      </div>
    </div>
  )
}
