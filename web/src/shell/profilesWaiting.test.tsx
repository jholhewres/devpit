import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { ProfileWaiting } from '../gen/bindings'
import { ProfilesWaiting } from './ProfilesWaiting'

afterEach(cleanup)

let waiting: ProfileWaiting[] = []
const settled = vi.fn()

vi.mock('./live', () => ({
  ask: (call: () => unknown) => Promise.resolve({ data: call(), error: null, loading: false }),
  commands: {
    agentProfilesWaiting: () => ({ profiles: waiting }),
    agentProfileSettle: (id: string, take: boolean) => (settled(id, take), { profiles: [] }),
  },
}))

describe('a profile from another computer', () => {
  it('says which values it needs here before it can start', async () => {
    waiting = [{ id: 'p1', label: 'GLM', arrived: false, needs: ['ANTHROPIC_AUTH_TOKEN'], changed: [] }]
    render(<ProfilesWaiting />)
    expect(await screen.findByText(/needs ANTHROPIC_AUTH_TOKEN here before it can start/)).toBeTruthy()
  })

  it('asks before a base URL changed elsewhere is used, and takes the answer', async () => {
    waiting = [
      {
        id: 'p1',
        label: 'GLM',
        arrived: false,
        needs: [],
        changed: [{ name: 'env.ANTHROPIC_BASE_URL', value: 'https://elsewhere.example/api' }],
      },
    ]
    render(<ProfilesWaiting />)
    expect(await screen.findByText('https://elsewhere.example/api')).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Keep this one' }))

    await waitFor(() => expect(settled).toHaveBeenCalledWith('p1', false))
    await waitFor(() => expect(screen.queryByRole('status')).toBeNull())
  })

  it('asks to take a profile that came from another computer before it runs', async () => {
    waiting = [{ id: 'p1', label: 'GLM', arrived: true, needs: [], changed: [{ name: 'command', value: 'claude' }] }]
    render(<ProfilesWaiting />)
    fireEvent.click(await screen.findByRole('button', { name: 'Take it' }))
    await waitFor(() => expect(settled).toHaveBeenCalledWith('p1', true))
  })

  it('shows nothing when no profile waits', async () => {
    waiting = []
    const { container } = render(<ProfilesWaiting />)
    await waitFor(() => expect(container.textContent).toBe(''))
  })
})
