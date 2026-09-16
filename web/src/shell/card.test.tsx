import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { useState } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Card, Checkout, Comment, Notice, Notices as Rung, Pinned } from '../gen/bindings'
import { drawingTab } from '../plugins/excalidraw/drawings'
import { Attachments } from './Attachments'
import { Comments } from './Comments'
import { Notices } from './Notices'
import { Tile } from './Lane'

afterEach(cleanup)

let bell: Rung = { notices: [], unread: 0 }
const marked = vi.fn()
const markedAll = vi.fn()
const revealed = vi.fn()

vi.mock('./live', () => ({
  ask: (call: () => unknown) =>
    Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    pathReveal: (path: string) => {
      revealed(path)
      return null
    },
    appsList: () => [],
    noticesRead: () => bell,
    noticesSweepDue: () => bell,
    noticesMark: (id: string) => {
      marked(id)
      return bell
    },
    noticesMarkAll: () => {
      markedAll()
      return bell
    },
  },
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: () => Promise.resolve(null) }))

/* `Markdown` reaches for the shell to resolve a link against the project.
   These render one comment, not a window. */
const shell = { project: { id: 'p1' }, show: vi.fn() }
vi.mock('./useShell', () => ({ useShell: () => shell }))

let drawingsOn = true
vi.mock('./usePlugins', () => ({
  usePlugins: () => ({ offers: (kind: string) => kind !== 'drawing' || drawingsOn }),
}))

const card = (over: Partial<Card> = {}): Card => ({
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
  ...over,
})

describe('what a tile says', () => {
  it('says nothing it does not know', () => {
    render(<Tile card={card()} />)
    expect(screen.queryByTitle(/comments/)).toBeNull()
    expect(screen.queryByTitle(/files/)).toBeNull()
  })

  /* Counted on the card rather than carried: a board of thirty cards would
     otherwise read thirty conversations to draw thirty badges. */
  it('counts the conversation and the files', () => {
    render(<Tile card={card({ comments: 3, pinned: 2 })} />)
    expect(screen.getByTitle('3 comments')).toBeTruthy()
    expect(screen.getByTitle('2 files')).toBeTruthy()
  })

  /* A colour and a phrase, never a badge shouting: the board is the person's
     own, and a date that passed is information, not a telling-off. */
  it('marks a deadline by how near it is', () => {
    const past = new Date()
    past.setDate(past.getDate() - 3)
    const { container } = render(<Tile card={card({ dueAt: past.getTime() / 1000 })} />)
    expect(container.querySelector('[data-near="past"]')).toBeTruthy()
  })
})

describe('the card conversation', () => {
  const said = (over: Partial<Comment> = {}): Comment => ({
    id: 'cmt_1',
    author: 'you',
    body: 'a thought',
    createdAt: Date.now() / 1000 - 60,
    editedAt: null,
    ...over,
  })

  it('says so when nothing has been said', () => {
    render(<Comments comments={[]} onSay={vi.fn()} onEdit={vi.fn()} onDelete={vi.fn()} />)
    expect(screen.getByText('Nothing said yet.')).toBeTruthy()
  })

  it('resolves the author id to a name', () => {
    render(
      <Comments comments={[said({ author: 'claude' })]} onSay={vi.fn()} onEdit={vi.fn()} onDelete={vi.fn()} />,
    )
    expect(screen.getByText('Claude Code')).toBeTruthy()
  })

  /* A line that can change under you with no mark is a line you cannot rely
     on, so the edit leaves one. */
  it('marks a line that was changed after it was said', () => {
    render(
      <Comments comments={[said({ editedAt: Date.now() / 1000 })]} onSay={vi.fn()} onEdit={vi.fn()} onDelete={vi.fn()} />,
    )
    expect(screen.getByText('edited')).toBeTruthy()
  })

  it('keeps what was typed when the write is refused', async () => {
    const onSay = vi.fn(() => Promise.resolve('an empty comment is not a comment'))
    render(<Comments comments={[]} onSay={onSay} onEdit={vi.fn()} onDelete={vi.fn()} />)
    const field = screen.getByPlaceholderText(/Say something/)
    fireEvent.change(field, { target: { value: 'mine' } })
    fireEvent.click(screen.getByText('Comment'))
    await waitFor(() => expect(screen.getByText(/not a comment/)).toBeTruthy())
    expect((field as HTMLTextAreaElement).value).toBe('mine')
  })

  it('clears the field once the write lands', async () => {
    const onSay = vi.fn(() => Promise.resolve(null))
    render(<Comments comments={[]} onSay={onSay} onEdit={vi.fn()} onDelete={vi.fn()} />)
    const field = screen.getByPlaceholderText(/Say something/)
    fireEvent.change(field, { target: { value: 'mine' } })
    fireEvent.click(screen.getByText('Comment'))
    await waitFor(() => expect((field as HTMLTextAreaElement).value).toBe(''))
    expect(onSay).toHaveBeenCalledWith('mine')
  })
})

