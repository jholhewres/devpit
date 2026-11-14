import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, expect, it, vi } from 'vitest'

import type { Thread } from '../gen/bindings'
import { Threads } from './Threads'

const thread = (id: string, title: string): Thread =>
  ({ id, title, profile: 'claude', model: null, costUsd: 0, lastAt: 1_800_000_000, cwd: null, marks: {} }) as Thread

/* What chat.list answers, which leaves archived conversations out. */
let listed = [thread('c1', 'Fix the parser'), thread('c2', 'Old spike')]
const shell = { project: { id: 'p1' }, show: vi.fn(), open: [{ id: 'c2', kind: 'chat' }] }

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { chatList: async () => ({ conversations: listed }) },
}))
vi.mock('./useShell', () => ({ useShell: () => shell }))
vi.mock('./SessionSearch', () => ({ SessionSearch: () => null }))
vi.mock('./OutsideThreads', () => ({ OutsideThreads: () => null }))

afterEach(cleanup)

it('Earlier reads the list again when a tab closes, so one archived there stays out', async () => {
  const { rerender } = render(<Threads />)
  fireEvent.click(screen.getByRole('button', { name: /Earlier/ }))
  await waitFor(() => expect(screen.getByText('Fix the parser')).toBeTruthy())
  /* c2 is archived and its tab closes. */
  listed = [thread('c1', 'Fix the parser')]
  shell.open = []
  rerender(<Threads />)
  await new Promise((done) => setTimeout(done, 0))
  expect(screen.queryByText('Old spike')).toBeNull()
})
