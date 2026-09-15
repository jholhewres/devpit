import { describe, expect, it } from 'vitest'

import type { Board, Card, Column, Run } from '../gen/bindings'
import { landed, lanes, playable } from './board'

const column = (id: string, position: number): Column => ({ id, name: id, position, step: null, onPass: null, autonomy: 'manual' })
const card = (id: string, columnId: string, position: number): Card =>
  ({ id, columnId, title: id, body: '', position, worktreePath: null,
  dueAt: null,
  comments: 0,
  pinned: 0, costUsd: null, runs: [], activity: null })

const board: Board = {
  projectId: 'p',
  columns: [column('b', 1), column('a', 0)],
  cards: [card('c2', 'a', 1), card('c1', 'a', 0), card('c3', 'b', 0)],
  steps: [],
}

describe('filing cards under columns', () => {
  it('puts the columns in their own order, not the order they arrived', () => {
    expect(lanes(board).map((lane) => lane.column.id)).toEqual(['a', 'b'])
  })

  it('puts the cards in position order inside a lane', () => {
    expect(lanes(board)[0]!.cards.map((c) => c.id)).toEqual(['c1', 'c2'])
  })

  it('gives an empty board no lanes rather than throwing', () => {
    expect(lanes(null)).toEqual([])
  })
})

describe('landing a card', () => {
  it('moves it to the column it was dropped in', () => {
    const after = landed(board, 'c1', 'b', 0)
    expect(after.cards.find((c) => c.id === 'c1')!.columnId).toBe('b')
  })

  it('renumbers the lane it landed in, so no two cards claim one position', () => {
    const after = lanes(landed(board, 'c1', 'b', 0))
    const positions = after.find((l) => l.column.id === 'b')!.cards.map((c) => c.position)
    expect(positions).toEqual([0, 1])
  })

  it('honours where in the lane it was dropped', () => {
    const after = lanes(landed(board, 'c1', 'b', 1))
    expect(after.find((l) => l.column.id === 'b')!.cards.map((c) => c.id)).toEqual(['c3', 'c1'])
  })

  it('leaves the board alone when the card is not on it', () => {
    expect(landed(board, 'nope', 'b', 0)).toBe(board)
  })
})

describe('whether a tile can be played', () => {
  const step = { id: 's', kind: 'command', name: 'tests', config: '{}', irreversible: false } as const
  const run = (state: Run['state']): Run => ({
    id: 'r',
    stepId: 's',
    stepName: 'tests',
    state,
    output: null,
    exitCode: null,
    costUsd: null,
    durationMs: null,
    startedAt: 0,
  })

  it('is not, in a lane that runs nothing', () => {
    expect(playable(card('c', 'a', 0), column('a', 0))).toBe(false)
  })

  it('is not, while a run is going on the card', () => {
    expect(playable({ ...card('c', 'a', 0), runs: [run('running')] }, { ...column('a', 0), step })).toBe(false)
  })

  it('is, in a lane with a step and nothing running', () => {
    expect(playable({ ...card('c', 'a', 0), runs: [run('failed')] }, { ...column('a', 0), step })).toBe(true)
  })
})
