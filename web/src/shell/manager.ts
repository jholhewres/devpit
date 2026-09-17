import type { Board, Project } from '../gen/bindings'

/*
 * Every project's board, read as one.
 *
 * A project is a board and a board is its own columns, so there is no shared
 * lane to group by — only names that happen to repeat. That is what this uses:
 * two projects that both call a column "Doing" have one Doing here, and a
 * project that calls it something else keeps its own lane rather than being
 * folded into the nearest word.
 *
 * Order is by where the name sits on the boards that have it, lowest first, so
 * the lanes read left to right the way each board does. A tie goes to the name
 * that appears on more boards: the shared spine of everybody's process is what
 * belongs on the left.
 *
 * This is the whole Manager for now: one kanban over every card, filtered.
 * Nothing here writes — moving a card across projects is not a thing the
 * boards could agree on, and a view that moves things is a promise this cannot
 * keep yet.
 */

/** One card, carrying enough of its project to be read away from it. */
export interface ManagedCard {
  readonly cardId: string
  readonly projectId: string
  readonly projectName: string
  /** The project's own colour, so a tile says whose it is before it is read. */
  readonly accent: string
  readonly lane: string
  readonly title: string
  readonly dueAt: number | null
  readonly comments: number
  readonly costUsd: number | null
}

export interface ManagedLane {
  readonly name: string
  readonly cards: readonly ManagedCard[]
}

/** What a board answered, beside the project it answers for. */
export interface ProjectBoard {
  readonly project: Project
  readonly board: Board
}

export interface Filters {
  /** Looked for in the card's title and its project's name, in any case. */
  readonly query: string
  /** A project id, or null for every project. */
  readonly projectId: string | null
  /** A lane name, or null for every lane. */
  readonly lane: string | null
}

export const NO_FILTERS: Filters = { query: '', projectId: null, lane: null }

/** Every card of every board, in the order each board keeps them. */
export function cardsOf(boards: readonly ProjectBoard[]): ManagedCard[] {
  return boards.flatMap(({ project, board }) => {
    const laneOf = new Map(board.columns.map((column) => [column.id, column.name]))
    return board.cards
      .slice()
      .sort((one, two) => one.position - two.position)
      .flatMap((card) => {
        const lane = laneOf.get(card.columnId)
        /* A card whose column is not on the board is not drawn under a lane
           invented for it: the board is what says where a card is. */
        return lane === undefined
          ? []
          : [
              {
                cardId: card.id,
                projectId: project.id,
                projectName: project.name,
                accent: project.accent,
                lane,
                title: card.title,
                dueAt: card.dueAt,
                comments: card.comments,
                costUsd: card.costUsd,
              },
            ]
      })
  })
}

/** Every lane name across the boards, in the order described at the top. */
export function laneOrder(boards: readonly ProjectBoard[]): string[] {
  const seen = new Map<string, { at: number; on: number }>()
  for (const { board } of boards) {
    for (const column of board.columns) {
      const had = seen.get(column.name)
      seen.set(column.name, {
        at: had === undefined ? column.position : Math.min(had.at, column.position),
        on: (had?.on ?? 0) + 1,
      })
    }
  }
  return [...seen.entries()]
    .sort(([oneName, one], [twoName, two]) =>
      one.at !== two.at ? one.at - two.at : one.on !== two.on ? two.on - one.on : oneName.localeCompare(twoName),
    )
    .map(([name]) => name)
}

/** The cards a filter keeps. */
export function kept(cards: readonly ManagedCard[], filters: Filters): ManagedCard[] {
  const wanted = filters.query.trim().toLowerCase()
  return cards.filter((card) => {
    if (filters.projectId !== null && card.projectId !== filters.projectId) return false
    if (filters.lane !== null && card.lane !== filters.lane) return false
    if (wanted === '') return true
    return `${card.title} ${card.projectName}`.toLowerCase().includes(wanted)
  })
}

/**
 * The board to draw: the lanes in order, each with the cards a filter kept.
 *
 * A lane left empty by a filter is still drawn, because a kanban whose columns
 * come and go as you type is one you cannot aim at. A lane no board has is not
 * invented.
 */
export function lanesOf(boards: readonly ProjectBoard[], filters: Filters): ManagedLane[] {
  const cards = kept(cardsOf(boards), filters)
  return laneOrder(boards)
    .filter((name) => filters.lane === null || name === filters.lane)
    .map((name) => ({ name, cards: cards.filter((card) => card.lane === name) }))
}

/** "12 cards · 3 projects", for the line under the filters. */
export function tally(lanes: readonly ManagedLane[]): string {
  const cards = lanes.flatMap((lane) => lane.cards)
  const projects = new Set(cards.map((card) => card.projectId)).size
  return `${cards.length} ${cards.length === 1 ? 'card' : 'cards'} · ${projects} ${projects === 1 ? 'project' : 'projects'}`
}
