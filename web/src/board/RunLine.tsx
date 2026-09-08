import type { Run } from '../gen/bindings'

/**
 * One run on a card: what ran, how it ended, and what it cost.
 */
export function RunLine({ run }: { run: Run }): React.JSX.Element {
  return (
    <li className={`run run--${run.state}`}>
      <span className="run__step">{run.stepName}</span>
      <span className="run__state">{run.state}</span>
      {run.costUsd !== null && <span className="run__cost">${run.costUsd.toFixed(4)}</span>}
      {run.durationMs !== null && (
        <span className="run__time">{(run.durationMs / 1000).toFixed(1)}s</span>
      )}
      {run.output !== null && <div className="run__output">{run.output}</div>}
    </li>
  )
}
