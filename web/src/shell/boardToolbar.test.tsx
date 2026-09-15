import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board, Card } from '../gen/bindings'
import { matches } from './board'
import { BoardPane } from './BoardPane'

afterEach(cleanup)

const card = (id: string, title: string, over: Partial<Card> = {}): Card => ({
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
  ...over,
})

const board = (): Board => ({
  projectId: 'p1',
  columns: [{ id: 'col_1', name: 'Todo', position: 0, step: null, onPass: null, autonomy: 'manual' }],
  cards: [card('card_1', 'Wire the board'), card('card_2', 'Check the tests', { position: 1 })],
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    boardGet: () => board(),
    cardMove: (_project: string, cardId: string, columnId: string, position: number) => {
      moved(cardId, columnId, position)
      return { card: {}, started: null }
    },
    boardArchived: () => ({ cards: [{ id: 'card_x', title: 'Old idea', columnName: 'Todo', archivedAt: 1 }] }),
    runsList: () => ({ runs: [], next: null }),
  },
}))
vi.mock('./window', () => ({ onCarried: () => () => undefined }))

const moved = vi.fn()
let ended: unknown = { what: 'nothing' }
vi.mock('./useDrag', () => ({
  useDrag: () => ({ held: null, landing: null, landed: null, down: vi.fn(), move: vi.fn(), up: () => ended }),
}))
vi.mock('./useShell', () => ({
  useShell: () => ({ project: { id: 'p1' }, wantedCard: null, openCard: vi.fn(), show: vi.fn(), closeNow: vi.fn() }),
}))

describe('the board filter', () => {
  it('keeps the cards whose title holds the query, in any case', () => {
    expect(matches(card('a', 'Wire the Board'), 'board')).toBe(true)
    expect(matches(card('a', 'Wire the board'), '  WIRE ')).toBe(true)
    expect(matches(card('a', 'Wire the board'), 'tests')).toBe(false)
    expect(matches(card('a', 'Anything'), '')).toBe(true)
  })
})

describe("the board's toolbar", () => {
  it('holds the filter, Runs, Archived with its count and + Column, and nothing waits at the end of the lanes', async () => {
    const { container } = render(<BoardPane />)
    await screen.findByText('Check the tests')
    const toolbar = container.querySelector<HTMLElement>('.btools')!
    expect(within(toolbar).getByRole('searchbox', { name: 'Filter cards' })).toBeTruthy()
    await waitFor(() => expect(within(toolbar).getByRole('button', { name: 'Archived (1)' })).toBeTruthy())
    expect(within(toolbar).getAllByRole('button').map((button) => button.textContent)).toEqual([
      'Runs',
      'Archived (1)',
      '+ Column',
    ])
    const lanes = container.querySelector<HTMLElement>('.board')!
    for (const name of ['Runs', 'Archived (1)', '+ Column']) {
      expect(within(lanes).queryByRole('button', { name })).toBeNull()
    }
  })

  it('shows only the cards the filter keeps', async () => {
    render(<BoardPane />)
    await screen.findByText('Check the tests')
    fireEvent.change(screen.getByRole('searchbox', { name: 'Filter cards' }), { target: { value: 'CHECK' } })
    expect(screen.queryByText('Wire the board')).toBeNull()
    expect(screen.getByText('Check the tests')).toBeTruthy()
  })

  it('drops a card at the end of its lane while the board is filtered, where the slot means nothing', async () => {
    const { container } = render(<BoardPane />)
    await screen.findByText('Check the tests')
    ended = { what: 'moved', card: card('card_2', 'Check the tests'), lane: 'col_1', index: 0 }
    fireEvent.pointerUp(container.querySelector('.board')!)
    await waitFor(() => expect(moved).toHaveBeenLastCalledWith('card_2', 'col_1', 0))

    fireEvent.change(screen.getByRole('searchbox', { name: 'Filter cards' }), { target: { value: 'check' } })
    fireEvent.pointerUp(container.querySelector('.board')!)
    await waitFor(() => expect(moved).toHaveBeenLastCalledWith('card_2', 'col_1', 2))
    ended = { what: 'nothing' }
  })
})
