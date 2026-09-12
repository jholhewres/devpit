import { useRef, useState } from 'react'

import type { Step } from '../gen/bindings'
import { useAway } from './away'

/*
 * What a lane runs when a card lands in it.
 *
 * The rule the whole product turns on, and until now there was no way to set
 * it: `column_set_step` and `step_create` both existed in the contract and
 * nothing anywhere called them, so every lane ran nothing forever.
 *
 * A step marked irreversible is not fired by a drag — the backend already
 * refuses that, and the badge here says so, because a person choosing a deploy
 * step should learn the rule when they choose it and not when it surprises
 * them.
 */

const KINDS = [
  { id: 'agent', label: 'Agent', hint: 'One headless turn. Returns JSON, takes no terminal.' },
  { id: 'session', label: 'Session', hint: 'A session you drive. Takes the terminal.' },
  { id: 'command', label: 'Command', hint: 'A command of yours: tests, a build, a deploy.' },
] as const

export function LaneStep({
  step,
  steps,
  lanes,
  onPass,
  autonomy,
  onPick,
  onCreate,
  onFlow,
}: {
  step: Step | null
  steps: readonly Step[]
  /** Every other lane on this board, for the one a pass sends a card to. */
  lanes: readonly { id: string; name: string }[]
  onPass: string | null
  autonomy: string
  onPick: (stepId: string | null) => void
  onCreate: (kind: string, name: string, config: string, irreversible: boolean) => void
  onFlow: (onPass: string | null, autonomy: string) => void
}): React.JSX.Element {
  const [open, setOpen] = useState(false)
  const [making, setMaking] = useState(false)
  const box = useRef<HTMLDivElement>(null)
  useAway(box, () => setOpen(false), open)

  return (
    <div className="lstep" ref={box}>
      <button
        className="lstep__b"
        aria-expanded={open}
        title={step ? `Runs ${step.name}` : 'This lane runs nothing'}
        onClick={() => setOpen((was) => !was)}
      >
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 3v3M5.6 5.6l2.1 2.1M3 12h3M18 12h3M16.3 7.7l2.1-2.1" />
          <rect x="7" y="12" width="10" height="9" rx="2" />
        </svg>
        {step?.name ?? 'no step'}
        {/* Said on the lane, not hidden in its menu: a lane that moves cards
            without being asked should look different from one that does not. */}
        {step && autonomy === 'auto' && <span className="lstep__auto">auto</span>}
      </button>

      {open && !making && (
        <div className="lstep__pop" role="menu">
          <p className="lstep__t">Runs when a card lands here</p>
          <button
            className="apps__opt"
            role="menuitem"
            aria-checked={step === null}
            onClick={() => {
              onPick(null)
              setOpen(false)
            }}
          >
            Nothing
          </button>
          {steps.map((one) => (
            <button
              className="apps__opt"
              key={one.id}
              role="menuitem"
              aria-checked={step?.id === one.id}
              onClick={() => {
                onPick(one.id)
                setOpen(false)
              }}
            >
              <span>{one.name}</span>
              {one.irreversible && <span className="lstep__warn">no undo</span>}
            </button>
          ))}
          <div className="newmenu__rule" />
          <button className="apps__opt" role="menuitem" onClick={() => setMaking(true)}>
            New step&hellip;
          </button>

          {/* Only once the lane runs something. Where a pass goes and how much
              the lane decides are answers about a step, and a lane with no
              step has nothing to decide about. */}
          {step && (
            <>
              <div className="newmenu__rule" />
              <p className="lstep__t">When it passes</p>
              <button
                className="apps__opt"
                role="menuitem"
                aria-checked={!onPass}
                onClick={() => onFlow(null, 'manual')}
              >
                Stay here
              </button>
              {lanes.map((one) => (
                <button
                  className="apps__opt"
                  key={one.id}
                  role="menuitem"
                  aria-checked={onPass === one.id}
                  onClick={() => onFlow(one.id, autonomy === 'manual' ? 'ask' : autonomy)}
                >
                  Move to {one.name}
                </button>
              ))}

              {onPass && (
                <>
                  <div className="newmenu__rule" />
                  <p className="lstep__t">And moves it</p>
                  <button
                    className="apps__opt"
                    role="menuitem"
                    aria-checked={autonomy === 'ask'}
                    onClick={() => onFlow(onPass, 'ask')}
                  >
                    <span>When you say so</span>
                  </button>
                  <button
                    className="apps__opt"
                    role="menuitem"
                    aria-checked={autonomy === 'auto'}
                    onClick={() => onFlow(onPass, 'auto')}
                  >
                    <span>On its own</span>
                    <span className="lstep__warn">no prompt</span>
                  </button>
                </>
              )}
            </>
          )}
        </div>
      )}

      {open && making && (
        <Making
          onCancel={() => setMaking(false)}
          onDone={(kind, name, config, irreversible) => {
            onCreate(kind, name, config, irreversible)
            setMaking(false)
            setOpen(false)
          }}
        />
      )}
    </div>
  )
}

function Making({
  onDone,
  onCancel,
}: {
  onDone: (kind: string, name: string, config: string, irreversible: boolean) => void
  onCancel: () => void
}): React.JSX.Element {
  const [kind, setKind] = useState<string>('command')
  const [name, setName] = useState('')
  const [config, setConfig] = useState('')
  const [irreversible, setIrreversible] = useState(false)
  const chosen = KINDS.find((one) => one.id === kind)

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

      <label className="fld">
        <span className="fld__l">{kind === 'command' ? 'Command' : 'Agent'}</span>
        <input
          className="fld__b"
          value={config}
          spellCheck={false}
          placeholder={kind === 'command' ? 'make test' : 'reviewer'}
          onChange={(event) => setConfig(event.target.value)}
        />
      </label>

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
          disabled={!name.trim() || !config.trim()}
          onClick={() => onDone(kind, name.trim(), config.trim(), irreversible)}
        >
          Create
        </button>
      </div>
    </div>
  )
}
