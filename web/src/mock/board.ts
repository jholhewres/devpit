/* The board, as data.
 *
 * It was markup in the prototype and moving a card meant moving DOM. A card
 * that can be dragged is state: the lane it is in is the whole point of the
 * board, and state is what the backend will hand over. */

export interface Card {
  readonly id: string
  readonly title: string
  /** What the card says under its title: a duration, a run, a diff. */
  readonly note?: string
  readonly agent?: string
  readonly warn?: true
  readonly running?: true
  readonly added?: number
  readonly deleted?: number
  readonly merged?: string
}

export interface Lane {
  readonly name: string
  readonly label: string
  /** The agent this lane runs when a card lands in it, if it runs one. */
  readonly agent?: string
  readonly cards: readonly Card[]
}

export const LANES: readonly Lane[] = [
  {
    name: 'inbox',
    label: 'Inbox',
    cards: [
      { id: 'c1', title: 'Persist the sidebar width', note: '2d' },
      { id: 'c2', title: 'File-type icons in the tree', note: '2d' },
      { id: 'c3', title: 'Split the middle horizontally', note: '1d' },
    ],
  },
  {
    name: 'refine',
    label: 'Refine',
    agent: 'planner',
    cards: [
      { id: 'c4', title: 'Board opens in the content', agent: 'planner', warn: true, note: 'needs you' },
    ],
  },
  {
    name: 'doing',
    label: 'Doing',
    agent: 'executor',
    cards: [
      { id: 'c5', title: 'Rebuild the shell on GPUI', agent: 'executor', running: true, note: '3m' },
    ],
  },
  {
    name: 'check',
    label: 'Check',
    agent: 'reviewer',
    cards: [
      { id: 'c6', title: 'Two read ceilings were guarding nothing', added: 70, deleted: 35, note: '28m' },
    ],
  },
  {
    name: 'ship',
    label: 'Ship',
    cards: [
      { id: 'c7', title: 'The Changes panel opens a diff', merged: 'merged · 3h' },
      { id: 'c8', title: 'Background sessions get hooks too', merged: 'merged · 5h' },
    ],
  },
]
