import type { CardRef, PlannedCard } from '../gen/bindings'

/*
 * A Devflow plan as the person changes it on the board, before any card
 * exists: moved, cut or edited. Functions so the tests call them — a cut
 * that left a card waiting on the one cut is a plan the backend refuses.
 */

/** The card at `at` moved `by` places, within the plan. */
export function moved(cards: readonly PlannedCard[], at: number, by: number): PlannedCard[] {
  const to = at + by
  if (to < 0 || to >= cards.length) return [...cards]
  const next = [...cards]
  const [card] = next.splice(at, 1)
  next.splice(to, 0, card!)
  return next
}

/** The plan without one card, and nothing left waiting on it. */
export function cut(cards: readonly PlannedCard[], key: string): PlannedCard[] {
  return cards
    .filter((card) => card.key !== key)
    .map((card) => ({ ...card, dependsOn: card.dependsOn.filter((on) => on !== key) }))
}

/** One card changed, the rest as they were. */
export function edited(cards: readonly PlannedCard[], key: string, change: Partial<PlannedCard>): PlannedCard[] {
  return cards.map((card) => (card.key === key ? { ...card, ...change, key: card.key } : card))
}

/** Criteria typed one per line. */
export function linesOf(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '')
}

/** What still blocks a card: the ones it waits on that are not done. */
export function stillBlocking(blockedBy: readonly CardRef[] | undefined): CardRef[] {
  return (blockedBy ?? []).filter((by) => !by.done)
}
