import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { SyncStatus } from '../gen/bindings'

let status: SyncStatus
const calls = vi.fn()

const OFFER = 'devpit-sync:host_01ABC.aa.bb.cc'
const CODE = 'ABCDE-FGHJK-LMNPQ'

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    syncRead: async () => status,
    syncSet: async (on: boolean) => {
      calls('set', on)
      status = { ...status, on }
      return status
    },
    syncNow: async () => (calls('now'), status),
    syncJoin: async (given: string) => {
      calls('join', given)
      status = { ...status, hasKey: true }
      return status
    },
    syncOffer: async () => {
      calls('offer')
      status = { ...status, offering: { text: OFFER, expiresAt: Date.now() / 1000 + 120 } }
      return status
    },
    syncOfferWithdraw: async () => {
      calls('withdraw')
      status = { ...status, offering: null }
      return status
    },
    syncRecovery: async () => (calls('recovery'), { code: CODE }),
    syncStartOver: async () => (calls('start over'), status),
  },
}))

import { SyncSettings } from './SyncSettings'

const machines = [
  { id: 'm_a', name: 'laptop', seenAt: null, thisOne: true },
  { id: 'm_b', name: 'desktop', seenAt: Date.now() / 1000 - 7200, thisOne: false },
]

beforeEach(() => {
  status = { signedIn: true, on: false, hasKey: false, syncedAt: null, problem: null, machines: [], offering: null }
})

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
})

describe('sync between machines in Settings → Account', () => {
  it('is off until it is turned on, and turns on from its switch', async () => {
    render(<SyncSettings />)
    const toggle = (await screen.findByRole('switch', { name: 'Sync between machines' })) as HTMLButtonElement
    expect(toggle.getAttribute('aria-checked')).toBe('false')
    fireEvent.click(toggle)
    await waitFor(() => expect(calls).toHaveBeenCalledWith('set', true))
  })

  it('cannot be turned on before signing in', async () => {
    status = { ...status, signedIn: false }
    render(<SyncSettings />)
    const toggle = (await screen.findByRole('switch', { name: 'Sync between machines' })) as HTMLButtonElement
    expect(toggle.disabled).toBe(true)
  })

  it('joins an account that already syncs with what is pasted', async () => {
    status = { ...status, on: true }
    render(<SyncSettings />)
    const field = await screen.findByRole('textbox', { name: 'Offer or recovery code' })
    fireEvent.change(field, { target: { value: OFFER } })
    fireEvent.click(screen.getByRole('button', { name: 'Join' }))
    await waitFor(() => expect(calls).toHaveBeenCalledWith('join', OFFER))
    await screen.findByRole('button', { name: 'Pair another computer' })
  })

  it('lists the computers that sync and pairs another with an offer it can withdraw', async () => {
    status = { ...status, on: true, hasKey: true, syncedAt: Date.now() / 1000, machines }
    render(<SyncSettings />)
    await screen.findByText('laptop — this computer')
    screen.getByText(/desktop — seen 2h ago/)
    fireEvent.click(screen.getByRole('button', { name: 'Pair another computer' }))
    await screen.findByText(OFFER)
    fireEvent.click(screen.getByRole('button', { name: 'Withdraw' }))
    await waitFor(() => expect(screen.queryByText(OFFER)).toBeNull())
    expect(calls).toHaveBeenCalledWith('withdraw')
  })

  it('shows the recovery code only when asked, and hides it again', async () => {
    status = { ...status, on: true, hasKey: true }
    render(<SyncSettings />)
    expect(screen.queryByText(CODE)).toBeNull()
    fireEvent.click(await screen.findByRole('button', { name: 'Show the recovery code' }))
    await screen.findByText(CODE)
    fireEvent.click(screen.getByRole('button', { name: 'Hide' }))
    await waitFor(() => expect(screen.queryByText(CODE)).toBeNull())
  })

  it('says why the last round stopped', async () => {
    status = { ...status, on: true, hasKey: true, problem: 'devpit.app did not answer' }
    render(<SyncSettings />)
    await screen.findByText('devpit.app did not answer')
  })
})
