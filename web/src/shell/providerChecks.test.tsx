import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ProviderChecks } from './ProviderChecks'
import { useCheckEvery } from './checkEvery'

let minutes = 5
const write = vi.fn(async (next: number) => ({ minutes: (minutes = next) }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    providersCheckRead: async () => ({ minutes }),
    providersCheckWrite: (next: number) => write(next),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  minutes = 5
})

describe('how often sign-ins and MCP servers are checked', () => {
  it('is every five minutes, as before, until another time is chosen', async () => {
    const chips = renderHook(() => useCheckEvery())
    expect(chips.result.current).toBe(5 * 60 * 1000)
    render(<ProviderChecks />)
    const every = (await screen.findByLabelText('Check sign-ins and MCP servers every')) as HTMLSelectElement
    await waitFor(() => expect(every.value).toBe('5'))
    await act(async () => {
      fireEvent.change(every, { target: { value: '15' } })
    })
    await waitFor(() => expect(write).toHaveBeenLastCalledWith(15))
    await waitFor(() => expect(chips.result.current).toBe(15 * 60 * 1000))
  })
})
