import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Change, Project } from '../gen/bindings'
import { Changes } from './ChangesPanel'
import type { UseTree } from './useTree'

afterEach(cleanup)

const staged = vi.fn()
const unstaged = vi.fn()
const committed = vi.fn()

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    changesStage: (_p: string, _w: null, paths: string[]) => (staged(paths), null),
    changesUnstage: (_p: string, _w: null, paths: string[]) => (unstaged(paths), null),
    changesCommit: (_p: string, _w: null, message: string) => {
      committed(message)
      return { sha: 'abc1234', subject: message }
    },
  },
}))

const project = { id: 'p', name: 'devpit', worktrees: [] } as unknown as Project
vi.mock('./useShell', () => ({ useShell: () => ({ project, show: vi.fn() }) }))

const change = (path: string, over: Partial<Change> = {}): Change => ({
  path,
  status: 'modified',
  added: 1,
  removed: 0,
  staged: false,
  ...over,
})

const tree = (changes: readonly Change[], totals = { added: 0, removed: 0 }): UseTree =>
  ({
    changes,
    totals,
    loading: false,
    nodes: [],
    error: null,
    version: 1,
    reload: vi.fn(),
  }) as unknown as UseTree

beforeEach(() => {
  staged.mockClear()
  unstaged.mockClear()
  committed.mockClear()
})

/*
 * The panel used to offer three controls for one decision: `Stage all` and
 * `Unstage all` at the top, and a Commit button at the far bottom of a list
 * forty rows long. The button you needed was either competing with two you did
 * not, or off the screen.
 */

describe('the one button at the top', () => {
  it('offers to stage while the index is empty', () => {
    render(<Changes tree={tree([change('a.rs'), change('b.rs')])} />)
    fireEvent.click(screen.getByText('Stage All'))
    expect(staged).toHaveBeenCalledWith(['a.rs', 'b.rs'])
  })

  it('turns into a commit once something is staged', async () => {
    render(<Changes tree={tree([change('a.rs', { staged: true })])} />)
    const button = screen.getByText('Commit 1') as HTMLButtonElement
    expect(button.disabled).toBe(true)

    fireEvent.change(screen.getByLabelText('Commit message'), {
      target: { value: 'why it changed' },
    })
    fireEvent.click(screen.getByText('Commit 1'))
    await waitFor(() => expect(committed).toHaveBeenCalledWith('why it changed'))
  })

  it('says why it will not go', () => {
    render(<Changes tree={tree([change('a.rs', { staged: true })])} />)
    expect(screen.getByText('Say what changed, and why.')).toBeTruthy()
  })

  it('is above the list, not under it', () => {
    // Forty rows between the field and the button is the whole reason.
    const { container } = render(<Changes tree={tree([change('a.rs')])} />)
    const head = container.querySelector('.git__head')!
    const body = container.querySelector('.git__body')!
    expect(head.compareDocumentPosition(body) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(head.querySelector('.git__msg')).toBeTruthy()
  })
})

describe('each section acts on itself', () => {
  it('unstages the staged ones from their own heading', () => {
    // It used to be a chip at the top that applied to a group two screens down.
    render(<Changes tree={tree([change('a.rs', { staged: true }), change('b.rs')])} />)
    fireEvent.click(screen.getByLabelText('Unstage everything staged'))
    expect(unstaged).toHaveBeenCalledWith(['a.rs'])
  })

  it('keeps untracked apart from changed', () => {
    // Collapsing them is how a new file gets left out of a commit nobody
    // noticed was missing it.
    render(<Changes tree={tree([change('a.rs'), change('new.rs', { status: 'untracked' })])} />)
    expect(screen.getByText('Changed')).toBeTruthy()
    expect(screen.getByText('Untracked')).toBeTruthy()
  })
})

describe('the totals', () => {
  it('groups thousands, so six thousand does not read as six hundred', () => {
    render(<Changes tree={tree([change('a.rs')], { added: 6624, removed: 241 })} />)
    expect(screen.getByText('+6,624')).toBeTruthy()
    expect(screen.getByText('−241')).toBeTruthy()
  })
})
