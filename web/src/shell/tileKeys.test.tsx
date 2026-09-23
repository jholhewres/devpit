import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Card, Column, DeleteRefusal } from '../gen/bindings'
import type { Lane } from './board'
import { cardMenu } from './cardMenu'
import { menuFocus } from './menuRules'
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
  activity: null,
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

  it('leaves a key pressed on a button inside the tile to that button', () => {
    const tile = {}
    expect(tileAction({ key: 'Enter', target: tile, currentTarget: tile })).toBe('open')
    expect(tileAction({ key: 'Enter', target: {}, currentTarget: tile })).toBeNull()
    expect(tileAction({ key: 'Delete', target: {}, currentTarget: tile })).toBeNull()
  })

  it('are the labels the menu shows, each on the entry it performs', () => {
    const entries = cardMenu({
      open: vi.fn(),
      rename: vi.fn(),
      moveTo: vi.fn(),
      terminal: vi.fn(),
      chat: vi.fn(),
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
    chat: vi.fn(),
    archive: vi.fn(() => Promise.resolve(null)),
    remove: vi.fn(() => Promise.resolve<DeleteRefusal | string | null>(null)),
    liveWork: vi.fn(() => Promise.resolve({ tabs: [], runs: [] })),
    stopLive: vi.fn(() => Promise.resolve(null)),
    archived: vi.fn(),
    problem: vi.fn(),
    ...over,
  })

  const board = (hands: CardBoardActs, onOpen = vi.fn(), over: { step?: Column['step']; onPlay?: () => Promise<null> } = {}) => {
    const column: Column = { id: 'col_1', name: 'Todo', position: 0, step: over.step ?? null, onPass: null, autonomy: 'manual' }
    const lane = { column, cards: [card] } as Lane
    const view = render(
      <LaneCards
        lane={lane}
        drag={drag}
        dropping={false}
        progress={{}}
        onOpen={onOpen}
        onPlay={over.onPlay ?? vi.fn(() => Promise.resolve(null))}
        onRename={vi.fn()}
        others={[{ id: 'col_2', name: 'Doing' }, { id: 'col_3', name: 'Done' }]}
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

  it('archives on Delete once asked, and the board offers undo', async () => {
    const hands = acts()
    fireEvent.keyDown(board(hands), { key: 'Delete' })
    fireEvent.click(await screen.findByRole('button', { name: 'Archive' }))
    await waitFor(() => expect(hands.archived).toHaveBeenCalled())
    expect(hands.archive).toHaveBeenCalledWith(false)
  })

  it('asks on Delete what the menu asks: to stop the work still going, and again when the archive is refused', async () => {
    const archive = vi.fn((force: boolean) => Promise.resolve(force ? null : '3 changes in /w that nothing has saved — archive anyway?'))
    const hands = acts({ archive, liveWork: vi.fn(() => Promise.resolve({ tabs: ['tab_1'], runs: [] })) })
    fireEvent.keyDown(board(hands), { key: 'Delete' })
    fireEvent.click(await screen.findByRole('button', { name: 'Stop it and archive' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Archive anyway' }))
    await waitFor(() => expect(hands.archived).toHaveBeenCalled())
    expect(hands.stopLive).toHaveBeenCalled()
    expect(archive.mock.calls).toEqual([[false], [true]])
    expect(hands.problem).not.toHaveBeenCalled()
  })

  it('leaves Enter and Delete on its play button to the button', async () => {
    const onOpen = vi.fn()
    const onPlay = vi.fn(() => Promise.resolve(null))
    const step = { id: 'step_1', kind: 'command', name: 'tests', config: '{}', irreversible: false } as Column['step']
    const tile = board(acts(), onOpen, { step, onPlay })
    const play = within(tile).getByRole('button', { name: 'Run tests' })
    fireEvent.keyDown(play, { key: 'Enter' })
    fireEvent.keyDown(play, { key: 'Delete' })
    expect(onOpen).not.toHaveBeenCalled()
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Archive' })).toBeNull())
  })

  it('opens the lanes to move to on Ctrl+M', () => {
    fireEvent.keyDown(board(acts()), { key: 'm', ctrlKey: true })
    expect(within(screen.getByRole('menu')).getAllByRole('menuitem').map((item) => item.textContent)).toEqual(['Doing', 'Done'])
  })

  it('puts the keyboard in the lanes on Ctrl+M, walks them with the arrows, and gives it back on Escape', () => {
    const tile = board(acts())
    tile.focus()
    fireEvent.keyDown(tile, { key: 'm', ctrlKey: true })
    expect(document.activeElement?.textContent).toBe('Doing')
    fireEvent.keyDown(document.activeElement!, { key: 'ArrowDown' })
    expect(document.activeElement?.textContent).toBe('Done')
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu')).toBeNull()
    expect(document.activeElement).toBe(tile)
  })
})

describe('the keys a card menu answers to', () => {
  it('walk its entries without wrapping, and jump to either end', () => {
    expect(menuFocus('ArrowDown', 0, 3)).toBe(1)
    expect(menuFocus('ArrowDown', 2, 3)).toBe(2)
    expect(menuFocus('ArrowUp', 0, 3)).toBe(0)
    expect(menuFocus('ArrowDown', -1, 3)).toBe(0)
    expect(menuFocus('Home', 2, 3)).toBe(0)
    expect(menuFocus('End', 0, 3)).toBe(2)
    expect(menuFocus('Enter', 0, 3)).toBeNull()
    expect(menuFocus('ArrowDown', -1, 0)).toBeNull()
  })
})
