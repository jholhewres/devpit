import type { Card, Run } from '../gen/bindings'

/**
 * A card, and what it has cost.
 *
 * The cost is shown whenever any run has one. "The agent is doing something"
 * stops being an acceptable answer once the number is on the card.
 */
export function CardTile({
  card,
  expanded,
  onToggle,
  onDragStart,
  onDragEnd
}: {
  card: Card
  expanded: boolean
  onToggle: () => void
  onDragStart: () => void
  onDragEnd: () => void
}): React.JSX.Element {
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
        {card.costUsd !== null && card.costUsd > 0 && (
          <span className="card__cost">${card.costUsd.toFixed(4)}</span>
        )}
        {card.runs.length > 0 && (
          <span className="card__runs">
            {card.runs.length} run{card.runs.length === 1 ? '' : 's'}
          </span>
        )}
      </div>
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
