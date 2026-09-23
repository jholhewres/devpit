import type { Board, Card, Column, ErrorCode } from '../gen/bindings'

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

/** Where the end of a lane is: past the highest position, never one a card
 *  holds. The store renumbers the lane a card lands in, and a position past
 *  the end is the end. */
export function endOf(cards: readonly Card[]): number {
  return cards.reduce((end, card) => Math.max(end, card.position + 1), 0)
}

/** A card put at a position, applied to the board we already hold. Nothing
 *  else is renumbered, so the next end is still read off what the backend has. */
export function placed(board: Board, cardId: string, columnId: string, position: number): Board {
  return {
    ...board,
    cards: board.cards.map((card) => (card.id === cardId ? { ...card, columnId, position } : card)),
  }
}

/** The question a refused move asks, or null when the refusal is only a refusal.
 *  A run still going on the card is a conflict, and it can be moved on purpose. */
export function moveQuestion(answer: { error: string | null; code?: ErrorCode | null }, confirmed: boolean): string | null {
  return !confirmed && answer.code === 'conflict' ? answer.error : null
}

/* Whether a tile's play button can do anything: the lane has to run a step,
   and a card with a run already going would be two processes in one checkout. */
export function playable(card: Card, column: Column): boolean {
  return column.step !== null && !card.runs.some((run) => run.state === 'running')
}

/** Whether a card is one the board's filter keeps: its title holds the query, in any case. */
export function matches(card: Card, query: string): boolean {
  const wanted = query.trim().toLowerCase()
  return wanted === '' || card.title.toLowerCase().includes(wanted)
}
