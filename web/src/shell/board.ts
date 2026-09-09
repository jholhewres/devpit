import type { Board, Card, Column } from '../gen/bindings'

/* What a board looks like once the cards are filed under their columns.
   A function so the test calls it: the ordering rules are the board. */
export interface Lane {
  readonly column: Column
  readonly cards: readonly Card[]
}

export function lanes(board: Board | null): readonly Lane[] {
  if (!board) return []
  const byPosition = <T extends { position: number }>(a: T, b: T): number => a.position - b.position
  return [...board.columns].sort(byPosition).map((column) => ({
    column,
    cards: board.cards.filter((card) => card.columnId === column.id).sort(byPosition),
  }))
}

/* Where a card lands, applied to the board we already hold — the screen must
   not wait for a round trip to show what the drop did. */
export function landed(board: Board, cardId: string, columnId: string, at: number): Board {
  const moving = board.cards.find((card) => card.id === cardId)
  if (!moving) return board
  const rest = board.cards.filter((card) => card.id !== cardId)
  const target = rest
    .filter((card) => card.columnId === columnId)
    .sort((a, b) => a.position - b.position)
  target.splice(at, 0, { ...moving, columnId })
  const renumbered = target.map((card, position) => ({ ...card, position }))
  return {
    ...board,
    cards: [...rest.filter((card) => card.columnId !== columnId), ...renumbered],
  }
}
