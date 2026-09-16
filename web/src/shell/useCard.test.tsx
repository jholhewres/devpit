import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Card, CardDetail } from '../gen/bindings'
import { useCard } from './useCard'

afterEach(cleanup)

const card = (id: string, title: string): Card => ({
  id,
  columnId: 'col_1',
  title,
  body: '',
  position: 0,
  worktreePath: null,
  dueAt: null,
  costUsd: null,
  comments: 0,
  pinned: 0,
  runs: [],
  activity: null,
})

const detail = (id: string, title: string): CardDetail => ({
  card: card(id, title),
  columnName: 'Todo',
  columnStep: null,
  comments: [],
  pinned: [],
  worktree: null,
  runs: [],
  sessions: [],
})

/* Each read waits until the test lets it land, so the order replies arrive in is the test's. */
const reads: { cardId: string; land: () => void }[] = []

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    cardDetail: (_project: string, cardId: string) =>
      new Promise<CardDetail>((resolve) => reads.push({ cardId, land: () => resolve(detail(cardId, `${cardId} as read`)) })),
    /* `card_update` answers with the card alone, not the detail. */
    cardUpdate: (_project: string, cardId: string, title: string) => card(cardId, title),
  },
}))

const land = async (cardId: string): Promise<void> => {
  const read = reads.find((one) => one.cardId === cardId)!
  reads.splice(reads.indexOf(read), 1)
  await act(async () => read.land())
}

describe('an open card', () => {
  it('stays whole after its title is saved', async () => {
    const { result } = renderHook(() => useCard('p1', 'card_1'))
    await land('card_1')
    let refused: string | null = 'not saved'
    await act(async () => {
      refused = await result.current.save('Wire the whole board', '')
    })
    expect(refused).toBeNull()
    expect(result.current.detail?.card.title).toBe('Wire the whole board')
    expect(result.current.detail?.comments).toEqual([])
  })

  it('never shows one card under the id of the next', async () => {
    const { result, rerender } = renderHook(({ cardId }) => useCard('p1', cardId), {
      initialProps: { cardId: 'card_a' },
    })
    await land('card_a')
    act(() => result.current.reload())
    rerender({ cardId: 'card_b' })
    expect(result.current.detail).toBeNull()

    await land('card_b')
    await land('card_a')
    await waitFor(() => expect(result.current.detail?.card.id).toBe('card_b'))
  })
})
