import { act, cleanup, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { KnownAgent } from '../gen/bindings'
import { profilesChanged } from './profiles'

let known: KnownAgent[] = []

vi.mock('./live', () => ({
  ask: async <T,>(call: () => T) => ({ data: call(), error: null, loading: false }),
  commands: { agentsKnown: () => known },
}))

const agent = (id: string, enabled: boolean): KnownAgent => ({
  id,
  label: id,
  launch: id,
  installed: true,
  enabled,
  homepage: '',
})

afterEach(cleanup)

describe('the agent list', () => {
  it('asks again when a profile or a switch changes in Settings', async () => {
    // A launcher already open when an agent was switched off would otherwise
    // keep offering it until it was closed and opened again.
    const { useKnownAgents } = await import('./useKnownAgents')
    known = [agent('claude', true)]
    const { result } = renderHook(() => useKnownAgents())
    await waitFor(() => expect(result.current[0]?.enabled).toBe(true))

    known = [agent('claude', false)]
    act(() => profilesChanged())
    await waitFor(() => expect(result.current[0]?.enabled).toBe(false))
  })
})
