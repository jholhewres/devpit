import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Heartbeat } from '../gen/bindings'

const saved = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    assistantHeartbeat: async (): Promise<Heartbeat> => ({ minutes: 15, waitingMinutes: 10, ci: true, waiting: true, blocked: true }),
    assistantHeartbeatSet: async (heartbeat: Heartbeat) => (saved(heartbeat), heartbeat),
  },
}))

const { AssistantSettings } = await import('./AssistantSettings')

afterEach(() => {
  cleanup()
  saved.mockClear()
})

describe('Settings → Assistant', () => {
  it('sets how often the heartbeat looks and what needs the person', async () => {
    render(<AssistantSettings />)
    fireEvent.change(await screen.findByLabelText('Heartbeat every'), { target: { value: '30' } })
    await waitFor(() => expect(saved).toHaveBeenLastCalledWith(expect.objectContaining({ minutes: 30 })))
    fireEvent.click(screen.getByLabelText('CI fails on an open pull request'))
    await waitFor(() => expect(saved).toHaveBeenLastCalledWith(expect.objectContaining({ ci: false, minutes: 30 })))
    fireEvent.change(screen.getByLabelText('Stopped for more than'), { target: { value: '30' } })
    await waitFor(() => expect(saved).toHaveBeenLastCalledWith(expect.objectContaining({ waitingMinutes: 30 })))
  })
})
