import { useId, useState } from 'react'

import type { Closing } from './useShell'

/*
 * Closing a terminal that is still doing something.
 *
 * Not `Confirm`: that dialog is for things that cannot be undone, and this one
 * has a tick box and two different sentences. The sentences are the point —
 * stopping an agent halfway through a task and stopping a build are not the
 * same loss, and a prompt that words them alike is a prompt people learn to
 * click through without reading.
 *
 * The tick is a decision, not a default. It is off every time the dialog
 * opens, so a habit of pressing Enter cannot silently turn the prompt off for
 * good.
 */

export function StopRunning({
  closing,
  onCancel,
  onConfirm,
}: {
  closing: Closing
  onCancel: () => void
  onConfirm: (dontAskAgain: boolean) => void
}): React.JSX.Element {
  const box = useId()
  const [dontAskAgain, setDontAskAgain] = useState(false)
  const agent = closing.stops.kind === 'agent'

  return (
    <div
      className="ask"
      data-open="true"
      onClick={(event) => event.target === event.currentTarget && onCancel()}
    >
      <div className="ask__box" role="dialog" aria-modal="true">
        <h2 className="ask__t">{agent ? 'Stop this agent?' : 'Stop running command?'}</h2>
        <p className="ask__d">
          {agent
            ? "Closing this terminal will stop the agent's current work."
            : 'Closing this terminal will stop the command running inside it.'}
        </p>
        <p className="ask__subj">
          {closing.tab} · {closing.stops.label}
        </p>
        <label className="ask__opt" htmlFor={box}>
          <input
            id={box}
            type="checkbox"
            checked={dontAskAgain}
            onChange={(event) => setDontAskAgain(event.target.checked)}
          />
          Don&rsquo;t ask again for running terminals
        </label>
        <div className="ask__row">
          <button className="btn" onClick={onCancel}>
            Cancel
          </button>
          <button className="btn btn--danger" autoFocus onClick={() => onConfirm(dontAskAgain)}>
            {agent ? 'Stop Agent' : 'Stop and Close'}
          </button>
        </div>
      </div>
    </div>
  )
}
