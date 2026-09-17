import { describe, expect, it } from 'vitest'

import type { Board, Project } from '../gen/bindings'
import { cardsOf, kept, lanesOf, laneOrder, NO_FILTERS, type ProjectBoard, tally } from './manager'

/* Enough of a project and a board to be grouped; the fields the Manager does
   not read are not invented here. */
const project = (id: string, name: string): Project =>
  ({ id, name, rootPath: `/tmp/${id}`, group: null, accent: '#f60', worktrees: [] }) as unknown as Project

const card = (id: string, columnId: string, title: string, position: number) =>
  ({ id, columnId, title, body: '', position, worktreePath: null, dueAt: null, costUsd: null, comments: 0, pinned: 0 }) as unknown as Board['cards'][number]

const column = (id: string, name: string, position: number) =>
  ({ id, name, position, step: null }) as unknown as Board['columns'][number]

const board = (projectId: string, columns: Board['columns'], cards: Board['cards']): Board =>
  ({ projectId, columns, cards, steps: [] }) as Board

/* Two boards that agree on Inbox and Doing and disagree about the rest —
   which is the ordinary case this view exists for. */
const two: ProjectBoard[] = [
  {
    project: project('p1', 'devpit'),
    board: board(
      'p1',
      [column('c1', 'Inbox', 0), column('c2', 'Doing', 1), column('c3', 'Ship', 2)],
      [card('k1', 'c1', 'Fix the parser', 1), card('k2', 'c2', 'Name the socket', 0)],
    ),
  },
  {
    project: project('p2', 'orca'),
    board: board(
      'p2',
      [column('d1', 'Inbox', 0), column('d2', 'Review', 1), column('d3', 'Doing', 2)],
      [card('k3', 'd1', 'Read the manifest', 0), card('k4', 'd3', 'Fix the window', 0)],
    ),
  },
]

describe('every board read as one', () => {
  it('a lane both boards have is one lane, and a lane only one has keeps its own', () => {
    expect(laneOrder(two)).toEqual(['Inbox', 'Doing', 'Review', 'Ship'])
  })

  /* Doing is position 1 on one board and 2 on the other; it takes the lower,
     so the shared spine stays where both boards draw it. Sabotage: take the
     highest and Doing falls behind Review, which only one board has. */
  it('a lane sits at the earliest place any board gives it', () => {
    const order = laneOrder(two)
    expect(order.indexOf('Doing')).toBeLessThan(order.indexOf('Review'))
  })

  it('carries the project on every card, so a tile can say whose it is', () => {
    const cards = cardsOf(two)
    expect(cards.map((one) => [one.title, one.projectName, one.lane])).toEqual([
      ['Name the socket', 'devpit', 'Doing'],
      ['Fix the parser', 'devpit', 'Inbox'],
      ['Read the manifest', 'orca', 'Inbox'],
      ['Fix the window', 'orca', 'Doing'],
    ])
  })

  /* A card whose column is gone is dropped rather than drawn under a lane
     invented for it: the board is what says where a card is. */
  it('drops a card whose column the board does not have', () => {
    const orphan: ProjectBoard[] = [
      { project: project('p3', 'ghost'), board: board('p3', [column('e1', 'Inbox', 0)], [card('k9', 'gone', 'Nowhere', 0)]) },
    ]
    expect(cardsOf(orphan)).toEqual([])
  })
})

describe('the filters', () => {
  it('looks for the words in the title and in the project name', () => {
    const cards = cardsOf(two)
    expect(kept(cards, { ...NO_FILTERS, query: 'fix' }).map((one) => one.title)).toEqual([
      'Fix the parser',
      'Fix the window',
    ])
    expect(kept(cards, { ...NO_FILTERS, query: 'ORCA' }).map((one) => one.projectName)).toEqual(['orca', 'orca'])
  })

  it('keeps one project, or one lane', () => {
    const cards = cardsOf(two)
    expect(kept(cards, { ...NO_FILTERS, projectId: 'p2' })).toHaveLength(2)
    expect(kept(cards, { ...NO_FILTERS, lane: 'Doing' }).map((one) => one.title)).toEqual([
      'Name the socket',
      'Fix the window',
    ])
  })

  /* A kanban whose columns come and go as you type is one you cannot aim at,
     so a lane emptied by a filter is still drawn. */
  it('a filter empties a lane rather than removing it', () => {
    const lanes = lanesOf(two, { ...NO_FILTERS, query: 'parser' })
    expect(lanes.map((lane) => lane.name)).toEqual(['Inbox', 'Doing', 'Review', 'Ship'])
    expect(lanes.find((lane) => lane.name === 'Doing')?.cards).toEqual([])
  })

  it('a lane filter draws that lane alone', () => {
    expect(lanesOf(two, { ...NO_FILTERS, lane: 'Review' }).map((lane) => lane.name)).toEqual(['Review'])
  })
})

describe('the tally', () => {
  it('counts what the filters left, cards and projects, and says it in words', () => {
    expect(tally(lanesOf(two, NO_FILTERS))).toBe('4 cards · 2 projects')
    expect(tally(lanesOf(two, { ...NO_FILTERS, query: 'parser' }))).toBe('1 card · 1 project')
    expect(tally(lanesOf(two, { ...NO_FILTERS, query: 'nothing at all' }))).toBe('0 cards · 0 projects')
  })
})
