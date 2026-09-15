import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Board } from '../gen/bindings'
import type { Lane } from './board'
import { wired } from './fileMenu'
import { LaneHead } from './Lane'
import { laneMenu } from './laneMenu'
import { shifted } from './laneOrder'
import { useBoard } from './useBoard'

afterEach(cleanup)

const reordered = vi.fn()

const board = (): Board => ({
  projectId: 'p1',
  columns: ['todo', 'doing', 'done'].map((id, position) => ({
    id,
    name: id,
    position,
    step: null,
    onPass: null,
    autonomy: 'manual',
  })),
  cards: [],
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    boardGet: () => board(),
    columnReorder: (_project: string, ids: string[]) => {
      reordered(ids)
      return board()
    },
  },
}))
vi.mock('./window', () => ({ onCarried: () => () => undefined }))

const hands = { rename: vi.fn(), addCard: vi.fn(), shift: vi.fn(), remove: vi.fn() }

describe('the lane menu', () => {
  it('has something behind every entry', () => {
    expect(wired(laneMenu(hands))).toBe(true)
  })

  it('would catch an entry that only looks like a control', () => {
    expect(wired([...laneMenu(hands), { label: 'Does nothing' }])).toBe(false)
  })
})

describe('moving a lane a place', () => {
  const ids = ['todo', 'doing', 'done']

  it('swaps it with its neighbour', () => {
    expect(shifted(ids, 'doing', -1)).toEqual(['doing', 'todo', 'done'])
    expect(shifted(ids, 'doing', 1)).toEqual(['todo', 'done', 'doing'])
  })

  it('leaves the list alone at an edge or for a lane it does not have', () => {
    expect(shifted(ids, 'todo', -1)).toBe(ids)
    expect(shifted(ids, 'done', 1)).toBe(ids)
    expect(shifted(ids, 'gone', 1)).toBe(ids)
  })

  it('sends the shifted order to the backend', async () => {
    const { result } = renderHook(() => useBoard('p1'))
    await waitFor(() => expect(result.current.lanes).toHaveLength(3))
    act(() => result.current.shiftColumn('doing', 1))
    await waitFor(() => expect(reordered).toHaveBeenCalledWith(['todo', 'done', 'doing']))
  })
})

describe('the lane head', () => {
  const lane = { column: board().columns[1], cards: [] } as unknown as Lane
  const head = (onShift = vi.fn()) =>
    render(
      <LaneHead
        lane={lane}
        steps={[]}
        onRename={vi.fn()}
        onPickStep={vi.fn()}
        onCreateStep={vi.fn()}
        others={[]}
        onFlow={vi.fn()}
        onAddCard={vi.fn()}
        onShift={onShift}
        onDelete={vi.fn(() => Promise.resolve(null))}
      />,
    )

  it('does not edit the name on a single click, and does on a double click', () => {
    head()
    const name = screen.getByRole('textbox', { name: 'doing name' })
    fireEvent.click(name)
    expect(name.getAttribute('contenteditable')).toBe('false')
    fireEvent.doubleClick(name)
    expect(name.getAttribute('contenteditable')).toBe('true')
  })

  it('offers the lane menu on a right-click, and moves the lane from it', () => {
    const onShift = vi.fn()
    const { container } = head(onShift)
    fireEvent.contextMenu(container.querySelector('.blane__top')!)
    const items = screen.getAllByRole('menuitem').map((item) => item.textContent)
    expect(items).toEqual(['Rename', 'Add card', 'Move left', 'Move right', 'Delete…'])
    fireEvent.click(screen.getByRole('menuitem', { name: 'Move right' }))
    expect(onShift).toHaveBeenCalledWith(1)
  })

  it('renames from the menu too', () => {
    head()
    fireEvent.click(screen.getByRole('button', { name: 'doing actions' }))
    fireEvent.click(screen.getByRole('menuitem', { name: 'Rename' }))
    expect(screen.getByRole('textbox', { name: 'doing name' }).getAttribute('contenteditable')).toBe('true')
  })
})
