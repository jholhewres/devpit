import { renderHook } from '@testing-library/react'
import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { shouldPoll, whileWatched } from './whileWatched'

/* The window hidden is the case this exists for: two timers asked the kernel
   things every two and three seconds for a minimised devpit. */
describe('asking on a timer, while somebody is looking', () => {
  afterEach(() => {
    vi.useRealTimers()
    hide(false)
  })

  const hide = (hidden: boolean): void => {
    Object.defineProperty(document, 'hidden', { value: hidden, configurable: true })
  }

  it('answers for each reason a timer has to stay quiet', () => {
    expect(shouldPoll(false, 'prj_1')).toBe(true)
    expect(shouldPoll(true, 'prj_1')).toBe(false)
    expect(shouldPoll(false, null)).toBe(false)
    expect(shouldPoll(false, 'prj_1', false)).toBe(false)
  })

  it('asks on the clock while the window is on screen', () => {
    vi.useFakeTimers()
    const look = vi.fn()
    renderHook(() => whileWatched(look, 2000, 'prj_1'))

    expect(look).toHaveBeenCalledTimes(1)
    act(() => void vi.advanceTimersByTime(6000))
    expect(look).toHaveBeenCalledTimes(4)
  })

  it('asks nothing at all while the window is hidden', () => {
    vi.useFakeTimers()
    hide(true)
    const look = vi.fn()
    renderHook(() => whileWatched(look, 2000, 'prj_1'))

    act(() => void vi.advanceTimersByTime(10_000))
    expect(look).not.toHaveBeenCalled()
  })

  /* Coming back has to ask at once: a window restored to what was true a
     minute ago is the staleness the timer exists to avoid. */
  it('asks immediately when the window comes back, not after the interval', () => {
    vi.useFakeTimers()
    hide(true)
    const look = vi.fn()
    renderHook(() => whileWatched(look, 2000, 'prj_1'))
    expect(look).not.toHaveBeenCalled()

    hide(false)
    act(() => document.dispatchEvent(new Event('visibilitychange')))
    expect(look).toHaveBeenCalledTimes(1)

    act(() => void vi.advanceTimersByTime(2000))
    expect(look).toHaveBeenCalledTimes(2)
  })

  it('stops when it is taken off the screen', () => {
    vi.useFakeTimers()
    const look = vi.fn()
    const { unmount } = renderHook(() => whileWatched(look, 2000, 'prj_1'))
    unmount()

    act(() => void vi.advanceTimersByTime(10_000))
    expect(look).toHaveBeenCalledTimes(1)
  })
})
