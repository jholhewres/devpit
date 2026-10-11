import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { AccountSignIn, SignIns } from '../gen/bindings'

const openPrefs = vi.fn()
let now: SignIns = { accounts: [], needed: [] }

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: { signIns: async () => now },
}))
vi.mock('./useShell', () => ({ useShell: () => ({ openPrefs }) }))

const { SignInChip, tallied } = await import('./SignInChip')

const account = (over: Partial<AccountSignIn>): AccountSignIn => ({
  label: 'Work',
  directory: '/home/me/.claude-work',
  state: 'saved',
  until: null,
  fix: "CLAUDE_CONFIG_DIR='/home/me/.claude-work' claude auth login",
  ...over,
})

afterEach(cleanup)

describe('the sign-ins in the footer', () => {
  it('counts what holds, and the Keychain is not held against', () => {
    expect(tallied({ accounts: [account({}), account({ state: 'unknown' })], needed: [] })).toEqual({ ok: 2, all: 2, bad: false })
    expect(
      tallied({ accounts: [account({ state: 'expired' })], needed: [{ what: 'AWS profile asc', holds: false, fix: 'aws sso login --profile asc' }] }),
    ).toEqual({ ok: 0, all: 2, bad: true })
  })

  it('reads as all signed in when every login is valid', async () => {
    now = { accounts: [account({}), account({ label: 'Claude Code', directory: '/home/me/.claude' })], needed: [] }
    render(<SignInChip projectId="p1" />)
    const chip = await screen.findByTitle('Sign-ins')
    expect(chip.textContent).toBe('Sign-ins2/2')
    expect(chip.getAttribute('data-state')).toBeNull()
    fireEvent.click(chip)
    expect(screen.getAllByText('signed in')).toHaveLength(2)
    expect(screen.queryByText(/claude auth login/)).toBeNull()
  })

  it('marks an expired login and says the command that signs it in again', async () => {
    now = {
      accounts: [account({ state: 'expired' }), account({ label: 'Claude Code', directory: '/home/me/.claude' })],
      needed: [{ what: 'AWS profile asc', holds: false, fix: 'aws sso login --profile asc' }],
    }
    render(<SignInChip projectId="p1" />)
    const chip = await screen.findByTitle('Sign-ins')
    expect(chip.textContent).toBe('Sign-ins1/3')
    expect(chip.getAttribute('data-state')).toBe('bad')
    fireEvent.click(chip)
    expect(screen.getAllByText('expired')).toHaveLength(2)
    expect(screen.getByText("CLAUDE_CONFIG_DIR='/home/me/.claude-work' claude auth login")).toBeTruthy()
    expect(screen.getByText('aws sso login --profile asc')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Accounts' }))
    expect(openPrefs).toHaveBeenCalledWith('providers')
  })
})
