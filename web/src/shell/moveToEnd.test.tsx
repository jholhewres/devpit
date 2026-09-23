import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board, Card } from '../gen/bindings'
import { useBoard } from './useBoard'

afterEach(cleanup)

const moved = vi.fn()

const card = (id: string, columnId: string, position: number): Card => ({
  id,
  columnId,
  title: id,
  body: '',
  position,
  worktreePath: null,
  dueAt: null,
  costUsd: null,
  comments: 0,
  pinned: 0,
  runs: [],
  activity: null,
})

let cards = (): Card[] => [card('a', 'todo', 0), card('b', 'done', 0), card('c', 'done', 1)]

const board = (): Board => ({
  projectId: 'p1',
  columns: ['todo', 'done'].map((id, position) => ({ id, name: id, position, step: null, onPass: null, autonomy: 'manual' })),
  cards: cards(),
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    boardGet: () => board(),
    cardMove: (project: string, cardId: string, columnId: string, at: number, confirmed: boolean) => {
      moved(project, cardId, columnId, at, confirmed)
      return { card: card(cardId, columnId, at), started: null }
    },
  },
}))
vi.mock('./window', () => ({ onCarried: () => () => undefined }))

describe('a card picked into another lane', () => {
  it('goes to the end of it, through the same move a drop makes', async () => {
    const { result } = renderHook(() => useBoard('p1'))
    await waitFor(() => expect(result.current.lanes).toHaveLength(2))
    act(() => result.current.moveToEnd('a', 'done'))
    await waitFor(() => expect(moved).toHaveBeenCalledWith('p1', 'a', 'done', 2, false))
    expect(result.current.lanes[1]!.cards.map((one) => one.id)).toEqual(['b', 'c', 'a'])
  })

  /* `done` held a card at 1 that has left: c is at 2. The first card goes past
     c; the lane is then renumbered from nought, as the store renumbers it, so
     the next end is past the two cards now at 0 and 1. */
  it('goes past the last card, not to the count, once a card has left the lane', async () => {
    moved.mockClear()
    cards = () => [card('a', 'todo', 0), card('d', 'todo', 1), card('c', 'done', 2)]
    const { result } = renderHook(() => useBoard('p1'))
    await waitFor(() => expect(result.current.lanes).toHaveLength(2))
    act(() => result.current.moveToEnd('a', 'done'))
    await waitFor(() => expect(moved).toHaveBeenCalledWith('p1', 'a', 'done', 3, false))
    act(() => result.current.moveToEnd('d', 'done'))
    await waitFor(() => expect(moved).toHaveBeenCalledWith('p1', 'd', 'done', 2, false))
    expect(result.current.lanes[1]!.cards.map((one) => one.id)).toEqual(['c', 'a', 'd'])
  })
})
