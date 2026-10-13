import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Board } from '../gen/bindings'
import { coalesced, GATHER_MS, useBoard } from './useBoard'

/*
 * An agent writes to the board many times a turn, and each `board:changed`
 * was a read of every card on it — two hundred of them, behind whatever tab
 * was in front.
 */

const handlers: Record<string, (payload: unknown) => void> = {}
const boardGets = vi.fn()

const board = (): Board => ({
  projectId: 'p1',
  columns: [{ id: 'col_1', name: 'Todo', position: 0, step: null, onPass: null, autonomy: 'manual', role: null, roleChosen: false }],
  cards: [],
  steps: [],
})

vi.mock('./live', () => ({
  ask: async (call: () => unknown) => ({ data: await call(), error: null, loading: false }),
  commands: {
    boardGet: () => {
      boardGets()
      return board()
    },
  },
}))
vi.mock('./window', () => ({
  onCarried: (name: string, then: (payload: unknown) => void) => {
    handlers[name] = then
    return () => undefined
  },
}))

const hide = (hidden: boolean): void => {
  Object.defineProperty(document, 'hidden', { value: hidden, configurable: true })
}

/* Lets the timers run and every answer they started land. */
const wait = async (ms: number): Promise<void> => {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms)
  })
}

describe('reads asked for in a burst', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('are one read, once the burst has had time to arrive', async () => {
    const pull = vi.fn(() => Promise.resolve())
    const gathered = coalesced(pull)
    for (let n = 0; n < 10; n += 1) gathered.want()
    expect(pull).not.toHaveBeenCalled()

    await vi.advanceTimersByTimeAsync(GATHER_MS)
    expect(pull).toHaveBeenCalledTimes(1)
  })

  /* A steady stream that pushed the read back on every ask would never read. */
  it('still read on a steady stream of asks', async () => {
    const pull = vi.fn(() => Promise.resolve())
    const gathered = coalesced(pull)
    for (let n = 0; n < 10; n += 1) {
      gathered.want()
      await vi.advanceTimersByTimeAsync(GATHER_MS / 2)
    }
    expect(pull).toHaveBeenCalledTimes(5)
  })

  it('never send a second read while one is out, and read once more after it', async () => {
    let land = (): void => {}
    const pull = vi.fn(() => new Promise<void>((resolve) => (land = resolve)))
    const gathered = coalesced(pull)
    gathered.want()
    await vi.advanceTimersByTimeAsync(GATHER_MS)
    for (let n = 0; n < 3; n += 1) {
      gathered.want()
      await vi.advanceTimersByTimeAsync(GATHER_MS)
    }
    expect(pull).toHaveBeenCalledTimes(1)

    land()
    await vi.advanceTimersByTimeAsync(0)
    expect(pull).toHaveBeenCalledTimes(2)
    land()
    await vi.advanceTimersByTimeAsync(GATHER_MS)
    expect(pull).toHaveBeenCalledTimes(2)
  })

  it('read nothing once stopped', async () => {
    const pull = vi.fn(() => Promise.resolve())
    const gathered = coalesced(pull)
    gathered.want()
    gathered.stop()
    await vi.advanceTimersByTimeAsync(GATHER_MS * 2)
    expect(pull).not.toHaveBeenCalled()
  })
})

describe("the board hearing it was changed", () => {
  beforeEach(() => {
    vi.useFakeTimers()
    boardGets.mockClear()
  })
  afterEach(() => {
    cleanup()
    vi.useRealTimers()
    hide(false)
  })

  it('reads itself once for a burst of changes', async () => {
    renderHook(() => useBoard('p1', true))
    await wait(0)
    expect(boardGets).toHaveBeenCalledTimes(1)

    act(() => {
      for (let n = 0; n < 8; n += 1) handlers['board:changed']!('p1')
    })
    await wait(GATHER_MS)
    expect(boardGets).toHaveBeenCalledTimes(2)
  })

  it('reads nothing behind another tab, and catches up when it is shown', async () => {
    const { rerender } = renderHook(({ inFront }) => useBoard('p1', inFront), { initialProps: { inFront: false } })
    await wait(0)
    expect(boardGets).toHaveBeenCalledTimes(1)

    act(() => handlers['board:changed']!('p1'))
    await wait(GATHER_MS * 4)
    expect(boardGets).toHaveBeenCalledTimes(1)

    rerender({ inFront: true })
    await wait(0)
    expect(boardGets).toHaveBeenCalledTimes(2)
  })

  it('owes nothing when nothing changed while it was away', async () => {
    const { rerender } = renderHook(({ inFront }) => useBoard('p1', inFront), { initialProps: { inFront: false } })
    await wait(0)
    rerender({ inFront: true })
    await wait(GATHER_MS)
    expect(boardGets).toHaveBeenCalledTimes(1)
  })

  it('reads nothing in a hidden window, and catches up when it comes back', async () => {
    renderHook(() => useBoard('p1', true))
    await wait(0)
    hide(true)
    act(() => handlers['board:changed']!('p1'))
    await wait(GATHER_MS * 4)
    expect(boardGets).toHaveBeenCalledTimes(1)

    hide(false)
    act(() => document.dispatchEvent(new Event('visibilitychange')))
    await wait(0)
    expect(boardGets).toHaveBeenCalledTimes(2)
  })
})
