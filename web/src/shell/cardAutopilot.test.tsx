import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { DevflowAutopilot } from '../gen/bindings'

vi.mock('./window', () => ({ onCarried: () => () => {} }))
vi.mock('./useShell', () => ({ useShell: () => ({ project: { id: 'prj_api' } }) }))

const idle: DevflowAutopilot = { projectId: 'prj_api', epicId: null, running: false, said: null, order: [] }
let now = idle
const called = vi.fn()
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    devflowAutopilot: async () => now,
    devflowAutopilotStart: async (...args: unknown[]) => {
      called('start', ...args)
      now = { ...idle, epicId: 'epic', running: true, said: 'started "Index the bodies"', order: ['Index the bodies', 'List what is found'] }
      return now
    },
    devflowAutopilotStop: async (...args: unknown[]) => {
      called('stop', ...args)
      now = { ...now, running: false, said: 'stopped by you' }
      return now
    },
  },
}))

const { CardAutopilot } = await import('./CardAutopilot')

afterEach(() => {
  cleanup()
  called.mockReset()
  now = idle
})

describe("an epic's autopilot", () => {
  it('starts only on the click, says what it did and the order, and stops on another', async () => {
    render(<CardAutopilot epicId="epic" cards={2} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Start autopilot' }))
    await waitFor(() => expect(called).toHaveBeenCalledWith('start', 'prj_api', 'epic'))
    expect(await screen.findByText('started "Index the bodies"')).toBeTruthy()
    expect(screen.getByText('In order: Index the bodies → List what is found')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Stop autopilot' }))
    expect(await screen.findByText('stopped by you')).toBeTruthy()
  })

  it('is absent from a card that is not an epic with cards', () => {
    const { container } = render(<CardAutopilot epicId="lonely" cards={0} />)
    expect(container.innerHTML).toBe('')
  })
})
