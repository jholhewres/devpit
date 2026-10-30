import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { AccountQuota, Accounts } from '../gen/bindings'
import { accountOf, elsewhere, withDefault, without } from './accounts'

const saved = vi.fn()
let kept: Accounts
let quotas: AccountQuota[]

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    accountsRead: () => kept,
    accountsSave: (next: Accounts) => (saved(next), (kept = next)),
    accountsQuota: () => ({ accounts: quotas }),
  },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'prj_a', name: 'api' } }) }))

const { AccountsSettings } = await import('./AccountsSettings')
const { QuotaBanner } = await import('./QuotaBanner')

const quota = (name: string, fiveHour: number, over: Partial<AccountQuota> = {}): AccountQuota => ({
  name,
  configDir: `/h/.cfg-${name}`,
  state: 'read',
  windows: [{ label: '5-hour', percent: fiveHour, resetsAt: null }],
  nearlyOut: fiveHour > 90,
  readAt: 1,
  ...over,
})

const two: Accounts = {
  accounts: [
    { name: 'work', configDir: '/h/.cfg-work' },
    { name: 'home', configDir: '/h/.cfg-home' },
  ],
  projects: [{ projectId: 'prj_a', account: 'work' }],
}

afterEach(cleanup)
beforeEach(() => {
  saved.mockReset()
  kept = two
  quotas = [quota('work', 95), quota('home', 20)]
})

describe('the account rules', () => {
  it('reads, sets and clears a project default, and removing an account drops its defaults', () => {
    expect(accountOf(two, 'prj_a')).toBe('work')
    expect(accountOf(two, 'prj_b')).toBeNull()
    expect(accountOf(withDefault(two, 'prj_a', 'home'), 'prj_a')).toBe('home')
    expect(accountOf(withDefault(two, 'prj_a', null), 'prj_a')).toBeNull()
    expect(without(two, 'work').projects).toEqual([])
  })

  it('suggests the account with the most of its 5-hour window left, never one nearly out or unread', () => {
    const all = [quota('work', 95), quota('a', 60), quota('b', 10), quota('c', 92), quota('d', 0, { state: 'unavailable', windows: [] })]
    expect(elsewhere(all, 'work')?.name).toBe('b')
    expect(elsewhere([quota('work', 95), quota('c', 99)], 'work')).toBeNull()
  })
})

describe('Settings → Accounts', () => {
  it("shows each account's quota, or says why there is none", async () => {
    quotas = [quota('work', 42), quota('home', 0, { state: 'expired', windows: [], nearlyOut: false })]
    render(<AccountsSettings />)
    expect(await screen.findByText('42%')).toBeTruthy()
    expect(await screen.findByText('open a session in this account')).toBeTruthy()
    quotas = [quota('work', 0, { state: 'unavailable', windows: [], nearlyOut: false })]
    cleanup()
    render(<AccountsSettings />)
    expect(await screen.findByText('unavailable')).toBeTruthy()
  })

  it('adds an account and picks the one this project opens in', async () => {
    render(<AccountsSettings />)
    await screen.findByText('/h/.cfg-work')
    fireEvent.change(screen.getByLabelText('Account name'), { target: { value: 'third' } })
    fireEvent.change(screen.getByLabelText('Account config directory'), { target: { value: '~/.cfg-3' } })
    fireEvent.click(screen.getByRole('button', { name: 'Add' }))
    await waitFor(() => expect(saved).toHaveBeenCalled())
    expect(saved.mock.calls[0][0].accounts.at(-1)).toEqual({ name: 'third', configDir: '~/.cfg-3' })
    fireEvent.change(screen.getByLabelText('Account for api'), { target: { value: 'home' } })
    await waitFor(() => expect(accountOf(saved.mock.calls.at(-1)[0], 'prj_a')).toBe('home'))
  })
})

describe('the quota warning', () => {
  it('says when the project account is past 90%, and one click opens the next session elsewhere', async () => {
    render(<QuotaBanner />)
    fireEvent.click(await screen.findByRole('button', { name: 'Next session in home' }))
    await waitFor(() => expect(accountOf(saved.mock.calls[0][0], 'prj_a')).toBe('home'))
  })

  it('says nothing while the account has room', async () => {
    quotas = [quota('work', 50), quota('home', 20)]
    render(<QuotaBanner />)
    await new Promise((done) => setTimeout(done, 20))
    expect(screen.queryByRole('alert')).toBeNull()
  })
})
