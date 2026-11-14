import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { WorktreeBranch as Branch } from '../gen/bindings'
import { WorktreeBranch } from './WorktreeBranch'

const DEFAULT = '{type}/{number}-{slug}'
let stored: Branch = { typed: '', default: DEFAULT, example: 'fix/42-the-login-redirect' }
const write = vi.fn(async (typed: string) => {
  if (!typed.includes('{')) throw new Error('the name needs {number}, {slug} or {id}')
  return (stored = { ...stored, typed, example: 'devpit/fix-the-login-redirect-01hzxy9k' })
})
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (err) {
      return { data: null, error: (err as Error).message, loading: false }
    }
  },
  commands: {
    worktreeBranchRead: async () => stored,
    worktreeBranchWrite: (typed: string) => write(typed),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  stored = { typed: '', default: DEFAULT, example: 'fix/42-the-login-redirect' }
})

describe('branch name in Settings', () => {
  it('shows the default and the branch it gives', async () => {
    render(<WorktreeBranch />)
    const field = await screen.findByRole('textbox', { name: 'Branch name' })
    await waitFor(() => expect(field.getAttribute('placeholder')).toBe(DEFAULT))
    expect(screen.getByText('fix/42-the-login-redirect')).toBeTruthy()
  })

  it('changes the pattern and shows the new example', async () => {
    render(<WorktreeBranch />)
    const field = await screen.findByRole('textbox', { name: 'Branch name' })
    fireEvent.change(field, { target: { value: 'devpit/{slug}-{id}' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    await waitFor(() => expect(write).toHaveBeenLastCalledWith('devpit/{slug}-{id}'))
    expect(await screen.findByText('devpit/fix-the-login-redirect-01hzxy9k')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Reset' })).toBeTruthy()
  })

  it('keeps what was typed and says why when it is refused', async () => {
    render(<WorktreeBranch />)
    const field = await screen.findByRole('textbox', { name: 'Branch name' })
    fireEvent.change(field, { target: { value: 'work' } })
    fireEvent.keyDown(field, { key: 'Enter' })
    expect(await screen.findByText(/the name needs/)).toBeTruthy()
    expect((field as HTMLInputElement).value).toBe('work')
  })
})
