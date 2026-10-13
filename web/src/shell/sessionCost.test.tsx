import { act, cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { SessionCost as Cost } from '../gen/bindings'
import { SessionCost } from './SessionCost'

/*
 * Every terminal pane of every project stays mounted, each with its cost in
 * the corner, and each asked on a timer of its own: a read per pane every
 * three seconds, hidden window or not.
 */

const COST: Cost = {
  costUsd: 1.234,
  lastTurnUsd: 0.2,
  sinceSeenUsd: 0.5,
  tokens: { input: 1200, output: 3400, cacheRead: 0, cacheWrite: 0 },
  model: 'claude-sonnet-4-5',
  unpricedTokens: 0,
}

const asked = vi.fn((_sessionId: string): Promise<Cost> => Promise.resolve(COST))

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null }),
  commands: { sessionCost: (sessionId: string) => asked(sessionId) },
}))

const hide = (hidden: boolean): void => {
  Object.defineProperty(document, 'hidden', { value: hidden, configurable: true })
}

const tick = async (ms: number): Promise<void> => {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })
}

const reads = (sessionId: string): number => asked.mock.calls.filter(([id]) => id === sessionId).length

describe('what the sessions on screen have spent', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    asked.mockClear()
  })
  afterEach(() => {
    cleanup()
    vi.useRealTimers()
    hide(false)
  })

  it('is read once a tick per session, however many widgets show it', async () => {
    render(
      <>
        <SessionCost sessionId="s1" />
        <SessionCost sessionId="s1" />
        <SessionCost sessionId="s1" />
        <SessionCost sessionId="s2" />
      </>,
    )
    await tick(0)
    expect(reads('s1')).toBe(1)
    expect(reads('s2')).toBe(1)

    await tick(3000 * 3)
    expect(reads('s1')).toBe(4)
    expect(reads('s2')).toBe(4)
  })

  it('is not read while the window is hidden, and is read at once when it shows', async () => {
    render(<SessionCost sessionId="s1" />)
    await tick(0)
    hide(true)
    await tick(3000 * 5)
    expect(reads('s1')).toBe(1)

    hide(false)
    await act(async () => void document.dispatchEvent(new Event('visibilitychange')))
    expect(reads('s1')).toBe(2)
  })

  it('is not asked again while the last answer is still out', async () => {
    let answer = (): void => {}
    asked.mockImplementation(() => new Promise<Cost>((resolve) => (answer = () => resolve(COST))))
    render(<SessionCost sessionId="s1" />)
    await tick(3000 * 5)
    expect(reads('s1')).toBe(1)

    await act(async () => answer())
    await tick(3000)
    expect(reads('s1')).toBe(2)
    asked.mockImplementation(() => Promise.resolve(COST))
  })

  it('stops being read once nothing shows it', async () => {
    const { unmount } = render(<SessionCost sessionId="s1" />)
    await tick(0)
    unmount()
    await tick(3000 * 5)
    expect(reads('s1')).toBe(1)
  })
})
