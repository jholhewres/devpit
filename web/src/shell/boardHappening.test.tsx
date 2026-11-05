import { cleanup, renderHook, waitFor } from '@testing-library/react'
import { useLayoutEffect } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board, CardHappening } from '../gen/bindings'
import { useBoard } from './useBoard'

/*
 * What a card is doing arrives on its own event, and the board took it only
 * for a card it knew it held. It learnt that after paint, so a report heard
 * between the commit and the effects was dropped for a tile already on
 * screen: a loaded machine left it there long enough to fail `tileRedraw`.
 */

afterEach(cleanup)

const handlers: Record<string, (payload: unknown) => void> = {}

const board = (): Board => ({
  projectId: 'p1',
  columns: [{ id: 'col_1', name: 'Todo', position: 0, step: null, onPass: null, autonomy: 'manual', role: null, roleChosen: false }],
  cards: [
    {
      id: 'a',
      columnId: 'col_1',
      title: 'Card a',
      body: '',
      position: 0,
      worktreePath: null,
      dueAt: null,
      dueTime: false,
      costUsd: null,
      comments: 0,
      pinned: 0,
      runs: [],
      activity: null,
    },
  ],
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { boardGet: () => board() },
}))
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => {
    handlers[name] = then
    return () => undefined
  },
}))

describe("a card's activity", () => {
  it('lands on a tile heard of in the commit that drew it', async () => {
    const working: CardHappening = { cardId: 'a', activity: 'working', sessions: [] }
    const { result } = renderHook(() => {
      const live = useBoard('p1')
      /* The earliest a report can arrive once the tile is in the DOM. */
      useLayoutEffect(() => {
        if (live.cards > 0) handlers['card:happening']!(working)
      }, [live.cards])
      return live
    })

    await waitFor(() => expect(result.current.lanes[0]?.cards[0]?.activity).toBe('working'))
    expect(result.current.sessions).toEqual({ a: [] })
  })
})
