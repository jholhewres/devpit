import type { Card } from '../gen/bindings'

/**
 * What you can do to a card without opening it.
 *
 * Plain words rather than icons: there are four of them, they are rare, and a
 * row of glyphs here would be four things to learn for actions taken once a
 * day.
 */
export function CardActions({
  card,
  onEdit,
  onArchive,
  onAttach,
  onChanges
}: {
  card: Card
  onEdit: (event: React.MouseEvent) => void
  onArchive: (event: React.MouseEvent) => void
  onAttach: (event: React.MouseEvent) => void
  onChanges: (event: React.MouseEvent) => void
}): React.JSX.Element {
  return (
    <>
      <button type="button" className="card__attach" onClick={onEdit}>
        edit
      </button>
      <button type="button" className="card__attach" onClick={onArchive}>
        archive
      </button>
      <button type="button" className="card__attach" onClick={onAttach}>
        terminal
      </button>
      {card.worktreePath !== null && (
        <button type="button" className="card__attach" onClick={onChanges}>
          changes
        </button>
      )}
    </>
  )
}
