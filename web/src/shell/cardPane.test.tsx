import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { CardDetail } from '../gen/bindings'
import { CardPane, escapeMeans } from './CardPane'

afterEach(cleanup)

const detail: CardDetail = {
  card: {
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
  },
  columnName: 'Todo',
  columnStep: null,
  comments: [],
  pinned: [],
  worktree: null,
  runs: [],
  sessions: [],
}

const save = vi.fn()

vi.mock('./useCard', () => ({
  useCard: () => ({
    detail,
    error: null,
    busy: false,
    save,
    setDue: vi.fn(),
    comment: vi.fn(),
    editComment: vi.fn(),
    deleteComment: vi.fn(),
    pin: vi.fn(),
    unpin: vi.fn(),
    archive: vi.fn(() => Promise.resolve(null)),
    remove: vi.fn(() => Promise.resolve(null)),
    reload: vi.fn(),
  }),
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'p1' } }) }))
/* The sections below the description have their own tests and their own commands. */
vi.mock('./CardPlay', () => ({ CardPlay: () => null }))
vi.mock('./CardWork', () => ({ CardWork: () => null }))
vi.mock('./CardDiff', () => ({ CardDiff: () => null }))
vi.mock('./Attachments', () => ({ Attachments: () => null }))
vi.mock('./Comments', () => ({ Comments: () => null }))

const escape = (): void => {
  fireEvent.keyDown(window, { key: 'Escape' })
}

let onClose = vi.fn()
beforeEach(() => {
  save.mockClear()
  onClose = vi.fn()
  render(<CardPane cardId="card_1" onClose={onClose} onChanged={vi.fn()} />)
})

const title = (): HTMLInputElement => screen.getByRole('textbox', { name: 'Title' }) as HTMLInputElement
const backdrop = (): Element => document.querySelector('.cardp')!

describe('what Escape means in an open card', () => {
  it('is decided by what is over the card and what has the keyboard', () => {
    const field = document.createElement('textarea')
    expect(escapeMeans(field, true)).toBe('nothing')
    expect(escapeMeans(field, false)).toBe('blur')
    expect(escapeMeans(document.body, false)).toBe('close')
  })

  it('lets go of a field and keeps the card open', () => {
    title().focus()
    escape()
    expect(onClose).not.toHaveBeenCalled()
    expect(document.activeElement).not.toBe(title())
  })

  it('closes the card when nothing is being typed in', () => {
    escape()
    expect(onClose).toHaveBeenCalled()
  })

  it('closes only the dialog asked over the card', () => {
    fireEvent.click(screen.getByRole('button', { name: 'Card actions' }))
    fireEvent.click(screen.getByRole('menuitem', { name: 'Archive' }))
    expect(screen.getByText('Archive this card?')).toBeTruthy()
    escape()
    expect(screen.queryByText('Archive this card?')).toBeNull()
    expect(onClose).not.toHaveBeenCalled()
  })
})

describe('closing an open card keeps what was typed', () => {
  const typed = (): void => {
    fireEvent.change(title(), { target: { value: 'Wire the whole board' } })
  }
  const savedBeforeClosing = (): void => {
    expect(save).toHaveBeenCalledWith('Wire the whole board', '')
    expect(save.mock.invocationCallOrder[0]!).toBeLessThan(onClose.mock.invocationCallOrder[0]!)
  }

  it('by the close button', () => {
    typed()
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    savedBeforeClosing()
  })

  it('by Escape', () => {
    typed()
    escape()
    savedBeforeClosing()
  })

  it('by a click on the backdrop', () => {
    typed()
    fireEvent.pointerDown(backdrop())
    fireEvent.click(backdrop())
    savedBeforeClosing()
  })
})

describe('the backdrop', () => {
  it('does not close for a press that started inside the card', () => {
    fireEvent.pointerDown(title())
    fireEvent.click(backdrop())
    expect(onClose).not.toHaveBeenCalled()
  })
})
