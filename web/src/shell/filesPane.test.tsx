import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Project, WorkspaceEntry, WorkspaceListing } from '../gen/bindings'
import { FilesPane } from './FilesPane'

afterEach(cleanup)

const listed = vi.fn()
const revealed = vi.fn()

/* What the fake backend holds, by folder. The panel asks for `null` the first
   time — "wherever this project's files are" — which is a question only the
   backend can answer, so the fake has to answer it too. */
const FOLDERS: Record<string, readonly WorkspaceEntry[]> = {
  'projects/p': [
    { name: 'sessions', path: 'projects/p/sessions', isDir: true, bytes: 0, modified: 0, count: 2 },
    { name: 'prime.json', path: 'projects/p/prime.json', isDir: false, bytes: 2048, modified: 0, count: null },
  ],
  'projects/p/sessions': [
    { name: 'one.jsonl', path: 'projects/p/sessions/one.jsonl', isDir: false, bytes: 10, modified: 0, count: null },
  ],
  '': [{ name: 'agents', path: 'agents', isDir: true, bytes: 0, modified: 0, count: 9 }],
}

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    workspaceList: (_project: string | null, path: string | null): WorkspaceListing => {
      const at = path ?? 'projects/p'
      listed(at)
      return {
        root: '/home/me/.devpit',
        path: at,
        entries: [...(FOLDERS[at] ?? [])],
        places: [
          { label: 'Sessions', path: 'projects/p' },
          { label: 'Agents', path: 'agents' },
        ],
      }
    },
    workspaceFile: () => ({
      fullPath: '/home/me/.devpit/projects/p/prime.json',
      path: 'projects/p/prime.json',
      text: '{ "a": 1 }',
      notShown: null,
      bytes: 2048,
      kind: 'text',
      dataUrl: null,
      readAt: 0,
    }),
    pathReveal: (path: string) => (revealed(path), null),
    pathOpen: () => null,
  },
}))

const project = { id: 'p', name: 'demos' } as unknown as Project
vi.mock('./useShell', () => ({ useShell: () => ({ project, close: vi.fn() }) }))

beforeEach(() => {
  listed.mockClear()
  revealed.mockClear()
})

/*
 * The panel drew the project's checkout, which the Explorer beside it already
 * draws. What it shows now is the devpit workspace — the half with no reader.
 */

describe('the files panel browses the workspace', () => {
  it('opens where this project keeps its own files, not at a list of ULIDs', async () => {
    render(<FilesPane />)
    // `null` is the panel asking the backend where that is: it cannot know
    // without a round trip whether the folder has been made yet.
    await waitFor(() => expect(listed).toHaveBeenCalledWith('projects/p'))
    expect(await screen.findByText('prime.json')).toBeTruthy()
  })

  it('reads the project folder by its name in the crumbs', async () => {
    render(<FilesPane />)
    // The folder on disk is `projects/p`; `p` here stands for a ULID.
    await waitFor(() => expect(screen.getByText('demos')).toBeTruthy())
    expect(screen.getByText('Workspace')).toBeTruthy()
  })

  it('says what a row weighs, and what is in a folder', async () => {
    render(<FilesPane />)
    expect(await screen.findByText('2 KB')).toBeTruthy()
    expect(screen.getByText('2 items')).toBeTruthy()
  })

  it('goes into a folder on a click, and back up from the crumbs', async () => {
    render(<FilesPane />)
    fireEvent.click(await screen.findByText('sessions'))
    expect(await screen.findByText('one.jsonl')).toBeTruthy()

    fireEvent.click(screen.getByText('Workspace'))
    expect(await screen.findByText('agents')).toBeTruthy()
  })

  it('previews a file beside the list rather than opening a tab', async () => {
    render(<FilesPane />)
    fireEvent.click(await screen.findByText('prime.json'))
    // Read-only on purpose: these are records of what the product did.
    expect(await screen.findByLabelText('Close preview')).toBeTruthy()
    expect(screen.queryByText('Save')).toBeNull()
  })

  it('filters by name without leaving the folder', async () => {
    render(<FilesPane />)
    await screen.findByText('prime.json')
    fireEvent.change(screen.getByLabelText('Filter files'), { target: { value: 'prime' } })
    await waitFor(() => expect(screen.queryByText('sessions')).toBeNull())
    expect(screen.getByText('prime.json')).toBeTruthy()
    // Filtering is not navigation: no second read of the folder.
    expect(listed).toHaveBeenCalledTimes(1)
  })

  it('says where on disk this is', async () => {
    render(<FilesPane />)
    expect(await screen.findByText('/home/me/.devpit/projects/p')).toBeTruthy()
  })

  it('hands the finder the folder being looked at', async () => {
    render(<FilesPane />)
    await screen.findByText('prime.json')
    fireEvent.click(screen.getByLabelText('Show this folder in the finder'))
    await waitFor(() => expect(revealed).toHaveBeenCalledWith('/home/me/.devpit/projects/p'))
  })

  it('offers the shortcuts the backend says exist', async () => {
    render(<FilesPane />)
    fireEvent.click(await screen.findByText('Agents'))
    await waitFor(() => expect(listed).toHaveBeenCalledWith('agents'))
  })
})
