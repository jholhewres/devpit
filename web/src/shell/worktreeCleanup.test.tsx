import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { CleanupRun, WorktreeCleanup as Cleanup } from '../gen/bindings'
import { WorktreeCleanup } from './WorktreeCleanup'

const GB = 1024 * 1024 * 1024
const ran: CleanupRun = {
  at: Date.now() / 1000,
  removed: [{ cardId: 'card_1', title: 'Merged one', why: 'landed', bytes: 1.5 * GB, detail: null }],
  kept: [{ cardId: 'card_2', title: 'Busy one', why: 'running', bytes: 0, detail: null }],
  branches: 1,
  freedBytes: 1.5 * GB,
}

let stored: Cleanup = { deleteMerged: true, last: null }
const write = vi.fn(async (deleteMerged: boolean) => (stored = { ...stored, deleteMerged }))
const now = vi.fn(async () => (stored = { ...stored, last: ran }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    worktreeCleanupRead: async () => stored,
    worktreeCleanupWrite: (deleteMerged: boolean) => write(deleteMerged),
    worktreeCleanupNow: () => now(),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  stored = { deleteMerged: true, last: null }
})

describe('worktree cleanup in Settings', () => {
  it('is on unless turned off, and turns off', async () => {
    render(<WorktreeCleanup />)
    const toggle = await screen.findByRole('switch', { name: 'Delete merged worktrees' })
    await waitFor(() => expect(toggle.getAttribute('aria-checked')).toBe('true'))
    fireEvent.click(toggle)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith(false))
  })

  it('deletes now and says what went, what stayed and the GB freed', async () => {
    render(<WorktreeCleanup />)
    fireEvent.click(await screen.findByRole('button', { name: 'Delete now' }))
    await waitFor(() => expect(now).toHaveBeenCalled())
    expect(await screen.findByText('Last run just now: removed 1, kept 1, freed 1.5 GB.')).toBeTruthy()
    expect(screen.getByText(/Kept Busy one: a session is running in it/)).toBeTruthy()
  })
})
