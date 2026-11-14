import { useEffect, useState } from 'react'

import type { Diagnostics } from '../gen/bindings'
import { bytes } from './disk'
import { ask, commands } from './live'
import { PrefRow } from './PrefRow'

/*
 * Settings → Diagnostics: what devpit itself is running and costing, the
 * checks `devpit diagnostics` prints in a terminal, and its error log. Read
 * when the pane opens and when asked again, never on a timer.
 */

function percent(tenths: number): string {
  return `${(tenths / 10).toFixed(1)}%`
}

export function DiagnosticsPane(): React.JSX.Element {
  const [shown, setShown] = useState<Diagnostics | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const took = (answer: { data: Diagnostics | null; error: string | null }): void => {
    setBusy(false)
    setError(answer.error)
    if (answer.data) setShown(answer.data)
  }

  useEffect(() => {
    void ask(() => commands.diagnosticsRead()).then(took)
  }, [])

  const look = (): void => {
    setBusy(true)
    void ask(() => commands.diagnosticsRead()).then(took)
  }

  const processes = shown?.processes ?? []
  const cpu = processes.reduce((sum, one) => sum + one.cpuTenths, 0)
  const memory = processes.reduce((sum, one) => sum + (one.memoryBytes ?? 0), 0)
  const totals = shown ? `${processes.length} processes, ${percent(cpu)} of a core, ${bytes(memory) ?? '0 B'} of memory.` : 'Looking…'
  return (
    <>
      <PrefRow title="Processes" said={totals}>
        <button className="btn" disabled={busy} onClick={look}>
          {busy ? 'Looking…' : 'Look again'}
        </button>
      </PrefRow>
      {error && <p className="pref__d">{error}</p>}
      {processes.length > 0 && (
        <table className="diag__t">
          <thead>
            <tr>
              <th>Process</th>
              <th>PID</th>
              <th>CPU</th>
              <th>Memory</th>
            </tr>
          </thead>
          <tbody>
            {processes.map((one) => (
              <tr key={one.pid}>
                <td>{one.name}</td>
                <td>{one.pid}</td>
                <td>{percent(one.cpuTenths)}</td>
                <td>{bytes(one.memoryBytes ?? 0) ?? '—'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      <div className="acc__sub">Checks</div>
      {shown?.checks.map((check) => (
        <div className="pref" key={check.name}>
          <span className="pref__body">
            <span className="pref__t">
              {check.ok ? '✓' : '✗'} {check.name}
            </span>
            <span className="pref__d">
              {check.detail}
              {!check.ok && check.fix ? ` — fix: ${check.fix}` : ''}
            </span>
          </span>
        </div>
      ))}

      <div className="acc__sub">Error log</div>
      <p className="pref__d">{shown?.logPath}</p>
      <pre className="diag__log">{shown && shown.log.length > 0 ? shown.log.join('\n') : 'Nothing logged. Error reports keep this log while they are on.'}</pre>
    </>
  )
}
