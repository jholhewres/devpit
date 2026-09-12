import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { KnownApp, OpenApp, Source, WorktreeBase as Base } from '../gen/bindings'
import { OpenApps } from './OpenApps'
import { Sources } from './Sources'
import { WorktreeBase } from './WorktreeBase'

afterEach(cleanup)

let apps: OpenApp[] = []
let base: Base | null = null
let refusal: string | null = null
let sources: Source[] = []

const added = vi.fn()
const removed = vi.fn()
const wrote = vi.fn()
const switched = vi.fn()
const reloaded = vi.fn()

vi.mock('./live', () => ({
  ask: (call: () => unknown) =>
    Promise.resolve(
      refusal
        ? { data: null, error: refusal, loading: false }
        : { data: call(), error: null, loading: false },
    ),
  commands: {
    appsList: () => apps,
    appsKnown: (): KnownApp[] => [
      { id: 'vscode', label: 'VS Code', command: 'code' },
      { id: 'zed', label: 'Zed', command: 'zed' },
    ],
    appsAdd: (label: string, command: string) => {
      added(label, command)
      return apps
    },
    appsRemove: (id: string) => {
      removed(id)
      return apps
    },
    worktreeSources: () => sources,
    worktreeSourceShow: (_p: string | null, id: string, shown: boolean) => {
      switched(id, shown)
      return sources
    },
    worktreeBaseRead: () => base,
    worktreeBaseWrite: (_project: string | null, typed: string) => {
      wrote(typed)
      return base
    },
  },
}))

vi.mock('./useShell', () => ({
  useShell: () => ({ project: { id: 'p1' }, reloadProjects: () => reloaded() }),
}))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: () => Promise.resolve(null) }))

beforeEach(() => {
  apps = []
  base = null
  refusal = null
  sources = []
  switched.mockClear()
  reloaded.mockClear()
  added.mockClear()
  removed.mockClear()
  wrote.mockClear()
})

describe('the apps a folder can be handed to', () => {
  it('says there are none rather than drawing an empty list', async () => {
    render(<OpenApps />)
    expect(await screen.findByText(/No apps yet/)).toBeTruthy()
  })

  it('offers the known ones and adds the one you pick', async () => {
    render(<OpenApps />)
    fireEvent.click(await screen.findByText('Add app'))
    fireEvent.click(screen.getByText('Zed'))
    await waitFor(() => expect(added).toHaveBeenCalledWith('Zed', 'zed'))
  })

  /* Adding the same app twice is a menu that let you. */
  it('marks one already in the list and will not offer it again', async () => {
    apps = [{ id: 'zed', label: 'Zed', command: 'zed', installed: true }]
    render(<OpenApps />)
    fireEvent.click(await screen.findByText('Add app'))
    expect(screen.getByText('Added')).toBeTruthy()
    const zed = screen.getAllByText('Zed').find((node) => node.closest('.apps__opt'))
    expect(zed?.closest('button')).toHaveProperty('disabled', true)
  })

  /* Measured, not assumed: the row says so instead of failing on the click,
     which is the moment it would be least explainable. */
  it('says when a configured app is not on this machine', async () => {
    apps = [{ id: 'zed', label: 'Zed', command: 'zed', installed: false }]
    render(<OpenApps />)
    expect(await screen.findByText('not on this machine')).toBeTruthy()
  })

  it('removes the one you asked to remove', async () => {
    apps = [{ id: 'zed', label: 'Zed', command: 'zed', installed: true }]
    render(<OpenApps />)
    fireEvent.click(await screen.findByText('Remove'))
    await waitFor(() => expect(removed).toHaveBeenCalledWith('zed'))
  })

  it('prints the refusal instead of swallowing it', async () => {
    render(<OpenApps />)
    fireEvent.click(await screen.findByText('Add app'))
    // Set only now: the list itself has to load before there is anything to
    // click, and a refusal from the start would leave the menu empty.
    refusal = 'that has to be one program'
    fireEvent.click(screen.getByText('VS Code'))
    expect(await screen.findByText('that has to be one program')).toBeTruthy()
  })
})

