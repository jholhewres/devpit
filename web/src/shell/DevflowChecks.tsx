import { useState } from 'react'

import type { DevflowCheck } from '../gen/bindings'
import { checkOf, REPORTS, type Edit } from './devflowEdit'

/*
 * The execution stage's checks: what devpit runs for a card's evidence, or,
 * under `release`, once for a release. A
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
    <div className="dfcheck">
      <input className="dfinput" aria-label="Check name" value={name} onChange={(event) => setName(event.target.value)} onBlur={leave} />
      <span className="dfcheck__run">
        <input className="dfinput dfinput--mono" aria-label={`${check.name} command`} value={run} onChange={(event) => setRun(event.target.value)} onBlur={leave} />
        {check.source && <span className="dfcheck__from">found in {check.source}</span>}
      </span>
      <select aria-label={`${check.name} report`} value={check.report} onChange={(event) => onSave({ ...check, report: event.target.value })}>
          {REPORTS.map((report) => (
            <option key={report} value={report}>
              {report}
            </option>
          ))}
      </select>
      <button className="btn btn--quiet btn--sm" data-danger onClick={onRemove}>
        Remove
      </button>
    </div>
  )
}

const EMPTY = {
  checks: 'No checks: nothing proves a card done until one is added.',
  release: 'No release checks: a long suite, such as E2E, runs here once for a release, not for each card.',
}

export function DevflowChecks({
  checks,
  edit,
  list = 'checks',
}: {
  checks: readonly DevflowCheck[]
  edit: Edit
  list?: keyof typeof EMPTY
}): React.JSX.Element {
  const [name, setName] = useState('')
  const [run, setRun] = useState('')
  const noun = list === 'release' ? 'New release check' : 'New check'
  const save = (next: readonly DevflowCheck[]): void => edit(`stages.execution.${list}`, next.map(checkOf))
  const add = (): void => {
    save([...checks, { name: name.trim(), run: run.trim(), report: 'none', evidence: [], source: null }])
    setName('')
    setRun('')
  }
  return (
    <div className="dfchecks">
      <div className="dfcheck dfcheck--head" aria-hidden>
        <span>Name</span>
        <span>Command</span>
        <span>Report</span>
        <span />
      </div>
      {checks.length === 0 && <span className="dfempty">{EMPTY[list]}</span>}
      {checks.map((check, at) => (
        <CheckRow
          key={`${check.name}\u0000${check.run}\u0000${check.report}`}
          check={check}
          onSave={(next) => save(checks.map((one, here) => (here === at ? next : one)))}
          onRemove={() => save(checks.filter((_, here) => here !== at))}
        />
      ))}
      <div className="dfcheck dfcheck--new">
        <input className="dfinput" aria-label={`${noun} name`} placeholder="name" value={name} onChange={(event) => setName(event.target.value)} />
        <input className="dfinput dfinput--mono" aria-label={`${noun} command`} placeholder="command, run from the project's root" value={run} onChange={(event) => setRun(event.target.value)} />
        <span />
        <button className="btn btn--sm" disabled={!name.trim() || !run.trim()} onClick={add}>
          {list === 'release' ? 'Add release check' : 'Add check'}
        </button>
      </div>
    </div>
  )
}
