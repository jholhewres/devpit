import { useState } from 'react'

import type { DevflowCheck } from '../gen/bindings'
import { checkOf, REPORTS, type Edit } from './devflowEdit'

/*
 * The execution stage's checks: what devpit runs for a card's evidence. A
 * field is sent when it is left, not on every key, so the file is written
 * once per change rather than once per letter.
 */

function CheckRow({ check, onSave, onRemove }: { check: DevflowCheck; onSave: (next: DevflowCheck) => void; onRemove: () => void }): React.JSX.Element {
  const [name, setName] = useState(check.name)
  const [run, setRun] = useState(check.run)
  const leave = (): void => {
    if (name.trim() !== check.name || run.trim() !== check.run) onSave({ ...check, name: name.trim(), run: run.trim() })
  }
  return (
    <div className="pref__body">
      <div className="voice__row">
        <input className="voice__in voice__in--short" aria-label="Check name" value={name} onChange={(event) => setName(event.target.value)} onBlur={leave} />
        <input className="voice__in" aria-label={`${check.name} command`} value={run} onChange={(event) => setRun(event.target.value)} onBlur={leave} />
        <select aria-label={`${check.name} report`} value={check.report} onChange={(event) => onSave({ ...check, report: event.target.value })}>
          {REPORTS.map((report) => (
            <option key={report} value={report}>
              {report}
            </option>
          ))}
        </select>
        <button className="btn" onClick={onRemove}>
          Remove
        </button>
      </div>
      {check.source && <span className="pref__d">Found in {check.source}</span>}
    </div>
  )
}

export function DevflowChecks({ checks, edit }: { checks: readonly DevflowCheck[]; edit: Edit }): React.JSX.Element {
  const [name, setName] = useState('')
  const [run, setRun] = useState('')
  const save = (next: readonly DevflowCheck[]): void => edit('stages.execution.checks', next.map(checkOf))
  const add = (): void => {
    save([...checks, { name: name.trim(), run: run.trim(), report: 'none', evidence: [], source: null }])
    setName('')
    setRun('')
  }
  return (
    <>
      {checks.length === 0 && <span className="pref__d">No checks: nothing proves a card done until one is added.</span>}
      {checks.map((check, at) => (
        <CheckRow
          key={`${check.name}\u0000${check.run}\u0000${check.report}`}
          check={check}
          onSave={(next) => save(checks.map((one, here) => (here === at ? next : one)))}
          onRemove={() => save(checks.filter((_, here) => here !== at))}
        />
      ))}
      <div className="voice__row">
        <input className="voice__in voice__in--short" aria-label="New check name" placeholder="name" value={name} onChange={(event) => setName(event.target.value)} />
        <input className="voice__in" aria-label="New check command" placeholder="command, run from the project's root" value={run} onChange={(event) => setRun(event.target.value)} />
        <button className="btn" disabled={!name.trim() || !run.trim()} onClick={add}>
          Add check
        </button>
      </div>
    </>
  )
}
