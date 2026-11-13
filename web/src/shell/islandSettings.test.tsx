import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { IslandDrawn } from '../gen/bindings'
import { IslandSettings } from './IslandSettings'

let drawn: IslandDrawn = 'plain'
let anyway = false
const set = vi.fn(async (on: boolean) => ((anyway = on), { drawn, anyway }))
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    islandDrawn: async () => ({ drawn, anyway }),
    islandAnywaySet: (on: boolean) => set(on),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  drawn = 'plain'
  anyway = false
})

describe('the island where it would be an ordinary window', () => {
  it('stays off until it is shown anyway', async () => {
    render(<IslandSettings on onFlip={() => undefined} />)
    const shown = await screen.findByRole('switch', { name: 'Show the island anyway' })
    expect(shown.getAttribute('aria-checked')).toBe('false')
    fireEvent.click(shown)
    await waitFor(() => expect(set).toHaveBeenCalledWith(true))
    await waitFor(() => expect(shown.getAttribute('aria-checked')).toBe('true'))
  })

  it('is not offered where the island never takes focus', async () => {
    drawn = 'above'
    render(<IslandSettings on onFlip={() => undefined} />)
    await screen.findByRole('switch', { name: 'Island' })
    await waitFor(() => expect(screen.queryByRole('switch', { name: 'Show the island anyway' })).toBeNull())
  })

  it('is not offered while the island is switched off', async () => {
    render(<IslandSettings on={false} onFlip={() => undefined} />)
    await screen.findByRole('switch', { name: 'Island' })
    await waitFor(() => expect(screen.queryByRole('switch', { name: 'Show the island anyway' })).toBeNull())
  })
})
