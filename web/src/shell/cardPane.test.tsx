import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
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

const save = vi.fn((_title: string, _body: string) => Promise.resolve<string | null>(null))

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
vi.mock('./CardSessions', () => ({ CardSessions: () => null }))
vi.mock('./CardDiff', () => ({ CardDiff: () => null }))
vi.mock('./Attachments', () => ({ Attachments: () => null }))
vi.mock('./Comments', () => ({ Comments: () => <div>comments</div> }))
vi.mock('./Markdown', () => ({ Markdown: ({ source }: { source: string }) => <div>{source}</div> }))

const escape = (): void => {
  fireEvent.keyDown(window, { key: 'Escape' })
}

let onClose = vi.fn()
let unmount = (): void => undefined
beforeEach(() => {
  save.mockClear()
  onClose = vi.fn()
  unmount = render(<CardPane cardId="card_1" onClose={onClose} onChanged={vi.fn()} />).unmount
})

const title = (): HTMLInputElement => screen.getByRole('textbox', { name: 'Title' }) as HTMLInputElement
const backdrop = (): Element => document.querySelector('.cardp')!

describe('what Escape means in an open card', () => {
  it('is decided by what is over the card and what has the keyboard', () => {
    const field = document.createElement('textarea')
    expect(escapeMeans(field, true)).toBe('nothing')
    expect(escapeMeans(field, false)).toBe('blur')
    expect(escapeMeans(document.body, false)).toBe('close')
    expect(escapeMeans(document.body, false, true)).toBe('nothing')
  })

  it("closes the card's menu, not the card", () => {
    fireEvent.click(screen.getByRole('button', { name: 'Card actions' }))
    escape()
    expect(onClose).not.toHaveBeenCalled()
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('still closes the card past a menu that is in the page but hidden', () => {
    const hidden = document.body.appendChild(document.createElement('div'))
    hidden.setAttribute('role', 'menu')
    hidden.hidden = true
    escape()
    hidden.remove()
    expect(onClose).toHaveBeenCalled()
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
  const savedBeforeClosing = async (): Promise<void> => {
    await waitFor(() => expect(onClose).toHaveBeenCalled())
    expect(save).toHaveBeenCalledWith('Wire the whole board', '')
    expect(save.mock.invocationCallOrder[0]!).toBeLessThan(onClose.mock.invocationCallOrder[0]!)
  }

  it('by the close button', async () => {
    typed()
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    await savedBeforeClosing()
  })

  it('by Escape', async () => {
    typed()
    escape()
    await savedBeforeClosing()
  })

  it('by a click on the backdrop', async () => {
    typed()
    fireEvent.pointerDown(backdrop())
    fireEvent.click(backdrop())
    await savedBeforeClosing()
  })

  it('and stays open when the write is refused, with the words still in the field', async () => {
    save.mockResolvedValueOnce('the card is gone').mockResolvedValueOnce('the card is gone')
    typed()
    fireEvent.blur(title())
    await act(async () => undefined)
    expect(title().value).toBe('Wire the whole board')
    expect(screen.queryByRole('status')).toBeNull()
    escape()
    await act(async () => undefined)
    expect(save).toHaveBeenCalledTimes(2)
    expect(onClose).not.toHaveBeenCalled()
  })

  it('without calling newer words saved when an older write lands', async () => {
    let land = (_refused: string | null): void => undefined
    save.mockImplementationOnce(() => new Promise((resolve) => (land = resolve)))
    typed()
    fireEvent.blur(title())
    fireEvent.change(title(), { target: { value: 'Wire the whole board, twice' } })
    await act(async () => land(null))
    expect(screen.queryByRole('status')).toBeNull()
    expect(title().value).toBe('Wire the whole board, twice')
  })

  it('when it goes without a blur, hidden with the board', () => {
    typed()
    unmount()
    expect(save).toHaveBeenCalledWith('Wire the whole board', '')
  })
})

describe('the backdrop', () => {
  it('does not close for a press that started inside the card', () => {
    fireEvent.pointerDown(title())
    fireEvent.click(backdrop())
    expect(onClose).not.toHaveBeenCalled()
  })
})

describe('the open card, laid out', () => {
  it('keeps what the card is in the main column and what is done with it beside', () => {
    const main = document.querySelector('.cardp__main')!
    const side = document.querySelector('.cardp__side')!
    expect(main.contains(title())).toBe(true)
    expect(main.textContent).toContain('comments')
    expect(side.textContent).toContain('Due')
  })

  it('says Saved where the Save button was, once what was written is kept', async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Edit the description' }))
    const field = screen.getByRole('textbox', { name: 'Description' })
    fireEvent.change(field, { target: { value: 'All of it' } })
    expect(screen.queryByRole('status')).toBeNull()
    fireEvent.blur(field)
    expect(save).toHaveBeenCalledWith('Wire the board', 'All of it')
    expect((await screen.findByRole('status')).textContent).toBe('Saved')
    expect(screen.queryByRole('button', { name: 'Save' })).toBeNull()
  })
})