describe('where new worktrees go', () => {
  const answer = (over: Partial<Base> = {}): Base => ({
    typed: '',
    example: '/home/me/.devpit/worktrees/<project>/<card>',
    isDefault: true,
    ...over,
  })

  /* A sentence about relative and absolute is not an answer. A path is. */
  it('shows where the next one would land', async () => {
    base = answer()
    render(<WorktreeBase />)
    expect(await screen.findByText('/home/me/.devpit/worktrees/<project>/<card>')).toBeTruthy()
  })

  /* `!base?.isDefault` is true while the read is still in flight, so Reset
     appeared before anything had been chosen. */
  it('offers no Reset while nothing has been chosen', async () => {
    base = answer()
    render(<WorktreeBase />)
    await screen.findByText(/Next one lands in/)
    expect(screen.queryByText('Reset')).toBeNull()
  })

  it('offers Reset once something has', async () => {
    base = answer({ typed: '.devpit/wt', isDefault: false })
    render(<WorktreeBase />)
    expect(await screen.findByText('Reset')).toBeTruthy()
  })

  it('saves what was typed on Enter', async () => {
    base = answer()
    render(<WorktreeBase />)
    const field = await screen.findByLabelText('Worktree folder')
    fireEvent.change(field, { target: { value: '.devpit/wt' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await waitFor(() => expect(wrote).toHaveBeenCalledWith('.devpit/wt'))
  })

  /* A field that clears itself on a refusal makes you retype the whole thing
     to find out what was wrong with it. */
  it('keeps what was typed when the backend refuses it', async () => {
    base = answer()
    render(<WorktreeBase />)
    const field = await screen.findByLabelText('Worktree folder')
    refusal = 'a worktree cannot live inside `.git`'
    fireEvent.change(field, { target: { value: '.git/wt' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(await screen.findByText('a worktree cannot live inside `.git`')).toBeTruthy()
    expect((field as HTMLInputElement).value).toBe('.git/wt')
  })
})

describe('which checkouts the lists are about', () => {
  const source = (over: Partial<Source> = {}): Source => ({
    id: 'claude',
    label: 'Claude Code',
    hint: '.claude/worktrees',
    count: 3,
    shown: true,
    ...over,
  })

  it('draws nothing at all until it has an answer', () => {
    const { container } = render(<Sources />)
    expect(container.innerHTML).toBe('')
  })

  /* The count is what makes the switch meaningful: hiding a kind the project
     has none of is a control that does nothing, and the number says so. */
  it('says how many each kind has', async () => {
    sources = [source(), source({ id: 'other', label: 'Other locations', count: 0 })]
    render(<Sources />)
    expect(await screen.findByText('3')).toBeTruthy()
    expect(screen.getByText('0')).toBeTruthy()
  })

  it('marks the half that is in force', async () => {
    sources = [source({ shown: false })]
    render(<Sources />)
    const hide = await screen.findByText('Hide')
    expect(hide.getAttribute('aria-checked')).toBe('true')
    expect(screen.getByText('Show').getAttribute('aria-checked')).toBe('false')
  })

  it('hides the kind you asked to hide', async () => {
    sources = [source()]
    render(<Sources />)
    fireEvent.click(await screen.findByText('Hide'))
    await waitFor(() => expect(switched).toHaveBeenCalledWith('claude', false))
  })

  /* The project row counts the same worktrees. Without this it keeps the old
     number until something unrelated happens to reload the list. */
  it('tells the project list its counts just changed', async () => {
    sources = [source()]
    render(<Sources />)
    fireEvent.click(await screen.findByText('Hide'))
    await waitFor(() => expect(reloaded).toHaveBeenCalled())
  })
})
