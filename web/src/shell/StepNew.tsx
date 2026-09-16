import { useEffect, useState } from 'react'

import type { Agent, Profile, Step } from '../gen/bindings'
import { ask, commands } from './live'
import { CONTEXT_KEYS, stepConfig, stepFields, type Fields } from './stepConfig'

/*
 * The form that makes a step.
 *
 * One field per kind, because the kinds share nothing: a command has a command
 * and a patience, a session has a model and an account to run it under. Until
 * this form asked for them, every step made here was saved as the one line the
 * person typed, which is not JSON, which is not what any runner reads.
 *
 * The same form edits one. The kind is not offered then: a command that became
 * an agent would be a different step, and the runs already filed under this
 * one say what it was when they ran.
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
  /** Prose rather than a line: a prompt, or the schema of an answer. */
  prose?: boolean
}

const FIELDS: Readonly<Record<string, readonly Field[]>> = {
  command: [
    { key: 'command', label: 'Command', placeholder: 'make test', needed: true },
    { key: 'timeoutSeconds', label: 'Give up after (seconds)', placeholder: 'as long as it takes' },
  ],
  session: [{ key: 'model', label: 'Model', placeholder: 'whatever the account defaults to' }],
  agent: [
    { key: 'prompt', label: 'Ask', placeholder: 'Review the change', needed: true, prose: true },
    // A step with no ceiling is a bill nobody agreed to, so the backend
    // refuses one. Asked for here rather than discovered on the lane.
    { key: 'capUsd', label: 'Spending cap (USD)', placeholder: '2', needed: true },
    { key: 'model', label: 'Model', placeholder: 'whatever the account defaults to' },
    { key: 'expects', label: 'The answer must match (JSON Schema)', placeholder: 'any answer will do', prose: true },
  ],
}

export function StepNew({
  step,
  onDone,
  onCancel,
}: {
  /** The step being edited, or nothing when one is being made. */
  step?: Step | null
  onDone: (kind: string, name: string, config: string, irreversible: boolean) => void
  onCancel: () => void
}): React.JSX.Element {
  const [kind, setKind] = useState<string>(step?.kind ?? 'command')
  const [name, setName] = useState(step?.name ?? '')
  // A config nobody can read starts the form empty rather than half filled:
  // what is on screen is then what will be saved, all of it.
  const [fields, setFields] = useState<Fields>(
    step ? (stepFields(step.kind, step.config) ?? {}) : {},
  )
  const [profiles, setProfiles] = useState<readonly Profile[]>([])
  const [agents, setAgents] = useState<readonly Agent[]>([])
  const [irreversible, setIrreversible] = useState(step?.irreversible ?? false)
  const chosen = KINDS.find((one) => one.id === kind)
  const asked = FIELDS[kind] ?? []

  // A command runs as itself; the two kinds that spend somebody's account ask
  // which one.
  useEffect(() => {
    if (kind === 'command') return
    void ask(() => commands.agentProfiles()).then((answer) => setProfiles(answer.data ?? []))
  }, [kind])

  // Offered rather than typed: a step stores an agent by the name in its
  // frontmatter, and a free-text field over the files on disk is a typo that
  // fails when the card lands on the lane.
  useEffect(() => {
    if (kind !== 'agent') return
    void ask(() => commands.agentsList()).then((answer) => setAgents(answer.data?.agents ?? []))
  }, [kind])

  const typed = (key: string, value: string): void =>
    setFields((was) => ({ ...was, [key]: value }))
  const missing = asked.some((one) => one.needed && !(fields[one.key] ?? '').trim())
  const injected = (fields.inject ?? '').split(',').map((one) => one.trim())
  const inject = (key: string): void =>
    typed(
      'inject',
      (injected.includes(key) ? injected.filter((one) => one !== key) : [...injected, key])
        .filter(Boolean)
        .join(', '),
    )

  return (
    <div className="lstep__pop lstep__pop--wide">
      <p className="lstep__t">{step ? 'This step' : 'A new step'}</p>

      {!step && (
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
      )}
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

      {kind === 'agent' && (
        <label className="fld">
          <span className="fld__l">Agent</span>
          <input
            className="fld__b"
            list="stepnew-agents"
            value={fields.agent ?? ''}
            spellCheck={false}
            placeholder="whichever the account defaults to"
            onChange={(event) => typed('agent', event.target.value)}
          />
          <datalist id="stepnew-agents">
            {agents.map((one) => (
              <option key={one.name} value={one.name} />
            ))}
          </datalist>
        </label>
      )}

      {asked.map((one) => (
        <label className="fld" key={one.key}>
          <span className="fld__l">{one.label}</span>
          {one.prose ? (
            <textarea
              className="lstep__ta"
              value={fields[one.key] ?? ''}
              spellCheck={false}
              placeholder={one.placeholder}
              onChange={(event) => typed(one.key, event.target.value)}
            />
          ) : (
            <input
              className="fld__b"
              value={fields[one.key] ?? ''}
              spellCheck={false}
              placeholder={one.placeholder}
              onChange={(event) => typed(one.key, event.target.value)}
            />
          )}
        </label>
      ))}

      {kind === 'agent' && (
        <label className="fld">
          <span className="fld__l">Tell it about the card</span>
          {CONTEXT_KEYS.map((key) => (
            <button
              className="ask__opt"
              key={key}
              role="checkbox"
              aria-checked={injected.includes(key)}
              onClick={() => inject(key)}
            >
              <span className="box">
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M20 6 9 17l-5-5" />
                </svg>
              </span>
              <span className="ask__ot">{key}</span>
            </button>
          ))}
        </label>
      )}

      {kind !== 'command' && (
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
          onClick={() => onDone(kind, name.trim(), stepConfig(kind, fields, step?.config), irreversible)}
        >
          {step ? 'Save' : 'Create'}
        </button>
      </div>
    </div>
  )
}
