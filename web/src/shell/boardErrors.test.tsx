import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board, Card } from '../gen/bindings'
import { BoardPane } from './BoardPane'
import { useBoard } from './useBoard'

afterEach(cleanup)

const card = (over: Partial<Card> = {}): Card => ({
  id: 'card_1',
  columnId: 'col_1',
  title: 'Wire the board',
  body: '',
  position: 0,
  worktreePath: null,
  dueAt: null,
  costUsd: null,
  comments: 0,
  pinned: 0,
  runs: [],
  activity: null,
  ...over,
})

const board = (): Board => ({
  projectId: 'p1',
  columns: [
    { id: 'col_1', name: 'Todo', position: 0, step: null, onPass: null, autonomy: 'manual' },
    { id: 'col_2', name: 'Doing', position: 1, step: null, onPass: null, autonomy: 'manual' },
  ],
  cards: [card()],
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (thrown) {
      return { data: null, error: (thrown as Error).message, loading: false }
    }
  },
  commands: {
    boardGet: () => board(),
    cardMove: () => {
      throw new Error('that lane refused the card')
    },
    columnRename: () => {
      throw new Error('a lane needs a name nobody else has')
    },
  },
}))

vi.mock('./window', () => ({ onCarried: () => () => undefined }))
vi.mock('./useShell', () => ({
  useShell: () => ({ project: { id: 'p1' }, wantedCard: null, openCard: vi.fn() }),
}))

const laneOf = (lanes: ReturnType<typeof useBoard>['lanes'], id: string): string | undefined =>
  lanes.find((lane) => lane.cards.some((one) => one.id === id))?.column.id

describe('reading the board again', () => {
  /* A slow answer arriving after a newer one would put the board back in time. */
  it('keeps the newest answer when an older one lands last', async () => {
    const live = await import('./live')
    const slow = board()
    slow.cards = []
    let calls = 0
    const release: Array<() => void> = []
    const spy = vi.spyOn(live.commands, 'boardGet').mockImplementation(() => {
      calls += 1
      const answer = calls === 1 ? slow : board()
      return new Promise((done) => release.push(() => done(answer))) as never
    })
    const { result } = renderHook(() => useBoard('p1'))
    act(() => result.current.reload())
    await waitFor(() => expect(release).toHaveLength(2))
    await act(async () => release[1]!())
    await act(async () => release[0]!())
    expect(laneOf(result.current.lanes, 'card_1')).toBe('col_1')
    spy.mockRestore()
  })
})

describe('a board command that is refused', () => {
  it('puts a moved card back where it was and says why', async () => {
    const { result } = renderHook(() => useBoard('p1'))
    await waitFor(() => expect(result.current.lanes).toHaveLength(2))

    act(() => result.current.move('card_1', 'col_2', 0))
    await waitFor(() => expect(result.current.error).toBe('that lane refused the card'))
    expect(laneOf(result.current.lanes, 'card_1')).toBe('col_1')
  })

  it('shows a refused rename on the board until it is dismissed', async () => {
    render(<BoardPane />)
    const label = (await screen.findAllByRole('textbox'))[0]!
    label.textContent = 'Later'
    fireEvent.blur(label)

    const strip = await screen.findByRole('button', { name: 'a lane needs a name nobody else has' })
    fireEvent.click(strip)
    await waitFor(() => expect(screen.queryByText('a lane needs a name nobody else has')).toBeNull())
  })
})
