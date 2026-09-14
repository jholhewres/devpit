import { useState } from 'react'

import type { KnownAgent } from '../gen/bindings'
import type { Draft } from './profiles'
import { argsOf, argsText, masked, ready, secret, withVar } from './profiles'
import { committed } from './typing'

/*
 * One profile, being written.
 *
 * Four fields and not a command box, because a command box would be a lie in
 * two directions: the board spawns a process and cannot run shell text, and a
 * person's `glm` is not a command at all — it is a `claude` with seven
 * variables in front of it.
 *
 * The program is optional and shows the base agent's own as its placeholder,
 * so the common case is a name and some variables.
 */

export function ProfileEditor({
  draft,
  agents,
  onChange,
  onSave,
  onCancel,
  onRemove,
}: {
  draft: Draft
  agents: readonly KnownAgent[]
  onChange: (next: Draft) => void
  onSave: () => void
  onCancel: () => void
  /** Absent for an agent devpit merely noticed: there is nothing of somebody's
   *  to remove, only an override to clear by emptying the fields. */
  onRemove?: () => void
}): React.JSX.Element {
  /* By name and not by row. Indexed by position, removing a row above a
     revealed one slid the reveal onto whatever took its place — a different
     token on screen that nobody asked to see. */
  const [shown, setShown] = useState<string | null>(null)
  const base = agents.find((one) => one.id === draft.base)

  const set = (over: Partial<Draft>): void => onChange({ ...draft, ...over })

  return (
    <div className="oapp oapp--new">
      <label className="fld">
        <span className="fld__l">Name</span>
        <input
          className="fld__b"
          autoFocus
          value={draft.label}
          placeholder="GLM"
          aria-label="Name"
          onChange={(event) => set({ label: event.target.value })}
          onKeyDown={(event) => committed(event) && ready(draft) && onSave()}
        />
      </label>

      <label className="fld">
        <span className="fld__l">Agent</span>
        <select
          className="fld__b"
          value={draft.base}
          aria-label="Agent"
          onChange={(event) => set({ base: event.target.value })}
        >
          {agents.map((one) => (
            <option key={one.id} value={one.id}>{one.label}</option>
          ))}
        </select>
      </label>

      <label className="fld">
        <span className="fld__l">Program</span>
        <input
          className="fld__b"
          value={draft.command}
          spellCheck={false}
          aria-label="Program"
          placeholder={base?.launch ?? ''}
          onChange={(event) => set({ command: event.target.value })}
        />
      </label>

      <label className="fld">
        <span className="fld__l">Arguments</span>
        <input
          className="fld__b"
          value={argsText(draft.args)}
          spellCheck={false}
          aria-label="Arguments"
          placeholder="--permission-mode bypassPermissions"
          onChange={(event) => set({ args: argsOf(event.target.value) })}
        />
      </label>

      <span className="fld__l">Environment</span>
      {draft.env.map((one, at) => (
        <div className="oapp" key={at}>
          <input
            className="fld__b"
            value={one.name}
            spellCheck={false}
            aria-label="Variable"
            placeholder="ANTHROPIC_BASE_URL"
            onChange={(event) =>
              set({ env: withVar(draft.env, at, { ...one, name: event.target.value }) })
            }
          />
          {/* Hidden by default only where the name says it is a secret: a
              masked base URL is a field nobody can check at a glance. */}
          {secret(one.name) && shown !== one.name ? (
            <button
              className="fld__b"
              aria-label={`Show ${one.name}`}
              onClick={() => setShown(one.name)}
            >
              {masked(one.value)}
            </button>
          ) : (
            <input
              className="fld__b"
              value={one.value}
              spellCheck={false}
              aria-label={one.name || 'Value'}
              onChange={(event) =>
                set({ env: withVar(draft.env, at, { ...one, value: event.target.value }) })
              }
            />
          )}
          <button
            className="btn"
            data-danger
            aria-label={`Remove ${one.name || 'variable'}`}
            onClick={() => set({ env: withVar(draft.env, at, null) })}
          >
            Remove
          </button>
        </div>
      ))}
      <button
        className="btn"
        onClick={() => set({ env: [...draft.env, { name: '', value: '' }] })}
      >
        Add variable
      </button>

      <div className="ask__row">
        {onRemove && (
          <button className="btn btn--danger" onClick={onRemove}>
            Remove
          </button>
        )}
        <button className="btn" onClick={onCancel}>Cancel</button>
        <button className="btn btn--go" disabled={!ready(draft)} onClick={onSave}>
          Save
        </button>
      </div>
      <p className="pref__d">
        The program runs with these variables set and these arguments added. Leave Program empty
        to use {base?.launch ?? 'the agent'}&rsquo;s own.
      </p>
    </div>
  )
}
