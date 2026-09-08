import { useState } from 'react'
import { commands } from '../gen/bindings'
import type { Board, Card, Front, Run } from '../gen/bindings'

/**
 * A card, and what it has cost.
 *
 * The cost is shown whenever any run has one. "The agent is doing something"
 * stops being an acceptable answer once the number is on the card.
 */
export function CardTile({
  card,
  projectId,
  onBoard,
  expanded,
  onToggle,
  onDragStart,
  onDragEnd,
  onProblem
}: {
  card: Card
  projectId: string
  onBoard: (board: Board) => void
  expanded: boolean
  onToggle: () => void
  onDragStart: () => void
  onDragEnd: () => void
  onProblem: (message: string) => void
}): React.JSX.Element {
  /**
   * Brings this card's session into the target terminal.
   *
   * One terminal per project: this swaps what is attached to it. The session
   * that was there keeps running detached — it stops taking up the screen, not
   * working.
   */
  const [front, setFront] = useState<Front | null>(null)

  /**
   * What this line of work changed, against where it began.
   *
   * Never against HEAD: that answers a different question, and drifts further
   * from this one every time anyone commits to the base branch.
   */
  const readFront = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const answer = await commands.cardDiff(card.id)
    if (answer.status === 'ok') setFront(answer.data)
    else onProblem(answer.error.message)
  }

  /**
   * Puts the card away, and refuses while its front holds unsaved work.
   *
   * The refusal comes back as a message naming how much would be lost; saying
   * yes to it is the person deciding, never this screen deciding for them.
   */
  const archive = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const first = await commands.cardArchive(projectId, card.id, false)
    if (first.status === 'ok') {
      onBoard(first.data)
      return
    }
    if (!window.confirm(`${first.error.message}`)) return
    const forced = await commands.cardArchive(projectId, card.id, true)
    if (forced.status === 'ok') onBoard(forced.data)
    else onProblem(forced.error.message)
  }

  const attach = async (event: React.MouseEvent): Promise<void> => {
    event.stopPropagation()
    const answer = await commands.terminalAttachAgent(projectId, card.id)
    if (answer.status === 'error') onProblem(answer.error.message)
  }

  return (
    <article
      className="card"
      draggable
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
      onClick={onToggle}
    >
      <div className="card__title">{card.title}</div>
      <div className="card__meta">
        <button type="button" className="card__attach" onClick={(e) => void attach(e)}>
          terminal
        </button>
        <button type="button" className="card__attach" onClick={(e) => void archive(e)}>
          archive
        </button>
        {card.worktreePath !== null && (
          <button type="button" className="card__attach" onClick={(e) => void readFront(e)}>
            changes
          </button>
        )}
        {card.costUsd !== null && card.costUsd > 0 && (
          <span className="card__cost">${card.costUsd.toFixed(4)}</span>
        )}
        {card.runs.length > 0 && (
          <span className="card__runs">
            {card.runs.length} run{card.runs.length === 1 ? '' : 's'}
          </span>
        )}
      </div>
      {front !== null && (
        <div className="card__front">
          <div className="card__front-base">against {front.baseRef ?? 'nothing recorded'}</div>
          {front.files.length === 0 ? (
            <div>nothing changed here yet</div>
          ) : (
            <ul>
              {front.files.map((file: string) => (
                <li key={file}>{file}</li>
              ))}
            </ul>
          )}
          {front.unsaved.length > 0 && (
            <div className="card__front-unsaved">
              {front.unsaved.length} change{front.unsaved.length === 1 ? '' : 's'} nothing has
              saved yet
            </div>
          )}
        </div>
      )}

      {expanded && card.runs.length > 0 && (
        <ol className="card__history">
          {card.runs.map((run: Run) => (
            <RunLine key={run.id} run={run} />
          ))}
        </ol>
      )}
    </article>
  )
}

function RunLine({ run }: { run: Run }): React.JSX.Element {
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
