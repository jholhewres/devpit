import { describe, expect, it } from 'vitest'

import type { Board, CardHappening } from '../gen/bindings'
import { withActivity } from './useBoard'

const board = { columns: [], cards: [{ id: 'card_a', activity: null }, { id: 'card_b', activity: null }] } as unknown as Board
const said = (cardId: string, activity: unknown): CardHappening => ({ cardId, activity, sessions: [] }) as unknown as CardHappening

describe('withActivity', () => {
  it('keeps the same board for a card that is not on it', () => {
    expect(withActivity(board, said('card_elsewhere', { doing: 'working' }))).toBe(board)
  })

  it('keeps the same board when the activity did not change', () => {
    expect(withActivity(board, said('card_a', null))).toBe(board)
  })

  it('replaces only the card whose activity changed', () => {
    const next = withActivity(board, said('card_a', { doing: 'working' }))!
    expect(next).not.toBe(board)
    expect(next.cards[0]!.activity).toEqual({ doing: 'working' })
    expect(next.cards[1]).toBe(board.cards[1])
  })
})