describe('the files pinned to a card', () => {
  const pin = (over: Partial<Pinned> = {}): Pinned => ({
    id: 'att_1',
    path: '/home/me/project/notes.md',
    label: 'notes.md',
    exists: true,
    bytes: 2048,
    createdAt: Date.now() / 1000,
    plugin: null,
    ...over,
  })

  beforeEach(() => {
    drawingsOn = true
    shell.show.mockClear()
    revealed.mockClear()
  })

  it('explains that nothing is copied', () => {
    render(<Attachments pinned={[]} onPin={vi.fn()} onUnpin={vi.fn()} />)
    expect(screen.getByText(/nothing is copied/)).toBeTruthy()
  })

  /* A pin is a path, so it can stop being true. Dropping it would take the
     memory of it too — and the memory is sometimes the whole point. */
  it('draws a pin whose file has moved rather than dropping it', () => {
    render(<Attachments pinned={[pin({ exists: false, bytes: null })]} onPin={vi.fn()} onUnpin={vi.fn()} />)
    expect(screen.getByText('notes.md')).toBeTruthy()
    expect(screen.getByText('not where it was')).toBeTruthy()
    // Nothing to reveal, because there is nothing there.
    expect(screen.queryByText('Reveal')).toBeNull()
  })

  it('unpins the one asked for', () => {
    const onUnpin = vi.fn()
    render(<Attachments pinned={[pin()]} onPin={vi.fn()} onUnpin={onUnpin} />)
    fireEvent.click(screen.getByText('Unpin'))
    expect(onUnpin).toHaveBeenCalledWith('att_1')
  })

  const drawn = pin({
    id: 'att_2',
    path: '/ws/projects/demo-1/data/excalidraw/flow.excalidraw',
    label: 'flow',
    plugin: 'excalidraw',
  })

  it('opens a drawing its plugin pinned in the drawing tab while the plugin is on', () => {
    render(<Attachments pinned={[drawn]} onPin={vi.fn()} onUnpin={vi.fn()} />)
    fireEvent.click(screen.getByText('Open'))
    expect(shell.show).toHaveBeenCalledWith('drawing', drawingTab('flow.excalidraw'))
    expect(screen.queryByText('Reveal')).toBeNull()
  })

  it('reveals that drawing in its folder once the plugin is off', () => {
    drawingsOn = false
    render(<Attachments pinned={[drawn]} onPin={vi.fn()} onUnpin={vi.fn()} />)
    expect(screen.queryByText('Open')).toBeNull()
    fireEvent.click(screen.getByText('Reveal'))
    expect(revealed).toHaveBeenCalledWith(drawn.path)
    expect(shell.show).not.toHaveBeenCalled()
  })

  it('keeps a file no plugin pinned in its folder', () => {
    render(<Attachments pinned={[pin({ path: '/p/flow.excalidraw' })]} onPin={vi.fn()} onUnpin={vi.fn()} />)
    expect(screen.queryByText('Open')).toBeNull()
    expect(screen.getByText('Reveal')).toBeTruthy()
  })
})

