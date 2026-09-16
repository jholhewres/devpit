import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Board, Card } from '../gen/bindings'
import { CardLane } from './CardLane'
import { useBoard } from './useBoard'

afterEach(cleanup)

const QUESTION = 'a run is still going on this card — move it anyway?'
const moved = vi.fn()
const read = vi.fn()

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

const board = (): Board => ({
  projectId: 'p1',
  columns: ['todo', 'done'].map((id, position) => ({ id, name: id, position, step: null, onPass: null, autonomy: 'manual' })),
  cards: [card('a', 'todo', 0)],
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (thrown) {
      const refused = thrown as { message: string; code: string }
      return { data: null, error: refused.message, code: refused.code, loading: false }
    }
  },
  commands: {
    boardGet: () => {
      read()
      return board()
    },
    /* The backend's answer while a run is going: a conflict, until it is confirmed. */
    cardMove: (project: string, cardId: string, columnId: string, at: number, confirmed: boolean) => {
      moved(project, cardId, columnId, at, confirmed)
      if (!confirmed) throw { message: QUESTION, code: 'conflict' }
      return { card: card(cardId, columnId, at), started: null }
    },
  },
}))
vi.mock('./window', () => ({ onCarried: () => () => undefined }))

beforeEach(() => {
  moved.mockClear()
  read.mockClear()
})

describe('a move while a run is still going on the card', () => {
  it('on the board is asked, not printed, and made once the answer is yes', async () => {
    const { result } = renderHook(() => useBoard('p1'))
    await waitFor(() => expect(result.current.lanes).toHaveLength(2))
    act(() => result.current.moveToEnd('a', 'done'))
    await waitFor(() => expect(result.current.asked).toBe(QUESTION))
    expect(result.current.error).toBeNull()
    expect(result.current.lanes[0]!.cards.map((one) => one.id)).toEqual(['a'])

    act(() => result.current.answer(true))
    await waitFor(() => expect(moved).toHaveBeenLastCalledWith('p1', 'a', 'done', 0, true))
    expect(result.current.asked).toBeNull()
    await waitFor(() => expect(read).toHaveBeenCalledTimes(2))
  })

  it('from the open card is asked, and made once the answer is yes', async () => {
    const onMoved = vi.fn()
    render(
      <CardLane
        projectId="p1"
        cardId="a"
        columnId="todo"
        lanes={[
          { id: 'todo', name: 'Todo', end: 1 },
          { id: 'done', name: 'Done', end: 4 },
        ]}
        onMoved={onMoved}
      />,
    )
    fireEvent.change(screen.getByRole('combobox', { name: 'Lane' }), { target: { value: 'done' } })
    expect(await screen.findByText(QUESTION)).toBeTruthy()
    expect(onMoved).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Move it anyway' }))
    await waitFor(() => expect(onMoved).toHaveBeenCalled())
    expect(moved).toHaveBeenLastCalledWith('p1', 'a', 'done', 4, true)
  })
})
