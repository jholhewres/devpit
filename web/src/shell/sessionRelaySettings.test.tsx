import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { SessionRelay } from '../gen/bindings'
import { SessionRelaySettings } from './SessionRelaySettings'

let stored: SessionRelay = { nextCard: false, handoffAtPercent: null }
const write = vi.fn(async (relay: SessionRelay) => (stored = relay))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    sessionRelayRead: async () => stored,
    sessionRelayWrite: (relay: SessionRelay) => write(relay),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  stored = { nextCard: false, handoffAtPercent: null }
})

describe('sessions devpit starts on its own', () => {
  it('turns the next card on and off', async () => {
    render(<SessionRelaySettings />)
    const toggles = await screen.findAllByRole('switch')
    fireEvent.click(toggles[0]!)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith({ nextCard: true, handoffAtPercent: null }))
    fireEvent.click(toggles[0]!)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith({ nextCard: false, handoffAtPercent: null }))
  })

  it('turns the handoff on at seventy percent, takes another share, and off again', async () => {
    render(<SessionRelaySettings />)
    const toggles = await screen.findAllByRole('switch')
    fireEvent.click(toggles[1]!)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith({ nextCard: false, handoffAtPercent: 70 }))
    const share = await screen.findByLabelText('Hand off at this percent of the context')
    fireEvent.change(share, { target: { value: '55' } })
    fireEvent.blur(share)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith({ nextCard: false, handoffAtPercent: 55 }))
    fireEvent.click((await screen.findAllByRole('switch'))[1]!)
    await waitFor(() => expect(write).toHaveBeenLastCalledWith({ nextCard: false, handoffAtPercent: null }))
  })
})