describe('the bell', () => {
  const notice = (over: Partial<Notice> = {}): Notice => ({
    id: 'ntc_1',
    projectId: null,
    kind: 'run',
    title: 'tests finished on “Wire the board”',
    detail: null,
    cardId: 'card_1',
    createdAt: Date.now() / 1000 - 30,
    readAt: null,
    ...over,
  })

  beforeEach(() => {
    bell = { notices: [], unread: 0 }
    marked.mockClear()
    markedAll.mockClear()
  })

  /* The top bar owns the bell's state now, so one reader serves the panel and
     the focus pill. The tests supply it the same way the top bar does. */
  function Bell({ onOpenCard }: { onOpenCard?: (cardId: string) => void }): React.JSX.Element {
    const [open, setOpen] = useState(false)
    return (
      <Notices
        bell={{ ...bell, waiting: [], markRead: marked, markAllRead: markedAll, reload: () => {} }}
        open={open}
        setOpen={setOpen}
        onOpenCard={onOpenCard}
      />
    )
  }

  /* One key to whatever has been waiting longest, in the order somebody
     between five agents wants. It opens the card the notice points at, which
     is what every other route into a notice does. */
  it('goes to the next thing that needs you on the key', async () => {
    const onOpenCard = vi.fn()
    bell = {
      notices: [
        notice({ id: 'ntc_run', kind: 'run', cardId: 'card_run' }),
        notice({ id: 'ntc_agent', kind: 'agent', cardId: 'card_agent' }),
      ],
      unread: 2,
    }
    render(<Bell onOpenCard={onOpenCard} />)
    await waitFor(() => expect(screen.getByLabelText('2 unread')).toBeTruthy())

    fireEvent.keyDown(window, { key: 'N', shiftKey: true, metaKey: true })
    await waitFor(() => expect(onOpenCard).toHaveBeenCalledWith('card_agent'))
    expect(marked).toHaveBeenCalledWith('ntc_agent')
  })

  it('shows no count when there is nothing unread', async () => {
    bell = { notices: [notice({ readAt: Date.now() / 1000 })], unread: 0 }
    const { container } = render(<Bell />)
    await waitFor(() => expect(container.querySelector('.bell__b')).toBeTruthy())
    expect(container.querySelector('.bell__n')).toBeNull()
  })

  it('counts what is unread', async () => {
    bell = { notices: [notice()], unread: 2 }
    render(<Bell />)
    expect(await screen.findByLabelText('2 unread')).toBeTruthy()
  })

  /* Past a point the exact figure stops being information and becomes a wall
     of digits. */
  it('caps the count', async () => {
    bell = { notices: [notice()], unread: 348 }
    render(<Bell />)
    fireEvent.click(await screen.findByLabelText('348 unread'))
    expect(screen.getByText('99+')).toBeTruthy()
  })

  it('opens the card a notice is about, and marks it read', async () => {
    bell = { notices: [notice()], unread: 1 }
    const onOpenCard = vi.fn()
    render(<Bell onOpenCard={onOpenCard} />)
    fireEvent.click(await screen.findByLabelText('1 unread'))
    fireEvent.click(screen.getByText(/tests finished/))
    expect(marked).toHaveBeenCalledWith('ntc_1')
    expect(onOpenCard).toHaveBeenCalledWith('card_1')
  })

  /* Read, never cleared: opening the panel marks nothing, because a list that
     empties itself is a list where the thing you meant to come back to is
     gone. */
  it('marks nothing merely by being opened', async () => {
    bell = { notices: [notice()], unread: 1 }
    render(<Bell />)
    fireEvent.click(await screen.findByLabelText('1 unread'))
    expect(marked).not.toHaveBeenCalled()
    expect(markedAll).not.toHaveBeenCalled()
  })

  it('marks them all when asked', async () => {
    bell = { notices: [notice()], unread: 1 }
    render(<Bell />)
    fireEvent.click(await screen.findByLabelText('1 unread'))
    fireEvent.click(screen.getByText('Mark all read'))
    expect(markedAll).toHaveBeenCalled()
  })

  it('draws a kind it does not know rather than failing', async () => {
    bell = { notices: [notice({ kind: 'something-newer', title: 'a new thing' })], unread: 1 }
    render(<Bell />)
    fireEvent.click(await screen.findByLabelText('1 unread'))
    expect(screen.getByText('a new thing')).toBeTruthy()
  })
})

/* The shape the card pane needs, asserted so a contract change that drops one
   of these is a failing test rather than a blank section. */
describe('what a card carries', () => {
  it('has somewhere for each of them', () => {
    const full: Checkout = {
      path: '/tmp/wt',
      branch: 'card/wire-the-board',
      baseRef: 'abc123',
      dirtyFiles: 2,
      exists: true,
    }
    expect(full.branch).toBe('card/wire-the-board')
    expect(card({ dueAt: 1, comments: 1, pinned: 1 }).dueAt).toBe(1)
  })
})
