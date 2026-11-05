import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { DRAFT_WAIT, keepFiles, keptDraft, keptFiles, useDraft } from './useDraft'

afterEach(() => {
  cleanup()
  localStorage.clear()
  vi.useRealTimers()
})

describe('useDraft', () => {
  it('keeps unsent text for the next time the chat opens', () => {
    const first = renderHook(() => useDraft('conv_1', undefined))
    act(() => first.result.current[1]('half a thought'))
    first.unmount()

    const again = renderHook(() => useDraft('conv_1', undefined))
    expect(again.result.current[0]).toBe('half a thought')
  })

  it('writes a beat after the last key, not on every one', () => {
    vi.useFakeTimers()
    const { result } = renderHook(() => useDraft('conv_1', undefined))
    act(() => result.current[1]('a'))
    act(() => result.current[1]('ab'))
    expect(keptDraft('conv_1')).toBeNull()
    act(() => vi.advanceTimersByTime(DRAFT_WAIT))
    expect(keptDraft('conv_1')).toBe('ab')
  })

  it('forgets it once the composer is empty again', () => {
    vi.useFakeTimers()
    const { result } = renderHook(() => useDraft('conv_1', undefined))
    act(() => result.current[1]('sent soon'))
    act(() => vi.advanceTimersByTime(DRAFT_WAIT))
    act(() => result.current[1](''))
    act(() => vi.advanceTimersByTime(DRAFT_WAIT))
    expect(keptDraft('conv_1')).toBeNull()
  })

  it('lets a card’s draft win over an old one', () => {
    localStorage.setItem('devpit.chatDraft.conv_1', 'old')
    const { result } = renderHook(() => useDraft('conv_1', 'from the card'))
    expect(result.current[0]).toBe('from the card')
  })

  it('keeps the files attached to the draft, and only files', () => {
    const file = { name: 'a.png', path: '/p/a.png', kind: 'png' }
    keepFiles('conv_1', [file])
    expect(keptFiles('conv_1')).toEqual([file])
    localStorage.setItem('devpit.chatDraftFiles.conv_1', JSON.stringify([file, { name: 1 }, 'x']))
    expect(keptFiles('conv_1')).toEqual([file])
    keepFiles('conv_1', [])
    expect(keptFiles('conv_1')).toEqual([])
  })
})
