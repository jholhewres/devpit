import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { useQueue } from './useQueue'

describe('messages typed while a turn runs', () => {
  it('waits for the turn, then sends one at a time, in order', () => {
    const say = vi.fn()
    const { result, rerender } = renderHook(({ sending }) => useQueue(sending, say), { initialProps: { sending: true } })
    act(() => {
      result.current.add('first')
      result.current.add('second')
    })
    expect(say).not.toHaveBeenCalled()
    rerender({ sending: false })
    expect(say).toHaveBeenCalledTimes(1)
    expect(say).toHaveBeenLastCalledWith('first')
    // The first one started a turn; the second waits for it.
    rerender({ sending: true })
    expect(result.current.waiting).toEqual(['second'])
    rerender({ sending: false })
    expect(say).toHaveBeenLastCalledWith('second')
  })

  it('hands everything back on a stop, and sends none of it', () => {
    const say = vi.fn()
    const { result, rerender } = renderHook(({ sending }) => useQueue(sending, say), { initialProps: { sending: true } })
    act(() => {
      result.current.add('a')
      result.current.add('b')
    })
    let back = ''
    act(() => {
      back = result.current.takeAll()
    })
    expect(back).toBe('a\n\nb')
    rerender({ sending: false })
    expect(say).not.toHaveBeenCalled()
  })

  it('sends one into the running turn now, and keeps it when the turn would not take it', async () => {
    const say = vi.fn()
    let takes = false
    const steer = vi.fn(async () => takes)
    const { result } = renderHook(() => useQueue(true, say, steer))
    act(() => {
      result.current.add('later')
      result.current.add('now')
    })
    await act(async () => result.current.now(1))
    expect(steer).toHaveBeenLastCalledWith('now')
    expect(result.current.waiting).toEqual(['later', 'now'])
    takes = true
    await act(async () => result.current.now(1))
    expect(result.current.waiting).toEqual(['later'])
    expect(say).not.toHaveBeenCalled()
  })
})
