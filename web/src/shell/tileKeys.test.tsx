import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Card, DeleteRefusal } from '../gen/bindings'
import type { Lane } from './board'
import { cardMenu } from './cardMenu'
import { LaneCards } from './LaneCards'
import { keyLabel, tileAction, type TileAction } from './tileKeys'
import type { CardBoardActs } from './useCardActs'
import type { Drag } from './useDrag'

afterEach(cleanup)

vi.mock('./live', () => ({ ask: vi.fn(), commands: {} }))
vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

const card: Card = {
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
  session: null,
}

const drag: Drag = { held: null, landing: null, landed: null, down: vi.fn(), move: vi.fn(), up: vi.fn(() => ({ what: 'nothing' as const })) }

describe('the keys a tile answers to', () => {
  it('reads each key as its action', () => {
    expect(tileAction({ key: 'Enter' })).toBe('open')
    expect(tileAction({ key: 'F2' })).toBe('rename')
    expect(tileAction({ key: 'Delete' })).toBe('archive')
    expect(tileAction({ key: 'm', ctrlKey: true })).toBe('moveTo')
    expect(tileAction({ key: 'M', metaKey: true })).toBe('moveTo')
    expect(tileAction({ key: 'm' })).toBeNull()
    expect(tileAction({ key: 'Enter', isComposing: true })).toBeNull()
  })

  it('are the labels the menu shows, each on the entry it performs', () => {
    const entries = cardMenu({
      open: vi.fn(),
      rename: vi.fn(),
      moveTo: vi.fn(),
      terminal: vi.fn(),
      archive: vi.fn(),
      remove: vi.fn(),
    })
    const performs: Record<TileAction, string> = { open: 'Open', rename: 'Rename', moveTo: 'Move to…', archive: 'Archive' }
    for (const [action, label] of Object.entries(performs) as [TileAction, string][]) {
      expect(entries.find((entry) => entry.label === label)?.key, label).toBe(keyLabel(action))
    }
    expect(entries.filter((entry) => entry.key).map((entry) => entry.label)).toEqual(Object.values(performs))
  })
})

describe('a focused tile', () => {
  const acts = (over: Partial<CardBoardActs> = {}): CardBoardActs => ({
    terminal: vi.fn(),
    archive: vi.fn(() => Promise.resolve(null)),
    remove: vi.fn(() => Promise.resolve<DeleteRefusal | string | null>(null)),
    archived: vi.fn(),
    problem: vi.fn(),
    ...over,
  })

  const board = (hands: CardBoardActs, onOpen = vi.fn()) => {
    const lane = { column: { id: 'col_1', name: 'Todo', position: 0, step: null, onPass: null, autonomy: 'manual' }, cards: [card] } as Lane
    const view = render(
      <LaneCards
        lane={lane}
        drag={drag}
        dropping={false}
        progress={{}}
        onOpen={onOpen}
        onPlay={vi.fn(() => Promise.resolve(null))}
        onRename={vi.fn()}
        others={[{ id: 'col_2', name: 'Doing' }]}
        onMove={vi.fn()}
        acts={() => hands}
      />,
    )
    return view.container.querySelector<HTMLElement>('[data-id="card_1"]')!
  }

  it('opens on Enter', () => {
    const onOpen = vi.fn()
    fireEvent.keyDown(board(acts(), onOpen), { key: 'Enter' })
    expect(onOpen).toHaveBeenCalledWith('card_1')
  })

  it('renames on F2', () => {
    fireEvent.keyDown(board(acts()), { key: 'F2' })
    expect(screen.getByRole('textbox', { name: 'Card title' })).toBeTruthy()
  })

  it('archives on Delete, and the board offers undo', async () => {
    const hands = acts()
    fireEvent.keyDown(board(hands), { key: 'Delete' })
    await waitFor(() => expect(hands.archived).toHaveBeenCalled())
    expect(hands.archive).toHaveBeenCalledWith(false)
  })

  it('opens the lanes to move to on Ctrl+M', () => {
    fireEvent.keyDown(board(acts()), { key: 'm', ctrlKey: true })
    expect(within(screen.getByRole('menu')).getAllByRole('menuitem').map((item) => item.textContent)).toEqual(['Doing'])
  })
})
