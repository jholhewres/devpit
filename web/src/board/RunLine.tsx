import { commands } from '../gen/bindings'
import type { Run } from '../gen/bindings'

/**
 * One run on a card: what ran, how it ended, and what it cost.
 */
export function RunLine({
  run,
  cardId,
  onProblem,
  onStopped
}: {
  run: Run
  cardId: string
  onProblem: (message: string) => void
  onStopped: () => void
}): React.JSX.Element {
  /**
   * Stops a run that is still going.
   *
   * The row lands on `cancelled`, not `failed`: a person stopping work is not
   * the work going wrong, and a board that cannot tell them apart teaches you
   * to distrust every red row on it.
   */
  const stop = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const answer = await commands.runCancel(cardId, run.id)
    if (answer.status === 'error') onProblem(answer.error.message)
    else onStopped()
  }

  return (
    <li className={`run run--${run.state}`}>
      <span className="run__step">{run.stepName}</span>
      <span className="run__state">{run.state}</span>
      {run.state === 'running' && (
        <button type="button" className="run__stop" onClick={(event) => void stop(event)}>
          stop
        </button>
      )}
      {run.costUsd !== null && <span className="run__cost">${run.costUsd.toFixed(4)}</span>}
      {run.durationMs !== null && (
        <span className="run__time">{(run.durationMs / 1000).toFixed(1)}s</span>
      )}
      {run.output !== null && <div className="run__output">{run.output}</div>}
    </li>
  )
}
