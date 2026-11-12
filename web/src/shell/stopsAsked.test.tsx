import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { LiveSession, StopAsked } from '../gen/bindings'
import { StopsAsked } from './StopsAsked'

let asked: StopAsked[] = []
const confirmed = vi.fn(async (..._args: unknown[]) => ({ stopped: [] as string[], failed: [] as string[] }))
const dropped = vi.fn(async (..._args: unknown[]) => undefined)
vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => ({ data: await call(), error: null, loading: false }),
  commands: {
    orchestratorStopsAsked: async () => ({ stops: asked }),
    orchestratorStopsConfirm: (...args: unknown[]) => confirmed(...args),
    orchestratorStopsDrop: (...args: unknown[]) => dropped(...args),
  },
}))

afterEach(() => {
  cleanup()
  vi.clearAllMocks()
  asked = []
})

const live = (name: string): LiveSession => ({ name, pid: 1, status: 'idle', projectName: 'api' }) as LiveSession
const stop = (name: string, losing: string | null = null): StopAsked => ({ name, pid: null, losing })

describe('the stops the orchestrator asked for', () => {
  it('lists every session and stops them all on one click', async () => {
    asked = [stop('a1'), stop('a2', 'its folder has 3 file(s) not committed'), stop('a3')]
    const onDone = vi.fn()
    render(<StopsAsked profileId="claude" sessions={[live('a1'), live('a2'), live('a3')]} onDone={onDone} />)
    await screen.findByText('a1')
    expect(screen.getByText('a2')).toBeTruthy()
    expect(screen.getByText('a3')).toBeTruthy()
    expect(screen.getByText(/3 file\(s\) not committed/)).toBeTruthy()
    expect(confirmed).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Stop all 3' }))
    await waitFor(() => expect(confirmed).toHaveBeenCalledTimes(1))
    expect(confirmed).toHaveBeenCalledWith('claude', ['a1', 'a2', 'a3'])
    await waitFor(() => expect(onDone).toHaveBeenCalled())
  })

  it('lists only the sessions still running', async () => {
    asked = [stop('a1'), stop('gone')]
    render(<StopsAsked profileId="claude" sessions={[live('a1')]} onDone={vi.fn()} />)
    await screen.findByText('a1')
    expect(screen.queryByText('gone')).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Stop it' }))
    await waitFor(() => expect(confirmed).toHaveBeenCalledWith('claude', ['a1']))
  })

  it('keeps them running without stopping anything', async () => {
    asked = [stop('a1')]
    render(<StopsAsked profileId="claude" sessions={[live('a1')]} onDone={vi.fn()} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Keep running' }))
    await waitFor(() => expect(dropped).toHaveBeenCalledWith('claude'))
    expect(confirmed).not.toHaveBeenCalled()
  })

  it('shows nothing when nothing was asked', async () => {
    const { container } = render(<StopsAsked profileId="claude" sessions={[live('a1')]} onDone={vi.fn()} />)
    await waitFor(() => expect(container.innerHTML).toBe(''))
  })
})
