import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { COPIED_MS, useCopied } from './useCopied'

/*
 * "Copied" went back to "Copy" on a timer nothing cleared. Fired after the
 * component and then the test's window were gone, it was an unhandled error
 * in a run whose tests all passed.
 */

let answer: (() => void) | null = null
const writeText = vi.fn(() => new Promise<void>((resolve) => (answer = resolve)))

beforeEach(() => {
  vi.useFakeTimers()
  Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true })
})
afterEach(() => {
  cleanup()
  vi.useRealTimers()
  answer = null
})

const copied = async (): Promise<void> => {
  await act(async () => {
    answer?.()
    await Promise.resolve()
  })
}

describe('a copy', () => {
  it('says so for a moment', async () => {
    const { result } = renderHook(() => useCopied())
    act(() => result.current[1]('text'))
    await copied()
    expect(writeText).toHaveBeenCalledWith('text')
    expect(result.current[0]).toBe(true)

    act(() => vi.advanceTimersByTime(COPIED_MS))
    expect(result.current[0]).toBe(false)
  })

  it('leaves no timer behind its component', async () => {
    const { result, unmount } = renderHook(() => useCopied())
    act(() => result.current[1]('text'))
    await copied()
    unmount()
    expect(vi.getTimerCount()).toBe(0)
  })

  it('starts no timer when the clipboard answers after its component is gone', async () => {
    const { result, unmount } = renderHook(() => useCopied())
    act(() => result.current[1]('text'))
    unmount()
    await copied()
    expect(vi.getTimerCount()).toBe(0)
  })
})
