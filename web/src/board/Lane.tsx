import type { Board, Card, Column } from '../gen/bindings'
import { CardTile } from './CardTile'
import { StepPicker } from './StepPicker'

/**
 * One column of the board.
 *
 * Everything it can do — rename, delete, choose a step, take a card — is
 * handed in. The lane draws; the board decides.
 */
export function Lane({
  projectId,
  column,
  cards,
  steps,
  dragging,
  picking,
  openCard,
  onDrop,
  onRename,
  onRemove,
  onAddCard,
  onPick,
  onOpenCard,
  onDragCard,
  onBoard,
  onProblem
}: {
  projectId: string
  column: Column
  cards: Card[]
  steps: Board['steps']
  dragging: string | null
  picking: string | null
  openCard: string | null
  onDrop: (column: Column) => void
  onRename: (column: Column) => void
  onRemove: (column: Column) => void
  onAddCard: (column: Column) => void
  onPick: (columnId: string | null) => void
  onOpenCard: (cardId: string | null) => void
  onDragCard: (cardId: string | null) => void
  onBoard: (board: Board) => void
  onProblem: (message: string) => void
}): React.JSX.Element {
  return (
        <section
          key={column.id}
          className={dragging === null ? 'lane' : 'lane lane--target'}
          onDragOver={(event) => event.preventDefault()}
          onDrop={() => onDrop(column)}
        >
          <header className="lane__head">
            <button type="button" className="lane__name" onClick={() => onRename(column)}>
              {column.name}
            </button>
            <span className="lane__count">{cards.length}</span>
            <button
              type="button"
              className="lane__remove"
              title="Delete this column"
              onClick={() => onRemove(column)}
            >
              ×
            </button>
          </header>

          <button
            type="button"
            className={
              column.step !== null ? 'lane__step' : 'lane__step lane__step--none'
            }
            title={
              column.step !== null
                ? `Runs ${column.step.name} on arrival`
                : 'Nothing runs when a card lands here'
            }
            onClick={() => onPick(picking === column.id ? null : column.id)}
          >
            {/* A lane that runs nothing is a lane doing its job, so it says
                so rather than looking half-configured. */}
            {column.step?.name ?? 'runs nothing'}
            {column.step?.irreversible === true && <span className="lane__warn">no undo</span>}
          </button>

          {picking === column.id && (
            <StepPicker
              projectId={projectId}
              column={column}
              steps={steps}
              onBoard={onBoard}
              onProblem={onProblem}
              onClose={() => onPick(null)}
            />
          )}

          <div className="lane__cards">
            {cards.map((card: Card) => (
              <CardTile
                key={card.id}
                card={card}
                projectId={projectId}
                onProblem={onProblem}
                expanded={openCard === card.id}
                onToggle={() => onOpenCard(openCard === card.id ? null : card.id)}
                onDragStart={() => onDragCard(card.id)}
                onDragEnd={() => onDragCard(null)}
              />
            ))}
          </div>

          <button type="button" className="lane__add" onClick={() => onAddCard(column)}>
            + card
          </button>
        </section>
  )
}
